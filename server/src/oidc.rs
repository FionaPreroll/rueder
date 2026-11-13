//! Logging in through an OpenID Connect provider, for servers that set one.
//!
//! The authorization code flow with PKCE: `GET /api/oidc/start` sends the
//! browser to the provider, the provider sends it back to
//! `GET /api/oidc/callback` with a code, and the server trades the code for an
//! ID token that says who logged in there. Who that is in rueder says
//! `account_identities`: nobody gets an account by logging in. An identity is
//! tied to an account on the command line, or by a reader who is logged in and
//! starts the flow with `?link` from a page of rueder itself.
//!
//! Starting is a `GET`, which the check of where changes come from lets pass
//! (see `hardening`), so `?link` checks for itself: a page of another site
//! could otherwise send a logged-in reader there, and tie to their account
//! whoever their browser is logged in as at the provider, which may be the
//! one who sent them. The flow keeps the account it was started for, and the
//! identity is tied to that one only.
//!
//! The ID token comes straight from the provider over TLS and is not passed
//! through the browser, so its signature is not checked a second time (OpenID
//! Connect Core 3.1.3.7 allows that). That is why the provider's addresses
//! must be https.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, bail, ensure};
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use reqwest::Url;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use time::OffsetDateTime;
use tokio::sync::Mutex;
use utoipa::IntoParams;

use crate::{
    AppState, accounts,
    http_client::{HttpClient, Request},
    sessions,
};

/// The cookie that holds one login on its way to the provider and back.
pub const COOKIE: &str = "__Host-rueder-oidc";
/// How long that way may take, in seconds.
const FLOW_AGE: u64 = 600;
/// How long what the provider says about itself is believed.
const PROVIDER_AGE: Duration = Duration::from_secs(3600);
/// The longest answer of the provider, and the longest ID token in it.
const MAX_ANSWER: usize = 256 * 1024;
const MAX_TOKEN: usize = 16 * 1024;
/// How far the provider's clock may be behind, in seconds.
const LEEWAY: i64 = 60;

/// The provider and what rueder is to it.
#[derive(Debug, Clone)]
pub struct Config {
    /// The provider's address, as it names itself.
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    /// The address of `/api/oidc/callback` as browsers reach it, which the
    /// provider has on its list.
    pub redirect_url: String,
    /// Take a provider without TLS. For tests only: nothing in the environment
    /// sets it.
    pub allow_http: bool,
}

impl Config {
    /// From `OIDC_ISSUER`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET` and
    /// `OIDC_REDIRECT_URL`. `None` without an issuer: then there is no such login.
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let var = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
        let Some(issuer) = var("OIDC_ISSUER") else {
            return Ok(None);
        };
        let need = |name: &str| var(name).with_context(|| format!("OIDC_ISSUER needs {name}"));
        Ok(Some(Self {
            issuer,
            client_id: need("OIDC_CLIENT_ID")?,
            client_secret: need("OIDC_CLIENT_SECRET")?,
            redirect_url: need("OIDC_REDIRECT_URL")?,
            allow_http: false,
        }))
    }
}

/// What the provider says about itself.
#[derive(Deserialize)]
struct Discovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    token_endpoint_auth_methods_supported: Option<Vec<String>>,
}

struct Provider {
    authorization_endpoint: Url,
    token_endpoint: Url,
    /// Whether the secret goes in the form, not in the `Authorization` header.
    secret_in_form: bool,
}

#[derive(Deserialize)]
struct TokenAnswer {
    id_token: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

#[derive(Deserialize)]
struct Claims {
    iss: String,
    sub: String,
    aud: Audience,
    azp: Option<String>,
    exp: i64,
    nonce: Option<String>,
}

/// Who logged in at the provider, and the account the flow was started to tie
/// that identity to, if it was.
#[derive(Debug, PartialEq, Eq)]
pub struct Identity {
    pub subject: String,
    pub link: Option<i32>,
}

pub struct Oidc {
    config: Config,
    client: HttpClient,
    provider: Mutex<Option<(Instant, Arc<Provider>)>>,
}

fn random() -> anyhow::Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|err| anyhow::anyhow!("no random numbers: {err}"))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

impl Oidc {
    /// The provider is reached through the hardened client, like everything
    /// else: one inside the own network needs `FETCH_ALLOW_NETS`.
    pub fn new(config: Config, client: HttpClient) -> anyhow::Result<Self> {
        let oidc = Self {
            config,
            client,
            provider: Mutex::new(None),
        };
        oidc.endpoint(&oidc.config.issuer)
            .context("OIDC_ISSUER is no address of a provider")?;
        let redirect = Url::parse(&oidc.config.redirect_url).context("OIDC_REDIRECT_URL")?;
        ensure!(
            matches!(redirect.scheme(), "http" | "https"),
            "OIDC_REDIRECT_URL is no web address"
        );
        Ok(oidc)
    }

