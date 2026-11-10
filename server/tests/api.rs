mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use rueder::{AppState, router};
use sqlx::PgPool;
use tower::ServiceExt;

async fn get(db: PgPool, static_dir: &std::path::Path, path: &str) -> (StatusCode, String) {
    let app = router(AppState::new(db), static_dir);
    let response = app
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

// #[sqlx::test] creates a fresh database per test and runs ./migrations on it.

#[sqlx::test]
async fn health_counts_feeds(db: PgPool) {
    common::feed(&db, "one").await;
    common::feed(&db, "two").await;
    let (status, body) = get(db, "missing".as_ref(), "/api/health").await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json, serde_json::json!({ "status": "ok", "feeds": 2 }));
}

#[sqlx::test]
async fn unknown_api_route_is_404(db: PgPool) {
    let (status, _) = get(db, "missing".as_ref(), "/api/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn other_paths_get_the_frontend(db: PgPool) {
    let dir = std::env::temp_dir().join(format!("rueder-static-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("_app")).unwrap();
    std::fs::write(dir.join("index.html"), "<title>rueder</title>").unwrap();
    std::fs::write(dir.join("_app/app.js"), "start()").unwrap();

    let (status, body) = get(db.clone(), &dir, "/_app/app.js").await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, "start()"));
    // A route of the single-page app: the app itself answers, so a reload works.
    let (status, body) = get(db, &dir, "/feed/12").await;
    assert_eq!(
        (status, body.as_str()),
        (StatusCode::OK, "<title>rueder</title>")
    );

    std::fs::remove_dir_all(dir).unwrap();
}
