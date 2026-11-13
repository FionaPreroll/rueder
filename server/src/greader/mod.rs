//! The Google Reader API, for apps of other makers: Reeder, NetNewsWire (as
//! a FreshRSS account), ReadKit, Read You and others read and use an account
//! through it, as they do with FreshRSS and Miniflux.
//!
//! - **Where:** under `/greader`. An app is told `https://<rueder>/greader`.
//!   `APP_API=off` answers 404 to all of it.
//! - **Login** only with an app password (see [`app_passwords`]):
//!   `POST /greader/accounts/ClientLogin` takes the account's name and an
//!   app password (`Email`, `Passwd`), counted like a login, and answers with
//!   `Auth=`; every other request names it in `Authorization: GoogleLogin
//!   auth=…`. No cookie counts here, so no page of another site can act for
//!   anybody. The token `T` the API has for changes is handed out and not
//!   needed.
//! - **What:** reading in [`read`], changing in [`mod@write`], what a request
//!   says in [`parse`]. Everything goes through the same functions and
//!   limits as the page's API, and what an app changes reaches the account's
//!   open tabs as an event, by its address (see `events::changed`).
//! - **Images** come through the proxy at addresses made for the app
//!   password (`Signer::app_address`), which load without a session as long
//!   as the app password counts.

mod described;
pub mod parse;
mod read;
mod write;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, FromRequest, FromRequestParts, Path, Request, State},
    http::{HeaderMap, HeaderName, StatusCode, header, request::Parts},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::any,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::time::Instant;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppError, AppState, MAX_BODY,
    app_passwords::{self, AppLogin},
    events::{self, Area, Change},
    login::{Key, Sender},
};
use parse::Params;

/// How the API's paths begin, under `/greader`.
pub const API: &str = "/reader/api/0";

/// The routes of the API, with their description, without what goes around
/// them ([`around`]). What only reads takes GET and POST: an app that sends a
/// read as a form, as NetNewsWire does with an app password that no longer
/// counts, is told to log in again (401), not 405.
pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(client_login))
        .routes(routes!(token))
        .routes(routes!(user_info))
        .routes(routes!(read::subscriptions))
        .routes(routes!(read::tags))
        .routes(routes!(read::unread_counts))
        .routes(routes!(read::item_ids))
        .routes(routes!(read::item_contents))
        .routes(routes!(read::stream_contents))
        .routes(routes!(read::stream_contents_of))
        .routes(routes!(write::edit_tag))
        .routes(routes!(write::mark_all_read))
        .routes(routes!(write::edit_subscription))
        .routes(routes!(write::quick_add))
        .routes(routes!(write::rename_tag))
        .routes(routes!(write::disable_tag))
        .routes(routes!(image))
}

/// What goes around the API: whatever else an app asks of it answers `[]`,
/// nothing without `APP_API`, the app password is found once and handed on,
/// and what a request changes is announced to the account's open tabs.
#[allow(
    clippy::disallowed_methods,
    reason = "the catch-all is no operation; the group's text in the description says what it answers"
)]
pub fn around(routes: Router<AppState>, state: &AppState) -> Router<AppState> {
    routes
        // Not an operation of its own, so not in the description, which says
        // in the group's text what it answers.
        .route("/reader/api/0/{*rest}", any(unknown))
        .layer(middleware::from_fn_with_state(state.clone(), around_each))
        .layer(DefaultBodyLimit::max(MAX_BODY))
}

/// The app a request comes from, by its app password.
#[derive(Debug, Clone)]
pub struct App(pub AppLogin);

/// What an app gets without a valid app password: 401, and the header with
/// which Google Reader told apps to log in again.
fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(HeaderName::from_static("x-reader-google-bad-token"), "true")],
        "Unauthorized",
    )
        .into_response()
}

impl FromRequestParts<AppState> for App {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _: &AppState) -> Result<Self, Response> {
        // What wraps the API has looked the app password up already.
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or_else(unauthorized)
    }
}

/// The token an app names in `Authorization: GoogleLogin auth=…`.
fn auth_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, rest) = value.trim().split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("GoogleLogin") {
        return None;
    }
    let token = rest.trim().strip_prefix("auth=")?;
    (!token.is_empty()).then_some(token)
}

