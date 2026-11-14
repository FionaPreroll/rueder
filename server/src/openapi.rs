//! The description of the HTTP interface (OpenAPI 3.1), written from the
//! code: each handler says what it takes and answers in its
//! `#[utoipa::path]`, each type it sends or reads derives `ToSchema` next to
//! `TS`, and the routes are registered with `utoipa_axum`, so that the router
//! serves exactly what the description lists ([`crate::description`]). This module
//! holds what is the same for all of them: the overview (`openapi.md`), the
//! groups, the two ways to log in, and the answers every operation that needs
//! a login can give.
//!
//! `server/openapi.json` is the description as a file, kept up to date by
//! `tests/api_types.rs` (`UPDATE_API_TYPES=1`) and shown on GitHub Pages with
//! Swagger UI (`pages/`).

use utoipa::{
    IntoResponses, Modify, OpenApi,
    openapi::{
        Components,
        security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
    },
};

use crate::sessions::COOKIE;

/// The overview, the groups and the ways to log in. The operations come from
/// the routes, in [`crate::description`].
#[derive(OpenApi)]
#[openapi(
    info(
        title = "rueder",
        description = include_str!("openapi.md"),
    ),
    modifiers(&Logins),
    tags(
        (name = "Session", description = "Logging in and out, with a password, a passkey or OpenID Connect, and who is logged in."),
        (name = "Reading", description = "The tree of folders and feeds with their unread counts, the lists of articles, an article with its versions, what was read, and the search."),
        (name = "Feeds and folders", description = "Changing the tree: folders, renaming, moving and ending subscriptions, and the reader's settings."),
        (name = "Subscribing", description = "Finding the feeds of a web page, subscribing to one, and OPML in and out."),
        (name = "Labels", description = "The reader's own colored marks for articles."),
        (name = "Filter rules", description = "Rules that hide articles, or mark them read, as they arrive."),
        (name = "Account", description = "The reader's own account: sessions, password, second factor, passkeys, app passwords, moving it to another rueder, and deleting it."),
        (name = "Registration", description = "Accounts that make themselves, where the operator allows it, and invitations."),
        (name = "Admin", description = "For admins only: the status of the server and the accounts."),
        (name = "Reports", description = "Reports of feeds and articles by their readers, and errors in browsers of readers who agreed to send them."),
        (name = "Legal", description = "Where the operator's imprint, privacy policy and terms of use are, and agreeing to the terms."),
        (name = "Live updates", description = "What changes for the account, as server-sent events to every open page."),
        (name = "Images", description = "The image proxy, at addresses the server signs and sends in what it answers."),
        (name = "Android", description = "What the Android app needs from the server."),
        (name = "Google Reader API", description = "For feed reader apps of other makers (NetNewsWire, Read You, Reeder and others), as FreshRSS and Miniflux have it: an app logs in with an app password at `ClientLogin` and names the token it gets in `Authorization: GoogleLogin auth=<token>`. Requests are forms or query parameters; what only reads takes GET and POST alike, as some apps send everything as a form."),
    ),
)]
pub(crate) struct ApiDoc;

/// The ways an operation knows who asks, as `security(...)` names them:
/// `session` for the page's API, `app_password` for the Google Reader API,
/// `metrics_token` for the metrics for Prometheus.
struct Logins;

impl Modify for Logins {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Components::new);
        components.add_security_scheme(
            "session",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
                COOKIE,
                "The session, as POST /api/login, a passkey or OpenID Connect set it.",
            ))),
        );
        components.add_security_scheme(
            "app_password",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
                "Authorization",
                "GoogleLogin auth=<token>, with the token that POST /greader/accounts/ClientLogin \
                 answers for an app password.",
            ))),
        );
        components.add_security_scheme(
            "metrics_token",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .description(Some(
                        "The token the operator set in METRICS_TOKEN, for Prometheus.",
                    ))
                    .build(),
            ),
        );
    }
}

/// What an operation of the page's API that needs a session answers without
/// a valid one.
#[derive(IntoResponses)]
#[response(status = 401, description = "No valid session: log in first.")]
pub(crate) struct NeedsSession;

/// What an operation of the Google Reader API answers without a valid app
/// password.
#[derive(IntoResponses)]
#[response(
    status = 401,
    description = "No valid app password: log in again at ClientLogin."
)]
pub(crate) struct NeedsAppPassword;
