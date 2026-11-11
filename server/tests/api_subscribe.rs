//! Subscribing: the search for feeds at an address, subscribing to one, and
//! OPML in and out. The addresses are those of a site on this machine.

mod common;

use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    extract::State,
    http::{Method, StatusCode, header},
    response::{IntoResponse, Redirect},
    routing::get,
};
use common::{call, raw};
use rueder::{
    AppState,
    http_client::{Config, HttpClient},
    rate::{REQUESTS, Rate, Rates},
    subscribe, tree,
};
use serde_json::{Value, json};
use sqlx::PgPool;

const FEED: &str = "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel>\
    <title>The Example</title><link>https://example.org/</link>\
    <item><guid>c</guid><title>Third</title><description>three</description></item>\
    <item><guid>b</guid><title>Second</title><description>two</description></item>\
    <item><guid>a</guid><title>First</title><description>one</description></item>\
    </channel></rss>";

const PAGE: &str = r#"<!doctype html><html><head><title>A blog</title>
    <link rel="stylesheet" href="/style.css">
    <link rel="alternate" type="application/rss+xml" title="  The   blog " href="/feed.xml">
    <link rel="ALTERNATE nofollow" type="Application/Atom+XML; charset=utf-8" href="comments.atom">
    <link rel="alternate" type="application/rss+xml" title="Again" href="/feed.xml">
    <link rel="alternate" type="text/html" href="/print">
    <link rel="alternate" type="application/rss+xml" href="javascript:alert(1)">
    <link rel="alternate" type="application/rss+xml" href="ftp://files.example/feed">
    </head><body><p>Hello</p>
    <link rel="alternate" type="application/feed+json" href="https://other.example/feed.json">
    </body></html>"#;