/// Around each request of the API: nothing without `APP_API`, the app
/// password is found once and handed on, and what a request changes is
/// announced to the account's open tabs.
async fn around_each(State(state): State<AppState>, mut request: Request, next: Next) -> Response {
    if !state.app_api {
        return StatusCode::NOT_FOUND.into_response();
    }
    let path = format!("/greader{}", request.uri().path());
    let login = match auth_token(request.headers()) {
        Some(token) => app_passwords::find(&state.db, token).await.ok().flatten(),
        None => None,
    };
    if let Some(login) = &login {
        if login.first_use {
            connected(&state, login.session.account_id);
        }
        request.extensions_mut().insert(App(login.clone()));
    }
    let response = next.run(request).await;
    if response.status().is_success()
        && let Some(login) = login
        && let Some(what) = events::changed(&path)
    {
        events::publish(&state, login.session.account_id, what, None).await;
    }
    response
}

/// Tells the account's tabs that an app logged in with an app password for
/// the first time, so that the page that set it up can say so.
fn connected(state: &AppState, account_id: i32) {
    let change = Change::Area {
        area: Area::Apps,
        by: None,
    };
    state.events.publish(vec![account_id], change);
}

/// The parameters of a request: its query and its form body, as apps send
/// them (see [`Params`]).
pub struct Fields(pub Params);

impl FromRequest<AppState> for Fields {
    type Rejection = Response;

    async fn from_request(request: Request, state: &AppState) -> Result<Self, Response> {
        let query = request.uri().query().unwrap_or_default().to_owned();
        let body = Bytes::from_request(request, state)
            .await
            .map_err(IntoResponse::into_response)?;
        match Params::parse(&query, &body) {
            Ok(params) => Ok(Self(params)),
            Err(parse::Refusal::TooMany) => {
                Err((StatusCode::BAD_REQUEST, "too many parameters").into_response())
            }
            Err(parse::Refusal::TooLong) => {
                Err((StatusCode::BAD_REQUEST, "a parameter is too long").into_response())
            }
        }
    }
}

/// The answer of a change that worked, as Google Reader gave it.
pub(crate) fn ok() -> Response {
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], "OK").into_response()
}

/// Where this server is, as an app reached it, for the addresses of images:
/// apps show a text in a page of their own, where an address without a host
/// would point to the article's site. Behind a reverse proxy that names it in
/// `X-Forwarded-Proto` its scheme counts, else HTTPS, which rueder needs
/// anyway, but for this machine itself.
pub(crate) fn origin(headers: &HeaderMap) -> Option<String> {
    let host = headers.get(header::HOST)?.to_str().ok()?.trim();
    let valid = !host.is_empty()
        && host.len() <= 255
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-.:[]".contains(&b));
    if !valid {
        return None;
    }
    let forwarded = headers
        .get(HeaderName::from_static("x-forwarded-proto"))
        .and_then(|value| value.to_str().ok())
        .map(str::trim);
    let local =
        host.starts_with("localhost") || host.starts_with("127.") || host.starts_with("[::1]");
    let scheme = match forwarded {
        Some("http") => "http",
        Some("https") => "https",
        _ if local => "http",
        _ => "https",
    };
    Some(format!("{scheme}://{host}"))
}

/// What `ClientLogin` answers: the same token three times, as Google Reader
/// did; apps take `Auth`.
#[derive(Serialize)]
struct Tokens {
    #[serde(rename = "SID")]
    sid: String,
    #[serde(rename = "LSID")]
    lsid: String,
    #[serde(rename = "Auth")]
    auth: String,
}

/// Logs an app in with the account's name and an app password. Wrong ones
/// count like wrong passwords at the page's login. The token is the app
/// password as it is compared, without dashes: apps read it up to an `=`.
#[utoipa::path(
    post,
    path = "/accounts/ClientLogin",
    tag = "Google Reader API",
    request_body(content = described::ClientLogin, content_type = "application/x-www-form-urlencoded"),
    responses((status = OK, description = "`SID=`, `LSID=` and `Auth=` with the token, each on a line of plain text; as JSON with `output=json`.", content_type = "text/plain"), (status = UNAUTHORIZED, description = "`Error=BadAuthentication`: a wrong name or app password."), (status = TOO_MANY_REQUESTS, description = "Too many wrong tries for this name or from this sender; see `Retry-After`.")),
)]
async fn client_login(
    State(state): State<AppState>,
    Sender(ip): Sender,
    Fields(params): Fields,
) -> Result<Response, AppError> {
    let name = params.one("Email").unwrap_or_default();
    let secret = params.one("Passwd").unwrap_or_default();
    let keys = Key::of(name, ip);
    if let Err(wait) = state.logins.attempt(&keys, Instant::now()) {
        let message = "too many wrong tries, try again later";
        return Err(AppError::TooMany(wait, message));
    }
    let login = app_passwords::find(&state.db, secret).await?;
    let Some(login) = login.filter(|login| Key::name(name) == Key::name(&login.session.username))
    else {
        return Ok((StatusCode::UNAUTHORIZED, "Error=BadAuthentication\n").into_response());
    };
    state.logins.succeeded(&keys);
    if login.first_use {
        connected(&state, login.session.account_id);
    }
    let token = app_passwords::token(secret);
    if params.json() {
        let tokens = Tokens {
            sid: token.clone(),
            lsid: token.clone(),
            auth: token,
        };
        return Ok(Json(tokens).into_response());
    }
    let text = format!("SID={token}\nLSID={token}\nAuth={token}\n");
    Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], text).into_response())
}

