//! What holds for every endpoint: the headers of an answer, the refusal of
//! changes that pages of other sites ask for, and the limits on how much a
//! sender may ask and send.

mod common;

use std::{path::Path, sync::Arc, time::Duration};

use axum::{
    body::Body,
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use rueder::{
    AppState, MAX_BODY,
    http_client::{self, HttpClient},
    images::{self, Config, ImageProxy, Size},
    rate::{LOOKUPS, Rate, Rates},
    subscribe,
};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

const PAGE: &str = "<!doctype html><html><head><script>theme()</script></head>\
    <body><script>start()</script></body></html>";
/// What a browser is told about the two scripts of [`PAGE`].
const THEME: &str = "'sha256-Rfr3GgHZFiXm+TmNr+li+0sZ4J9FGllo8jsXcJF5XZ8='";
const START: &str = "'sha256-DIm7WJS6ZKDYe5qFLPy+h4JFI9Bol5QmYC57mt3Fb00='";

/// A request with these headers, sent to a server with this frontend.
async fn send(
    state: &AppState,
    frontend: &Path,
    method: Method,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<String>,
) -> (StatusCode, HeaderMap, String) {
    let mut request = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let body = match body {
        Some(body) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(body)
        }
        None => Body::empty(),
    };
    let app = rueder::router(state.clone(), frontend);
    let response = app.oneshot(request.body(body).unwrap()).await.unwrap();
    let (status, headers) = (response.status(), response.headers().clone());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

fn frontend(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rueder-hardening-{name}-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("_app")).unwrap();
    std::fs::write(dir.join("index.html"), PAGE).unwrap();
    std::fs::write(dir.join("_app/app.js"), "start()").unwrap();
    dir
}

fn text<'a>(headers: &'a HeaderMap, name: &str) -> &'a str {
    headers
        .get(name)
        .unwrap_or_else(|| panic!("no header {name}"))
        .to_str()
        .unwrap()
}

/// The headers every answer has, whatever it answers.
fn assert_hardened(headers: &HeaderMap, what: &str) {
    for (name, value) in [
        ("x-content-type-options", "nosniff"),
        ("strict-transport-security", "max-age=31536000"),
        ("referrer-policy", "no-referrer"),
        ("x-frame-options", "DENY"),
        ("cross-origin-opener-policy", "same-origin"),
        ("cross-origin-resource-policy", "same-origin"),
    ] {
        assert_eq!(text(headers, name), value, "{what}: {name}");
    }
    assert!(
        text(headers, "permissions-policy").contains("camera=()"),
        "{what}"
    );
    let policy = text(headers, "content-security-policy");
    assert!(policy.starts_with("default-src 'none'"), "{what}: {policy}");
    assert!(
        policy.contains("frame-ancestors 'none'"),
        "{what}: {policy}"
    );
    assert!(!policy.contains("unsafe"), "{what}: {policy}");
}

