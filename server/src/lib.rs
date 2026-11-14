//! The rueder server: one program next to PostgreSQL that fetches feeds,
//! cleans and stores their articles, and serves the web page, the page's API
//! and the Google Reader API for apps of other makers. `src/main.rs` is the
//! command line (`rueder serve`, `rueder account add` and the others); all
//! else is in this library, where the tests and the benchmarks reach it.
//!
//! These pages come from the comments in the code, private items included
//! (`cargo doc --no-deps --lib --document-private-items`). How to build and
//! run the server is in `server/README.md`.
//!
//! # Where to find what
//!
//! - **The server as a whole:** [`router`] puts the routes together with the
//!   layers around them, [`AppState`] is what every request reaches, and
//!   [`AppError`] is how a request fails without giving away internals.
//! - **Feeds and articles:** [`fetch`] fetches what is due, [`feeds`] turns a
//!   feed into articles, [`cleaning`] makes their HTML safe, [`articles`]
//!   stores them, [`images`] is the image proxy, and [`http_client`] is the
//!   only way out to the internet.
//! - **Reading:** [`mod@api`] is the API the page lives on, with [`reading`],
//!   [`tree`], [`subscribe`], [`search`], [`filters`] and [`labels`];
//!   [`events`] brings what changes to every open tab.
//! - **Accounts:** [`accounts`], [`login`], [`sessions`], [`second_factor`],
//!   [`passkeys`] (with [`asset_links`] for the Android app), [`oidc`],
//!   [`registration`], [`profile`] and [`transfer`].
//! - **Apps of other makers:** [`greader`] and [`app_passwords`].
//! - **Safety:** [`hardening`] for every answer and every changing request,
//!   [`rate`] for how much one sender and one account may ask.
//! - **Running it:** [`status`], [`monitor`], [`history`], [`mod@health`],
//!   [`reports`], [`retention`], [`moderation`], [`legal`] and [`version`].
//! - **Tools:** [`import`] (the old rueder's data), [`reclean`],
//!   [`mod@bench`] and [`typescript`] (the API's types for the page).

pub mod accounts;
pub mod api;
pub mod app_passwords;
pub mod articles;
pub mod asset_links;
pub mod bench;
pub mod cleaning;
pub mod email;
pub mod events;
pub mod feeds;
pub mod fetch;
pub mod filters;
pub mod greader;
pub mod hardening;
pub mod health;
pub mod history;
pub mod http_client;
pub mod images;
pub mod import;
pub mod labels;
pub mod legal;
pub mod login;
pub mod mail;
pub mod moderation;
pub mod monitor;
pub mod oidc;
pub mod openapi;
pub mod passkeys;
pub mod profile;
pub mod rate;
pub mod reading;
pub mod reclean;
pub mod registration;
pub mod reports;
pub mod retention;
pub mod search;
pub mod second_factor;
pub mod sessions;
pub mod status;
pub mod subscribe;
pub mod transfer;
pub mod tree;
pub mod typescript;
pub mod version;

use std::{path::Path, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path as UrlPath, State},
    http::{HeaderName, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use sqlx::PgPool;
use ts_rs::TS;
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    cleaning::Rules, events::Events, hardening::Policies, http_client::HttpClient,
    images::ImageProxy, legal::Legal, login::Limiter, monitor::Monitor, oidc::Oidc, rate::Rates,
    sessions::Session,
};
use tower_http::{
    classify::SharedClassifier,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

/// The largest body a request to the API may have, in bytes.
pub const MAX_BODY: usize = 256 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    /// The image proxy, if this server has one.
    pub images: Option<Arc<ImageProxy>>,
    /// The count of wrong passwords.
    pub logins: Arc<Limiter>,
    /// The header a reverse proxy in front of the server names the sender's
    /// address in. Without one, the address of the connection counts.
    pub client_ip_header: Option<HeaderName>,
    /// The login through an OpenID Connect provider, if this server has one.
    pub oidc: Option<Arc<Oidc>>,
    /// The rules of the cleaning, for what is cleaned when it is asked for.
    /// Without them, the ones that come with rueder count.
    pub rules: Option<Arc<Rules>>,
    /// What changes for an account, for its open tabs to hear of.
    pub events: Events,
    /// The hardened HTTP client, for looking at an address a reader wants to
    /// subscribe to. Without it, nobody subscribes to a feed that is new.
    pub client: Option<HttpClient>,
    /// How much each sender and each account has asked of the server lately.
    pub rates: Arc<Rates>,
    /// What the server notices about itself, for the status page.
    pub monitor: Arc<Monitor>,
    /// Whether the server takes reports of errors from the browsers of
    /// readers who agreed to send them.
    pub reports: bool,
    /// Whether the right of an admin counts only in a session that began with
    /// a second factor (`ADMIN_SECOND_FACTOR=required`).
    pub admin_second_factor: bool,
    /// Where the operator's legal texts are, and which terms count.
    pub legal: Arc<Legal>,
    /// Whether readers may report feeds and articles (`CONTENT_REPORTS=on`).
    pub content_reports: bool,
    /// Whether apps of other makers may log in with app passwords, through
    /// the Google Reader API (on unless `APP_API=off`).
    pub app_api: bool,
    /// Passkeys, for the site at `PUBLIC_URL`; none without it.
    pub passkeys: Option<Arc<passkeys::Passkeys>>,
    /// What sends mail (`SMTP_URL`, see `mail`); none without it, and then
    /// accounts have no address.
    pub mail: Option<Arc<mail::Mailer>>,
}

