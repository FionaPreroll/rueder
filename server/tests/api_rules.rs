//! Filter rules: what they hide and mark read, when they arrive, change and
//! go, where they hold, and what a preview says.

mod common;

use axum::http::{Method, StatusCode};
use common::call;
use rueder::{AppState, filters};
use serde_json::{Value, json};
use sqlx::PgPool;

async fn get(state: &AppState, path: &str, cookie: &str) -> Value {
    let (status, body) = call(state, Method::GET, path, Some(cookie), None).await;
    assert_eq!(status, StatusCode::OK, "{path}");
    body
}

async fn send(
    state: &AppState,
    method: Method,
    path: &str,
    cookie: &str,
    body: Value,
) -> (StatusCode, Value) {
    let body = (body != Value::Null).then_some(body);
    call(state, method, path, Some(cookie), body).await
}

/// A rule for everything that hides by a word in the title, changed by `over`.
fn rule(over: Value) -> Value {
    let mut rule = json!({
        "scope": { "kind": "all" },
        "field": "title",
        "match": "contains",
        "pattern": "",
        "action": "hide",
    });
    for (key, value) in over.as_object().unwrap() {
        rule[key] = value.clone();
    }
    rule
}

/// Adds a rule and returns its id and what it changed: hidden, shown, read.
async fn add(state: &AppState, cookie: &str, over: Value) -> (i32, [i64; 3]) {
    let (status, made) = send(state, Method::POST, "/api/rules", cookie, rule(over)).await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    (made["id"].as_i64().unwrap() as i32, changed(&made))
}

fn changed(outcome: &Value) -> [i64; 3] {
    ["hidden", "shown", "read"].map(|key| outcome["changed"][key].as_i64().unwrap())
}

fn seqs(list: &Value) -> Vec<i64> {
    let articles = list["articles"].as_array().unwrap().iter();
    articles
        .map(|article| article["seq"].as_i64().unwrap())
        .collect()
}

async fn hidden(state: &AppState, cookie: &str) -> Vec<i64> {
    seqs(&get(state, "/api/hidden", cookie).await)
}

async fn listed(state: &AppState, cookie: &str, feed: i32) -> Vec<i64> {
    seqs(&get(state, &format!("/api/feeds/{feed}/articles"), cookie).await)
}

async fn is_read(state: &AppState, cookie: &str, feed: i32, seq: i64) -> bool {
    let list = get(state, &format!("/api/feeds/{feed}/articles"), cookie).await;
    let articles = list["articles"].as_array().unwrap().iter();
    let article = articles.clone().find(|a| a["seq"] == seq).unwrap();
    article["read"] == true
}

async fn unread(state: &AppState, cookie: &str, feed: i32) -> i64 {
    let tree = get(state, "/api/tree", cookie).await;
    let feeds = tree["feeds"].as_array().unwrap().iter();
    let feed = feeds.clone().find(|f| f["id"] == feed).unwrap();
    feed["unread"].as_i64().unwrap()
}