/// The token `T` for changes. It proves nothing here, the app password in
/// the header does; it is handed out for apps that want one, and is not the
/// app password.
#[utoipa::path(
    method(get, post),
    path = "/reader/api/0/token",
    tag = "Google Reader API",
    security(("app_password" = [])),
    responses((status = OK, description = "The token, as plain text.", content_type = "text/plain"), crate::openapi::NeedsAppPassword),
)]
async fn token(App(login): App) -> Response {
    let digest = Sha256::digest(format!("rueder-greader-token-{}", login.id).as_bytes());
    let token: String = digest.iter().take(20).map(|b| format!("{b:02x}")).collect();
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], token).into_response()
}

/// Who is logged in, as `GET /reader/api/0/user-info` answers.
#[derive(Serialize, ToSchema)]
#[schema(as = GreaderUserInfo)]
#[serde(rename_all = "camelCase")]
struct UserInfo {
    user_id: String,
    user_name: String,
    user_profile_id: String,
    user_email: String,
}

/// Who is logged in: the account's number and name.
#[utoipa::path(
    method(get, post),
    path = "/reader/api/0/user-info",
    tag = "Google Reader API",
    security(("app_password" = [])),
    responses((status = OK, body = UserInfo), crate::openapi::NeedsAppPassword),
)]
async fn user_info(App(login): App) -> Json<UserInfo> {
    let id = login.session.account_id.to_string();
    Json(UserInfo {
        user_id: id.clone(),
        user_name: login.session.username.clone(),
        user_profile_id: id,
        user_email: login.session.username,
    })
}

/// What rueder does not have of the API: an empty answer, as Miniflux gives,
/// since apps ask for such things in passing and take nothing for an error.
async fn unknown(App(_): App) -> Response {
    Json(serde_json::json!([])).into_response()
}

/// An image through the proxy for an app: no session, the address proves
/// that the server made it for an app password, which has to still count.
#[utoipa::path(
    get,
    path = "/img/{app}/{signature}/{size}/{address}",
    operation_id = "app_image",
    tag = "Google Reader API",
    params(("app" = i32, Path, description = "The app password the address was made for."), ("signature" = String, Path, description = "Proves that the server made the address for it."), ("size" = String, Path, description = "`icon`, `thumbnail` or `image`."), ("address" = String, Path, description = "The image's own address, encoded.")),
    responses((status = OK, description = "The image.", content_type = "image/*"), (status = NOT_FOUND, description = "No such image, an address the server did not make, an app password that no longer counts, or no image proxy."), (status = BAD_GATEWAY, description = "The image's own server did not deliver it, or not as an image.")),
)]
async fn image(
    State(state): State<AppState>,
    Path((app, signature, size, address)): Path<(i32, String, String, String)>,
) -> Response {
    let Some(images) = state.images.clone() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    // The cheap check first: an address the server did not make costs no
    // query. Then the app password, before anything is fetched.
    let made = images
        .signer()
        .open_for_app(app, &signature, &size, &address);
    if made.is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    match app_passwords::counts(&state.db, app).await {
        Ok(true) => {}
        Ok(false) => return StatusCode::NOT_FOUND.into_response(),
        Err(err) => return AppError::Internal(err).into_response(),
    }
    match images.get_for_app(app, &signature, &size, &address).await {
        Ok(picture) => (
            [
                (header::CONTENT_TYPE, picture.format.mime()),
                (
                    header::CACHE_CONTROL,
                    "private, max-age=31536000, immutable",
                ),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                (
                    header::CONTENT_SECURITY_POLICY,
                    "default-src 'none'; sandbox",
                ),
                // Apps show texts in pages of their own, from no origin of
                // this server.
                (
                    HeaderName::from_static("cross-origin-resource-policy"),
                    "cross-origin",
                ),
            ],
            picture.bytes,
        )
            .into_response(),
        Err(refused) => {
            tracing::debug!("image for an app refused: {refused}");
            refused.into_response()
        }
    }
}