    pub fn issuer(&self) -> &str {
        &self.config.issuer
    }

    /// An address of the provider, which has to be https.
    fn endpoint(&self, address: &str) -> anyhow::Result<Url> {
        let url = Url::parse(address).with_context(|| format!("{address:?} is no address"))?;
        let fine = url.scheme() == "https" || (self.config.allow_http && url.scheme() == "http");
        ensure!(fine, "the provider's address {address:?} is not https");
        Ok(url)
    }

    async fn provider(&self) -> anyhow::Result<Arc<Provider>> {
        let mut cached = self.provider.lock().await;
        if let Some((at, provider)) = cached.as_ref()
            && at.elapsed() < PROVIDER_AGE
        {
            return Ok(provider.clone());
        }
        let issuer = &self.config.issuer;
        let address = format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        );
        let request = Request {
            accept: Some("application/json"),
            max_body: Some(MAX_ANSWER),
            ..Request::new(&address)
        };
        let response = self.client.get(request).await?;
        ensure!(
            response.status == StatusCode::OK,
            "the provider answers with {}",
            response.status
        );
        let discovery: Discovery = serde_json::from_slice(&response.body)
            .context("the provider does not say where to log in")?;
        ensure!(
            discovery.issuer == *issuer,
            "the provider calls itself {:?}, not {issuer:?}",
            discovery.issuer
        );
        let methods = discovery.token_endpoint_auth_methods_supported;
        let provider = Arc::new(Provider {
            authorization_endpoint: self.endpoint(&discovery.authorization_endpoint)?,
            token_endpoint: self.endpoint(&discovery.token_endpoint)?,
            // The header is what a provider takes if it does not say.
            secret_in_form: methods.is_some_and(|methods| {
                !methods.iter().any(|method| method == "client_secret_basic")
                    && methods.iter().any(|method| method == "client_secret_post")
            }),
        });
        *cached = Some((Instant::now(), provider.clone()));
        Ok(provider)
    }

    /// Where to send the browser, and what it has to keep until it is back.
    /// `link` is the account the identity is to be tied to, if that is what
    /// the flow is for.
    pub async fn start(&self, link: Option<i32>) -> anyhow::Result<(Url, String)> {
        let provider = self.provider().await?;
        let (state, nonce, verifier) = (random()?, random()?, random()?);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let mut to = provider.authorization_endpoint.clone();
        to.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", &self.config.redirect_url)
            .append_pair("scope", "openid")
            .append_pair("state", &state)
            .append_pair("nonce", &nonce)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256");
        let link = match link {
            Some(account_id) => format!("link-{account_id}"),
            None => "login".into(),
        };
        Ok((to, format!("{state}.{nonce}.{verifier}.{link}")))
    }

    /// Trades the code for who logged in. `kept` is what [`Oidc::start`] gave
    /// the browser to keep, `state` what the provider sent back with the code.
    pub async fn finish(&self, kept: &str, code: &str, state: &str) -> anyhow::Result<Identity> {
        let mut parts = kept.split('.');
        let (Some(kept_state), Some(nonce), Some(verifier), Some(link), None) = (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
        ) else {
            bail!("the browser kept something else");
        };
        // Whoever sends a browser here with a code of their own does not know
        // what this browser kept.
        ensure!(
            !kept_state.is_empty() && kept_state == state,
            "the answer belongs to another login"
        );

        let provider = self.provider().await?;
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.config.redirect_url.as_str()),
            ("code_verifier", verifier),
        ];
        let escape = |text: &str| url::form_urlencoded::byte_serialize(text.as_bytes()).collect();
        let (id, secret): (String, String) = (
            escape(&self.config.client_id),
            escape(&self.config.client_secret),
        );
        let basic = format!("Basic {}", STANDARD.encode(format!("{id}:{secret}")));
        if provider.secret_in_form {
            form.push(("client_id", &self.config.client_id));
            form.push(("client_secret", &self.config.client_secret));
        }
        let request = Request {
            accept: Some("application/json"),
            max_body: Some(MAX_ANSWER),
            form: Some(&form),
            authorization: (!provider.secret_in_form).then_some(&basic),
            ..Request::new(provider.token_endpoint.as_str())
        };
        let response = self.client.get(request).await?;
        ensure!(
            response.status == StatusCode::OK,
            "the provider does not take the code: {}",
            response.status
        );
        let answer: TokenAnswer =
            serde_json::from_slice(&response.body).context("the provider sent no ID token")?;
        let subject = self.subject(&answer.id_token, nonce, OffsetDateTime::now_utc())?;
        let link = match link {
            "login" => None,
            link => {
                let account_id = link.strip_prefix("link-").and_then(|id| id.parse().ok());
                Some(account_id.context("the browser kept something else")?)
            }
        };
        Ok(Identity { subject, link })
    }

    /// Who an ID token is about, if it is one for this login.
    fn subject(&self, token: &str, nonce: &str, now: OffsetDateTime) -> anyhow::Result<String> {
        ensure!(token.len() <= MAX_TOKEN, "the ID token is too long");
        let mut parts = token.split('.');
        let (Some(_), Some(claims), Some(_), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            bail!("the ID token has not three parts");
        };
        let claims = URL_SAFE_NO_PAD
            .decode(claims.trim_end_matches('='))
            .context("the ID token cannot be read")?;
        let claims: Claims =
            serde_json::from_slice(&claims).context("the ID token cannot be read")?;

        ensure!(
            claims.iss == self.config.issuer,
            "the ID token is from someone else"
        );
        let id = &self.config.client_id;
        let for_us = match &claims.aud {
            Audience::One(audience) => audience == id,
            // Made for several, it has to say that it was asked for by us.
            Audience::Many(audiences) => {
                audiences.contains(id) && (audiences.len() == 1 || claims.azp.as_ref() == Some(id))
            }
        };
        ensure!(for_us, "the ID token is for someone else");
        ensure!(
            claims.azp.as_ref().is_none_or(|azp| azp == id),
            "the ID token is for someone else"
        );
        ensure!(
            claims.exp > now.unix_timestamp() - LEEWAY,
            "the ID token has run out"
        );
        ensure!(
            claims.nonce.as_deref() == Some(nonce),
            "the ID token belongs to another login"
        );
        let subject = claims.sub;
        ensure!(
            !subject.is_empty() && subject.len() <= 255,
            "the ID token names nobody"
        );
        Ok(subject)
    }
}