#[sqlx::test]
async fn every_answer_has_its_headers(db: PgPool) {
    let dir = frontend("headers");
    let ada = common::account(&db, "ada").await;
    let cookie = common::session(&db, ada).await;
    let state = AppState::new(db.clone());
    let get = async |path: &str, cookie: Option<&str>| {
        let headers: Vec<_> = cookie.iter().map(|cookie| ("cookie", *cookie)).collect();
        send(&state, &dir, Method::GET, path, &headers, None).await
    };

    // The frontend: its own files, the two scripts in the page by their
    // hash, and nothing else.
    for path in ["/", "/feed/12", "/index.html"] {
        let (status, headers, body) = get(path, None).await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, PAGE), "{path}");
        assert_hardened(&headers, path);
        assert_eq!(
            text(&headers, "content-security-policy"),
            format!(
                "default-src 'none'; script-src 'self' {THEME} {START}; style-src 'self'; \
                 img-src 'self' data:; font-src 'self'; connect-src 'self'; \
                 media-src http: https:; manifest-src 'self'; base-uri 'none'; \
                 form-action 'self'; frame-ancestors 'none'"
            ),
            "{path}"
        );
        assert!(headers.get(header::CACHE_CONTROL).is_none(), "{path}");
    }
    let (status, headers, _) = get("/_app/app.js", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_hardened(&headers, "a file of the frontend");

    // The API: data, which is no page and stays in no cache. That holds for
    // what it refuses and what it does not know, too.
    for (path, cookie, expected) in [
        ("/api/health", None, StatusCode::OK),
        ("/api/tree", Some(cookie.as_str()), StatusCode::OK),
        ("/api/tree", None, StatusCode::UNAUTHORIZED),
        ("/api/nope", None, StatusCode::NOT_FOUND),
        ("/img/x/thumb/y", None, StatusCode::UNAUTHORIZED),
        ("/metrics", None, StatusCode::NOT_FOUND),
    ] {
        let (status, headers, _) = get(path, cookie).await;
        assert_eq!(status, expected, "{path}");
        assert_hardened(&headers, path);
        assert_eq!(
            text(&headers, "content-security-policy"),
            "default-src 'none'; frame-ancestors 'none'",
            "{path}"
        );
        assert_eq!(text(&headers, "cache-control"), "no-store", "{path}");
    }

    // What an endpoint says itself stays: the live updates and an export.
    let (status, headers, _) = get("/api/opml", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(text(&headers, "content-type"), "text/x-opml; charset=utf-8");
    assert_hardened(&headers, "an export");

    std::fs::remove_dir_all(dir).unwrap();
}

#[sqlx::test]
async fn an_image_keeps_the_headers_of_the_proxy(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let cookie = common::session(&db, ada).await;
    let dir = std::env::temp_dir().join(format!("rueder-hardening-img-{}", std::process::id()));
    let config = Config {
        key: Some(b"a key for tests, long enough to be one".to_vec()),
        ..Config::new(&dir)
    };
    let client = HttpClient::new(http_client::Config::default()).unwrap();
    let images = Arc::new(ImageProxy::open(config, client).unwrap());
    // An image that is in the cache already, so nothing is fetched.
    let source = "https://pictures.example/a.png";
    let size = Size::Thumbnail;
    let picture = images::fit(PNG.to_vec(), size).unwrap();
    images.cache().put(size, source, &picture);
    let address = images.signer().address(size, source).unwrap();
    let state = AppState::new(db.clone()).with_images(images);

    let (status, headers, _) = send(
        &state,
        "missing".as_ref(),
        Method::GET,
        &address,
        &[("cookie", &cookie)],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        text(&headers, "content-security-policy"),
        "default-src 'none'; sandbox"
    );
    assert_eq!(
        text(&headers, "cache-control"),
        "private, max-age=31536000, immutable"
    );
    assert_eq!(text(&headers, "x-content-type-options"), "nosniff");
    assert_eq!(text(&headers, "referrer-policy"), "no-referrer");

    std::fs::remove_dir_all(dir).unwrap();
}

/// A PNG of one pixel.
const PNG: [u8; 67] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

async fn folders(db: &PgPool, account_id: i32) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM folders WHERE account_id = $1")
        .bind(account_id)
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn a_change_asked_for_by_another_site_is_refused(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let cookie = common::session(&db, ada).await;
    let state = AppState::new(db.clone());
    let add_folder = async |headers: &[(&str, &str)]| {
        let mut all = vec![("cookie", cookie.as_str())];
        all.extend_from_slice(headers);
        let body = json!({ "title": "News" }).to_string();
        let path = "/api/folders";
        send(
            &state,
            "missing".as_ref(),
            Method::POST,
            path,
            &all,
            Some(body),
        )
        .await
    };

    // What a browser says about where a request comes from.
    let refused: [&[(&str, &str)]; 9] = [
        &[("sec-fetch-site", "cross-site")],
        &[("sec-fetch-site", "same-site")],
        &[("sec-fetch-site", "something new")],
        // The browser's word counts, not what the page claims.
        &[
            ("sec-fetch-site", "cross-site"),
            ("origin", "https://reader.example"),
            ("host", "reader.example"),
        ],
        // Older browsers only name the page.
        &[
            ("origin", "https://evil.example"),
            ("host", "reader.example"),
        ],
        &[
            ("origin", "https://reader.example.evil.example"),
            ("host", "reader.example"),
        ],
        &[
            ("origin", "http://reader.example:8080"),
            ("host", "reader.example"),
        ],
        // A page in a sandbox, or one that came from a file.
        &[("origin", "null"), ("host", "reader.example")],
        // An origin, and nothing to compare it with.
        &[("origin", "https://reader.example")],
    ];
    for headers in refused {
        let (status, answer, body) = add_folder(headers).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{headers:?}");
        assert_eq!(body, "this request comes from another site", "{headers:?}");
        assert_hardened(&answer, "a refusal");
    }
    assert_eq!(folders(&db, ada).await, 0);

    let allowed: [&[(&str, &str)]; 6] = [
        &[("sec-fetch-site", "same-origin")],
        &[("sec-fetch-site", "none")],
        &[
            ("sec-fetch-site", "same-origin"),
            ("origin", "https://reader.example"),
            ("host", "reader.example"),
        ],
        &[
            ("origin", "https://reader.example"),
            ("host", "reader.example"),
        ],
        &[
            ("origin", "http://localhost:3000"),
            ("host", "localhost:3000"),
        ],
        // No browser: a script that has the cookie.
        &[],
    ];
    for headers in allowed {
        let (status, _, _) = add_folder(headers).await;
        assert_eq!(status, StatusCode::CREATED, "{headers:?}");
    }
    assert_eq!(folders(&db, ada).await, 6);
}

#[sqlx::test]
async fn reading_is_not_a_change_and_logging_out_is(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let cookie = common::session(&db, ada).await;
    let state = AppState::new(db.clone());
    let from_elsewhere = [
        ("cookie", cookie.as_str()),
        ("sec-fetch-site", "cross-site"),
    ];
    let ask = async |method: Method, path: &str| {
        send(
            &state,
            "missing".as_ref(),
            method,
            path,
            &from_elsewhere,
            None,
        )
        .await
        .0
    };

    // A link on another site that leads to rueder opens it.
    assert_eq!(ask(Method::GET, "/api/me").await, StatusCode::OK);
    assert_eq!(ask(Method::HEAD, "/api/health").await, StatusCode::OK);
    // No other site logs a reader out, ends the other sessions or marks
    // everything as read.
    for (method, path) in [
        (Method::POST, "/api/logout"),
        (Method::DELETE, "/api/sessions"),
        (Method::POST, "/api/read"),
        (Method::PUT, "/api/settings"),
        (Method::PATCH, "/api/folders/1"),
        (Method::POST, "/api/login"),
        // Nor anything the API does not know.
        (Method::POST, "/api/nope"),
        (Method::POST, "/"),
    ] {
        assert_eq!(
            ask(method.clone(), path).await,
            StatusCode::FORBIDDEN,
            "{method} {path}"
        );
    }
    assert_eq!(ask(Method::GET, "/api/me").await, StatusCode::OK);
}

#[sqlx::test]
async fn a_sender_can_ask_only_so_much(db: PgPool) {
    let dir = frontend("rate");
    let hour = Duration::from_secs(3600);
    let state = AppState::new(db.clone())
        .with_rates(Rates::new(Rate::new(3, hour), LOOKUPS))
        .with_client_ip_header("x-forwarded-for".parse().unwrap());
    let ask = async |path: &str, from: &str| {
        let from = [("x-forwarded-for", from)];
        send(&state, &dir, Method::GET, path, &from, None).await
    };

    // Whatever is asked for counts: the API, the frontend, what is not there.
    for path in ["/api/health", "/", "/api/nope"] {
        assert_ne!(
            ask(path, "192.0.2.1").await.0,
            StatusCode::TOO_MANY_REQUESTS,
            "{path}"
        );
    }
    for path in ["/api/health", "/", "/img/x/thumb/y"] {
        let (status, headers, body) = ask(path, "192.0.2.1").await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{path}");
        assert_eq!(body, "too many requests, try again later");
        // The next one comes back within the hour.
        let wait: u64 = text(&headers, "retry-after").parse().unwrap();
        assert!((3590..=3600).contains(&wait), "{wait}");
        assert_hardened(&headers, "over the limit");
    }
    // Others are not held up, and what a sender claims in front of the
    // proxy's entry does not make it another.
    assert_eq!(ask("/api/health", "192.0.2.2").await.0, StatusCode::OK);
    let (status, _, _) = ask("/api/health", "198.51.100.7, 192.0.2.1").await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

    std::fs::remove_dir_all(dir).unwrap();
}

#[sqlx::test]
async fn a_request_can_be_only_so_large(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let cookie = common::session(&db, ada).await;
    let state = AppState::new(db.clone());
    let with_cookie = [("cookie", cookie.as_str())];
    let post = async |path: &str, body: String| {
        send(
            &state,
            "missing".as_ref(),
            Method::POST,
            path,
            &with_cookie,
            Some(body),
        )
        .await
        .0
    };

    let large = json!({ "title": "x".repeat(MAX_BODY) }).to_string();
    assert_eq!(
        post("/api/folders", large.clone()).await,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        post("/api/rules/preview", large.clone()).await,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        post("/api/login", large).await,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(folders(&db, ada).await, 0);
    // Just below, it is read, and refused for what it says.
    let long = json!({ "title": "x".repeat(MAX_BODY - 100) }).to_string();
    assert_eq!(post("/api/folders", long).await, StatusCode::BAD_REQUEST);

    // An OPML file may be larger than that, up to its own limit.
    let file = format!(
        "<opml><body><!-- {} --></body></opml>",
        "x".repeat(MAX_BODY * 2)
    );
    assert!(file.len() < subscribe::MAX_OPML);
    let (status, _, _) = common::raw(
        &state,
        Method::POST,
        "/api/opml",
        Some(&cookie),
        Some(("text/x-opml", file.into_bytes())),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}
