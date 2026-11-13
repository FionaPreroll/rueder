//! The Google Reader API for apps of other makers, along the ways NetNewsWire
//! and Read You log in, sync and change (see their `ReaderAPICaller.swift`
//! and `GoogleReaderAPI.kt`): what they send and what they read back.

mod common;

use std::{collections::HashSet, sync::Arc, time::Duration};

use axum::{
    body::Body,
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use rueder::{
    AppState, http_client,
    http_client::HttpClient,
    images::{Config, ImageProxy},
    rate::{Rate, Rates},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

const HOST: &str = "rueder.example";

/// Asks the API as an app: a form body, the token in the header.
async fn ask(
    state: &AppState,
    method: Method,
    path: &str,
    token: Option<&str>,
    form: &str,
) -> (StatusCode, HeaderMap, String) {
    let mut request = Request::builder()
        .method(method.clone())
        .uri(format!("/greader{path}"))
        .header(header::HOST, HOST);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("GoogleLogin auth={token}"));
    }
    let body = if method == Method::POST {
        request = request.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
        Body::from(form.to_owned())
    } else {
        Body::empty()
    };
    let app = rueder::router(state.clone(), "missing".as_ref());
    let response = app.oneshot(request.body(body).unwrap()).await.unwrap();
    let (status, headers) = (response.status(), response.headers().clone());
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

async fn get(state: &AppState, path: &str, token: &str) -> Value {
    let (status, _, body) = ask(state, Method::GET, path, Some(token), "").await;
    assert_eq!(status, StatusCode::OK, "{path}: {body}");
    serde_json::from_str(&body).unwrap()
}

async fn post(state: &AppState, path: &str, token: &str, form: &str) -> (StatusCode, String) {
    let (status, _, body) = ask(state, Method::POST, path, Some(token), form).await;
    (status, body)
}

/// Makes an app password for the account, as the page does, and logs in
/// with it as an app; returns the token.
async fn logged_in(state: &AppState, account: i32, name: &str) -> String {
    let cookie = common::session(&state.db, account).await;
    let (status, made) = common::call(
        state,
        Method::POST,
        "/api/app-passwords",
        Some(&cookie),
        Some(json!({ "name": "App" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{made}");
    let secret = made["secret"].as_str().unwrap();
    let form = format!("Email={name}&Passwd={secret}");
    let (status, _, body) = ask(state, Method::POST, "/accounts/ClientLogin", None, &form).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let auth = body
        .lines()
        .find_map(|line| line.strip_prefix("Auth="))
        .unwrap();
    auth.to_owned()
}

fn ids(list: &Value) -> Vec<String> {
    list["itemRefs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap().to_owned())
        .collect()
}

fn form_of_ids(ids: &[String]) -> String {
    ids.iter()
        .map(|id| format!("i={id}"))
        .collect::<Vec<_>>()
        .join("&")
}

#[sqlx::test]
async fn an_app_logs_in_with_an_app_password_only(db: PgPool) {
    let ada = rueder::accounts::create(&db, "ada", Some("correct horse battery"), false)
        .await
        .unwrap()
        .unwrap();
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let (_, made) = common::call(
        &state,
        Method::POST,
        "/api/app-passwords",
        Some(&cookie),
        Some(json!({ "name": "Read You", "password": "correct horse battery" })),
    )
    .await;
    let secret = made["secret"].as_str().unwrap();
    let login = |form: String| {
        let state = state.clone();
        async move { ask(&state, Method::POST, "/accounts/ClientLogin", None, &form).await }
    };

    // Not with the account's password, another name, or a wrong secret.
    for form in [
        "Email=ada&Passwd=correct+horse+battery".to_owned(),
        format!("Email=grace&Passwd={secret}"),
        "Email=ada&Passwd=aaaa-bbbb-cccc-dddd-eeee-ffff".to_owned(),
    ] {
        let (status, _, body) = login(form).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(body.contains("BadAuthentication"));
    }

    // As NetNewsWire reads it: lines, Auth after the first `=`.
    let (status, _, body) = login(format!("Email=ADA&Passwd={secret}")).await;
    assert_eq!(status, StatusCode::OK);
    let auth = body
        .lines()
        .find_map(|line| line.strip_prefix("Auth="))
        .unwrap();
    assert!(!auth.contains('='));
    // As Read You asks: JSON.
    let (_, _, body) = login(format!(
        "output=json&Email=ada&Passwd={secret}&client=ReadYou"
    ))
    .await;
    let tokens: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(tokens["Auth"], auth);

    let me = get(&state, "/reader/api/0/user-info", auth).await;
    assert_eq!(me["userName"], "ada");
    assert_eq!(me["userId"], ada.to_string());
    let (status, _, token) = ask(&state, Method::GET, "/reader/api/0/token", Some(auth), "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!token.is_empty() && !token.contains(auth));

    // Without it, or with a made-up one, nothing; Google Reader's header says
    // to log in again.
    for token in [None, Some("aaaabbbbccccddddeeeeffff"), Some("nonsense")] {
        let (status, headers, _) =
            ask(&state, Method::GET, "/reader/api/0/user-info", token, "").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(headers["x-reader-google-bad-token"], "true");
    }
    // What rueder does not have answers empty, to an app that logged in.
    assert_eq!(
        get(&state, "/reader/api/0/preference/stream/list", auth).await,
        json!([])
    );

    // APP_API=off: none of it is there.
    let off = AppState::new(db.clone()).without_app_api();
    let (status, _, _) = ask(&off, Method::GET, "/reader/api/0/user-info", Some(auth), "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = ask(
        &off,
        Method::POST,
        "/accounts/ClientLogin",
        None,
        &format!("Email=ada&Passwd={secret}"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// An app password that is revoked: NetNewsWire then logs in again with it
/// and, when that fails too, sends every request as a form with the name and
/// the app password (`readerBasic` in its `URLRequest+ReaderAPI.swift`).
/// Whatever it sends, it is told to log in again, not that the method is
/// wrong.
#[sqlx::test]
async fn a_revoked_app_password_is_told_to_log_in_again(db: PgPool) {
    let (ada, _) = common::reader(&db, "ada", 2).await;
    let state = AppState::new(db.clone());
    let auth = logged_in(&state, ada, "ada").await;
    get(&state, "/reader/api/0/subscription/list?output=json", &auth).await;
    let cookie = common::session(&db, ada).await;
    let (_, list) = common::call(
        &state,
        Method::GET,
        "/api/app-passwords",
        Some(&cookie),
        None,
    )
    .await;
    let path = format!("/api/app-passwords/{}", list["app_passwords"][0]["id"]);
    let (status, _) = common::call(&state, Method::DELETE, &path, Some(&cookie), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let reads = [
        "/reader/api/0/token",
        "/reader/api/0/user-info",
        "/reader/api/0/subscription/list?output=json",
        "/reader/api/0/tag/list?output=json",
        "/reader/api/0/unread-count?output=json",
        "/reader/api/0/stream/items/ids?output=json&s=user/-/state/com.google/reading-list",
        "/reader/api/0/stream/contents/user/-/state/com.google/reading-list",
        "/reader/api/0/stream/contents?s=user/-/state/com.google/reading-list",
        "/reader/api/0/preference/stream/list",
    ];
    let basic = format!("Email=ada&Passwd={auth}");
    for path in reads {
        for (method, token, form) in [
            (Method::GET, Some(auth.as_str()), ""),
            (Method::POST, Some(auth.as_str()), ""),
            (Method::POST, None, basic.as_str()),
        ] {
            let (status, headers, _) = ask(&state, method.clone(), path, token, form).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {path}");
            assert_eq!(headers["x-reader-google-bad-token"], "true");
        }
    }
    let (status, _, body) = ask(&state, Method::POST, "/accounts/ClientLogin", None, &basic).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body.contains("BadAuthentication"));
}

#[sqlx::test]
async fn an_app_syncs_feeds_folders_counts_and_items(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 3).await;
    let news = common::folder(&db, ada, None, "News").await;
    let tech = common::folder(&db, ada, Some(news), "Tech/IT").await;
    common::move_to(&db, ada, feed, tech).await;
    let state = AppState::new(db.clone());
    let token = logged_in(&state, ada, "ada").await;

    let list = get(
        &state,
        "/reader/api/0/subscription/list?output=json",
        &token,
    )
    .await;
    let subscription = &list["subscriptions"][0];
    assert_eq!(subscription["id"], format!("feed/{feed}"));
    assert_eq!(subscription["title"], "ada");
    // A `/` in a folder's name stands as U+2215 in its path.
    let path = "News/Tech\u{2215}IT";
    assert_eq!(subscription["categories"][0]["label"], path);
    assert_eq!(
        subscription["categories"][0]["id"],
        format!("user/-/label/{path}")
    );

    let tags = get(&state, "/reader/api/0/tag/list?output=json", &token).await;
    let names: Vec<&str> = tags["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tag| tag["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "user/-/state/com.google/starred",
            "user/-/label/News",
            "user/-/label/News/Tech\u{2215}IT",
        ]
    );

    let counts = get(&state, "/reader/api/0/unread-count?output=json", &token).await;
    let count = |id: &str| {
        counts["unreadcounts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|count| count["id"] == id)
            .map(|count| count["count"].clone())
    };
    assert_eq!(count(&format!("feed/{feed}")), Some(json!(3)));
    assert_eq!(count("user/-/label/News"), Some(json!(3)));
    assert_eq!(
        count("user/-/state/com.google/reading-list"),
        Some(json!(3))
    );

    // NetNewsWire: the unread IDs, then their texts.
    let unread = get(
        &state,
        "/reader/api/0/stream/items/ids?n=1000&output=json&s=user/-/state/com.google/reading-list&xt=user/-/state/com.google/read",
        &token,
    )
    .await;
    let unread = ids(&unread);
    assert_eq!(unread.len(), 3);
    let (status, body) = post(
        &state,
        "/reader/api/0/stream/items/contents",
        &token,
        &format!("T=x&output=json&{}", form_of_ids(&unread)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let contents: Value = serde_json::from_str(&body).unwrap();
    assert!(contents["updated"].is_i64());
    let items = contents["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    let newest = &items[0];
    assert_eq!(newest["title"], "Article 3");
    assert!(
        newest["id"]
            .as_str()
            .unwrap()
            .starts_with("tag:google.com,2005:reader/item/")
    );
    assert_eq!(newest["origin"]["streamId"], format!("feed/{feed}"));
    assert_eq!(newest["summary"]["content"], "<p>Text of article 3.</p>");
    assert_eq!(newest["alternate"][0]["href"], "https://blog.example/3");
    let categories: HashSet<&str> = newest["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert!(categories.contains("user/-/state/com.google/reading-list"));
    assert!(categories.contains("user/-/label/News/Tech\u{2215}IT"));
    assert!(!categories.contains("user/-/state/com.google/read"));

    // The same through a folder's stream, and in pages of two.
    let first = get(
        &state,
        "/reader/api/0/stream/items/ids?output=json&n=2&s=user/-/label/News",
        &token,
    )
    .await;
    assert_eq!(ids(&first), unread[..2]);
    let c = first["continuation"].as_str().unwrap();
    let rest = get(
        &state,
        &format!("/reader/api/0/stream/items/ids?output=json&n=2&s=user/-/label/News&c={c}"),
        &token,
    )
    .await;
    assert_eq!(ids(&rest), unread[2..]);
    assert!(rest.get("continuation").is_none());
    // Oldest first.
    let oldest = get(
        &state,
        &format!("/reader/api/0/stream/items/ids?output=json&r=o&s=feed/{feed}"),
        &token,
    )
    .await;
    let mut reversed = unread.clone();
    reversed.reverse();
    assert_eq!(ids(&oldest), reversed);
    // A page with texts.
    let page = get(
        &state,
        &format!("/reader/api/0/stream/contents/feed/{feed}?n=1"),
        &token,
    )
    .await;
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert!(page["continuation"].is_string());
}

#[sqlx::test]
async fn others_see_nothing_and_change_nothing(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 2).await;
    let (grace, _) = common::reader(&db, "grace", 1).await;
    let state = AppState::new(db.clone());
    let ours = logged_in(&state, ada, "ada").await;
    let theirs = logged_in(&state, grace, "grace").await;
    let all = get(&state, "/reader/api/0/stream/items/ids?output=json", &ours).await;
    let ours_ids = ids(&all);
    assert_eq!(ours_ids.len(), 2);

    let (status, _, _) = ask(
        &state,
        Method::GET,
        &format!("/reader/api/0/stream/items/ids?output=json&s=feed/{feed}"),
        Some(&theirs),
        "",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, body) = post(
        &state,
        "/reader/api/0/stream/items/contents",
        &theirs,
        &form_of_ids(&ours_ids),
    )
    .await;
    let contents: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(contents["items"], json!([]));
    let (status, _) = post(
        &state,
        "/reader/api/0/edit-tag",
        &theirs,
        &format!(
            "{}&a=user/-/state/com.google/read&a=user/-/state/com.google/starred",
            form_of_ids(&ours_ids)
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let counts = get(&state, "/reader/api/0/unread-count", &ours).await;
    assert!(counts.to_string().contains(&format!("\"feed/{feed}\"")));
    let unread = get(
        &state,
        "/reader/api/0/stream/items/ids?output=json&xt=user/-/state/com.google/read",
        &ours,
    )
    .await;
    assert_eq!(ids(&unread).len(), 2);
    let starred = get(
        &state,
        "/reader/api/0/stream/items/ids?output=json&s=user/-/state/com.google/starred",
        &ours,
    )
    .await;
    assert_eq!(ids(&starred).len(), 0);
    for path in [
        "/reader/api/0/subscription/list",
        "/reader/api/0/tag/list",
        "/reader/api/0/unread-count",
    ] {
        let theirs = get(&state, path, &theirs).await.to_string();
        assert!(!theirs.contains(&format!("feed/{feed}\"")), "{path}");
    }
    let (status, _) = post(
        &state,
        "/reader/api/0/subscription/edit",
        &theirs,
        &format!("ac=unsubscribe&s=feed/{feed}"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = post(
        &state,
        "/reader/api/0/mark-all-as-read",
        &theirs,
        &format!("s=feed/{feed}"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn an_app_marks_read_unread_starred_and_labels(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 5).await;
    let state = AppState::new(db.clone());
    let token = logged_in(&state, ada, "ada").await;
    let all = ids(&get(&state, "/reader/api/0/stream/items/ids?output=json", &token).await);
    let unread_ids = |state: AppState, token: String| async move {
        ids(&get(
            &state,
            "/reader/api/0/stream/items/ids?output=json&xt=user/-/state/com.google/read",
            &token,
        )
        .await)
    };

    // Read, as NetNewsWire sends it.
    let (status, body) = post(
        &state,
        "/reader/api/0/edit-tag",
        &token,
        &format!(
            "T=x&{}&a=user/-/state/com.google/read",
            form_of_ids(&all[1..4])
        ),
    )
    .await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, "OK"));
    assert_eq!(
        unread_ids(state.clone(), token.clone()).await,
        [all[0].clone(), all[4].clone()]
    );
    // Unread again, as Read You sends it, in the long form.
    let long = format!(
        "tag:google.com,2005:reader/item/{:016x}",
        all[2].parse::<i64>().unwrap()
    );
    let (status, _) = post(
        &state,
        "/reader/api/0/edit-tag",
        &token,
        &format!("i={long}&r=user/-/state/com.google/read"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(unread_ids(state.clone(), token.clone()).await.len(), 3);
    // The page counts the same.
    let cookie = common::session(&db, ada).await;
    let (_, tree) = common::call(&state, Method::GET, "/api/tree", Some(&cookie), None).await;
    assert_eq!(tree["feeds"][0]["unread"], 3);

    // A star, and a label that is not there yet.
    let (status, _) = post(
        &state,
        "/reader/api/0/edit-tag",
        &token,
        &format!(
            "{}&a=user/-/state/com.google/starred&a=user/-/label/Later",
            form_of_ids(&all[..2])
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let starred = get(
        &state,
        "/reader/api/0/stream/items/ids?output=json&s=user/-/state/com.google/starred",
        &token,
    )
    .await;
    assert_eq!(ids(&starred), all[..2]);
    let later = get(
        &state,
        "/reader/api/0/stream/items/ids?output=json&s=user/-/label/Later",
        &token,
    )
    .await;
    assert_eq!(ids(&later), all[..2]);
    let (_, labels) = common::call(&state, Method::GET, "/api/labels", Some(&cookie), None).await;
    assert_eq!(labels[0]["title"], "Later");
    let (status, _) = post(
        &state,
        "/reader/api/0/edit-tag",
        &token,
        &format!(
            "i={}&r=user/-/label/Later&r=user/-/state/com.google/starred",
            all[0]
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let later = get(
        &state,
        "/reader/api/0/stream/items/ids?output=json&s=user/-/label/Later",
        &token,
    )
    .await;
    assert_eq!(ids(&later), all[1..2]);

    // Added and taken away at once is no change.
    let (status, _) = post(
        &state,
        "/reader/api/0/edit-tag",
        &token,
        &format!(
            "i={}&a=user/-/state/com.google/read&r=user/-/state/com.google/read",
            all[0]
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // Everything of the feed as read.
    let (status, _) = post(
        &state,
        "/reader/api/0/mark-all-as-read",
        &token,
        &format!("s=feed/{feed}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(unread_ids(state.clone(), token.clone()).await.is_empty());
}

#[sqlx::test]
async fn what_arrived_after_a_time_is_told_apart(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 2).await;
    // The first two arrived an hour ago.
    sqlx::query("UPDATE arrivals SET at = at - interval '1 hour'")
        .execute(&db)
        .await
        .unwrap();
    common::add(&db, feed, 2).await;
    let state = AppState::new(db.clone());
    let token = logged_in(&state, ada, "ada").await;
    let half_an_hour_ago = time::OffsetDateTime::now_utc().unix_timestamp() - 1800;
    let all = ids(&get(&state, "/reader/api/0/stream/items/ids?output=json", &token).await);
    assert_eq!(all.len(), 4);

    let since = get(
        &state,
        &format!("/reader/api/0/stream/items/ids?output=json&ot={half_an_hour_ago}"),
        &token,
    )
    .await;
    assert_eq!(ids(&since), all[..2]);
    let before = get(
        &state,
        &format!("/reader/api/0/stream/items/ids?output=json&nt={half_an_hour_ago}"),
        &token,
    )
    .await;
    assert_eq!(ids(&before), all[2..]);
    // Read You asks for what is read since a time.
    let read = get(
        &state,
        &format!(
            "/reader/api/0/stream/items/ids?output=json&s=user/-/state/com.google/read&ot={half_an_hour_ago}"
        ),
        &token,
    )
    .await;
    assert!(ids(&read).is_empty());

    // All as read up to then, in microseconds as Google Reader sent it: what
    // came after stays unread.
    let (status, _) = post(
        &state,
        "/reader/api/0/mark-all-as-read",
        &token,
        &format!(
            "s=user/-/state/com.google/reading-list&ts={}",
            half_an_hour_ago * 1_000_000
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let unread = get(
        &state,
        "/reader/api/0/stream/items/ids?output=json&xt=user/-/state/com.google/read",
        &token,
    )
    .await;
    assert_eq!(ids(&unread), all[..2]);
}

#[sqlx::test]
async fn an_app_edits_subscriptions_folders_and_labels(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 1).await;
    let state = AppState::new(db.clone());
    let token = logged_in(&state, ada, "ada").await;
    let cookie = common::session(&db, ada).await;
    let tree = |state: AppState, cookie: String| async move {
        common::call(&state, Method::GET, "/api/tree", Some(&cookie), None)
            .await
            .1
    };

    // Renamed and moved into a folder that is made on the way.
    let (status, body) = post(
        &state,
        "/reader/api/0/subscription/edit",
        &token,
        &format!("ac=edit&s=feed/{feed}&t=Mine&a=user/-/label/Reading/Later"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let now = tree(state.clone(), cookie.clone()).await;
    assert_eq!(now["feeds"][0]["title"], "Mine");
    let folders = now["folders"].as_array().unwrap();
    assert_eq!(folders.len(), 2);
    let later = folders.iter().find(|f| f["title"] == "Later").unwrap();
    assert_eq!(now["feeds"][0]["folder_id"], later["id"]);

    // Back to the top, and the folder renamed and taken out.
    let (status, _) = post(
        &state,
        "/reader/api/0/rename-tag",
        &token,
        "s=user/-/label/Reading/Later&dest=user/-/label/Reading/Soon",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = post(
        &state,
        "/reader/api/0/rename-tag",
        &token,
        "s=user/-/label/Reading/Soon&dest=user/-/label/Elsewhere/Soon",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = post(
        &state,
        "/reader/api/0/subscription/edit",
        &token,
        &format!("ac=edit&s=feed/{feed}&r=user/-/label/Reading/Soon"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = post(
        &state,
        "/reader/api/0/disable-tag",
        &token,
        "s=user/-/label/Reading",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let now = tree(state.clone(), cookie.clone()).await;
    assert_eq!(now["feeds"][0]["folder_id"], Value::Null);
    let titles: Vec<&str> = now["folders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["title"].as_str().unwrap())
        .collect();
    assert_eq!(titles, ["Soon"]);

    // Unsubscribed.
    let (status, _) = post(
        &state,
        "/reader/api/0/subscription/edit",
        &token,
        &format!("ac=unsubscribe&s=feed/{feed}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(tree(state.clone(), cookie).await["feeds"], json!([]));
    // Subscribing to something new needs the server to fetch it, which this
    // state cannot: a refusal, not a failure.
    let (status, _) = post(
        &state,
        "/reader/api/0/subscription/quickadd",
        &token,
        "quickadd=https://elsewhere.example/feed.xml",
    )
    .await;
    assert!(status.is_client_error() || status.is_server_error());
    assert_ne!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[sqlx::test]
async fn changes_have_limits(db: PgPool) {
    let (ada, _) = common::reader(&db, "ada", 1).await;
    let state = AppState::new(db.clone())
        .with_rates(Rates::default().with_app_changes(Rate::new(2, Duration::from_secs(3600))));
    let token = logged_in(&state, ada, "ada").await;

    let too_many: Vec<String> = (1..=1001).map(|n| (n + (1u64 << 32)).to_string()).collect();
    let (status, _) = post(
        &state,
        "/reader/api/0/edit-tag",
        &token,
        &format!("{}&a=user/-/state/com.google/read", form_of_ids(&too_many)),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = post(
        &state,
        "/reader/api/0/stream/items/contents",
        &token,
        &form_of_ids(&too_many[..251]),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = post(
        &state,
        "/reader/api/0/edit-tag",
        &token,
        "i=1&a=user/-/state/com.google/nothing-rueder-has",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = post(
        &state,
        "/reader/api/0/edit-tag",
        &token,
        "i=1&a=user/-/state/com.google/read",
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    // Reading is not counted.
    get(&state, "/reader/api/0/stream/items/ids?output=json", &token).await;
    // Parameters beyond their limits are refused.
    let (status, _) = post(
        &state,
        "/reader/api/0/stream/items/contents",
        &token,
        &"i=1&".repeat(3000),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// A state with an image proxy, which these tests never let fetch anything.
fn with_images(db: &PgPool) -> AppState {
    let dir = std::env::temp_dir().join(format!("rueder-greader-{}", std::process::id()));
    let config = Config {
        key: Some(b"a key for tests, long enough to be one".to_vec()),
        ..Config::new(dir)
    };
    let client = HttpClient::new(http_client::Config::default()).unwrap();
    let images = Arc::new(ImageProxy::open(config, client).unwrap());
    AppState::new(db.clone()).with_images(images)
}

#[sqlx::test]
async fn images_go_through_the_proxy_for_the_app_password(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 1).await;
    sqlx::query(
        "UPDATE article_content SET html = '<p><img src=\"https://tracker.example/pixel.png\"></p>'",
    )
    .execute(&db)
    .await
    .unwrap();
    sqlx::query("UPDATE feeds SET icon_url = 'https://blog.example/icon.png'")
        .execute(&db)
        .await
        .unwrap();
    let state = with_images(&db);
    let token = logged_in(&state, ada, "ada").await;

    let page = get(
        &state,
        &format!("/reader/api/0/stream/contents/feed/{feed}"),
        &token,
    )
    .await;
    let html = page["items"][0]["summary"]["content"].as_str().unwrap();
    let list = get(&state, "/reader/api/0/subscription/list", &token).await;
    let icon = list["subscriptions"][0]["iconUrl"].as_str().unwrap();
    // No other host's address goes out, only this server's, made for the app.
    for text in [page.to_string(), list.to_string()] {
        assert!(!text.contains("tracker.example") && !text.contains("blog.example/icon"));
    }
    let prefix = format!("https://{HOST}/greader/img/");
    assert!(html.contains(&prefix), "{html}");
    assert!(icon.starts_with(&prefix), "{icon}");

    // A changed address is none; a revoked app password ends them all before
    // anything is fetched.
    let path = icon
        .strip_prefix(&format!("https://{HOST}/greader"))
        .unwrap();
    let forged = path.replacen("/icon/", "/image/", 1);
    let (status, _, _) = ask(&state, Method::GET, &forged, None, "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let cookie = common::session(&db, ada).await;
    let (_, listed) = common::call(
        &state,
        Method::GET,
        "/api/app-passwords",
        Some(&cookie),
        None,
    )
    .await;
    let id = listed["app_passwords"][0]["id"].as_i64().unwrap();
    let (status, _) = common::call(
        &state,
        Method::DELETE,
        &format!("/api/app-passwords/{id}"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _, _) = ask(&state, Method::GET, path, None, "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
