//! The description of the HTTP interface (OpenAPI, work package 5.23): that
//! every operation in it is described in full, and that the server does what
//! it says about logging in. That every route the server answers is in it
//! follows from how routes are registered (`utoipa_axum::routes!` from each
//! handler's `#[utoipa::path]`; `clippy.toml` forbids any other way), and that
//! the file is current from `tests/api_types.rs`.

use std::{
    collections::{BTreeSet, HashSet},
    path::Path,
};

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use rueder::AppState;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

const METHODS: [&str; 5] = ["get", "post", "put", "patch", "delete"];

/// Every operation of the description: method, path and what it says.
fn operations(description: &Value) -> Vec<(String, String, Value)> {
    let paths = description["paths"].as_object().expect("paths");
    let mut found = Vec::new();
    for (path, item) in paths {
        for method in METHODS {
            if let Some(operation) = item.get(method) {
                found.push((method.to_owned(), path.clone(), operation.clone()));
            }
        }
    }
    found
}

fn description() -> Value {
    serde_json::to_value(rueder::description()).expect("the description is JSON")
}

/// The ways an operation asks to be logged in, by their names.
fn logins(operation: &Value) -> BTreeSet<String> {
    operation["security"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .flat_map(|requirement| requirement.keys().cloned())
        .collect()
}

#[test]
fn every_operation_is_described_in_full() {
    let description = description();
    let tags: HashSet<&str> = description["tags"]
        .as_array()
        .expect("tags")
        .iter()
        .filter_map(|tag| tag["name"].as_str())
        .collect();
    let schemes: HashSet<&str> = description["components"]["securitySchemes"]
        .as_object()
        .expect("the ways to log in")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(schemes, HashSet::from(["session", "app_password"]));

    let operations = operations(&description);
    assert!(operations.len() > 100, "{} operations", operations.len());
    let mut ids = HashSet::new();
    for (method, path, operation) in &operations {
        let what = format!("{} {path}", method.to_uppercase());
        // In a group the description declares, with what it does.
        let tagged = operation["tags"].as_array().expect("tags");
        assert_eq!(tagged.len(), 1, "{what}: one group");
        assert!(
            tags.contains(tagged[0].as_str().unwrap()),
            "{what}: {tagged:?}"
        );
        let summary = operation["summary"].as_str().unwrap_or_default();
        assert!(!summary.trim().is_empty(), "{what}: says what it does");
        // An id of its own, for the links of Swagger UI.
        let id = operation["operationId"].as_str().expect("an id");
        assert!(ids.insert(id.to_owned()), "{what}: the id {id} is taken");
        // What it answers when it works, and a reason with every answer.
        let responses = operation["responses"].as_object().expect("responses");
        assert!(
            responses
                .keys()
                .any(|status| status.starts_with('2') || status.starts_with('3')),
            "{what}: says what it answers when it works"
        );
        for (status, response) in responses {
            let reason = response["description"].as_str().unwrap_or_default();
            assert!(!reason.trim().is_empty(), "{what} {status}: says why");
        }
        // Every part of the path a parameter of its own, described.
        let declared: BTreeSet<String> = operation["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|parameter| parameter["in"] == "path")
            .map(|parameter| parameter["name"].as_str().unwrap().to_owned())
            .collect();
        let in_path: BTreeSet<String> = path
            .split('{')
            .skip(1)
            .map(|part| part.split('}').next().unwrap().to_owned())
            .collect();
        assert_eq!(declared, in_path, "{what}: its path parameters");
        for parameter in operation["parameters"].as_array().into_iter().flatten() {
            let text = parameter["description"].as_str().unwrap_or_default();
            assert!(!text.trim().is_empty(), "{what}: {parameter} is described");
        }
        // Who it is for: the page's API knows the session, the Google
        // Reader API the app password, and nothing knows both.
        let logins = logins(operation);
        assert!(logins.iter().all(|name| schemes.contains(name.as_str())));
        if path.starts_with("/greader/") {
            let open = ["/greader/accounts/ClientLogin", "/greader/img/"];
            if open.iter().any(|start| path.starts_with(start)) {
                assert!(logins.is_empty(), "{what}: for anybody");
            } else {
                assert_eq!(
                    logins,
                    BTreeSet::from(["app_password".to_owned()]),
                    "{what}"
                );
            }
        } else {
            assert!(!logins.contains("app_password"), "{what}");
        }
        if !logins.is_empty() {
            assert!(
                responses.contains_key("401"),
                "{what}: says it needs a login"
            );
        }
    }
}

/// The server is what the description says it is: everything that needs a
/// login answers 401 without one, and nothing else answers 401 unless it says
/// so (a login with wrong credentials).
#[sqlx::test]
async fn every_operation_needs_the_login_it_says(db: PgPool) {
    let state = AppState::new(db);
    let frontend = std::env::temp_dir().join(format!("rueder-openapi-{}", std::process::id()));
    std::fs::create_dir_all(&frontend).unwrap();
    std::fs::write(frontend.join("index.html"), "<!doctype html>").unwrap();
    let app = rueder::router(state, &frontend);

    for (method, path, operation) in operations(&description()) {
        let what = format!("{} {path}", method.to_uppercase());
        let uri = path
            .replace("{*stream}", "feed/1")
            .split('{')
            .enumerate()
            .map(|(at, part)| {
                if at == 0 {
                    part.to_owned()
                } else {
                    format!("1{}", part.split_once('}').unwrap().1)
                }
            })
            .collect::<String>();
        let request = Request::builder()
            .method(Method::from_bytes(method.to_uppercase().as_bytes()).unwrap())
            .uri(&uri)
            .body(Body::empty())
            .unwrap();
        let status = app.clone().oneshot(request).await.unwrap().status();
        let needs_login = !logins(&operation).is_empty();
        if needs_login {
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{what} without a login");
        } else if status == StatusCode::UNAUTHORIZED {
            assert!(
                operation["responses"].get("401").is_some(),
                "{what} answers 401 without saying so"
            );
        }
        // Routed for this method, as described.
        assert_ne!(status, StatusCode::METHOD_NOT_ALLOWED, "{what}");
    }
}

/// Every type in the description has the name `types.ts` gives it, where
/// every name stands once, so no two types of one name hide each other. Only
/// the Google Reader API's own, which the page never reads, are not there;
/// their names begin with `Greader`.
#[test]
fn every_type_has_the_name_the_page_knows() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rueder::typescript::FILE);
    let types = std::fs::read_to_string(path).expect("types.ts");
    let names: HashSet<&str> = types
        .lines()
        .filter_map(|line| line.strip_prefix("export type "))
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    let description = description();
    let schemas = description["components"]["schemas"]
        .as_object()
        .expect("schemas");
    assert!(schemas.len() > 100, "{} schemas", schemas.len());
    for name in schemas.keys() {
        assert!(
            names.contains(name.as_str()) || name.starts_with("Greader"),
            "{name} is neither in types.ts nor the Google Reader API's"
        );
    }
}