impl AppState {
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            images: None,
            logins: Arc::default(),
            client_ip_header: None,
            oidc: None,
            rules: None,
            events: Events::new(),
            client: None,
            rates: Arc::default(),
            monitor: Arc::default(),
            reports: true,
            admin_second_factor: false,
            legal: Arc::default(),
            content_reports: false,
            app_api: true,
            passkeys: None,
            mail: None,
        }
    }

    /// Has passkeys, for the site at this address (see `PUBLIC_URL`).
    /// Sends mail through this mailer (see `mail`).
    pub fn with_mail(self, mailer: mail::Mailer) -> Self {
        Self {
            mail: Some(Arc::new(mailer)),
            ..self
        }
    }

    pub fn with_passkeys(self, passkeys: passkeys::Passkeys) -> Self {
        Self {
            passkeys: Some(Arc::new(passkeys)),
            ..self
        }
    }

    /// Lets no app of another maker in, as with `APP_API=off`.
    pub fn without_app_api(self) -> Self {
        Self {
            app_api: false,
            ..self
        }
    }

    /// Takes no reports of errors from browsers, as with `CLIENT_REPORTS=off`.
    pub fn without_reports(self) -> Self {
        Self {
            reports: false,
            ..self
        }
    }

    /// Lets admins be admins only in a session with a second factor, as with
    /// `ADMIN_SECOND_FACTOR=required`.
    pub fn with_admin_second_factor(self) -> Self {
        Self {
            admin_second_factor: true,
            ..self
        }
    }

    /// Lets readers report feeds and articles, as with `CONTENT_REPORTS=on`.
    pub fn with_content_reports(self) -> Self {
        Self {
            content_reports: true,
            ..self
        }
    }

    /// Links to the operator's legal texts, and the terms an account agrees to.
    pub fn with_legal(self, legal: Legal) -> Self {
        Self {
            legal: Arc::new(legal),
            ..self
        }
    }

    /// Shares the monitor with whatever else it watches, such as fetching.
    pub fn with_monitor(self, monitor: Arc<Monitor>) -> Self {
        Self { monitor, ..self }
    }

    /// Other limits than [`rate::REQUESTS`] and [`rate::LOOKUPS`].
    pub fn with_rates(self, rates: Rates) -> Self {
        Self {
            rates: Arc::new(rates),
            ..self
        }
    }

    pub fn with_client(self, client: HttpClient) -> Self {
        Self {
            client: Some(client),
            ..self
        }
    }

    /// Shares the events with whatever else publishes them, such as fetching.
    pub fn with_events(self, events: Events) -> Self {
        Self { events, ..self }
    }

    pub fn with_rules(self, rules: Arc<Rules>) -> Self {
        Self {
            rules: Some(rules),
            ..self
        }
    }

    pub fn with_oidc(self, oidc: Arc<Oidc>) -> Self {
        Self {
            oidc: Some(oidc),
            ..self
        }
    }

    pub fn with_client_ip_header(self, name: HeaderName) -> Self {
        Self {
            client_ip_header: Some(name),
            ..self
        }
    }

    pub fn with_images(self, images: Arc<ImageProxy>) -> Self {
        Self {
            images: Some(images),
            ..self
        }
    }
}

/// Where the page's API is, in the router and in its description alike.
const API: &str = "/api";
/// Where the Google Reader API is.
const GREADER: &str = "/greader";

