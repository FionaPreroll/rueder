pub mod accounts;
pub mod api;
pub mod articles;
pub mod bench;
pub mod cleaning;
pub mod events;
pub mod feeds;
pub mod fetch;
pub mod filters;
pub mod hardening;
pub mod health;
pub mod http_client;
pub mod images;
pub mod import;
pub mod labels;
pub mod legal;
pub mod login;
pub mod monitor;
pub mod oidc;
pub mod profile;
pub mod rate;
pub mod reading;
pub mod reclean;
pub mod registration;
pub mod reports;
pub mod search;
pub mod second_factor;
pub mod sessions;
pub mod status;
pub mod subscribe;
pub mod transfer;
pub mod tree;
pub mod typescript;

use std::{path::Path, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path as UrlPath, State},
    http::{HeaderName, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;
use sqlx::PgPool;
use ts_rs::TS;

use crate::{
    cleaning::Rules,
    events::Events,
    hardening::Policies,
    http_client::HttpClient,
    images::{ImageProxy, Refused},
    legal::Legal,
    login::Limiter,
    monitor::Monitor,
    oidc::Oidc,
    rate::Rates,
    sessions::Session,
};
use tower_http::{
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

/// The whole app: JSON API under `/api`, everything else is the SvelteKit SPA.
pub fn router(state: AppState, static_dir: &Path) -> Router {
    // Unknown paths get index.html so client-side routes survive a reload.
    let spa = ServeDir::new(static_dir).fallback(ServeFile::new(static_dir.join("index.html")));

    let policies = Arc::new(Policies::for_frontend(static_dir));

    Router::new()
        .nest("/api", api(&state))
        .route("/img/{signature}/{size}/{address}", get(image))
        .fallback_service(spa)
        // Around everything, from the inside out: a change asked for by a
        // page of another site is refused, every request counts for its
        // sender, and every answer, a refusal too, gets its headers. Outside
        // of those, the time of each answer is counted for its route.
        .layer(middleware::from_fn(hardening::same_origin))
        .layer(middleware::from_fn_with_state(state.clone(), rate::limit))
        .layer(middleware::from_fn_with_state(policies, hardening::headers))
        .layer(middleware::from_fn_with_state(state.clone(), monitor::time))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn api(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/events", get(events::stream))
        .route("/login", post(login::login))
        .route("/logout", post(login::logout))
        .route("/me", get(login::me))
        // What the login page asks before anybody is logged in.
        .route("/login/ways", get(login::ways))
        .route("/oidc/start", get(oidc::start))
        .route("/oidc/callback", get(oidc::callback))
        .merge(api::routes())
        .merge(tree::routes())
        .merge(labels::routes())
        .merge(filters::routes())
        .merge(subscribe::routes())
        .merge(profile::routes())
        .merge(second_factor::routes())
        .merge(legal::routes())
        .merge(registration::routes())
        .merge(status::routes())
        .merge(reports::routes())
        .merge(transfer::routes())
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

/// The types of this module for the frontend, see [`typescript`].
pub(crate) fn types(types: &mut typescript::Types) {
    types.add::<Health>();
}

/// What `GET /api/health` answers.
#[derive(Serialize, TS)]
struct Health {
    status: &'static str,
    feeds: i64,
}

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
            let status = match refused {
                Refused::NotFound => StatusCode::NOT_FOUND,
                Refused::Source(_) => StatusCode::BAD_GATEWAY,
                Refused::NotAnImage => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                Refused::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            };
            status.into_response()
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
