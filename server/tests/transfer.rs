//! Moving an account to another rueder: out as a file, and in again where
//! feeds and articles have other numbers.

mod common;

use std::collections::BTreeSet;

use axum::http::{Method, StatusCode, header};
use common::{START, article, call, raw, session};
use rueder::{
    AppState, articles,
    rate::{Rate, Rates},
    reading, transfer,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use time::Duration;

/// A feed at this address, made up.
async fn feed_at(db: &PgPool, url: &str, title: &str) -> i32 {
    sqlx::query_scalar("INSERT INTO feeds (feed_url, title) VALUES ($1, $2) RETURNING id")
        .bind(url)
        .bind(title)
        .fetch_one(db)
        .await
        .unwrap()
}

/// Stores these made-up articles in a feed, in this order, and says it was
/// fetched.
async fn store(db: &PgPool, feed_id: i32, numbers: &[usize]) {
    let new: Vec<_> = numbers.iter().map(|n| article(*n)).collect();
    let mut tx = db.begin().await.unwrap();
    articles::insert(&mut tx, feed_id, &new).await.unwrap();
    tx.commit().await.unwrap();
    sqlx::query("UPDATE feeds SET last_success_at = now() WHERE id = $1")
        .bind(feed_id)
        .execute(db)
        .await
        .unwrap();
}

async fn send(state: &AppState, method: Method, path: &str, cookie: &str, body: Value) {
    let body = (body != Value::Null).then_some(body);
    let (status, answer) = call(state, method.clone(), path, Some(cookie), body).await;
    assert!(status.is_success(), "{method} {path}: {status} {answer}");
}

async fn subscribe(state: &AppState, cookie: &str, url: &str, folder: Option<i64>) -> i32 {
    // Feeds that are there already are subscribed to without being fetched.
    let body = json!({ "url": url, "folder_id": folder });
    let (status, answer) = call(state, Method::POST, "/api/feeds", Some(cookie), Some(body)).await;
    assert!(status.is_success(), "{status} {answer}");
    answer["id"].as_i64().unwrap() as i32
}

async fn new_folder(state: &AppState, cookie: &str, title: &str, parent: Option<i64>) -> i64 {
    let body = json!({ "title": title, "parent_id": parent });
    let (status, answer) = call(
        state,
        Method::POST,
        "/api/folders",
        Some(cookie),
        Some(body),
    )
    .await;
    assert!(status.is_success(), "{status} {answer}");
    answer["id"].as_i64().unwrap()
}

/// What an account has of a feed, by the GUIDs of the articles: what is read,
/// what is starred, and the labels.
#[derive(Debug, PartialEq, Eq)]
struct Seen {
    read: BTreeSet<String>,
    starred: BTreeSet<String>,
    labelled: BTreeSet<(String, String)>,
}

async fn seen(db: &PgPool, account_id: i32, feed_id: i32) -> Seen {
    let read: Vec<String> = sqlx::query_scalar(
        "SELECT c.guid FROM subscriptions s
         JOIN articles a ON a.feed_id = s.feed_id
         JOIN article_content c ON c.feed_id = a.feed_id AND c.seq = a.seq
         LEFT JOIN article_states st
             ON st.account_id = s.account_id AND st.feed_id = a.feed_id AND st.seq = a.seq
         WHERE s.account_id = $1 AND s.feed_id = $2
           AND CASE WHEN a.seq <= s.read_upto THEN NOT coalesce(st.kept_unread, false)
                    ELSE coalesce(st.read, false) END",
    )
    .bind(account_id)
    .bind(feed_id)
    .fetch_all(db)
    .await
    .unwrap();
    let starred: Vec<String> = sqlx::query_scalar(
        "SELECT c.guid FROM article_states st
         JOIN article_content c ON c.feed_id = st.feed_id AND c.seq = st.seq
         WHERE st.account_id = $1 AND st.feed_id = $2 AND st.starred",
    )
    .bind(account_id)
    .bind(feed_id)
    .fetch_all(db)
    .await
    .unwrap();
    let labelled: Vec<(String, String)> = sqlx::query_as(
        "SELECT l.title, c.guid FROM article_labels al
         JOIN labels l ON l.id = al.label_id
         JOIN article_content c ON c.feed_id = al.feed_id AND c.seq = al.seq
         WHERE l.account_id = $1 AND al.feed_id = $2",
    )
    .bind(account_id)
    .bind(feed_id)
    .fetch_all(db)
    .await
    .unwrap();
    Seen {
        read: read.into_iter().collect(),
        starred: starred.into_iter().collect(),
        labelled: labelled.into_iter().collect(),
    }
}

fn guids(numbers: &[usize]) -> BTreeSet<String> {
    numbers.iter().map(|n| format!("guid-{n}")).collect()
}

fn labelled(pairs: &[(&str, usize)]) -> BTreeSet<(String, String)> {
    let pairs = pairs
        .iter()
        .map(|(label, n)| (label.to_string(), format!("guid-{n}")));
    pairs.collect()
}

const A: &str = "https://a.example/feed.xml";
const B: &str = "https://b.example/feed.xml";
const C: &str = "https://c.example/feed.xml";

/// An account on the first rueder, with folders, three feeds, what was read
/// and starred, labels, rules and settings. Returns its export.
async fn first_rueder(db: &PgPool) -> Value {
    let state = AppState::new(db.clone());
    let ada = common::account(db, "ada").await;
    let cookie = session(db, ada).await;
    let (a, b, c) = (
        feed_at(db, A, "Feed A").await,
        feed_at(db, B, "Feed B").await,
        feed_at(db, C, "Feed C").await,
    );
    store(db, a, &(1..=10).collect::<Vec<_>>()).await;
    store(db, b, &(1..=5).collect::<Vec<_>>()).await;
    store(db, c, &[1, 2, 3]).await;
    let news = new_folder(&state, &cookie, "News", None).await;
    let tech = new_folder(&state, &cookie, "Tech", Some(news)).await;
    subscribe(&state, &cookie, A, Some(tech)).await;
    subscribe(&state, &cookie, B, None).await;
    subscribe(&state, &cookie, C, Some(news)).await;
    send(
        &state,
        Method::PATCH,
        &format!("/api/feeds/{c}"),
        &cookie,
        json!({ "title": "Cee" }),
    )
    .await;
    send(
        &state,
        Method::PUT,
        &format!("/api/feeds/{b}/agent"),
        &cookie,
        json!({ "agent": "browser" }),
    )
    .await;

    // A: read up to 6 but 3, and 8 on top; starred 2 and 9, 5 labelled.
    send(
        &state,
        Method::POST,
        &format!("/api/feeds/{a}/read"),
        &cookie,
        json!({ "upto": 6 }),
    )
    .await;
    let state_of = |feed: i32, seq: i32| format!("/api/feeds/{feed}/articles/{seq}/state");
    send(
        &state,
        Method::PUT,
        &state_of(a, 3),
        &cookie,
        json!({ "read": false }),
    )
    .await;
    send(
        &state,
        Method::PUT,
        &state_of(a, 8),
        &cookie,
        json!({ "read": true }),
    )
    .await;
    send(
        &state,
        Method::PUT,
        &state_of(a, 2),
        &cookie,
        json!({ "starred": true }),
    )
    .await;
    send(
        &state,
        Method::PUT,
        &state_of(a, 9),
        &cookie,
        json!({ "starred": true }),
    )
    .await;
    // B: read up to 2, 4 starred and labelled.
    send(
        &state,
        Method::POST,
        &format!("/api/feeds/{b}/read"),
        &cookie,
        json!({ "upto": 2 }),
    )
    .await;
    send(
        &state,
        Method::PUT,
        &state_of(b, 4),
        &cookie,
        json!({ "starred": true }),
    )
    .await;
    // C: everything read.
    send(
        &state,
        Method::POST,
        &format!("/api/feeds/{c}/read"),
        &cookie,
        json!({ "upto": 3 }),
    )
    .await;

    let (_, keep) = call(
        &state,
        Method::POST,
        "/api/labels",
        Some(&cookie),
        Some(json!({ "title": "Keep", "color": "#112233" })),
    )
    .await;
    let keep = keep["id"].as_i64().unwrap();
    send(
        &state,
        Method::PUT,
        &format!("/api/feeds/{a}/articles/5/labels/{keep}"),
        &cookie,
        Value::Null,
    )
    .await;
    send(
        &state,
        Method::PUT,
        &format!("/api/feeds/{b}/articles/4/labels/{keep}"),
        &cookie,
        Value::Null,
    )
    .await;

    let rule = |scope: Value, field: &str, matching: &str, pattern: &str, action: &str| json!({ "scope": scope, "field": field, "match": matching, "pattern": pattern, "action": action });
    for body in [
        rule(
            json!({ "kind": "folder", "id": news }),
            "title",
            "contains",
            "Werbung",
            "hide",
        ),
        rule(
            json!({ "kind": "feed", "id": b }),
            "author",
            "word",
            "Linus",
            "read",
        ),
        rule(
            json!({ "kind": "all" }),
            "any",
            "regex",
            "^nothing$",
            "hide",
        ),
    ] {
        send(&state, Method::POST, "/api/rules", &cookie, body).await;
    }
    send(
        &state,
        Method::PUT,
        "/api/settings",
        &cookie,
        json!({ "theme": "dark", "density": 2 }),
    )
    .await;

    let (status, headers, file) =
        raw(&state, Method::GET, "/api/export", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "application/json");
    let disposition = headers[header::CONTENT_DISPOSITION].to_str().unwrap();
    assert!(
        disposition.starts_with("attachment; filename=\"rueder-20"),
        "{disposition}"
    );
    serde_json::from_slice(&file).unwrap()
}

/// What the first rueder had is gone, and the second has its own numbers.
async fn forget(db: &PgPool) {
    sqlx::query("TRUNCATE accounts, feeds CASCADE")
        .execute(db)
        .await
        .unwrap();
}

async fn import(state: &AppState, cookie: &str, file: &Value) -> (StatusCode, Value) {
    call(
        state,
        Method::POST,
        "/api/import",
        Some(cookie),
        Some(file.clone()),
    )
    .await
}

#[sqlx::test]
async fn an_account_moves_to_a_rueder_where_numbers_differ(db: PgPool) {
    let file = first_rueder(&db).await;
    assert_eq!(file["format"], "rueder-account");
    assert_eq!(file["version"], 1);
    let kinds: Vec<&str> = file["tree"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["folder", "folder", "feed", "feed", "feed"]);
    let exported_a = &file["tree"][2];
    assert_eq!(exported_a["url"], A);
    assert_eq!(exported_a["folder"], json!(["News", "Tech"]));
    assert_eq!(exported_a["read_upto"]["guid"], "guid-6");
    // Only the starred and labelled articles bring their text.
    let a_articles = exported_a["articles"].as_array().unwrap();
    let with_text: Vec<&str> = a_articles
        .iter()
        .filter(|article| !article["content"].is_null())
        .map(|article| article["guid"].as_str().unwrap())
        .collect();
    assert_eq!(with_text, ["guid-2", "guid-5", "guid-9"]);

    forget(&db).await;
    // The second rueder: other numbers for everything. A has three older
    // articles before the ones of the first and lacks 9 and 10; B is not
    // there at all; C has the articles under other ids, but the same pages.
    for n in 0..5 {
        feed_at(&db, &format!("https://other{n}.example/feed.xml"), "other").await;
    }
    let a = feed_at(&db, A, "Feed A").await;
    store(&db, a, &[101, 102, 103, 1, 2, 3, 4, 5, 6, 7, 8]).await;
    let c = feed_at(&db, C, "Feed C").await;
    let mut tx = db.begin().await.unwrap();
    let moved: Vec<_> = [1, 2, 3]
        .iter()
        .map(|n| rueder::articles::NewArticle {
            guid: format!("elsewhere-{n}"),
            ..article(*n)
        })
        .collect();
    articles::insert(&mut tx, c, &moved).await.unwrap();
    tx.commit().await.unwrap();
    // Somebody here reads A already and has read all of it.
    let bob = common::account(&db, "bob").await;
    let bob_cookie = session(&db, bob).await;
    let state = AppState::new(db.clone());
    subscribe(&state, &bob_cookie, A, None).await;
    send(
        &state,
        Method::POST,
        &format!("/api/feeds/{a}/read"),
        &bob_cookie,
        json!({ "upto": 11 }),
    )
    .await;

    let grace = common::account(&db, "grace").await;
    let cookie = session(&db, grace).await;
    let (status, report) = import(&state, &cookie, &file).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(
        report,
        json!({
            "feeds": 3, "known": 0, "folders": 2,
            // A: 2, 3, 5, 8 found, 9 brought along; B: 4 brought along;
            // C has nothing to say but its mark.
            "articles": 4, "carried": 2, "missing": 0,
            "labels": 1, "rules": 3, "skipped": 0, "settings": true
        })
    );

    // The tree: folders by name, feeds in them, the own name.
    let folders: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT f.title, p.title FROM folders f LEFT JOIN folders p ON p.id = f.parent_id
         WHERE f.account_id = $1 ORDER BY f.id",
    )
    .bind(grace)
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(
        folders,
        [("News".into(), None), ("Tech".into(), Some("News".into()))]
    );
    let subs: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT f.feed_url, d.title, s.title FROM subscriptions s
         JOIN feeds f ON f.id = s.feed_id LEFT JOIN folders d ON d.id = s.folder_id
         WHERE s.account_id = $1 ORDER BY f.feed_url",
    )
    .bind(grace)
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(
        subs,
        [
            (A.into(), Some("Tech".into()), None),
            (B.into(), None, None),
            (C.into(), Some("News".into()), Some("Cee".into())),
        ]
    );
    let (b, agent): (i32, String) =
        sqlx::query_as("SELECT id, fetch_agent FROM feeds WHERE feed_url = $1")
            .bind(B)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(agent, "browser");

    // What was read, starred and labelled, by article and not by number.
    assert_eq!(
        seen(&db, grace, a).await,
        Seen {
            // 101 to 103 came before the mark here; they are as good as read.
            read: guids(&[101, 102, 103, 1, 2, 4, 5, 6, 8]),
            starred: guids(&[2, 9]),
            labelled: labelled(&[("Keep", 5)]),
        }
    );
    assert_eq!(
        seen(&db, grace, c).await,
        Seen {
            read: ["elsewhere-1", "elsewhere-2", "elsewhere-3"]
                .map(String::from)
                .into(),
            starred: BTreeSet::new(),
            labelled: BTreeSet::new(),
        }
    );
    // B was never fetched here: only the starred article is there, and the
    // time of the mark waits for the first fetch.
    assert_eq!(
        seen(&db, grace, b).await,
        Seen {
            read: BTreeSet::new(),
            starred: guids(&[4]),
            labelled: labelled(&[("Keep", 4)]),
        }
    );
    let text: String = sqlx::query_scalar(
        "SELECT c.html FROM article_content c WHERE c.feed_id = $1 AND c.guid = 'guid-4'",
    )
    .bind(b)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(text, "<p>Text of article 4.</p>");
    let mut tx = db.begin().await.unwrap();
    let all: Vec<_> = (1..=5).map(article).collect();
    rueder::feeds::store(&mut tx, b, &all).await.unwrap();
    reading::settle_read_before(&mut tx, b).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(seen(&db, grace, b).await.read, guids(&[1, 2]));
    let waiting: Option<time::OffsetDateTime> = sqlx::query_scalar(
        "SELECT read_before FROM subscriptions WHERE account_id = $1 AND feed_id = $2",
    )
    .bind(grace)
    .bind(b)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(waiting, None);

    // Bob still has nothing unread: what was brought along is old news.
    let counts = reading::unread_counts(&db, bob).await.unwrap();
    assert_eq!(counts.iter().map(|feed| feed.unread).sum::<i64>(), 0);
    // And Bob's file has nothing of Grace's.
    let (_, bobs) = call(&state, Method::GET, "/api/export", Some(&bob_cookie), None).await;
    assert_eq!(bobs["tree"].as_array().unwrap().len(), 1);
    assert_eq!(bobs["tree"][0]["articles"], json!([]));
    assert_eq!(bobs["labels"], json!([]));
    assert_eq!(bobs["rules"], json!([]));
    assert_eq!(bobs["settings"], json!({}));

    // Settings, labels and rules came along, the rules with their scopes.
    let (_, settings) = call(&state, Method::GET, "/api/settings", Some(&cookie), None).await;
    assert_eq!(settings, json!({ "theme": "dark", "density": 2 }));
    let (_, labels) = call(&state, Method::GET, "/api/labels", Some(&cookie), None).await;
    assert_eq!(labels[0]["title"], "Keep");
    assert_eq!(labels[0]["color"], "#112233");
    let rules: Vec<(Option<String>, Option<String>, String)> = sqlx::query_as(
        "SELECT d.title, f.feed_url, r.pattern FROM filter_rules r
         LEFT JOIN folders d ON d.id = r.folder_id LEFT JOIN feeds f ON f.id = r.feed_id
         WHERE r.account_id = $1 ORDER BY r.position",
    )
    .bind(grace)
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(
        rules,
        [
            (Some("News".into()), None, "Werbung".into()),
            (None, Some(B.into()), "Linus".into()),
            (None, None, "^nothing$".into()),
        ]
    );

    // The file of the new rueder says the same, with the numbers of here.
    let (_, again) = call(&state, Method::GET, "/api/export", Some(&cookie), None).await;
    assert_eq!(again["tree"][2]["id"], a);
    assert_eq!(again["tree"][2]["read_upto"]["guid"], "guid-6");
    assert_eq!(again["tree"][2]["read_upto"]["seq"], 9);
}