/// The whole app: the page's API under `/api`, the Google Reader API under
/// `/greader`, the image proxy, and everything else is the SvelteKit SPA.
///
/// The routes come from the same functions as their description
/// ([`description`]): each is registered with `utoipa_axum::routes!` from the
/// handler's `#[utoipa::path]`, so nothing is served that is not described.
pub fn router(state: AppState, static_dir: &Path) -> Router {
    // Unknown paths get index.html so client-side routes survive a reload.
    let spa = ServeDir::new(static_dir).fallback(ServeFile::new(static_dir.join("index.html")));

    let policies = Arc::new(Policies::for_frontend(static_dir));
    let (api_routes, _) = api_routes().split_for_parts();
    let (greader_routes, _) = greader::routes().split_for_parts();
    let (other_routes, _) = other_routes().split_for_parts();

    Router::new()
        .nest(API, api(api_routes, &state))
        .nest(GREADER, greader::around(greader_routes, &state))
        .merge(other_routes)
        .fallback_service(spa)
        // Around everything, from the inside out: a change asked for by a
        // page of another site is refused, every request counts for its
        // sender, and every answer, a refusal too, gets its headers. Outside
        // of those, the time of each answer is counted for its route.
        .layer(middleware::from_fn(hardening::same_origin))
        .layer(middleware::from_fn_with_state(state.clone(), rate::limit))
        .layer(middleware::from_fn_with_state(policies, hardening::headers))
        .layer(middleware::from_fn_with_state(state.clone(), monitor::time))
        .layer(TraceLayer::new(SharedClassifier::new(monitor::Failures)))
        .with_state(state)
}

/// The description of every route the server answers (OpenAPI), from the
/// same routes as [`router`] serves; see [`openapi`]. `server/openapi.json`
/// is it as a file.
pub fn description() -> utoipa::openapi::OpenApi {
    let (_, mut description) = OpenApiRouter::with_openapi(openapi::ApiDoc::openapi())
        .nest(API, api_routes())
        .nest(GREADER, greader::routes())
        .merge(other_routes())
        .split_for_parts();
    // What takes GET and POST alike is one operation under two methods. Its
    // id is the GET's; the POST's gets `_post`, as every id is one of a kind.
    for item in description.paths.paths.values_mut() {
        if let (Some(get), Some(post)) = (&item.get, &mut item.post)
            && get.operation_id == post.operation_id
        {
            post.operation_id = get.operation_id.as_ref().map(|id| format!("{id}_post"));
        }
        // An answer that works and says no more than its body: what its
        // status is called. Every refusal says why in its own words.
        let operations = [
            &mut item.get,
            &mut item.post,
            &mut item.put,
            &mut item.patch,
            &mut item.delete,
        ];
        for operation in operations.into_iter().flatten() {
            // A summary is one line, whichever lines its comment had.
            if let Some(summary) = &mut operation.summary {
                *summary = summary.split_whitespace().collect::<Vec<_>>().join(" ");
            }
            // Where the handler's comment begins with its route, as
            // "`GET /api/events`: the account's events", the route stands
            // next to it already: the summary is what follows.
            if let Some(summary) = &mut operation.summary
                && let Some((route, said)) = summary
                    .strip_prefix('`')
                    .and_then(|rest| rest.split_once("`: "))
                && route.split_once(' ').is_some_and(|(method, _)| {
                    !method.is_empty() && method.chars().all(|c| c.is_ascii_uppercase())
                })
            {
                let mut chars = said.chars();
                *summary = chars
                    .next()
                    .map(|first| first.to_uppercase().chain(chars).collect())
                    .unwrap_or_default();
            }
            for (status, response) in &mut operation.responses.responses {
                let reason = status
                    .parse()
                    .ok()
                    .filter(|status: &u16| (200..300).contains(status))
                    .and_then(|status| StatusCode::from_u16(status).ok())
                    .and_then(|status| status.canonical_reason());
                if let (utoipa::openapi::RefOr::T(response), Some(reason)) = (response, reason)
                    && response.description.is_empty()
                {
                    response.description = reason.to_owned();
                }
            }
        }
    }
    description
}

/// The routes of the page's API, without what goes around them ([`api()`]).
fn api_routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(health))
        .routes(routes!(events::stream))
        .routes(routes!(login::login))
        .routes(routes!(login::logout))
        .routes(routes!(login::me))
        // What the login page asks before anybody is logged in.
        .routes(routes!(login::ways))
        .routes(routes!(passkeys::start_login))
        .routes(routes!(passkeys::finish_login))
        .routes(routes!(oidc::start))
        .routes(routes!(oidc::callback))
        .merge(api::routes())
        .merge(tree::routes())
        .merge(labels::routes())
        .merge(filters::routes())
        .merge(subscribe::routes())
        .merge(profile::routes())
        .merge(second_factor::routes())
        .merge(legal::routes())
        .merge(moderation::routes())
        .merge(app_passwords::routes())
        .merge(passkeys::routes())
        .merge(registration::routes())
        .merge(email::routes())
        .merge(status::routes())
        .merge(reports::routes())
        .merge(transfer::routes())
}

