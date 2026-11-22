//! The HTTP interface: `GET /v1/feed` makes the feed of a page, `GET
//! /v1/usage` says what a token has used, `GET /health` that the server runs.
//! The description (OpenAPI) is written from the handlers and served at
//! `GET /openapi.json`.

use std::{sync::Arc, time::Instant};

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use hardened_http::{Error as FetchError, Request};
use serde::Deserialize;
use utoipa::{
    IntoParams, Modify, OpenApi,
    openapi::{
        Components,
        security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    },
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    AppState,
    extract::{self, MIN_ITEMS},
    feed::{JsonFeed, Problem, ProblemBody, Written},
    limits::{Over, Usage},
    tokens::Token,
};

/// The largest page that is read, in bytes after decompression.
pub const MAX_PAGE: usize = 4 * 1024 * 1024;
/// The longest address a client may name.
pub const MAX_URL: usize = 2000;

const ACCEPT_HTML: &str = "text/html, application/xhtml+xml;q=0.9, */*;q=0.1";

#[derive(OpenApi)]
#[openapi(
    info(
        title = "RSSifier",
        description = "Makes feeds of web pages that have none. Every request but `/health` and `/openapi.json` names an app token of the operator in `Authorization: Bearer rsf_…`. Problems come as `application/problem+json`; their `type` is a fixed word, their `detail` fit to be shown to a reader.",
    ),
    modifiers(&Bearer),
    components(schemas(ProblemBody)),
)]
struct ApiDoc;

struct Bearer;

impl Modify for Bearer {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Components::new);
        components.add_security_scheme(
            "token",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .description(Some("An app token, made with `rssifier token add`."))
                    .build(),
            ),
        );
    }
}

/// The routes with their description.
pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(feed))
        .routes(routes!(usage))
        .routes(routes!(health))
        .routes(routes!(openapi_json))
}

/// The description, as `GET /openapi.json` serves it.
pub fn description() -> utoipa::openapi::OpenApi {
    routes().split_for_parts().1
}

/// The token a request names, or 401.
fn token(state: &AppState, headers: &HeaderMap) -> Result<Token, Problem> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(|secret| state.tokens.find(secret.trim()))
        .ok_or_else(|| {
            Problem::new(
                StatusCode::UNAUTHORIZED,
                "token",
                "no token, or one this server does not know",
            )
        })
}

fn over(over: Over) -> Problem {
    match over {
        Over::Rate(wait) => Problem {
            retry_after: Some(wait),
            ..Problem::new(
                StatusCode::TOO_MANY_REQUESTS,
                "quota",
                "this token asked too often",
            )
        },
        Over::AtOnce => Problem {
            retry_after: Some(std::time::Duration::from_secs(1)),
            ..Problem::new(
                StatusCode::TOO_MANY_REQUESTS,
                "quota",
                "this token has too many requests under way",
            )
        },
    }
}

/// What `GET /v1/feed` reads.
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct FeedQuery {
    /// The address of the page, `http` or `https`.
    pub url: String,
}

/// Makes the feed of a page. The answer for a page is kept for a while (15
/// minutes), for every token; within that time the page is not fetched again.
/// With `If-None-Match` and the `ETag` of the last answer, an unchanged feed
/// is 304.
#[utoipa::path(
    get,
    path = "/v1/feed",
    params(FeedQuery),
    security(("token" = [])),
    responses(
        (status = OK, description = "The feed of the page.", content_type = "application/feed+json", body = JsonFeed),
        (status = NOT_MODIFIED, description = "The feed is the one of the `ETag` named."),
        (status = BAD_REQUEST, description = "`address`: not an address, or one inside the server's own network.", content_type = "application/problem+json", body = ProblemBody),
        (status = UNAUTHORIZED, description = "`token`: no token, or an unknown one.", content_type = "application/problem+json", body = ProblemBody),
        (status = UNPROCESSABLE_ENTITY, description = "`no-items`: the page lists nothing; `needs-javascript`: the page is made in the browser; `too-large`: the page is larger than 4 MB.", content_type = "application/problem+json", body = ProblemBody),
        (status = TOO_MANY_REQUESTS, description = "`quota`: a limit of the token; see `Retry-After`.", content_type = "application/problem+json", body = ProblemBody),
        (status = BAD_GATEWAY, description = "`upstream`: the site answers with an error, or not at all.", content_type = "application/problem+json", body = ProblemBody),
        (status = SERVICE_UNAVAILABLE, description = "`busy`: the host of the page is asked too much right now; see `Retry-After`.", content_type = "application/problem+json", body = ProblemBody),
        (status = GATEWAY_TIMEOUT, description = "`timeout`: the site took too long.", content_type = "application/problem+json", body = ProblemBody),
    ),
)]
async fn feed(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<FeedQuery>,
) -> Response {
    let if_none_match = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok());
    match answer(&state, &headers, &query.url).await {
        Ok(answer) => match &*answer {
            Ok(written) => written.answer(if_none_match),
            Err(problem) => problem.clone().into_response(),
        },
        Err(problem) => problem.into_response(),
    }
}