#[sqlx::test]
async fn an_import_only_adds_and_a_second_one_changes_nothing(db: PgPool) {
    let file = first_rueder(&db).await;
    let state = AppState::new(db.clone());
    // Into the same account on the same rueder: everything is there already.
    let ada: i32 = sqlx::query_scalar("SELECT id FROM accounts WHERE username = 'ada'")
        .fetch_one(&db)
        .await
        .unwrap();
    let cookie = session(&db, ada).await;
    let a: i32 = sqlx::query_scalar("SELECT id FROM feeds WHERE feed_url = $1")
        .bind(A)
        .fetch_one(&db)
        .await
        .unwrap();
    // Meanwhile Ada read more of A and starred 10.
    send(
        &state,
        Method::POST,
        &format!("/api/feeds/{a}/read"),
        &cookie,
        json!({ "upto": 9 }),
    )
    .await;
    send(
        &state,
        Method::PUT,
        &format!("/api/feeds/{a}/articles/10/state"),
        &cookie,
        json!({ "starred": true }),
    )
    .await;
    let before = seen(&db, ada, a).await;
    let (status, report) = import(&state, &cookie, &file).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(
        report,
        json!({
            "feeds": 0, "known": 3, "folders": 0,
            "articles": 6, "carried": 0, "missing": 0,
            "labels": 0, "rules": 0, "skipped": 0, "settings": true
        })
    );
    // The mark did not go back, and nothing was taken away.
    assert_eq!(seen(&db, ada, a).await, before);
    let rules: i64 = sqlx::query_scalar("SELECT count(*) FROM filter_rules WHERE account_id = $1")
        .bind(ada)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(rules, 3);
}