/// What goes around the page's API.
fn api(routes: Router<AppState>, state: &AppState) -> Router<AppState> {
    routes
        .fallback(|| async { StatusCode::NOT_FOUND })
        // Around all of the API: whatever a request changes is announced to
        // the account's other tabs.
        .layer(middleware::from_fn_with_state(
            state.clone(),
            events::announce,
        ))
        // What the API is sent is small. Only an OPML file and the file of an
        // account may be larger, and say so at their own endpoints.
        .layer(DefaultBodyLimit::max(MAX_BODY))
}

/// The routes outside the two APIs: the image proxy and what Android asks.
fn other_routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(image))
        .routes(routes!(asset_links::serve))
}

/// The types of this module for the frontend, see [`typescript`].
pub(crate) fn types(types: &mut typescript::Types) {
    types.add::<Health>();
}

/// What `GET /api/health` answers.
#[derive(Serialize, TS, ToSchema)]
struct Health {
    /// Always `ok`: the server answers and its database too.
    status: &'static str,
    /// How many feeds the server knows.
    feeds: i64,
}

/// Whether the server and its database answer, for a health check (also
/// `rueder health` on the command line).
#[utoipa::path(
    get,
    path = "/health",
    tag = "Admin",
    responses((status = OK, body = Health)),
)]
async fn health(State(state): State<AppState>) -> Result<Json<Health>, AppError> {
    let feeds = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM feeds"#)
        .fetch_one(&state.db)
        .await?;
    Ok(Json(Health {
        status: "ok",
        feeds,
    }))
}

/// An image through the proxy, for logged-in readers only. The address says
/// which one and proves that the server itself asked for it; see [`images`].
///
/// The page never makes such an address: it comes in what the API sends.
/// The answer is the image in a format the browser shows, made smaller where
/// the size asks for it, never anything that runs.
#[utoipa::path(
    get,
    path = "/img/{signature}/{size}/{address}",
    operation_id = "image",
    tag = "Images",
    params(
        ("signature" = String, Path, description = "Proves that the server made the address."),
        ("size" = String, Path, description = "What the image is for, and so how large it may be: `icon`, `thumbnail` or `image`."),
        ("address" = String, Path, description = "The image's own address, encoded."),
    ),
    security(("session" = [])),
    responses(
        (status = OK, description = "The image.", content_type = "image/*"),
        openapi::NeedsSession,
        (status = NOT_FOUND, description = "No such image, a signature that does not fit, or no image proxy on this server."),
        (status = BAD_GATEWAY, description = "The image's own server did not deliver it, or not as an image."),
    ),
)]
async fn image(
    State(state): State<AppState>,
    _session: Session,
    UrlPath((signature, size, address)): UrlPath<(String, String, String)>,
) -> Response {
    let Some(images) = state.images else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match images.get(&signature, &size, &address).await {
        Ok(picture) => (
            [
                (header::CONTENT_TYPE, picture.format.mime()),
                // The address says what the image is, so it never changes. It
                // is the reader's own to keep, not a shared cache's.
                (
                    header::CACHE_CONTROL,
                    "private, max-age=31536000, immutable",
                ),
                // Whatever the bytes are, the browser shows them as this image
                // or not at all, and nothing in them runs.
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                (
                    header::CONTENT_SECURITY_POLICY,
                    "default-src 'none'; sandbox",
                ),
                (
                    HeaderName::from_static("cross-origin-resource-policy"),
                    "same-origin",
                ),
            ],
            picture.bytes,
        )
            .into_response(),
        Err(refused) => {
            tracing::debug!("image refused: {refused}");
            refused.into_response()
        }
    }
}

/// What a handler can fail with. Anything unexpected becomes a logged 500.
pub enum AppError {
    NotFound,
    BadRequest(String),
    /// Any other refusal, with its reason.
    Status(StatusCode, String),
    /// Over a limit: how long to wait, and for what.
    TooMany(Duration, &'static str),
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND.into_response(),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message).into_response(),
            Self::Status(status, message) => (status, message).into_response(),
            Self::TooMany(wait, message) => rate::too_many(wait, message),
            Self::Internal(err) => {
                tracing::error!("{err:#}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
            }
        }
    }
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(err: E) -> Self {
        Self::Internal(err.into())
    }
}