/// Ties an identity at a provider to the account of this name. `false` if
/// there is no such account, or the identity belongs to an account already.
pub async fn link(db: &PgPool, name: &str, issuer: &str, subject: &str) -> anyhow::Result<bool> {
    let linked = sqlx::query!(
        "INSERT INTO account_identities (issuer, subject, account_id)
         SELECT $2, $3, id FROM accounts WHERE lower(username) = lower($1)
         ON CONFLICT DO NOTHING",
        name.trim(),
        issuer,
        subject,
    )
    .execute(db)
    .await?;
    Ok(linked.rows_affected() == 1)
}

fn flow_cookie(value: &str, age: u64) -> String {
    format!("{COOKIE}={value}; Path=/; Max-Age={age}; HttpOnly; Secure; SameSite=Lax")
}

/// Back to the interface, which says how it went. The cookie of the flow goes.
fn back(outcome: Option<&str>) -> Response {
    let to = match outcome {
        Some(outcome) => format!("/?oidc={outcome}"),
        None => "/".into(),
    };
    (
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, to),
            (header::SET_COOKIE, flow_cookie("", 0)),
        ],
    )
        .into_response()
}

/// What `GET /api/oidc/start` reads.
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct Start {
    /// Present to link the provider's account to the one logged in, instead
    /// of logging in.
    link: Option<String>,
}

/// Whether a request was sent by a page of rueder itself, or by the reader
/// by hand, as browsers say in `Sec-Fetch-Site`. A browser too old to say is
/// taken for another site's.
fn from_own_page(headers: &HeaderMap) -> bool {
    let site = headers
        .get("sec-fetch-site")
        .and_then(|site| site.to_str().ok());
    matches!(site, Some("same-origin" | "none"))
}

/// The account a flow with `?link` is for: the one logged in, if the request
/// comes from rueder's own page.
async fn linking(state: &AppState, headers: &HeaderMap) -> anyhow::Result<Option<i32>> {
    if !from_own_page(headers) {
        return Ok(None);
    }
    let session = match sessions::token(headers) {
        Some(token) => sessions::find(&state.db, token).await?,
        None => None,
    };
    Ok(session.map(|session| session.account_id))
}

