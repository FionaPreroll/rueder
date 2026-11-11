//! The API for reading: the tree, the lists, an article, its versions and
//! what was read. Every case also asks as another account.

mod common;

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use rueder::{
    AppState, articles,
    http_client::{self, HttpClient},
    images::{Config, ImageProxy},
    router,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

async fn call(
    state: &AppState,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(path);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let body = match body {
        Some(body) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let app = router(state.clone(), "missing".as_ref());
    let response = app.oneshot(request.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn get(state: &AppState, path: &str, cookie: &str) -> (StatusCode, Value) {
    call(state, Method::GET, path, Some(cookie), None).await
}

async fn post(state: &AppState, path: &str, cookie: &str, body: Value) -> StatusCode {
    call(state, Method::POST, path, Some(cookie), Some(body))
        .await
        .0
}

async fn put(state: &AppState, path: &str, cookie: &str, body: Value) -> StatusCode {
    call(state, Method::PUT, path, Some(cookie), Some(body))
        .await
        .0
}

/// A state with an image proxy, which these tests never let fetch anything.
fn with_images(db: &PgPool, name: &str) -> AppState {
    let dir = std::env::temp_dir().join(format!("rueder-api-{name}-{}", std::process::id()));
    let config = Config {
        key: Some(b"a key for tests, long enough to be one".to_vec()),
        ..Config::new(dir)
    };
    let client = HttpClient::new(http_client::Config::default()).unwrap();
    let images = Arc::new(ImageProxy::open(config, client).unwrap());
    AppState::new(db.clone()).with_images(images)
}

/// The numbers the tree shows for a feed: newest, mark and unread.
async fn counts(state: &AppState, cookie: &str, feed_id: i32) -> (i64, i64, i64) {
    let (_, tree) = get(state, "/api/tree", cookie).await;
    let feeds = tree["feeds"].as_array().unwrap();
    let feed = feeds.iter().find(|feed| feed["id"] == feed_id).unwrap();
    let number = |name: &str| feed[name].as_i64().unwrap();
    (number("last_seq"), number("read_upto"), number("unread"))
}

fn seqs(list: &Value) -> Vec<(i64, i64)> {
    let articles = list["articles"].as_array().unwrap();
    let pair = |a: &Value| (a["feed_id"].as_i64().unwrap(), a["seq"].as_i64().unwrap());
    articles.iter().map(pair).collect()
}

#[sqlx::test]
async fn nothing_is_answered_without_a_session(db: PgPool) {
    let (_, feed) = common::reader(&db, "ada", 3).await;
    let state = AppState::new(db);
    let asks = [
        (Method::GET, "/api/tree".to_owned()),
        (Method::GET, "/api/articles".to_owned()),
        (Method::POST, "/api/read".to_owned()),
        (Method::GET, format!("/api/feeds/{feed}/articles")),
        (Method::POST, format!("/api/feeds/{feed}/read")),
        (Method::GET, format!("/api/feeds/{feed}/articles/1")),
        (
            Method::GET,
            format!("/api/feeds/{feed}/articles/1/versions"),
        ),
        (Method::PUT, format!("/api/feeds/{feed}/articles/1/state")),
        (Method::GET, "/api/folders/1/articles".to_owned()),
        (Method::POST, "/api/folders/1/read".to_owned()),
    ];
    for (method, path) in asks {
        let body = (method != Method::GET).then(|| json!({ "read": true, "upto": 1 }));
        for cookie in [
            None,
            Some("__Host-rueder=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
        ] {
            let (status, json) = call(&state, method.clone(), &path, cookie, body.clone()).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {path}");
            assert_eq!(json, Value::Null);
        }
    }
}

#[sqlx::test]
async fn the_tree_has_the_readers_folders_feeds_and_counts(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 5).await;
    let blog = common::feed(&db, "blog").await;
    common::subscribe(&db, ada, blog).await;
    common::add(&db, blog, 2).await;
    let tech = common::folder(&db, ada, None, "Tech").await;
    let rust = common::folder(&db, ada, Some(tech), "Rust").await;
    common::move_to(&db, ada, blog, rust).await;
    sqlx::query("UPDATE subscriptions SET title = 'My blog' WHERE feed_id = $1")
        .bind(blog)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE feeds SET icon_url = 'https://blog.example/icon.png', error_message = 'gone',
             site_url = 'https://blog.example/' WHERE id = $1",
    )
    .bind(blog)
    .execute(&db)
    .await
    .unwrap();
    let (grace, other) = common::reader(&db, "grace", 1).await;
    common::folder(&db, grace, None, "Hers").await;
    let state = with_images(&db, "tree");
    let (ada_cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );

    let (status, tree) = get(&state, "/api/tree", &ada_cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        tree["folders"],
        json!([
            { "id": tech, "parent_id": null, "title": "Tech", "position": 1.0 },
            { "id": rust, "parent_id": tech, "title": "Rust", "position": 1.0 },
        ])
    );
    let feeds = tree["feeds"].as_array().unwrap();
    assert_eq!(feeds.len(), 2);
    let one = &feeds[0];
    assert_eq!(one["id"], news);
    assert_eq!(
        (&one["title"], &one["feed_title"]),
        (&json!("ada"), &json!("ada"))
    );
    assert_eq!(one["folder_id"], Value::Null);
    assert_eq!(one["feed_url"], "https://ada.example/feed.xml");
    assert_eq!(
        (&one["icon_url"], &one["error"]),
        (&Value::Null, &Value::Null)
    );
    assert_eq!(
        (&one["last_seq"], &one["read_upto"], &one["unread"]),
        (&json!(5), &json!(0), &json!(5))
    );
    let two = &feeds[1];
    assert_eq!(two["id"], blog);
    assert_eq!(
        (&two["title"], &two["feed_title"]),
        (&json!("My blog"), &json!("blog"))
    );
    assert_eq!(two["folder_id"], rust);
    assert_eq!(two["site_url"], "https://blog.example/");
    assert_eq!(two["error"], "gone");
    assert_eq!(two["fetch_agent"], "rueder");
    assert_eq!(two["unread"], 2);
    // The icon comes from the proxy, not from the site.
    let icon = two["icon_url"].as_str().unwrap();
    assert!(
        icon.starts_with("/img/") && icon.contains("/icon/"),
        "{icon}"
    );
    assert!(!tree.to_string().contains("icon.png"));

    // Without a proxy there is no address for it at all.
    let (_, tree) = get(&AppState::new(db.clone()), "/api/tree", &ada_cookie).await;
    assert_eq!(tree["feeds"][1]["icon_url"], Value::Null);

    // Grace sees her tree and nothing of ada's.
    let (_, tree) = get(&state, "/api/tree", &grace_cookie).await;
    assert_eq!(tree["folders"].as_array().unwrap().len(), 1);
    assert_eq!(tree["folders"][0]["title"], "Hers");
    let ids: Vec<_> = tree["feeds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].clone())
        .collect();
    assert_eq!(ids, [json!(other)]);
    let all = tree.to_string();
    assert!(!all.contains("Tech") && !all.contains("blog"), "{all}");
}

#[sqlx::test]
async fn lists_go_back_page_by_page(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 4).await;
    let blog = common::feed(&db, "blog").await;
    common::subscribe(&db, ada, blog).await;
    common::add(&db, blog, 3).await;
    common::add(&db, news, 1).await;
    let side = common::feed(&db, "side").await;
    common::subscribe(&db, ada, side).await;
    common::add(&db, side, 1).await;
    let tech = common::folder(&db, ada, None, "Tech").await;
    let rust = common::folder(&db, ada, Some(tech), "Rust").await;
    let empty = common::folder(&db, ada, None, "Empty").await;
    common::move_to(&db, ada, news, tech).await;
    common::move_to(&db, ada, blog, rust).await;
    // Somebody else's feed, which ada does not read, and her own reader.
    let (grace, hers) = common::reader(&db, "grace", 2).await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let (n, b, s) = (i64::from(news), i64::from(blog), i64::from(side));

    // One feed, newest first, three at a time.
    let (status, page) = get(
        &state,
        &format!("/api/feeds/{news}/articles?limit=3"),
        &cookie,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(seqs(&page), [(n, 5), (n, 4), (n, 3)]);
    let first = &page["articles"][0];
    assert_eq!(first["title"], "Article 5");
    assert_eq!(
        (&first["read"], &first["starred"]),
        (&json!(false), &json!(false))
    );
    let last = page["articles"][2]["id"].as_i64().unwrap();
    let next = format!("/api/feeds/{news}/articles?limit=3&before={last}");
    assert_eq!(seqs(&get(&state, &next, &cookie).await.1), [(n, 2), (n, 1)]);

    // A folder has the feeds of the folders inside it; everything has all.
    let (_, page) = get(&state, &format!("/api/folders/{tech}/articles"), &cookie).await;
    assert_eq!(
        seqs(&page),
        [
            (n, 5),
            (b, 3),
            (b, 2),
            (b, 1),
            (n, 4),
            (n, 3),
            (n, 2),
            (n, 1)
        ]
    );
    let (_, page) = get(&state, &format!("/api/folders/{rust}/articles"), &cookie).await;
    assert_eq!(seqs(&page), [(b, 3), (b, 2), (b, 1)]);
    let (status, page) = get(&state, &format!("/api/folders/{empty}/articles"), &cookie).await;
    assert_eq!((status, seqs(&page)), (StatusCode::OK, vec![]));
    let (_, page) = get(&state, "/api/articles?limit=2", &cookie).await;
    assert_eq!(seqs(&page), [(s, 1), (n, 5)]);
    let (_, page) = get(&state, "/api/articles", &cookie).await;
    assert_eq!(page["articles"].as_array().unwrap().len(), 9);

    // What was read can be left out.
    let state_of = format!("/api/feeds/{blog}/articles/2/state");
    assert_eq!(
        put(&state, &state_of, &cookie, json!({ "read": true })).await,
        StatusCode::NO_CONTENT
    );
    let (_, page) = get(
        &state,
        &format!("/api/folders/{rust}/articles?unread=true"),
        &cookie,
    )
    .await;
    assert_eq!(seqs(&page), [(b, 3), (b, 1)]);
    let (_, page) = get(&state, &format!("/api/folders/{rust}/articles"), &cookie).await;
    assert_eq!(page["articles"][1]["read"], true);

    // A page has a size whatever is asked for.
    for (limit, expected) in [("0", 1), ("-5", 1), ("100000", 9)] {
        let (status, page) = get(&state, &format!("/api/articles?limit={limit}"), &cookie).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            page["articles"].as_array().unwrap().len(),
            expected,
            "{limit}"
        );
    }
    assert_eq!(
        get(&state, "/api/articles?limit=many", &cookie).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        get(&state, "/api/feeds/x/articles", &cookie).await.0,
        StatusCode::BAD_REQUEST
    );

    // Ada cannot read grace's feed, and grace nothing of ada's.
    assert_eq!(
        get(&state, &format!("/api/feeds/{hers}/articles"), &cookie)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(&state, &format!("/api/feeds/{hers}/articles/1"), &cookie)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(&state, "/api/feeds/999999/articles", &cookie).await.0,
        StatusCode::NOT_FOUND
    );
    let grace_cookie = common::session(&db, grace).await;
    for path in [
        format!("/api/feeds/{news}/articles"),
        format!("/api/folders/{tech}/articles"),
        format!("/api/folders/{rust}/articles"),
        format!("/api/feeds/{news}/articles/1"),
        format!("/api/feeds/{news}/articles/1/versions"),
    ] {
        assert_eq!(
            get(&state, &path, &grace_cookie).await.0,
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
    let (_, page) = get(&state, "/api/articles", &grace_cookie).await;
    assert_eq!(seqs(&page), [(i64::from(hers), 2), (i64::from(hers), 1)]);
}

/// An article with pictures from a site, in a new version of its text if asked.
fn with_pictures(n: usize, text: &str) -> articles::NewArticle {
    let mut article = common::article(n);
    article.original.content = format!(
        "<p onclick=\"x()\">{text}</p><script>track()</script><img src=\"https://pics.example/{n}.png\">"
    );
    article.html = format!("<p>{text}</p><img src=\"https://pics.example/{n}.png\">");
    article.thumbnail_url = Some(format!("https://pics.example/{n}-small.png"));
    article.image_url = Some(format!("https://pics.example/{n}-large.png"));
    article
}

#[sqlx::test]
async fn an_article_goes_out_with_its_pictures_at_the_proxy(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 0).await;
    let mut tx = db.begin().await.unwrap();
    articles::insert(&mut tx, feed, &[with_pictures(1, "first")])
        .await
        .unwrap();
    articles::add_version(&mut tx, feed, 1, &with_pictures(1, "second"))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let (grace, _) = common::reader(&db, "grace", 1).await;
    let state = with_images(&db, "article");
    let cookie = common::session(&db, ada).await;
    let at_proxy = |address: &Value, size: &str| {
        let address = address.as_str().unwrap();
        address.starts_with("/img/") && address.contains(&format!("/{size}/"))
    };

    let (_, list) = get(&state, &format!("/api/feeds/{feed}/articles"), &cookie).await;
    assert!(
        at_proxy(&list["articles"][0]["thumbnail_url"], "thumbnail"),
        "{list}"
    );
    assert_eq!(list["articles"][0]["version"], 2);

    let (status, article) = get(&state, &format!("/api/feeds/{feed}/articles/1"), &cookie).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        (&article["feed_id"], &article["seq"]),
        (&json!(feed), &json!(1))
    );
    assert_eq!(article["title"], "Article 1");
    assert_eq!(article["tags"], json!(["news", "tag-1"]));
    assert!(at_proxy(&article["image_url"], "image"));
    let html = article["html"].as_str().unwrap();
    assert!(html.starts_with("<p>second</p><img src=\"/img/"), "{html}");

    // The versions are cleaned like the text: they are stored as they came.
    let path = format!("/api/feeds/{feed}/articles/1/versions");
    let (status, versions) = get(&state, &path, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    let versions = versions["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 2);
    for (version, text) in versions.iter().zip(["first", "second"]) {
        let html = version["html"].as_str().unwrap();
        assert!(html.starts_with(&format!("<p>{text}</p>")), "{html}");
        assert!(html.contains("<img src=\"/img/"), "{html}");
        assert!(
            !html.contains("script") && !html.contains("onclick"),
            "{html}"
        );
        assert_eq!(version["title"], "Article 1");
        assert_eq!(version["authors"], json!(["Ada", "Grace"]));
    }
    assert_eq!(
        (&versions[0]["version"], &versions[1]["version"]),
        (&json!(1), &json!(2))
    );

    // Nowhere does the site's own address go out for a picture.
    for body in [
        list.to_string(),
        article.to_string(),
        json!(versions).to_string(),
    ] {
        assert!(!body.contains("pics.example"), "{body}");
    }
    // Without a proxy the pictures go, and nothing else.
    let bare = AppState::new(db.clone());
    let (_, article) = get(&bare, &format!("/api/feeds/{feed}/articles/1"), &cookie).await;
    assert_eq!(article["html"], "<p>second</p>");
    assert_eq!(article["image_url"], Value::Null);
    let (_, list) = get(&bare, &format!("/api/feeds/{feed}/articles"), &cookie).await;
    assert_eq!(list["articles"][0]["thumbnail_url"], Value::Null);
    let (_, versions) = get(&bare, &path, &cookie).await;
    assert_eq!(versions["versions"][0]["html"], "<p>first</p>");

    // What is not there, and what is another's.
    for path in [
        format!("/api/feeds/{feed}/articles/2"),
        format!("/api/feeds/{feed}/articles/0"),
        format!("/api/feeds/{feed}/articles/2/versions"),
    ] {
        assert_eq!(
            get(&state, &path, &cookie).await.0,
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
    let grace_cookie = common::session(&db, grace).await;
    for path in [format!("/api/feeds/{feed}/articles/1"), path] {
        let (status, body) = get(&state, &path, &grace_cookie).await;
        assert_eq!(
            (status, body),
            (StatusCode::NOT_FOUND, Value::Null),
            "{path}"
        );
    }
}

#[sqlx::test]
async fn reading_and_starring_change_what_the_reader_sees(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 5).await;
    let grace = common::account(&db, "grace").await;
    common::subscribe(&db, grace, feed).await;
    let (eve, _) = common::reader(&db, "eve", 5).await;
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie, eve_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
        common::session(&db, eve).await,
    );
    let of = |seq: i32| format!("/api/feeds/{feed}/articles/{seq}/state");

    assert_eq!(
        put(&state, &of(2), &cookie, json!({ "read": true })).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(counts(&state, &cookie, feed).await, (5, 0, 4));
    // Read from the bottom up, the mark follows.
    assert_eq!(
        put(
            &state,
            &of(1),
            &cookie,
            json!({ "read": true, "starred": true })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(counts(&state, &cookie, feed).await, (5, 2, 3));
    let (_, list) = get(&state, &format!("/api/feeds/{feed}/articles"), &cookie).await;
    let flags: Vec<_> = list["articles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            (
                a["seq"].as_i64().unwrap(),
                a["read"] == true,
                a["starred"] == true,
            )
        })
        .collect();
    assert_eq!(
        flags,
        [
            (5, false, false),
            (4, false, false),
            (3, false, false),
            (2, true, false),
            (1, true, true)
        ]
    );
    // And back.
    assert_eq!(
        put(&state, &of(1), &cookie, json!({ "read": false })).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(counts(&state, &cookie, feed).await.2, 4);
    assert_eq!(
        put(&state, &of(1), &cookie, json!({ "starred": false })).await,
        StatusCode::NO_CONTENT
    );

    // Grace reads the same feed and has her own state of it.
    assert_eq!(counts(&state, &grace_cookie, feed).await, (5, 0, 5));
    assert_eq!(
        put(&state, &of(5), &grace_cookie, json!({ "read": true })).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(counts(&state, &cookie, feed).await.2, 4);

    // What makes no sense, what is not there, and what is another's feed.
    assert_eq!(
        put(&state, &of(1), &cookie, json!({})).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        put(&state, &of(1), &cookie, json!({ "read": "yes" })).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    for seq in [0, 6, -1] {
        assert_eq!(
            put(&state, &of(seq), &cookie, json!({ "read": true })).await,
            StatusCode::NOT_FOUND,
            "{seq}"
        );
        assert_eq!(
            put(&state, &of(seq), &cookie, json!({ "starred": true })).await,
            StatusCode::NOT_FOUND,
            "{seq}"
        );
    }
    for change in [json!({ "read": true }), json!({ "starred": true })] {
        assert_eq!(
            put(&state, &of(3), &eve_cookie, change).await,
            StatusCode::NOT_FOUND
        );
    }
    let states: i64 =
        sqlx::query_scalar("SELECT count(*) FROM article_states WHERE account_id = $1")
            .bind(eve)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(states, 0);
    assert_eq!(counts(&state, &cookie, feed).await.2, 4);
}

#[sqlx::test]
async fn everything_up_to_an_article_can_be_marked_read(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 3).await;
    let blog = common::feed(&db, "blog").await;
    let side = common::feed(&db, "side").await;
    for feed in [blog, side] {
        common::subscribe(&db, ada, feed).await;
    }
    common::add(&db, blog, 2).await;
    common::add(&db, side, 2).await;
    let tech = common::folder(&db, ada, None, "Tech").await;
    let rust = common::folder(&db, ada, Some(tech), "Rust").await;
    common::move_to(&db, ada, news, tech).await;
    common::move_to(&db, ada, blog, rust).await;
    let grace = common::account(&db, "grace").await;
    for feed in [news, blog, side] {
        common::subscribe(&db, grace, feed).await;
    }
    let hers = common::folder(&db, grace, None, "Hers").await;
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let unread = async |cookie: &str| {
        let mut all = Vec::new();
        for feed in [news, blog, side] {
            all.push(counts(&state, cookie, feed).await.2);
        }
        all
    };

    // One feed, up to a number.
    let path = format!("/api/feeds/{news}/read");
    assert_eq!(
        post(&state, &path, &cookie, json!({ "upto": 2 })).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(counts(&state, &cookie, news).await, (3, 2, 1));
    // A number beyond the newest means the newest.
    assert_eq!(
        post(&state, &path, &cookie, json!({ "upto": 99 })).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(counts(&state, &cookie, news).await, (3, 3, 0));

    // The list the reader looks at ends with some article; what arrives after
    // it stays unread when the folder is marked.
    let (_, list) = get(&state, &format!("/api/folders/{tech}/articles"), &cookie).await;
    let newest = list["articles"][0]["id"].as_i64().unwrap();
    common::add(&db, blog, 1).await;
    common::add(&db, news, 1).await;
    assert_eq!(unread(&cookie).await, [1, 3, 2]);
    let path = format!("/api/folders/{tech}/read");
    assert_eq!(
        post(&state, &path, &cookie, json!({ "upto_id": newest })).await,
        StatusCode::NO_CONTENT
    );
    // The folder's feeds, the one in the folder inside it too, and not the
    // feed outside of it.
    assert_eq!(unread(&cookie).await, [1, 1, 2]);

    // One feed up to an article of the mixed list.
    let (_, list) = get(&state, "/api/articles?limit=1", &cookie).await;
    let newest = list["articles"][0]["id"].as_i64().unwrap();
    let path = format!("/api/feeds/{blog}/read");
    assert_eq!(
        post(&state, &path, &cookie, json!({ "upto_id": newest })).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(unread(&cookie).await, [1, 0, 2]);
    // An id before everything marks nothing.
    assert_eq!(
        post(&state, "/api/read", &cookie, json!({ "upto_id": 0 })).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(unread(&cookie).await, [1, 0, 2]);
    // Everything.
    assert_eq!(
        post(&state, "/api/read", &cookie, json!({ "upto_id": newest })).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(unread(&cookie).await, [0, 0, 0]);

    // Grace reads the same feeds and none of this was hers.
    assert_eq!(unread(&grace_cookie).await, [4, 3, 2]);
    // She cannot mark through ada's folders, and ada not through hers.
    for folder in [tech, rust] {
        let path = format!("/api/folders/{folder}/read");
        assert_eq!(
            post(&state, &path, &grace_cookie, json!({ "upto_id": newest })).await,
            StatusCode::NOT_FOUND
        );
    }
    let path = format!("/api/folders/{hers}/read");
    assert_eq!(
        post(&state, &path, &cookie, json!({ "upto_id": newest })).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(unread(&grace_cookie).await, [4, 3, 2]);
    // A feed she does not read is not there for her.
    sqlx::query("DELETE FROM subscriptions WHERE account_id = $1 AND feed_id = $2")
        .bind(grace)
        .bind(side)
        .execute(&db)
        .await
        .unwrap();
    let path = format!("/api/feeds/{side}/read");
    assert_eq!(
        post(&state, &path, &grace_cookie, json!({ "upto": 1 })).await,
        StatusCode::NOT_FOUND
    );

    // Bodies that do not say how far.
    let feed_path = format!("/api/feeds/{news}/read");
    for (path, body) in [
        (feed_path.as_str(), json!({})),
        (feed_path.as_str(), json!({ "upto": 1, "upto_id": 1 })),
        (feed_path.as_str(), json!({ "upto": -1 })),
        ("/api/read", json!({ "upto": 1 })),
        ("/api/read", json!({})),
    ] {
        assert_eq!(
            post(&state, path, &cookie, body.clone()).await,
            StatusCode::BAD_REQUEST,
            "{path} {body}"
        );
    }
}
