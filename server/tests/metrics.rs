//! The metrics for Prometheus (`GET /metrics`, work package 5.25): who gets
//! them, that they are in the text format, that they count, and that they
//! name nobody.

mod common;

use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use rueder::{
    AppState, accounts,
    metrics::{CONTENT_TYPE, Metrics},
    monitor::{Fetched, Severity},
};
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

const TOKEN: &str = "metrics-0123456789-abcdefghijklmnopq";

fn with_metrics(db: PgPool) -> AppState {
    AppState::new(db).with_metrics(Metrics::new(TOKEN).unwrap())
}

/// Asks for the metrics with this `Authorization` and this cookie.
async fn ask(
    state: &AppState,
    authorization: Option<&str>,
    cookie: Option<&str>,
) -> (StatusCode, HeaderMap, String) {
    let mut request = Request::builder().method(Method::GET).uri("/metrics");
    if let Some(authorization) = authorization {
        request = request.header(header::AUTHORIZATION, authorization);
    }
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let app = rueder::router(state.clone(), "missing".as_ref());
    let response = app
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let (status, headers) = (response.status(), response.headers().clone());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

async fn metrics(state: &AppState) -> String {
    let (status, headers, body) = ask(state, Some(&format!("Bearer {TOKEN}")), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(headers[header::CONTENT_TYPE], CONTENT_TYPE);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    let samples = parse(&body);
    assert!(!samples.is_empty());
    body
}

/// The labels of a sample, by name and value.
type Labels = Vec<(String, String)>;
/// A sample: its name and its labels.
type Key = (String, Labels);

/// Reads the text format of Prometheus 0.0.4 as strictly as this server
/// writes it, and returns the samples. Every family has its help and type
/// before its samples, and its samples come together; a histogram's buckets
/// grow and end with `+Inf`, which is its count.
fn parse(text: &str) -> HashMap<Key, f64> {
    let mut samples = HashMap::new();
    let mut families: Vec<(String, String)> = Vec::new();
    let mut seen = HashSet::new();
    let mut helped = None;
    let name_ok = |name: &str| {
        !name.is_empty()
            && !name.starts_with(|c: char| c.is_ascii_digit())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    for line in text.lines() {
        assert!(!line.is_empty(), "no empty lines");
        if let Some(help) = line.strip_prefix("# HELP ") {
            let (name, help) = help.split_once(' ').expect("a help has a text");
            assert!(name_ok(name) && name.starts_with("rueder_"), "{line}");
            assert!(!help.trim().is_empty(), "{line}");
            helped = Some(name.to_owned());
            continue;
        }
        if let Some(kind) = line.strip_prefix("# TYPE ") {
            let (name, kind) = kind.split_once(' ').expect("a type");
            assert_eq!(helped.take().as_deref(), Some(name), "help before type");
            assert!(["counter", "gauge", "histogram"].contains(&kind), "{line}");
            assert!(seen.insert(name.to_owned()), "{name} is one family");
            families.push((name.to_owned(), kind.to_owned()));
            continue;
        }
        assert!(!line.starts_with('#'), "{line}");
        let (family, kind) = families.last().expect("a sample after its type");
        let (key, value) = line.rsplit_once(' ').expect("a sample has a value");
        let value: f64 = value.parse().expect("a number");
        assert!(value.is_finite(), "{line}");
        let (name, labels) = match key.split_once('{') {
            None => (key, Vec::new()),
            Some((name, rest)) => {
                let rest = rest.strip_suffix('}').expect("labels end with }");
                (name, labels_of(rest))
            }
        };
        assert!(name_ok(name), "{line}");
        let belongs = if kind == "histogram" {
            ["_bucket", "_sum", "_count"]
                .iter()
                .any(|end| name.strip_suffix(end) == Some(family.as_str()))
        } else {
            name == family
        };
        assert!(belongs, "{line} is not of {family}");
        if kind == "counter" {
            assert!(name.ends_with("_total"), "{line}");
            assert!(value >= 0.0, "{line}");
        }
        let key = (name.to_owned(), labels);
        assert!(samples.insert(key, value).is_none(), "{line} twice");
    }
    // The buckets of each histogram grow and end with its count.
    for (family, kind) in &families {
        if kind != "histogram" {
            continue;
        }
        let bucket = format!("{family}_bucket");
        let mut series: HashMap<Labels, Vec<(f64, f64)>> = HashMap::new();
        for ((name, labels), value) in &samples {
            if *name != bucket {
                continue;
            }
            let le = labels.iter().find(|(label, _)| label == "le").unwrap();
            let le = if le.1 == "+Inf" {
                f64::INFINITY
            } else {
                le.1.parse().unwrap()
            };
            let others: Vec<_> = labels
                .iter()
                .filter(|(label, _)| label != "le")
                .cloned()
                .collect();
            series.entry(others).or_default().push((le, *value));
        }
        for (labels, mut buckets) in series {
            buckets.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert!(buckets.windows(2).all(|pair| pair[0].1 <= pair[1].1));
            let (last, all) = *buckets.last().unwrap();
            assert_eq!(last, f64::INFINITY);
            let count = samples[&(format!("{family}_count"), labels.clone())];
            assert_eq!(all, count, "{family} {labels:?}");
            assert!(samples.contains_key(&(format!("{family}_sum"), labels)));
        }
    }
    samples
}

/// The labels of a sample, as `a="x",b="y"` with escapes.
fn labels_of(text: &str) -> Labels {
    let mut labels = Vec::new();
    let mut chars = text.chars().peekable();
    while chars.peek().is_some() {
        let name: String = chars.by_ref().take_while(|c| *c != '=').collect();
        assert_eq!(chars.next(), Some('"'), "{text}");
        let mut value = String::new();
        loop {
            match chars.next().expect("a value ends") {
                '"' => break,
                '\\' => match chars.next() {
                    Some('n') => value.push('\n'),
                    Some(c @ ('\\' | '"')) => value.push(c),
                    other => panic!("{other:?} escaped in {text}"),
                },
                c => value.push(c),
            }
        }
        labels.push((name, value));
        if chars.peek() == Some(&',') {
            chars.next();
        }
    }
    labels
}

fn value(samples: &HashMap<Key, f64>, name: &str, labels: &[(&str, &str)]) -> f64 {
    let key = (
        name.to_owned(),
        labels
            .iter()
            .map(|(label, value)| ((*label).to_owned(), (*value).to_owned()))
            .collect(),
    );
    *samples
        .get(&key)
        .unwrap_or_else(|| panic!("no {name} {labels:?}"))
}

#[sqlx::test]
async fn without_a_token_there_are_no_metrics(db: PgPool) {
    let state = AppState::new(db);
    let (status, _, _) = ask(&state, Some(&format!("Bearer {TOKEN}")), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = ask(&state, None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn only_the_token_opens_the_metrics(db: PgPool) {
    let state = with_metrics(db.clone());
    let root = common::account(&db, "root").await;
    accounts::set_admin(&db, "root", true).await.unwrap();
    let admin = common::session(&db, root).await;

    // An app password, as an app would send it, and as a bearer.
    let (status, made) = common::call(
        &state,
        Method::POST,
        "/api/app-passwords",
        Some(&admin),
        Some(json!({ "name": "Prometheus" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    let secret = made["secret"].as_str().unwrap();

    let wrong = format!("Bearer {}x", TOKEN);
    let basic = format!("Basic {TOKEN}");
    let app_bearer = format!("Bearer {secret}");
    let app = format!("GoogleLogin auth={secret}");
    for (authorization, cookie) in [
        (None, None),
        (None, Some(admin.as_str())),
        (Some(wrong.as_str()), None),
        (Some(basic.as_str()), None),
        (Some(TOKEN), None),
        (Some(app_bearer.as_str()), None),
        (Some(app.as_str()), None),
        (Some(wrong.as_str()), Some(admin.as_str())),
    ] {
        let (status, headers, body) = ask(&state, authorization, cookie).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "{authorization:?} {cookie:?}"
        );
        assert_eq!(headers[header::WWW_AUTHENTICATE], "Bearer");
        assert!(!body.contains("rueder_"));
    }
    metrics(&state).await;
}

#[sqlx::test]
async fn the_metrics_count(db: PgPool) {
    let state = with_metrics(db.clone());
    let before = parse(&metrics(&state).await);
    for _ in 0..3 {
        let (status, _) = common::call(&state, Method::GET, "/api/health", None, None).await;
        assert_eq!(status, StatusCode::OK);
    }
    let monitor = &state.monitor;
    let now = Instant::now();
    monitor.fetch(Fetched::Fetched { new: 4 }, Duration::from_millis(30), now);
    monitor.fetch(Fetched::Failed, Duration::from_secs(2), now);
    monitor
        .problems
        .note(Severity::Error, "rueder", "it broke", now);
    monitor
        .problems
        .note(Severity::Warning, "rueder", "it creaked", now);

    let after = parse(&metrics(&state).await);
    let health = [("route", "GET /api/health"), ("class", "2xx")];
    assert_eq!(value(&after, "rueder_http_requests_total", &health), 3.0);
    assert!(!before.contains_key(&(
        "rueder_http_requests_total".to_owned(),
        vec![
            ("route".to_owned(), "GET /api/health".to_owned()),
            ("class".to_owned(), "2xx".to_owned())
        ]
    )));
    // The first scrape counts itself.
    assert_eq!(
        value(
            &after,
            "rueder_http_requests_total",
            &[("route", "GET /metrics"), ("class", "2xx")]
        ),
        1.0
    );
    let route = [("route", "GET /api/health")];
    assert_eq!(
        value(&after, "rueder_http_request_duration_seconds_count", &route),
        3.0
    );
    assert_eq!(
        value(&after, "rueder_http_requests_limited_total", &route),
        0.0
    );
    let fetched = [("result", "fetched")];
    assert_eq!(value(&before, "rueder_fetches_total", &fetched), 0.0);
    assert_eq!(value(&after, "rueder_fetches_total", &fetched), 1.0);
    assert_eq!(
        value(&after, "rueder_fetches_total", &[("result", "failed")]),
        1.0
    );
    assert_eq!(value(&after, "rueder_fetch_new_articles_total", &[]), 4.0);
    // 30 ms are within 0.1 s, 2 s within 6.6 s but not 1.6 s.
    let bucket = "rueder_fetch_duration_seconds_bucket";
    assert_eq!(value(&after, bucket, &[("le", "0.1024")]), 1.0);
    assert_eq!(value(&after, bucket, &[("le", "1.6384")]), 1.0);
    assert_eq!(value(&after, bucket, &[("le", "6.5536")]), 2.0);
    assert_eq!(value(&after, bucket, &[("le", "+Inf")]), 2.0);
    let sum = value(&after, "rueder_fetch_duration_seconds_sum", &[]);
    assert!((sum - 2.03).abs() < 1e-9, "{sum}");
    assert_eq!(
        value(&after, "rueder_log_events_total", &[("severity", "error")]),
        1.0
    );
    assert_eq!(
        value(
            &after,
            "rueder_log_events_total",
            &[("severity", "warning")]
        ),
        1.0
    );
    assert_eq!(value(&after, "rueder_database_up", &[]), 1.0);
    assert_eq!(value(&after, "rueder_fetch_running", &[]), 0.0);
    assert_eq!(
        value(
            &after,
            "rueder_build_info",
            &[
                ("version", rueder::version::VERSION),
                ("commit", rueder::version::build().commit)
            ]
        ),
        1.0
    );
}

#[sqlx::test]
async fn the_database_is_asked_at_most_every_fifteen_seconds(db: PgPool) {
    common::account(&db, "ada").await;
    let state = with_metrics(db.clone());
    let first = parse(&metrics(&state).await);
    assert_eq!(value(&first, "rueder_accounts", &[]), 1.0);
    common::account(&db, "grace").await;
    let again = parse(&metrics(&state).await);
    assert_eq!(value(&again, "rueder_accounts", &[]), 1.0);
}

#[sqlx::test]
async fn the_metrics_name_nobody(db: PgPool) {
    let (reader, failing) = common::reader(&db, "grace-secret", 2).await;
    sqlx::query(
        "UPDATE feeds SET error_message = $2, fetched_at = now(), title = 'Secret Title'
         WHERE id = $1",
    )
    .bind(failing)
    .bind("intranet-secret.example is at 10.0.0.1, which is not on the public internet (private)")
    .execute(&db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO content_reports (account_id, feed_id, reason, note) VALUES ($1, $2, 'spam', 'note-secret')")
        .bind(reader)
        .bind(failing)
        .execute(&db)
        .await
        .unwrap();
    let state = with_metrics(db.clone());
    let cookie = common::session(&db, reader).await;
    let (status, _) = common::call(&state, Method::GET, "/api/tree", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    state.monitor.problems.note(
        Severity::Warning,
        "rueder::fetch",
        "problem-secret at https://feed-secret.example/",
        Instant::now(),
    );

    let text = metrics(&state).await;
    for secret in ["secret", "Secret", "10.0.0.1", "example", "spam"] {
        assert!(!text.contains(secret), "{secret} in the metrics");
    }
    let samples = parse(&text);
    assert_eq!(
        value(&samples, "rueder_feeds_failing", &[("reason", "blocked")]),
        1.0
    );
    assert_eq!(
        value(&samples, "rueder_feeds_reported", &[("blocked", "false")]),
        1.0
    );
    assert_eq!(value(&samples, "rueder_accounts", &[]), 1.0);
    // Labels come from fixed sets.
    let allowed: HashSet<&str> = [
        "route", "class", "result", "severity", "reason", "blocked", "from", "state", "le",
        "version", "commit",
    ]
    .into();
    for (_, labels) in samples.keys() {
        for (label, _) in labels {
            assert!(allowed.contains(label.as_str()), "{label}");
        }
    }
}