/// Starts logging in through the OpenID Connect provider: sends the browser
/// there. With `link`, from the logged-in reader's own page, the provider's
/// account is linked to this one instead.
#[utoipa::path(
    get,
    path = "/oidc/start",
    tag = "Session",
    params(Start),
    responses(
        (status = SEE_OTHER, description = "On to the provider, or back to the page if linking is refused."),
        (status = NOT_FOUND, description = "This server has no OpenID Connect."),
    ),
)]
pub async fn start(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<Start>,
) -> Response {
    let Some(oidc) = state.oidc.clone() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let link = match query.link {
        None => None,
        Some(_) => match linking(&state, &headers).await {
            Ok(Some(account_id)) => Some(account_id),
            Ok(None) => {
                tracing::debug!("OpenID Connect: refused to link from another site or nobody");
                return back(Some("failed"));
            }
            Err(err) => {
                tracing::error!("OpenID Connect: {err:#}");
                return back(Some("failed"));
            }
        },
    };
    match oidc.start(link).await {
        Ok((to, keep)) => (
            StatusCode::SEE_OTHER,
            [
                (header::LOCATION, to.to_string()),
                (header::SET_COOKIE, flow_cookie(&keep, FLOW_AGE)),
            ],
        )
            .into_response(),
        Err(err) => {
            tracing::warn!("OpenID Connect: {err:#}");
            back(Some("failed"))
        }
    }
}

/// What the provider sends back to `GET /api/oidc/callback`.
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct Callback {
    /// The code to exchange for the provider's answer.
    code: Option<String>,
    /// The flow it belongs to, as the start gave it.
    state: Option<String>,
    /// What the provider says went wrong, instead of a code.
    error: Option<String>,
}

/// Where the provider sends the browser back: logs in the account linked to the
/// provider's, or links it, and sends the browser on to the page, which says
/// how it went.
#[utoipa::path(
    get,
    path = "/oidc/callback",
    tag = "Session",
    params(Callback),
    responses(
        (status = SEE_OTHER, description = "Back to the page, logged in, or with what went wrong."),
        (status = NOT_FOUND, description = "This server has no OpenID Connect."),
    ),
)]
pub async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<Callback>,
) -> Response {
    let Some(oidc) = state.oidc.clone() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if query.error.is_some() {
        return back(Some("denied"));
    }
    let (Some(kept), Some(code), Some(sent)) = (
        sessions::cookie_value(&headers, COOKIE),
        query.code,
        query.state,
    ) else {
        return back(Some("failed"));
    };
    let identity = match oidc.finish(kept, &code, &sent).await {
        Ok(identity) => identity,
        Err(err) => {
            tracing::warn!("OpenID Connect: {err:#}");
            return back(Some("failed"));
        }
    };
    match arrive(&state, &oidc, &headers, identity).await {
        Ok(response) => response,
        Err(err) => {
            tracing::error!("OpenID Connect: {err:#}");
            back(Some("failed"))
        }
    }
}

/// Logs the identity in to its account, or ties it to the account that is
/// logged in, if that is what the flow was started for.
async fn arrive(
    state: &AppState,
    oidc: &Oidc,
    headers: &HeaderMap,
    identity: Identity,
) -> anyhow::Result<Response> {
    let old = sessions::token(headers);
    let account_id = sqlx::query_scalar!(
        "SELECT account_id FROM account_identities WHERE issuer = $1 AND subject = $2",
        oidc.issuer(),
        identity.subject,
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(account_id) = account_id else {
        // Tied only to the account the flow was started for, and only while
        // that account is still the one logged in here.
        let session = match (old, identity.link) {
            (Some(token), Some(_)) => sessions::find(&state.db, token).await?,
            _ => None,
        };
        let Some(session) = session.filter(|session| Some(session.account_id) == identity.link)
        else {
            return Ok(back(Some("unknown")));
        };
        sqlx::query!(
            "INSERT INTO account_identities (issuer, subject, account_id) VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING",
            oidc.issuer(),
            identity.subject,
            session.account_id,
        )
        .execute(&state.db)
        .await?;
        return Ok(back(Some("linked")));
    };

    // As at the login with a password: the provider vouched for who this is,
    // so they may hear that their account is disabled.
    if accounts::is_disabled(&state.db, account_id).await? {
        return Ok(back(Some("disabled")));
    }
    if let Some(old) = old {
        sessions::end(&state.db, old).await?;
    }
    // The provider vouches for the person; a second factor of rueder's own
    // is not asked for here.
    let token = sessions::start(&state.db, account_id, false).await?;
    let mut response = back(None);
    response
        .headers_mut()
        .append(header::SET_COOKIE, sessions::cookie(&token).parse()?);
    Ok(response)
}