/// Starts a site with a feed, a page that names feeds and a page without,
/// and returns its address and the count of the requests it saw.
async fn site() -> (String, Arc<AtomicUsize>) {
    let requests = Arc::new(AtomicUsize::new(0));
    let feed = async |State(requests): State<Arc<AtomicUsize>>| {
        requests.fetch_add(1, Ordering::SeqCst);
        ([(header::CONTENT_TYPE, "application/rss+xml")], FEED)
    };
    let page = async |State(requests): State<Arc<AtomicUsize>>| {
        requests.fetch_add(1, Ordering::SeqCst);
        ([(header::CONTENT_TYPE, "text/html")], PAGE)
    };
    let router = Router::new()
        .route("/feed.xml", get(feed))
        .route("/blog/", get(page))
        .route("/plain", get(async || "<p>Nothing to see</p>"))
        // A site that turns readers away sends them to its login.
        .route("/members/", get(async || Redirect::to("/plain")))
        .route(
            "/broken",
            get(async || StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        )
        .with_state(requests.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (format!("http://{addr}"), requests)
}

/// A server that may reach the site on this machine and nothing else inside.
fn state(db: &PgPool) -> AppState {
    let config = Config {
        allow: vec!["127.0.0.1".parse().unwrap()],
        ..Config::default()
    };
    AppState::new(db.clone()).with_client(HttpClient::new(config).unwrap())
}

async fn post(state: &AppState, path: &str, cookie: &str, body: Value) -> (StatusCode, Value) {
    call(state, Method::POST, path, Some(cookie), Some(body)).await
}

async fn tree(state: &AppState, cookie: &str) -> Value {
    let (status, tree) = call(state, Method::GET, "/api/tree", Some(cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    tree
}

/// The tree as lines, "Folder/Folder/feed address", in the order it is shown.
async fn lines(state: &AppState, cookie: &str) -> Vec<String> {
    let tree = tree(state, cookie).await;
    let (folders, feeds) = (
        tree["folders"].as_array().unwrap(),
        tree["feeds"].as_array().unwrap(),
    );
    fn walk(
        parent: &Value,
        prefix: &str,
        folders: &[Value],
        feeds: &[Value],
        out: &mut Vec<String>,
    ) {
        let position = |node: &Value| node["position"].as_f64().unwrap();
        let mut level: Vec<(f64, Option<&Value>, Option<&Value>)> = Vec::new();
        let inside = folders.iter().filter(|f| &f["parent_id"] == parent);
        level.extend(inside.map(|f| (position(f), Some(f), None)));
        let inside = feeds.iter().filter(|f| &f["folder_id"] == parent);
        level.extend(inside.map(|f| (position(f), None, Some(f))));
        level.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, folder, feed) in level {
            if let Some(folder) = folder {
                let path = format!("{prefix}{}/", folder["title"].as_str().unwrap());
                out.push(path.clone());
                walk(&folder["id"], &path, folders, feeds, out);
            }
            if let Some(feed) = feed {
                out.push(format!("{prefix}{}", feed["feed_url"].as_str().unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(&Value::Null, "", folders, feeds, &mut out);
    out
}

async fn feed_rows(db: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM feeds")
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn the_search_finds_the_feed_at_an_address_or_the_feeds_a_page_names(db: PgPool) {
    let (site, _) = site().await;
    let ada = common::account(&db, "ada").await;
    let state = state(&db);
    let cookie = common::session(&db, ada).await;
    let find =
        async |url: String| post(&state, "/api/feeds/find", &cookie, json!({ "url": url })).await;

    let (status, found) = find(format!(" {site}/feed.xml ")).await;
    assert_eq!(status, StatusCode::OK);
    let feed = |url: String, title: &str, feed_id: Value| json!({ "url": url, "title": title, "feed_id": feed_id });
    assert_eq!(
        found,
        json!({ "feeds": [feed(format!("{site}/feed.xml"), "The Example", Value::Null)], "moved_to": null })
    );

    // A page names its feeds; addresses are made whole, each is there once,
    // and what is no feed on the web is left out.
    let (status, found) = find(format!("{site}/blog/")).await;
    assert_eq!(status, StatusCode::OK);
    let named = json!({ "feeds": [
        feed(format!("{site}/feed.xml"), "The blog", Value::Null),
        feed(format!("{site}/blog/comments.atom"), "", Value::Null),
        feed("https://other.example/feed.json".into(), "", Value::Null),
    ], "moved_to": null });
    assert_eq!(found, named);
    let (status, found) = find(format!("{site}/plain")).await;
    assert_eq!(
        (status, found),
        (StatusCode::OK, json!({ "feeds": [], "moved_to": null }))
    );
    // Where the site sent the request on to is said.
    let (status, found) = find(format!("{site}/members/")).await;
    assert_eq!(
        (status, found),
        (
            StatusCode::OK,
            json!({ "feeds": [], "moved_to": format!("{site}/plain") })
        )
    );

    // Looking for feeds makes none.
    assert_eq!(feed_rows(&db).await, 0);

    // A feed the reader subscribes to says which it is; somebody else's
    // subscription to it does not show.
    let grace = common::account(&db, "grace").await;
    let theirs = common::feed(&db, "theirs").await;
    sqlx::query("UPDATE feeds SET feed_url = $1 WHERE id = $2")
        .bind(format!("{site}/blog/comments.atom"))
        .bind(theirs)
        .execute(&db)
        .await
        .unwrap();
    common::subscribe(&db, grace, theirs).await;
    let mine = common::feed(&db, "mine").await;
    sqlx::query("UPDATE feeds SET feed_url = $1 WHERE id = $2")
        .bind(format!("{site}/feed.xml"))
        .bind(mine)
        .execute(&db)
        .await
        .unwrap();
    common::subscribe(&db, ada, mine).await;
    let (_, found) = find(format!("{site}/blog/")).await;
    assert_eq!(found["feeds"][0]["feed_id"], json!(mine));
    assert_eq!(found["feeds"][1]["feed_id"], Value::Null);
    let grace_cookie = common::session(&db, grace).await;
    let (_, found) = post(
        &state,
        "/api/feeds/find",
        &grace_cookie,
        json!({ "url": format!("{site}/blog/") }),
    )
    .await;
    assert_eq!(found["feeds"][0]["feed_id"], Value::Null);
    assert_eq!(found["feeds"][1]["feed_id"], json!(theirs));

    for wrong in [
        format!("{site}/broken"),
        format!("{site}/nowhere"),
        "not an address".to_owned(),
        "ftp://files.example/feed".to_owned(),
        "https://ada:secret@blog.example/feed".to_owned(),
        String::new(),
    ] {
        let (status, _) = find(wrong.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{wrong}");
    }
    // The status page counts the lookups: what was not even an address is
    // none.
    let lookups = state.monitor.lookups(std::time::Instant::now());
    assert_eq!(
        (lookups.found, lookups.none, lookups.failed, lookups.limited),
        (4, 2, 2, 0)
    );
    assert_eq!(lookups.timing.count, 8);
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/feeds/find",
        None,
        Some(json!({ "url": site })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn the_search_does_not_reach_into_the_own_network(db: PgPool) {
    let (site, requests) = site().await;
    let ada = common::account(&db, "ada").await;
    let cookie = common::session(&db, ada).await;
    // A server as it runs: nothing inside is allowed.
    let strict = AppState::new(db.clone()).with_client(HttpClient::new(Config::default()).unwrap());
    for path in ["/api/feeds/find", "/api/feeds"] {
        for url in [
            format!("{site}/feed.xml"),
            "http://169.254.169.254/latest/".to_owned(),
        ] {
            let (status, _) = post(&strict, path, &cookie, json!({ "url": url })).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{path} {url}");
        }
    }
    assert_eq!(requests.load(Ordering::SeqCst), 0);
    assert_eq!(feed_rows(&db).await, 0);

    // And a server without a client says so.
    let without = AppState::new(db.clone());
    let url = json!({ "url": format!("{site}/feed.xml") });
    let (status, _) = post(&without, "/api/feeds/find", &cookie, url.clone()).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let (status, _) = post(&without, "/api/feeds", &cookie, url).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
}

#[sqlx::test]
async fn subscribing_fetches_a_new_feed_and_shares_one_that_is_there(db: PgPool) {
    let (site, requests) = site().await;
    let url = format!("{site}/feed.xml");
    let ada = common::account(&db, "ada").await;
    let grace = common::account(&db, "grace").await;
    let state = state(&db);
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let folder = common::folder(&db, ada, None, "Blogs").await;
    let theirs = common::folder(&db, grace, None, "Theirs").await;

    // Into somebody else's folder: no, and nothing is fetched for it.
    let wrong = json!({ "url": url, "folder_id": theirs });
    let (status, _) = post(&state, "/api/feeds", &cookie, wrong).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(requests.load(Ordering::SeqCst), 0);

    let new = json!({ "url": url, "folder_id": folder });
    let (status, made) = post(&state, "/api/feeds", &cookie, new.clone()).await;
    assert_eq!(status, StatusCode::CREATED);
    let id = made["id"].as_i64().unwrap();
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    let feed = &tree(&state, &cookie).await["feeds"][0];
    assert_eq!(
        (
            &feed["id"],
            &feed["folder_id"],
            &feed["title"],
            &feed["unread"]
        ),
        (&json!(id), &json!(folder), &json!("The Example"), &json!(3))
    );
    assert_eq!(feed["error"], Value::Null);

    // Once more is no second subscription.
    let (status, again) = post(&state, "/api/feeds", &cookie, new).await;
    assert_eq!((status, &again["id"]), (StatusCode::OK, &json!(id)));

    // Grace gets the same feed without another fetch, and sees nothing of
    // where Ada keeps it.
    assert_eq!(tree(&state, &grace_cookie).await["feeds"], json!([]));
    let (status, shared) = post(&state, "/api/feeds", &grace_cookie, json!({ "url": url })).await;
    assert_eq!((status, &shared["id"]), (StatusCode::CREATED, &json!(id)));
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    assert_eq!(feed_rows(&db).await, 1);
    let feed = &tree(&state, &grace_cookie).await["feeds"][0];
    assert_eq!(
        (&feed["folder_id"], &feed["unread"]),
        (&Value::Null, &json!(3))
    );

    // Of a feed with a history, the newest articles are unread.
    common::add(&db, id as i32, 30).await;
    let heidi = common::account(&db, "heidi").await;
    let heidi_cookie = common::session(&db, heidi).await;
    let (status, _) = post(&state, "/api/feeds", &heidi_cookie, json!({ "url": url })).await;
    assert_eq!(status, StatusCode::CREATED);
    let feed = &tree(&state, &heidi_cookie).await["feeds"][0];
    assert_eq!(feed["unread"], subscribe::NEWEST_UNREAD);
    assert_eq!(feed["read_upto"], 33 - subscribe::NEWEST_UNREAD);
}

#[sqlx::test]
async fn an_address_that_is_no_feed_does_not_become_one(db: PgPool) {
    let (site, _) = site().await;
    let ada = common::account(&db, "ada").await;
    let state = state(&db);
    let cookie = common::session(&db, ada).await;
    for wrong in ["/blog/", "/plain", "/broken", "/nowhere"] {
        let url = json!({ "url": format!("{site}{wrong}") });
        let (status, _) = post(&state, "/api/feeds", &cookie, url).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{wrong}");
    }
    assert_eq!(feed_rows(&db).await, 0);
    assert_eq!(tree(&state, &cookie).await["feeds"], json!([]));
    let url = Some(json!({ "url": format!("{site}/feed.xml") }));
    let (status, _) = call(&state, Method::POST, "/api/feeds", None, url).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(feed_rows(&db).await, 0);
}

#[sqlx::test]
async fn the_rules_of_a_reader_look_at_a_feed_they_subscribe_to(db: PgPool) {
    let (site, _) = site().await;
    let ada = common::account(&db, "ada").await;
    let state = state(&db);
    let cookie = common::session(&db, ada).await;
    let rule = json!({
        "scope": { "kind": "all" }, "field": "title", "match": "word",
        "pattern": "second", "action": "hide",
    });
    let (status, _) = post(&state, "/api/rules", &cookie, rule).await;
    assert_eq!(status, StatusCode::CREATED);
    let url = json!({ "url": format!("{site}/feed.xml") });
    let (status, _) = post(&state, "/api/feeds", &cookie, url).await;
    assert_eq!(status, StatusCode::CREATED);
    let (_, hidden) = call(&state, Method::GET, "/api/hidden", Some(&cookie), None).await;
    let titles: Vec<_> = hidden["articles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| &a["title"])
        .collect();
    assert_eq!(titles, ["Second"]);
    assert_eq!(tree(&state, &cookie).await["feeds"][0]["unread"], 2);
}

const OPML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="1.0"><head><title>Subscriptions</title></head><body>
  <outline text="Tech &amp; &lt;Science&gt;">
    <outline type="rss" text="Kernel" xmlUrl="https://kernel.example/feed" htmlUrl="https://kernel.example/"/>
    <outline text="Space">
      <outline type="rss" text="Rockets" xmlUrl="https://rockets.example/atom.xml"/>
    </outline>
  </outline>
  <outline type="rss" text="Comics" xmlUrl="https://comics.example/rss"/>
  <outline type="rss" text="Not the web" xmlUrl="ftp://files.example/feed"/>
</body></opml>"#;

async fn import(state: &AppState, cookie: &str, file: &str) -> (StatusCode, Value) {
    let body = file.as_bytes().to_vec();
    let (status, _, answer) = raw(
        state,
        Method::POST,
        "/api/opml",
        Some(cookie),
        Some(("text/x-opml; charset=utf-8", body)),
    )
    .await;
    (
        status,
        serde_json::from_slice(&answer).unwrap_or(Value::Null),
    )
}

async fn export(state: &AppState, cookie: &str) -> String {
    let (status, headers, file) = raw(state, Method::GET, "/api/opml", Some(cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "text/x-opml; charset=utf-8");
    assert_eq!(
        headers[header::CONTENT_DISPOSITION],
        "attachment; filename=\"rueder.opml\""
    );
    String::from_utf8(file).unwrap()
}

#[sqlx::test]
async fn an_opml_file_brings_folders_and_feeds_and_goes_out_again(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 1).await;
    let grace = common::account(&db, "grace").await;
    // No client: an import fetches nothing itself.
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let news_url: String = sqlx::query_scalar("SELECT feed_url FROM feeds WHERE id = $1")
        .bind(news)
        .fetch_one(&db)
        .await
        .unwrap();

    let (status, did) = import(&state, &cookie, OPML).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        did,
        json!({ "feeds": 3, "folders": 2, "known": 0, "skipped": 1 })
    );
    // What was there stays first; the rest comes in the order of the file.
    let expected = [
        news_url.clone(),
        "Tech & <Science>/".to_owned(),
        "Tech & <Science>/https://kernel.example/feed".to_owned(),
        "Tech & <Science>/Space/".to_owned(),
        "Tech & <Science>/Space/https://rockets.example/atom.xml".to_owned(),
        "https://comics.example/rss".to_owned(),
    ];
    assert_eq!(lines(&state, &cookie).await, expected);
    // Until they are fetched, the feeds go by the names of the file, and
    // they are due at once.
    let comics = tree(&state, &cookie).await["feeds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|feed| feed["feed_url"] == "https://comics.example/rss")
        .cloned()
        .unwrap();
    assert_eq!(
        (&comics["title"], &comics["unread"]),
        (&json!("Comics"), &json!(0))
    );
    let due: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM feeds WHERE next_fetch_at <= now() AND feed_url LIKE '%.example/%'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(due >= 3);

    // The same file again changes nothing.
    let (status, did) = import(&state, &cookie, OPML).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        did,
        json!({ "feeds": 0, "folders": 0, "known": 3, "skipped": 1 })
    );
    assert_eq!(lines(&state, &cookie).await, expected);
    assert_eq!(feed_rows(&db).await, 4);

    // What goes out is what is there, in its order and with its names
    // written so that XML can hold them.
    let file = export(&state, &cookie).await;
    assert!(file.contains(
        "<outline text=\"Tech &amp; &lt;Science&gt;\" title=\"Tech &amp; &lt;Science&gt;\">"
    ));
    assert!(file.contains(
        "<outline type=\"rss\" text=\"Kernel\" title=\"Kernel\" xmlUrl=\"https://kernel.example/feed\"/>"
    ));
    assert_eq!(file.matches("<outline").count(), 6);
    assert_eq!(file.matches("</outline>").count(), 2);

    // Grace has nothing, and Ada's file gives her the same tree with the
    // same feeds.
    let empty = export(&state, &grace_cookie).await;
    assert_eq!(empty.matches("<outline").count(), 0);
    assert_eq!(lines(&state, &grace_cookie).await, Vec::<String>::new());
    let (status, did) = import(&state, &grace_cookie, &file).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        did,
        json!({ "feeds": 4, "folders": 2, "known": 0, "skipped": 0 })
    );
    assert_eq!(lines(&state, &grace_cookie).await, expected);
    assert_eq!(feed_rows(&db).await, 4);
    assert_eq!(export(&state, &grace_cookie).await, file);
}

#[sqlx::test]
async fn an_opml_file_that_breaks_the_rules_is_refused(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;

    let (status, _) = import(
        &state,
        &cookie,
        "<opml><body><outline text=\"a\"></body></opml>",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let large = format!(
        "<opml><body><!-- {} --></body></opml>",
        "x".repeat(subscribe::MAX_OPML)
    );
    let (status, _) = import(&state, &cookie, &large).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    // A file that is no OPML has nothing to import.
    let (status, did) = import(&state, &cookie, "Dear diary").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        did,
        json!({ "feeds": 0, "folders": 0, "known": 0, "skipped": 0 })
    );
    assert_eq!(feed_rows(&db).await, 0);

    // A file that does not say it is XML is what a form on another site
    // could send, and is not read.
    let form = Some(json!({ "file": OPML }));
    let (status, _) = call(&state, Method::POST, "/api/opml", Some(&cookie), form).await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(feed_rows(&db).await, 0);

    for method in [Method::GET, Method::POST] {
        let (status, _, _) = raw(
            &state,
            method,
            "/api/opml",
            None,
            Some(("text/xml", OPML.into())),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    assert_eq!(feed_rows(&db).await, 0);
}

#[sqlx::test]
async fn an_account_can_have_the_server_look_at_only_so_many_addresses(db: PgPool) {
    let (site, requests) = site().await;
    let url = format!("{site}/feed.xml");
    let ada = common::account(&db, "ada").await;
    let grace = common::account(&db, "grace").await;
    let hour = Duration::from_secs(3600);
    let state = state(&db).with_rates(Rates::new(REQUESTS, Rate::new(2, hour)));
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let find = async |cookie: &str| {
        let body = json!({ "url": format!("{site}/blog/") });
        post(&state, "/api/feeds/find", cookie, body).await.0
    };
    let subscribe = async |cookie: &str| {
        post(&state, "/api/feeds", cookie, json!({ "url": url }))
            .await
            .0
    };

    // An address that is none is not looked at and does not count.
    let none = json!({ "url": "ftp://files.example/feed" });
    for _ in 0..5 {
        let (status, _) = post(&state, "/api/feeds/find", &cookie, none.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    assert_eq!(find(&cookie).await, StatusCode::OK);
    assert_eq!(subscribe(&cookie).await, StatusCode::CREATED);
    assert_eq!(requests.load(Ordering::SeqCst), 2);
    // Both are used up: neither the search nor a new feed reaches the site.
    assert_eq!(find(&cookie).await, StatusCode::TOO_MANY_REQUESTS);
    let other = json!({ "url": format!("{site}/feed.xml?other") });
    let (status, headers, _) = raw(
        &state,
        Method::POST,
        "/api/feeds",
        Some(&cookie),
        Some(("application/json", other.to_string().into_bytes())),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let wait: u64 = headers[header::RETRY_AFTER]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((3590..=3600).contains(&wait), "{wait}");
    assert_eq!(requests.load(Ordering::SeqCst), 2);
    assert_eq!(feed_rows(&db).await, 1);
    // A feed that is there already is not fetched, so it is no lookup.
    assert_eq!(subscribe(&cookie).await, StatusCode::OK);

    // Another account has its own.
    assert_eq!(subscribe(&grace_cookie).await, StatusCode::CREATED);
    assert_eq!(find(&grace_cookie).await, StatusCode::OK);
    assert_eq!(find(&grace_cookie).await, StatusCode::OK);
    assert_eq!(find(&grace_cookie).await, StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test]
async fn an_account_can_subscribe_to_only_so_many_feeds(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let grace = common::account(&db, "grace").await;
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    // One short of the limit, in folders that each have room.
    let first = common::folder(&db, ada, None, "First").await;
    let second = common::folder(&db, ada, None, "Second").await;
    sqlx::query(
        "WITH new AS (
             INSERT INTO feeds (feed_url, title)
             SELECT 'https://many.example/' || n, 'Feed ' || n FROM generate_series(1, $1) AS n
             RETURNING id)
         INSERT INTO subscriptions (account_id, feed_id, folder_id, position)
         SELECT $2, id, CASE WHEN row_number() OVER (ORDER BY id) % 2 = 0 THEN $3 ELSE $4 END, id
         FROM new",
    )
    .bind(tree::MAX_FEEDS as i32 - 1)
    .bind(ada)
    .bind(first)
    .bind(second)
    .execute(&db)
    .await
    .unwrap();
    let last = common::feed(&db, "last").await;
    let beyond = common::feed(&db, "beyond").await;
    let url = |name: &str| format!("https://{name}.example/feed.xml");
    let count = async |account: i32| -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM subscriptions WHERE account_id = $1")
            .bind(account)
            .fetch_one(&db)
            .await
            .unwrap()
    };

    let (status, made) = post(&state, "/api/feeds", &cookie, json!({ "url": url("last") })).await;
    assert_eq!(
        (status, made["id"].as_i64()),
        (StatusCode::CREATED, Some(last.into()))
    );
    assert_eq!(count(ada).await, tree::MAX_FEEDS);
    let (status, _) = post(
        &state,
        "/api/feeds",
        &cookie,
        json!({ "url": url("beyond") }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // A feed the account reads already is no new one.
    let (status, _) = post(&state, "/api/feeds", &cookie, json!({ "url": url("last") })).await;
    assert_eq!(status, StatusCode::OK);

    // An OPML file does not get around it: all of it is refused.
    let file = r#"<opml><body>
        <outline type="rss" xmlUrl="https://last.example/feed.xml"/>
        <outline type="rss" xmlUrl="https://beyond.example/feed.xml"/>
        </body></opml>"#;
    let (status, _) = import(&state, &cookie, file).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(count(ada).await, tree::MAX_FEEDS);

    // The limit is the account's own.
    let (status, did) = import(&state, &grace_cookie, file).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(did["feeds"], 2);
    let (status, made) = post(
        &state,
        "/api/feeds",
        &grace_cookie,
        json!({ "url": url("beyond") }),
    )
    .await;
    assert_eq!(
        (status, made["id"].as_i64()),
        (StatusCode::OK, Some(beyond.into()))
    );
    assert_eq!(count(grace).await, 2);
}