#[sqlx::test]
async fn a_rule_hides_or_marks_read_what_is_there_and_gives_it_back_when_it_goes(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 5).await;
    let grace = common::account(&db, "grace").await;
    common::subscribe(&db, grace, news).await;
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let path = |id: i32| format!("/api/rules/{id}");

    // A rule hides what it matches of what is there.
    let (hide, did) = add(
        &state,
        &cookie,
        json!({ "match": "word", "pattern": " article 3 " }),
    )
    .await;
    assert_eq!(did, [1, 0, 0]);
    assert_eq!(hidden(&state, &cookie).await, [3]);
    assert_eq!(listed(&state, &cookie, news).await, [5, 4, 2, 1]);
    assert_eq!(unread(&state, &cookie, news).await, 4);

    // Another one marks read, by a tag.
    let (read, did) = add(
        &state,
        &cookie,
        json!({ "field": "tag", "pattern": "TAG-5", "action": "read" }),
    )
    .await;
    assert_eq!(did, [0, 0, 1]);
    assert_eq!(unread(&state, &cookie, news).await, 3);
    assert!(is_read(&state, &cookie, news, 5).await);

    // The rules are listed as they were given, with the pattern trimmed.
    let rules = get(&state, "/api/rules", &cookie).await;
    assert_eq!(
        rules,
        json!([
            { "id": hide, "enabled": true, "scope": { "kind": "all" }, "field": "title",
              "match": "word", "pattern": "article 3", "action": "hide" },
            { "id": read, "enabled": true, "scope": { "kind": "all" }, "field": "tag",
              "match": "contains", "pattern": "TAG-5", "action": "read" },
        ])
    );

    // They are Ada's alone: Grace reads the same feed and sees all of it.
    assert_eq!(get(&state, "/api/rules", &grace_cookie).await, json!([]));
    assert_eq!(hidden(&state, &grace_cookie).await, Vec::<i64>::new());
    assert_eq!(listed(&state, &grace_cookie, news).await, [5, 4, 3, 2, 1]);
    assert_eq!(unread(&state, &grace_cookie, news).await, 5);

    // A changed rule shows what it no longer matches and hides what it does.
    let change = rule(json!({ "match": "regex", "pattern": "^article [24]$" }));
    let (status, did) = send(&state, Method::PUT, &path(hide), &cookie, change).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(changed(&did), [2, 1, 0]);
    assert_eq!(hidden(&state, &cookie).await, [4, 2]);
    assert_eq!(listed(&state, &cookie, news).await, [5, 3, 1]);

    // Switched off, it hides nothing.
    let off = rule(json!({ "match": "regex", "pattern": "^article [24]$", "enabled": false }));
    let (status, did) = send(&state, Method::PUT, &path(hide), &cookie, off).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(changed(&did), [0, 2, 0]);
    assert_eq!(hidden(&state, &cookie).await, Vec::<i64>::new());
    assert_eq!(
        get(&state, "/api/rules", &cookie).await[0]["enabled"],
        false
    );

    // What a rule marked read stays read when the rule goes.
    let (status, did) = send(&state, Method::DELETE, &path(read), &cookie, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(changed(&did), [0, 0, 0]);
    assert_eq!(unread(&state, &cookie, news).await, 4);
    let (status, _) = send(&state, Method::DELETE, &path(read), &cookie, Value::Null).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        get(&state, "/api/rules", &cookie)
            .await
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[sqlx::test]
async fn hiding_wins_over_marking_read(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 3).await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;

    let (hide, did) = add(&state, &cookie, json!({ "pattern": "article 2" })).await;
    assert_eq!(did, [1, 0, 0]);
    let (_, did) = add(
        &state,
        &cookie,
        json!({ "field": "content", "pattern": "of article 2", "action": "read" }),
    )
    .await;
    // The article is hidden, and nobody marked it read behind the reader's back.
    assert_eq!(did, [0, 0, 0]);
    assert_eq!(unread(&state, &cookie, news).await, 2);

    // Once it is no longer hidden, the other rule has its say.
    let path = format!("/api/rules/{hide}");
    let (status, did) = send(&state, Method::DELETE, &path, &cookie, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(changed(&did), [0, 1, 1]);
    assert_eq!(listed(&state, &cookie, news).await, [3, 2, 1]);
    assert_eq!(unread(&state, &cookie, news).await, 2);
}

#[sqlx::test]
async fn a_rule_holds_where_its_scope_says(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 2).await;
    let blog = common::feed(&db, "blog").await;
    let comics = common::feed(&db, "comics").await;
    for feed in [blog, comics] {
        common::subscribe(&db, ada, feed).await;
        common::add(&db, feed, 2).await;
    }
    let outer = common::folder(&db, ada, None, "Outer").await;
    let inner = common::folder(&db, ada, Some(outer), "Inner").await;
    common::move_to(&db, ada, blog, inner).await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let hidden_of = async |feed: i32| 2 - listed(&state, &cookie, feed).await.len();

    // A folder's rule holds for all that is in it, however deep.
    let scope = json!({ "kind": "folder", "id": outer });
    let (_, did) = add(
        &state,
        &cookie,
        json!({ "scope": scope, "pattern": "article" }),
    )
    .await;
    assert_eq!(did, [2, 0, 0]);
    assert_eq!(
        [
            hidden_of(news).await,
            hidden_of(blog).await,
            hidden_of(comics).await
        ],
        [0, 2, 0]
    );

    // A feed's rule holds for the feed.
    let scope = json!({ "kind": "feed", "id": comics });
    let (_, did) = add(
        &state,
        &cookie,
        json!({ "scope": scope, "pattern": "article 1" }),
    )
    .await;
    assert_eq!(did, [1, 0, 0]);
    assert_eq!(
        [
            hidden_of(news).await,
            hidden_of(blog).await,
            hidden_of(comics).await
        ],
        [0, 2, 1]
    );
    let rules = get(&state, "/api/rules", &cookie).await;
    assert_eq!(rules[0]["scope"], json!({ "kind": "folder", "id": outer }));
    assert_eq!(rules[1]["scope"], json!({ "kind": "feed", "id": comics }));

    // Moving a feed into the folder or out of it takes it into the rule or out.
    let drop = |node: i32, target: Value| json!({ "node": { "kind": "feed", "id": node }, "target": target, "position": "inside" });
    let into = drop(news, json!({ "kind": "folder", "id": outer }));
    let (status, _) = send(&state, Method::POST, "/api/tree/move", &cookie, into).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let out = json!({
        "node": { "kind": "feed", "id": blog },
        "target": { "kind": "folder", "id": outer },
        "position": "after",
    });
    let (status, _) = send(&state, Method::POST, "/api/tree/move", &cookie, out).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        [
            hidden_of(news).await,
            hidden_of(blog).await,
            hidden_of(comics).await
        ],
        [2, 0, 1]
    );

    // A rule goes with its folder, and with the subscription to its feed.
    let path = format!("/api/folders/{outer}");
    let (status, _) = send(&state, Method::DELETE, &path, &cookie, Value::Null).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(
        get(&state, "/api/rules", &cookie)
            .await
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(hidden_of(news).await, 0);
    let path = format!("/api/feeds/{comics}");
    let (status, _) = send(&state, Method::DELETE, &path, &cookie, Value::Null).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(get(&state, "/api/rules", &cookie).await, json!([]));
}

