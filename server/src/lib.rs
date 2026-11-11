pub mod accounts;
pub mod articles;
pub mod bench;
pub mod cleaning;
pub mod feeds;
pub mod fetch;
pub mod health;
pub mod http_client;
pub mod images;
pub mod import;
pub mod login;
pub mod oidc;
pub mod reading;
pub mod reclean;
pub mod sessions;

use std::{path::Path, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path as UrlPath, State},
    http::{HeaderName, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;
use sqlx::PgPool;

use crate::{
    images::{ImageProxy, Refused},
    login::Limiter,
    oidc::Oidc,
    sessions::Session,
};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

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
}

impl AppState {
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            images: None,
            logins: Arc::default(),
            client_ip_header: None,
            oidc: None,
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

    Router::new()
        .nest("/api", api())
        .route("/img/{signature}/{size}/{address}", get(image))
        .fallback_service(spa)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn api() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/login", post(login::login))
        .route("/logout", post(login::logout))
        .route("/me", get(login::me))
        // What the login page asks before anybody is logged in.
        .route("/login/ways", get(login::ways))
        .route("/oidc/start", get(oidc::start))
        .route("/oidc/callback", get(oidc::callback))
        .fallback(|| async { StatusCode::NOT_FOUND })
}

#[derive(Serialize)]
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
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND.into_response(),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message).into_response(),
            Self::Status(status, message) => (status, message).into_response(),
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