#[sqlx::test]
async fn an_article_that_cannot_be_found_loses_what_was_read_of_it(db: PgPool) {
    let file = first_rueder(&db).await;
    forget(&db).await;
    // Here A only has its newest articles, under other ids and pages, and
    // they all came out later than the mark: none of it is read.
    let a = feed_at(&db, A, "Feed A").await;
    let mut tx = db.begin().await.unwrap();
    let later: Vec<_> = (20..23)
        .map(|n| rueder::articles::NewArticle {
            published_at: START + Duration::days(1) + Duration::minutes(n),
            ..article(n as usize)
        })
        .collect();
    articles::insert(&mut tx, a, &later).await.unwrap();
    tx.commit().await.unwrap();
    let state = AppState::new(db.clone());
    let grace = common::account(&db, "grace").await;
    let cookie = session(&db, grace).await;
    let (status, report) = import(&state, &cookie, &file).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    // Of A, 3 (kept unread) and 8 (read) are lost; 2, 5 and 9 come along.
    assert_eq!(report["missing"], 2);
    assert_eq!(report["carried"], 4);
    let a_seen = seen(&db, grace, a).await;
    assert_eq!(a_seen.read, guids(&[2, 5]));
    assert_eq!(a_seen.starred, guids(&[2, 9]));
}