#[sqlx::test]
async fn new_articles_meet_the_rules_of_everybody_who_reads_the_feed(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 2).await;
    let grace = common::account(&db, "grace").await;
    common::subscribe(&db, grace, news).await;
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    add(&state, &cookie, json!({ "pattern": "article 4" })).await;
    add(
        &state,
        &grace_cookie,
        json!({ "field": "any", "pattern": "tag-3", "action": "read" }),
    )
    .await;
    // A rule that is switched off, and one for another feed, say nothing.
    add(
        &state,
        &grace_cookie,
        json!({ "pattern": "article 4", "enabled": false }),
    )
    .await;
    let blog = common::feed(&db, "blog").await;
    common::subscribe(&db, grace, blog).await;
    let scope = json!({ "kind": "feed", "id": blog });
    add(
        &state,
        &grace_cookie,
        json!({ "scope": scope, "pattern": "article" }),
    )
    .await;

    // What the fetch does when a feed has news.
    let new: Vec<_> = (3..=5).map(common::article).collect();
    let mut tx = db.begin().await.unwrap();
    let first = rueder::articles::insert(&mut tx, news, &new).await.unwrap();
    filters::apply_new(&mut tx, news, first).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(listed(&state, &cookie, news).await, [5, 3, 2, 1]);
    assert_eq!(hidden(&state, &cookie).await, [4]);
    assert_eq!(unread(&state, &cookie, news).await, 4);
    assert_eq!(listed(&state, &grace_cookie, news).await, [5, 4, 3, 2, 1]);
    assert_eq!(unread(&state, &grace_cookie, news).await, 4);
    assert!(is_read(&state, &grace_cookie, news, 3).await);
}