async fn answer(
    state: &AppState,
    headers: &HeaderMap,
    address: &str,
) -> Result<crate::cache::Answer, Problem> {
    let token = token(state, headers)?;
    let _turn = state.usage.request(&token, Instant::now()).map_err(over)?;
    let refused =
        |err: FetchError| Problem::new(StatusCode::BAD_REQUEST, "address", err.to_string());
    if address.len() > MAX_URL {
        return Err(Problem::new(
            StatusCode::BAD_REQUEST,
            "address",
            "the address is too long",
        ));
    }
    let url = state.client.parse(address).map_err(refused)?;
    let url = extract::link(&url, url.as_str())
        .ok_or_else(|| refused(FetchError::Address("no host".into())))?;
    let key = url.to_string();
    if let Some(answer) = state.cache.get(&key, Instant::now()) {
        return Ok(answer);
    }
    state.usage.fetch(&token, Instant::now()).map_err(over)?;
    let host = url.host_str().unwrap_or_default().to_owned();
    let Some(_host) = state.gates.host(&host).await else {
        return Err(Problem {
            retry_after: Some(state.gates.pace.host_wait),
            ..Problem::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "busy",
                format!("{host} is asked too much right now"),
            )
        });
    };
    // Somebody else may have fetched the page while this one waited.
    if let Some(answer) = state.cache.get(&key, Instant::now()) {
        return Ok(answer);
    }
    let answer = Arc::new(make(state, &key).await);
    // An address that is not allowed tells nothing about the page.
    if !matches!(&*answer, Err(problem) if problem.kind == "address") {
        state.cache.put(&key, answer.clone(), Instant::now());
    }
    Ok(answer)
}

/// Fetches the page and reads it.
async fn make(state: &AppState, address: &str) -> Result<Written, Problem> {
    let fetched = {
        let _place = state.gates.fetch().await;
        let request = Request {
            accept: Some(ACCEPT_HTML),
            max_body: Some(MAX_PAGE),
            ..Request::new(address)
        };
        state.client.get(request).await
    };
    let response = fetched.map_err(|err| match err {
        FetchError::Timeout => {
            Problem::new(StatusCode::GATEWAY_TIMEOUT, "timeout", err.to_string())
        }
        FetchError::TooLarge(_) => Problem::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "too-large",
            err.to_string(),
        ),
        FetchError::Address(_) | FetchError::Internal { .. } => {
            Problem::new(StatusCode::BAD_REQUEST, "address", err.to_string())
        }
        _ => Problem::new(StatusCode::BAD_GATEWAY, "upstream", err.to_string()),
    })?;
    if !response.status.is_success() {
        let detail = format!("the site answers with {}", response.status);
        return Err(Problem::new(StatusCode::BAD_GATEWAY, "upstream", detail));
    }
    let page = {
        let _place = state.gates.read().await;
        let base = response.url.clone();
        let body = response.body;
        tokio::task::spawn_blocking(move || extract::page(&String::from_utf8_lossy(&body), &base))
            .await
            .unwrap_or_default()
    };
    if page.items.len() < MIN_ITEMS {
        return Err(if page.scripted {
            Problem::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "needs-javascript",
                "the page is made in the browser, by scripts",
            )
        } else {
            Problem::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "no-items",
                "nothing on the page looks like a list of entries",
            )
        });
    }
    Ok(Written::of(&JsonFeed::of(page, address)))
}

/// What the token has used of its limits. This request does not count.
#[utoipa::path(
    get,
    path = "/v1/usage",
    security(("token" = [])),
    responses(
        (status = OK, description = "The token's limits and what it used.", body = Usage),
        (status = UNAUTHORIZED, description = "`token`: no token, or an unknown one.", content_type = "application/problem+json", body = ProblemBody),
    ),
)]
async fn usage(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Usage>, Problem> {
    let token = token(&state, &headers)?;
    Ok(Json(state.usage.usage(&token, Instant::now())))
}

/// Says that the server runs. Without a token, and without numbers.
#[utoipa::path(get, path = "/health", responses((status = OK, description = "The server runs.")))]
async fn health() -> &'static str {
    "ok"
}

/// This description.
#[utoipa::path(get, path = "/openapi.json", responses((status = OK, description = "The description of RSSifier, OpenAPI 3.1.")))]
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(description())
}