#[sqlx::test]
async fn a_file_that_breaks_the_rules_is_refused(db: PgPool) {
    let rates = Rates::default().with_imports(Rate::new(100, std::time::Duration::from_secs(1)));
    let state = AppState {
        rates: rates.into(),
        ..AppState::new(db.clone())
    };
    let grace = common::account(&db, "grace").await;
    let cookie = session(&db, grace).await;
    let good =
        json!({ "format": "rueder-account", "version": 1, "exported_at": "2026-11-12T00:00:00Z" });
    let (status, report) = import(&state, &cookie, &good).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["settings"], false);

    let mut wrong = Vec::new();
    for (key, value) in [
        ("format", json!("opml")),
        ("version", json!(2)),
        ("settings", json!([1])),
        ("settings", json!({ "big": "x".repeat(20_000) })),
        ("tree", json!("nothing")),
        (
            "labels",
            json!(
                (0..101)
                    .map(|n| json!({ "title": format!("l{n}"), "color": "#000000" }))
                    .collect::<Vec<_>>()
            ),
        ),
    ] {
        let mut file = good.clone();
        file[key] = value;
        wrong.push(file);
    }
    let feed = |n: usize, articles: Value| json!({ "kind": "feed", "id": n, "url": format!("https://f{n}.example/"), "articles": articles });
    let many: Vec<Value> = (0..2001).map(|n| feed(n, json!([]))).collect();
    let mut file = good.clone();
    file["tree"] = json!(many);
    wrong.push(file);
    let article = |n: usize| json!({ "seq": n, "guid": format!("g{n}"), "url": null, "published_at": "2026-01-01T00:00:00Z" });
    let lots: Vec<Value> = (0..50_001).map(article).collect();
    let mut file = good.clone();
    file["tree"] = json!([feed(1, json!(lots))]);
    wrong.push(file);
    let deep: Vec<Value> = (0..101)
        .map(|n| json!({ "kind": "folder", "path": [format!("f{n}")] }))
        .collect();
    let mut file = good.clone();
    file["tree"] = json!(deep);
    wrong.push(file);
    for file in wrong {
        let (status, answer) = import(&state, &cookie, &file).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    }
    // Nothing of them was taken.
    let feeds: i64 = sqlx::query_scalar("SELECT count(*) FROM subscriptions")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(feeds, 0);

    // What is merely wrong is left out and counted.
    let mut file = good.clone();
    file["tree"] = json!([
        feed(1, json!([article(1), article(1), { "seq": 2, "guid": "", "url": null, "published_at": "2026-01-01T00:00:00Z" }])),
        { "kind": "feed", "id": 2, "url": "javascript:alert(1)" },
        { "kind": "feed", "id": 3, "url": "https://f1.example/" },
        { "kind": "folder", "path": ["a", "b", "c", "d", "e", "f", "g"] },
    ]);
    file["labels"] =
        json!([{ "title": "", "color": "#000000" }, { "title": "ok", "color": "red" }]);
    file["rules"] = json!([
        { "enabled": true, "scope": { "kind": "all" }, "field": "any", "match": "regex", "pattern": "(", "action": "hide" },
        { "enabled": true, "scope": { "kind": "folder", "path": ["nowhere"] }, "field": "any", "match": "contains", "pattern": "x", "action": "hide" },
    ]);
    let (status, report) = import(&state, &cookie, &file).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["feeds"], 1);
    // Five folders deep at most.
    assert_eq!(report["folders"], 5);
    assert_eq!(report["skipped"], 8);

    // Not JSON, not said to be JSON, too large, not logged in.
    let (status, ..) = raw(
        &state,
        Method::POST,
        "/api/import",
        Some(&cookie),
        Some(("application/json", b"{".to_vec())),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body = good.to_string().into_bytes();
    let (status, ..) = raw(
        &state,
        Method::POST,
        "/api/import",
        Some(&cookie),
        Some(("text/plain", body.clone())),
    )
    .await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let huge = vec![b' '; transfer::MAX_FILE + 1];
    let (status, ..) = raw(
        &state,
        Method::POST,
        "/api/import",
        Some(&cookie),
        Some(("application/json", huge)),
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    let (status, ..) = raw(
        &state,
        Method::POST,
        "/api/import",
        None,
        Some(("application/json", body)),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, ..) = raw(&state, Method::GET, "/api/export", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn an_account_cannot_import_without_end(db: PgPool) {
    let rates = Rates::default().with_imports(Rate::new(2, std::time::Duration::from_secs(3600)));
    let state = AppState {
        rates: rates.into(),
        ..AppState::new(db.clone())
    };
    let grace = common::account(&db, "grace").await;
    let cookie = session(&db, grace).await;
    let bob = common::account(&db, "bob").await;
    let bob_cookie = session(&db, bob).await;
    let file =
        json!({ "format": "rueder-account", "version": 1, "exported_at": "2026-11-12T00:00:00Z" });
    for _ in 0..2 {
        assert_eq!(import(&state, &cookie, &file).await.0, StatusCode::OK);
    }
    let (status, headers, _) = raw(
        &state,
        Method::POST,
        "/api/import",
        Some(&cookie),
        Some(("application/json", file.to_string().into_bytes())),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(headers.contains_key(header::RETRY_AFTER));
    // Another account has its own.
    assert_eq!(import(&state, &bob_cookie, &file).await.0, StatusCode::OK);
}

#[sqlx::test]
async fn the_command_line_moves_an_account_too(db: PgPool) {
    let file = first_rueder(&db).await;
    let ada: i32 = sqlx::query_scalar("SELECT id FROM accounts WHERE username = 'ada'")
        .fetch_one(&db)
        .await
        .unwrap();
    let exported = transfer::export(&db, ada).await.unwrap();
    let exported = serde_json::to_value(&exported).unwrap();
    // The same as the API's, but for the time of the export.
    let strip = |mut file: Value| {
        file["exported_at"] = Value::Null;
        file
    };
    assert_eq!(strip(exported.clone()), strip(file));
    forget(&db).await;
    let grace = common::account(&db, "grace").await;
    let rules = rueder::cleaning::Rules::built_in().unwrap();
    let file: transfer::Export = serde_json::from_value(exported).unwrap();
    let Ok(imported) = transfer::import(&db, &rules, grace, file).await else {
        panic!("the import failed");
    };
    assert_eq!(imported.feeds, 3);
    // Not a single article was here, so the starred and labelled ones came.
    assert_eq!(imported.carried, 4);
    assert!(imported.to_string().contains("3 new, 0 known"));
}