#[sqlx::test]
async fn a_changed_rule_looks_at_the_newest_articles_and_at_the_hidden_ones(db: PgPool) {
    let more = filters::RECENT as usize + 5;
    let (ada, news) = common::reader(&db, "ada", more).await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;

    let preview = rule(json!({ "pattern": "article" }));
    let (status, said) = send(&state, Method::POST, "/api/rules/preview", &cookie, preview).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(said, json!({ "matches": 200, "of": 200 }));
    // A preview changes nothing and stores nothing.
    assert_eq!(hidden(&state, &cookie).await, Vec::<i64>::new());
    assert_eq!(get(&state, "/api/rules", &cookie).await, json!([]));

    let (id, did) = add(&state, &cookie, json!({ "pattern": "article" })).await;
    assert_eq!(did, [200, 0, 0]);
    assert_eq!(listed(&state, &cookie, news).await, [5, 4, 3, 2, 1]);

    // Later the hidden ones are older than what a rule looks at, and come
    // back all the same once no rule hides them.
    common::add(&db, news, 10).await;
    let path = format!("/api/rules/{id}");
    let (status, did) = send(&state, Method::DELETE, &path, &cookie, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(changed(&did), [0, 200, 0]);
    assert_eq!(unread(&state, &cookie, news).await, more as i64 + 10);
}

#[sqlx::test]
async fn a_preview_counts_what_a_rule_would_catch(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 12).await;
    let blog = common::feed(&db, "blog").await;
    common::subscribe(&db, ada, blog).await;
    common::add(&db, blog, 3).await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let preview = async |over: Value| {
        send(
            &state,
            Method::POST,
            "/api/rules/preview",
            &cookie,
            rule(over),
        )
        .await
    };

    let (status, said) = preview(json!({ "match": "word", "pattern": "article 1" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(said, json!({ "matches": 2, "of": 15 }));
    let (_, said) = preview(json!({ "pattern": "article 1" })).await;
    assert_eq!(said, json!({ "matches": 5, "of": 15 }));
    let scope = json!({ "kind": "feed", "id": news });
    let (_, said) = preview(json!({ "scope": scope, "field": "author", "pattern": "grace" })).await;
    assert_eq!(said, json!({ "matches": 12, "of": 12 }));
    let (_, said) = preview(json!({ "field": "content", "pattern": "<p>" })).await;
    assert_eq!(said, json!({ "matches": 0, "of": 15 }));

    // This is how the interface learns that a pattern is none.
    for broken in [
        json!({ "pattern": "   " }),
        json!({ "pattern": "x".repeat(201) }),
        json!({ "match": "regex", "pattern": "(" }),
        json!({ "match": "regex", "pattern": r"(\pL{50}){50}" }),
    ] {
        let (status, _) = preview(broken.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{broken}");
    }
}

#[sqlx::test]
async fn a_rule_that_breaks_the_rules_is_refused(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 1).await;
    let (grace, blog) = common::reader(&db, "grace", 1).await;
    let theirs = common::folder(&db, grace, None, "Theirs").await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;

    for (broken, status) in [
        (json!({ "pattern": "" }), StatusCode::BAD_REQUEST),
        (
            json!({ "pattern": "x".repeat(201) }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "match": "regex", "pattern": "[" }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "pattern": "x", "field": "link" }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({ "pattern": "x", "match": "glob" }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({ "pattern": "x", "action": "delete" }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            json!({ "pattern": "x", "scope": { "kind": "folder" } }),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        // Nobody's folder, somebody else's folder, somebody else's feed.
        (
            json!({ "pattern": "x", "scope": { "kind": "folder", "id": 999 } }),
            StatusCode::NOT_FOUND,
        ),
        (
            json!({ "pattern": "x", "scope": { "kind": "folder", "id": theirs } }),
            StatusCode::NOT_FOUND,
        ),
        (
            json!({ "pattern": "x", "scope": { "kind": "feed", "id": blog } }),
            StatusCode::NOT_FOUND,
        ),
    ] {
        for path in ["/api/rules", "/api/rules/preview"] {
            let (got, _) = send(&state, Method::POST, path, &cookie, rule(broken.clone())).await;
            assert_eq!(got, status, "{path} {broken}");
        }
    }
    assert_eq!(get(&state, "/api/rules", &cookie).await, json!([]));

    // There is an end to the rules an account can have.
    sqlx::query(
        "INSERT INTO filter_rules (account_id, position, field, match, pattern, action)
         SELECT $1, n, 'title', 'contains', 'nothing like it', 'hide'
         FROM generate_series(1, $2) AS n",
    )
    .bind(ada)
    .bind(filters::MAX_RULES - 1)
    .execute(&db)
    .await
    .unwrap();
    let full = rule(json!({ "pattern": "x" }));
    let (status, _) = send(&state, Method::POST, "/api/rules", &cookie, full.clone()).await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _) = send(&state, Method::POST, "/api/rules", &cookie, full).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(listed(&state, &cookie, news).await, [1]);
}

#[sqlx::test]
async fn rules_can_be_put_in_order(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let mut ids = Vec::new();
    for pattern in ["one", "two", "three", "four"] {
        ids.push(add(&state, &cookie, json!({ "pattern": pattern })).await.0);
    }
    let order = async || {
        let rules = get(&state, "/api/rules", &cookie).await;
        let rules = rules.as_array().unwrap().iter();
        rules
            .map(|rule| rule["pattern"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(order().await, ["one", "two", "three", "four"]);

    // Rules that are not named, or named twice, or not there, break nothing.
    let wanted = json!({ "ids": [ids[3], ids[1], 999, ids[3]] });
    let (status, _) = send(&state, Method::POST, "/api/rules/order", &cookie, wanted).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(order().await, ["four", "two", "one", "three"]);

    // A new rule comes last.
    add(&state, &cookie, json!({ "pattern": "five" })).await;
    assert_eq!(order().await, ["four", "two", "one", "three", "five"]);
    let many = json!({ "ids": vec![1; filters::MAX_RULES as usize + 1] });
    let (status, _) = send(&state, Method::POST, "/api/rules/order", &cookie, many).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn nobody_sees_or_changes_the_rules_of_others(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 3).await;
    let grace = common::account(&db, "grace").await;
    common::subscribe(&db, grace, news).await;
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let (id, _) = add(&state, &cookie, json!({ "pattern": "article 2" })).await;
    let (second, _) = add(&state, &cookie, json!({ "pattern": "nothing" })).await;
    let path = format!("/api/rules/{id}");

    assert_eq!(get(&state, "/api/rules", &grace_cookie).await, json!([]));
    let other = rule(json!({ "pattern": "article" }));
    let (status, _) = send(&state, Method::PUT, &path, &grace_cookie, other).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(&state, Method::DELETE, &path, &grace_cookie, Value::Null).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let wanted = json!({ "ids": [second, id] });
    let (status, _) = send(
        &state,
        Method::POST,
        "/api/rules/order",
        &grace_cookie,
        wanted,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Ada's rules are as they were, and do for Grace nothing.
    let rules = get(&state, "/api/rules", &cookie).await;
    assert_eq!(rules[0]["id"], id);
    assert_eq!(rules[0]["pattern"], "article 2");
    assert_eq!(rules[1]["id"], second);
    assert_eq!(hidden(&state, &cookie).await, [2]);
    assert_eq!(listed(&state, &grace_cookie, news).await, [3, 2, 1]);

    // Without a session there is nothing at all.
    for (method, path) in [
        (Method::GET, "/api/rules".to_owned()),
        (Method::POST, "/api/rules".to_owned()),
        (Method::POST, "/api/rules/order".to_owned()),
        (Method::POST, "/api/rules/preview".to_owned()),
        (Method::PUT, path.clone()),
        (Method::DELETE, path.clone()),
    ] {
        let body = Some(rule(json!({ "pattern": "x" })));
        let (status, _) = call(&state, method.clone(), &path, None, body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {path}");
    }
}
