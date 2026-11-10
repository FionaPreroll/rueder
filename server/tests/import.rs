//! The import, against the old rueder's schema rebuilt from `backend/migrations/`
//! (`fixtures/legacy_schema.sql`). The old database lives in a schema of its
//! own inside the test's database, so no second server is needed.

mod common;

use rueder::{
    articles::{self, Page},
    cleaning::Rules,
    import::{self, Report},
    reading,
};
use serde_json::{Value, json};
use sqlx::{ConnectOptions, PgConnection, PgPool, Row};
use time::{Duration, OffsetDateTime, macros::datetime};
use uuid::Uuid;

const ALL: Page = Page {
    before: None,
    limit: 50,
    unread_only: false,
};

fn rules() -> Rules {
    Rules::built_in().unwrap()
}

/// An empty old database, and a connection to it.
async fn legacy(db: &PgPool) -> PgConnection {
    sqlx::query("CREATE SCHEMA legacy")
        .execute(db)
        .await
        .unwrap();
    let options = db
        .connect_options()
        .as_ref()
        .clone()
        .options([("search_path", "legacy")]);
    let mut old = options.connect().await.unwrap();
    sqlx::raw_sql(include_str!("fixtures/legacy_schema.sql"))
        .execute(&mut old)
        .await
        .unwrap();
    old
}

fn feed_id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn article_id(seq: i64) -> Uuid {
    Uuid::from_u128(1000 + seq as u128)
}

fn user_id(n: u128) -> Uuid {
    Uuid::from_u128(100 + n)
}

/// A feed the old rueder fetched on 2026-11-01 at 10:00.
async fn old_feed(old: &mut PgConnection, n: u128, name: &str, state: Value) -> Uuid {
    sqlx::query(
        "INSERT INTO feeds (id, created_at, updated_at, fetched_at, fetch_delay_s, fetcher_state,
                            feed_url, site_url, title, icon)
         VALUES ($1, '2021-05-01 08:00', '2026-11-01 10:00', '2026-11-01 10:00', 1800, $2,
                 'https://' || $3 || '.example/feed', 'https://' || $3 || '.example/',
                 'Feed ' || upper($3), 'https://' || $3 || '.example/icon.png')",
    )
    .bind(feed_id(n))
    .bind(state)
    .bind(name)
    .execute(old)
    .await
    .unwrap();
    feed_id(n)
}

/// An article the old rueder stored `seq` hours after the start of 2022.
async fn old_article(old: &mut PgConnection, seq: i64, feed: Uuid, guid: &str, content: Value) {
    sqlx::query(
        "INSERT INTO articles (id, created_at, updated_at, seq, feed_id, site_guid, posted_at, link,
                               thumbnail, image, image_title, title, teaser, content)
         VALUES ($1, timestamp '2022-01-01' + $2 * interval '1 hour', now(), $2, $3, $4,
                 timestamp '2021-12-31' + $2 * interval '1 hour', 'https://blog.example/' || $4,
                 'https://blog.example/' || $4 || '-small.jpg', 'https://blog.example/' || $4 || '.jpg',
                 'Picture of ' || $4, 'Title of ' || $4, 'Teaser of ' || $4, $5)",
    )
    .bind(article_id(seq))
    .bind(seq)
    .bind(feed)
    .bind(guid)
    .bind(content)
    .execute(old)
    .await
    .unwrap();
}

async fn old_user(old: &mut PgConnection, n: u128, origin: &str, subject: &str, folders: Value) {
    sqlx::query(
        "INSERT INTO users (id, created_at, updated_at, auth_origin, auth_subject, folders)
         VALUES ($1, '2021-06-01 09:00', now(), $2, $3, $4)",
    )
    .bind(user_id(n))
    .bind(origin)
    .bind(subject)
    .bind(folders)
    .execute(old)
    .await
    .unwrap();
}

async fn old_subscription(old: &mut PgConnection, user: u128, feed: Uuid) {
    sqlx::query("INSERT INTO user_feeds (user_id, feed_id) VALUES ($1, $2)")
        .bind(user_id(user))
        .bind(feed)
        .execute(old)
        .await
        .unwrap();
}

fn text(guid: &str) -> Value {
    json!({ "text": format!("<p>Text of {guid}</p>") })
}

/// Two feeds with articles that arrived mixed, a feed that was never fetched,
/// and two users. Returns the old feeds' ids.
async fn old_rueder(old: &mut PgConnection) -> (Uuid, Uuid, Uuid) {
    let a = old_feed(
        old,
        1,
        "a",
        json!({
            "etag": "\"abc\"", "last_modified": "Sun, 01 Nov 2026 09:00:00 GMT", "working": true,
            "last_success": "2026-11-01T10:00:00Z", "last_error": "0001-01-01T00:00:00Z"
        }),
    )
    .await;
    let b = old_feed(
        old,
        2,
        "b",
        json!({
            "working": false, "last_success": "2026-10-01T10:00:00+02:00",
            "last_error": "2026-11-01T10:00:00.5Z", "message": "Fetcher Error: timeout"
        }),
    )
    .await;
    let c = old_feed(old, 3, "c", json!({ "working": false })).await;
    sqlx::query(
        "UPDATE feeds SET title = NULL, site_url = '', icon = NULL, fetch_delay_s = 60
         WHERE id = $1",
    )
    .bind(b)
    .execute(&mut *old)
    .await
    .unwrap();
    // Go's zero time, which is what "never" looks like.
    sqlx::query(
        "UPDATE feeds SET fetched_at = '0001-01-01 00:00', fetch_delay_s = 0 WHERE id = $1",
    )
    .bind(c)
    .execute(&mut *old)
    .await
    .unwrap();

    // The old numbers have holes where articles were deleted.
    let full = json!({
        "authors": ["Ada", "Grace"], "tags": ["news", "rust"], "text": "<p>Text of a-1</p>",
        "enclosures": [{ "url": "https://a.example/1.mp3", "length": "123", "type": "audio/mpeg" }]
    });
    old_article(old, 1, a, "a-1", full).await;
    old_article(old, 2, b, "b-1", text("b-1")).await;
    old_article(old, 3, a, "a-2", text("a-2")).await;
    old_article(old, 5, a, "a-3", json!({})).await;
    old_article(old, 6, b, "b-2", text("b-2")).await;
    old_article(old, 9, a, "a-4", text("a-4")).await;
    sqlx::query(
        "UPDATE articles SET title = NULL, teaser = NULL, link = NULL, thumbnail = '', image = NULL,
                             image_title = NULL, posted_at = '0001-01-01 00:00'
         WHERE seq = 5",
    )
    .execute(&mut *old)
    .await
    .unwrap();

    let folders = json!([
        { "id": Uuid::from_u128(501), "title": "News", "feeds": [
            { "id": b, "title": "My B", "icon": "" },
            { "id": a, "title": "Feed A" }
        ] },
        { "id": Uuid::from_u128(502), "title": "", "feeds": [
            { "id": Uuid::from_u128(999), "title": "Gone" },
            { "id": a, "title": "Feed A again" }
        ] }
    ]);
    old_user(old, 1, "htpasswd", "alice", folders).await;
    for feed in [a, b, c] {
        old_subscription(old, 1, feed).await;
    }
    old_user(old, 2, "htpasswd", "bob", Value::Null).await;
    old_subscription(old, 2, a).await;
    (a, b, c)
}

/// The new id of an old feed.
async fn new_feed(db: &PgPool, legacy_id: Uuid) -> i32 {
    sqlx::query_scalar("SELECT id FROM feeds WHERE legacy_id = $1")
        .bind(legacy_id)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn new_account(db: &PgPool, username: &str) -> i32 {
    sqlx::query_scalar("SELECT id FROM accounts WHERE username = $1")
        .bind(username)
        .fetch_one(db)
        .await
        .unwrap()
}

/// The articles of the new database as (feed, number, place in the order of
/// arrival, id in the feed), in the order of arrival.
async fn new_articles(db: &PgPool) -> Vec<(i32, i32, i64, String)> {
    sqlx::query_as(
        "SELECT a.feed_id, a.seq, a.id, c.guid FROM articles a JOIN article_content c
         USING (feed_id, seq) ORDER BY a.id",
    )
    .fetch_all(db)
    .await
    .unwrap()
}

/// Whether every imported article has the number the old rueder gives it.
async fn numbers_match(db: &PgPool) -> bool {
    let off: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM (
             SELECT id, row_number() OVER (PARTITION BY feed_id ORDER BY seq) AS feed_seq
             FROM legacy.articles
         ) o JOIN legacy_articles l ON l.legacy_id = o.id
         WHERE l.seq <> o.feed_seq",
    )
    .fetch_one(db)
    .await
    .unwrap();
    off == 0
}

#[sqlx::test]
async fn brings_over_feeds_articles_accounts_folders_and_subscriptions(db: PgPool) {
    let mut old = legacy(&db).await;
    let (a, b, c) = old_rueder(&mut old).await;
    let before = OffsetDateTime::now_utc();

    let report = import::run(&db, &mut old, &rules()).await.unwrap();
    assert_eq!(
        report,
        Report {
            feeds_added: 3,
            articles_added: 6,
            accounts_added: 2,
            folders_added: 2,
            subscriptions_added: 4,
            notes: vec!["1 subscriptions to feeds that do not exist were left out".into()],
            ..Report::default()
        }
    );
    let (a, b, c) = (
        new_feed(&db, a).await,
        new_feed(&db, b).await,
        new_feed(&db, c).await,
    );

    // Feeds, with what the fetcher needs to carry on where the old one stopped.
    let feed = |id| {
        sqlx::query("SELECT * FROM feeds WHERE id = $1")
            .bind(id)
            .fetch_one(&db)
    };
    let at =
        |row: &sqlx::postgres::PgRow, column: &str| -> Option<OffsetDateTime> { row.get(column) };
    let text = |row: &sqlx::postgres::PgRow, column: &str| -> Option<String> { row.get(column) };
    let row = feed(a).await.unwrap();
    assert_eq!(
        text(&row, "feed_url").as_deref(),
        Some("https://a.example/feed")
    );
    assert_eq!(
        text(&row, "site_url").as_deref(),
        Some("https://a.example/")
    );
    assert_eq!(text(&row, "title").as_deref(), Some("Feed A"));
    assert_eq!(
        text(&row, "icon_url").as_deref(),
        Some("https://a.example/icon.png")
    );
    assert_eq!(text(&row, "etag").as_deref(), Some("\"abc\""));
    assert_eq!(
        text(&row, "last_modified").as_deref(),
        Some("Sun, 01 Nov 2026 09:00:00 GMT")
    );
    assert_eq!(text(&row, "fetch_agent").as_deref(), Some("rueder"));
    assert_eq!(row.get::<i32, _>("fetch_interval_s"), 1800);
    assert_eq!(row.get::<i32, _>("last_seq"), 4);
    assert_eq!(
        at(&row, "created_at"),
        Some(datetime!(2021-05-01 08:00 UTC))
    );
    assert_eq!(
        at(&row, "fetched_at"),
        Some(datetime!(2026-11-01 10:00 UTC))
    );
    assert_eq!(
        at(&row, "next_fetch_at"),
        Some(datetime!(2026-11-01 10:30 UTC))
    );
    assert_eq!(
        at(&row, "last_success_at"),
        Some(datetime!(2026-11-01 10:00 UTC))
    );
    assert_eq!(at(&row, "last_error_at"), None);
    assert_eq!(text(&row, "error_message"), None);

    // A feed that fails, without a title, with a delay under the lower bound.
    let row = feed(b).await.unwrap();
    assert_eq!(text(&row, "title").as_deref(), Some(""));
    assert_eq!(text(&row, "site_url"), None);
    assert_eq!(text(&row, "icon_url"), None);
    assert_eq!(text(&row, "etag"), None);
    assert_eq!(row.get::<i32, _>("fetch_interval_s"), 900);
    assert_eq!(row.get::<i32, _>("last_seq"), 2);
    assert_eq!(
        at(&row, "last_success_at"),
        Some(datetime!(2026-10-01 08:00 UTC))
    );
    assert_eq!(
        at(&row, "last_error_at"),
        Some(datetime!(2026-11-01 10:00:00.5 UTC))
    );
    assert_eq!(
        text(&row, "error_message").as_deref(),
        Some("Fetcher Error: timeout")
    );

    // A feed that was never fetched is due now.
    let row = feed(c).await.unwrap();
    assert_eq!(at(&row, "fetched_at"), None);
    assert_eq!(text(&row, "error_message"), None);
    assert_eq!(row.get::<i32, _>("last_seq"), 0);
    let due = at(&row, "next_fetch_at").unwrap();
    assert!(due >= before - Duration::seconds(1) && due <= OffsetDateTime::now_utc());

    // Articles: numbered per feed the way the old rueder counts them, in the old
    // order of arrival.
    let guid = |name: &str| name.to_owned();
    assert_eq!(
        new_articles(&db).await,
        [
            (a, 1, 1, guid("a-1")),
            (b, 1, 2, guid("b-1")),
            (a, 2, 3, guid("a-2")),
            (a, 3, 5, guid("a-3")),
            (b, 2, 6, guid("b-2")),
            (a, 4, 9, guid("a-4")),
        ]
    );
    assert!(numbers_match(&db).await);
    let mapped: Vec<(Uuid, i32, i32)> =
        sqlx::query_as("SELECT legacy_id, feed_id, seq FROM legacy_articles ORDER BY legacy_id")
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(
        mapped,
        [
            (article_id(1), a, 1),
            (article_id(2), b, 1),
            (article_id(3), a, 2),
            (article_id(5), a, 3),
            (article_id(6), b, 2),
            (article_id(9), a, 4),
        ]
    );

    // Accounts log in under their old name and have no password yet.
    let accounts: Vec<(String, Option<String>, Uuid, OffsetDateTime)> = sqlx::query_as(
        "SELECT username, password_hash, legacy_id, created_at FROM accounts ORDER BY username",
    )
    .fetch_all(&db)
    .await
    .unwrap();
    let created = datetime!(2021-06-01 09:00 UTC);
    assert_eq!(
        accounts,
        [
            ("alice".to_owned(), None, user_id(1), created),
            ("bob".to_owned(), None, user_id(2), created),
        ]
    );
    let alice = new_account(&db, "alice").await;
    let bob = new_account(&db, "bob").await;
    let identities: Vec<(String, String, i32)> = sqlx::query_as(
        "SELECT issuer, subject, account_id FROM account_identities ORDER BY subject",
    )
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(
        identities,
        [
            ("htpasswd".to_owned(), "alice".to_owned(), alice),
            ("htpasswd".to_owned(), "bob".to_owned(), bob),
        ]
    );

    // The tree: folders and the feeds in them in the order of the old JSON, a
    // feed that was in no folder behind the folders.
    let folders: Vec<(String, Option<i32>, f64)> = sqlx::query_as(
        "SELECT title, parent_id, position FROM folders WHERE account_id = $1 ORDER BY position",
    )
    .bind(alice)
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(
        folders,
        [
            ("News".to_owned(), None, 1.0),
            ("Folder 2".to_owned(), None, 2.0)
        ]
    );
    let subscriptions = |account_id| {
        sqlx::query_as::<_, (i32, Option<String>, f64, Option<String>, i32)>(
            "SELECT s.feed_id, f.title, s.position, s.title, s.read_upto
             FROM subscriptions s LEFT JOIN folders f ON f.id = s.folder_id
             WHERE s.account_id = $1 ORDER BY f.position, s.position",
        )
        .bind(account_id)
        .fetch_all(&db)
    };
    assert_eq!(
        subscriptions(alice).await.unwrap(),
        [
            // The user's own name for the feed is kept.
            (b, Some("News".to_owned()), 1.0, Some("My B".to_owned()), 2),
            // A feed stands where it stood first.
            (a, Some("News".to_owned()), 2.0, None, 4),
            (c, None, 3.0, None, 0),
        ]
    );
    assert_eq!(subscriptions(bob).await.unwrap(), [(a, None, 1.0, None, 4)]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM folders WHERE account_id = $1")
            .bind(bob)
            .fetch_one(&db)
            .await
            .unwrap(),
        0
    );

    // Nothing is unread: what was read is in the browsers until the switch.
    let unread = reading::unread_counts(&db, alice).await.unwrap();
    assert_eq!(unread.len(), 3);
    assert!(unread.iter().all(|feed| feed.unread == 0));

    // The reader's own queries see the articles.
    let list = articles::page(&db, alice, &[a, b, c], ALL).await.unwrap();
    let listed: Vec<_> = list.iter().map(|item| (item.feed_id, item.seq)).collect();
    assert_eq!(listed, [(a, 4), (b, 2), (a, 3), (a, 2), (b, 1), (a, 1)]);
    assert!(list.iter().all(|item| item.read && item.version == 1));
    let first = list.last().unwrap();
    assert_eq!(first.title, "Title of a-1");
    assert_eq!(first.teaser, "Teaser of a-1");
    assert_eq!(first.author, "Ada, Grace");
    assert_eq!(first.url.as_deref(), Some("https://blog.example/a-1"));
    assert_eq!(
        first.thumbnail_url.as_deref(),
        Some("https://blog.example/a-1-small.jpg")
    );
    assert_eq!(first.published_at, datetime!(2021-12-31 01:00 UTC));

    let opened = articles::open(&db, alice, a, 1).await.unwrap().unwrap();
    assert_eq!(opened.html, "<p>Text of a-1</p>");
    assert_eq!(opened.tags, ["news", "rust"]);
    assert_eq!(
        opened.enclosures,
        json!([{ "url": "https://a.example/1.mp3", "length": "123", "type": "audio/mpeg" }])
    );
    assert_eq!(
        opened.image_url.as_deref(),
        Some("https://blog.example/a-1.jpg")
    );
    assert_eq!(opened.image_title.as_deref(), Some("Picture of a-1"));

    // What the old rueder has is the article's first version.
    let versions = articles::versions(&db, alice, a, 1).await.unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].version, 1);
    assert_eq!(versions[0].fetched_at, datetime!(2022-01-01 01:00 UTC));
    assert_eq!(versions[0].original.title, "Title of a-1");
    assert_eq!(versions[0].original.content, "<p>Text of a-1</p>");
    assert_eq!(versions[0].original.authors, ["Ada", "Grace"]);

    // An article with next to nothing: no title, no text, no date of its own.
    let bare = articles::open(&db, alice, a, 3).await.unwrap().unwrap();
    assert_eq!((bare.title.as_str(), bare.html.as_str()), ("", ""));
    assert_eq!(
        (bare.url, bare.image_url, bare.image_title),
        (None, None, None)
    );
    assert!(bare.tags.is_empty());
    assert_eq!(bare.enclosures, json!([]));
    assert_eq!(bare.published_at, datetime!(2022-01-01 05:00 UTC));
    assert_eq!(list[2].thumbnail_url, None);

    // What arrives from now on stands behind everything imported.
    assert_eq!(common::add(&db, a, 1).await, 5);
    let newest = &articles::page(&db, alice, &[a, b], ALL).await.unwrap()[0];
    assert_eq!((newest.feed_id, newest.seq, newest.read), (a, 5, false));
    assert!(newest.id > 9);

    // The old database is as it was.
    let old_rows: i64 = sqlx::query_scalar("SELECT count(*) FROM articles")
        .fetch_one(&mut old)
        .await
        .unwrap();
    assert_eq!(old_rows, 6);
    assert!(
        sqlx::query("DELETE FROM user_feeds")
            .execute(&mut old)
            .await
            .is_ok(),
        "the import leaves no transaction open on the old database"
    );
}

#[sqlx::test]
async fn a_second_run_adds_only_what_is_new(db: PgPool) {
    let mut old = legacy(&db).await;
    let (old_a, old_b, _) = old_rueder(&mut old).await;
    import::run(&db, &mut old, &rules()).await.unwrap();
    let (a, b) = (new_feed(&db, old_a).await, new_feed(&db, old_b).await);
    let alice = new_account(&db, "alice").await;

    // Right away, a second run finds nothing to do.
    let again = import::run(&db, &mut old, &rules()).await.unwrap();
    assert_eq!(
        again,
        Report {
            feeds_known: 3,
            articles_known: 6,
            accounts_known: 2,
            notes: vec!["1 subscriptions to feeds that do not exist were left out".into()],
            ..Report::default()
        }
    );

    // The new rueder is used: a feed gets another name, a subscription moves to
    // the top level, an article is set back to unread.
    sqlx::query("UPDATE feeds SET title = 'Renamed', etag = 'new' WHERE id = $1")
        .bind(a)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE subscriptions SET folder_id = NULL, position = 2.5
         WHERE account_id = $1 AND feed_id = $2",
    )
    .bind(alice)
    .bind(b)
    .execute(&db)
    .await
    .unwrap();
    assert!(reading::set_read(&db, alice, a, 2, false).await.unwrap());

    // Meanwhile the old one goes on: articles for a known feed, a new feed with
    // an article, a new user, and alice subscribes to the new feed.
    old_article(&mut old, 10, old_a, "a-5", text("a-5")).await;
    let old_d = old_feed(&mut old, 4, "d", json!({ "working": true })).await;
    old_article(&mut old, 11, old_d, "d-1", text("d-1")).await;
    old_article(&mut old, 12, old_a, "a-6", text("a-6")).await;
    old_user(&mut old, 3, "htpasswd", "carol", json!([])).await;
    old_subscription(&mut old, 3, old_d).await;
    old_subscription(&mut old, 1, old_d).await;

    let report = import::run(&db, &mut old, &rules()).await.unwrap();
    assert_eq!(
        report,
        Report {
            feeds_added: 1,
            feeds_known: 3,
            articles_added: 3,
            articles_known: 6,
            accounts_added: 1,
            accounts_known: 2,
            subscriptions_added: 2,
            notes: vec!["1 subscriptions to feeds that do not exist were left out".into()],
            ..Report::default()
        }
    );
    let d = new_feed(&db, old_d).await;

    // The new articles follow the old ones, numbered as the old rueder counts.
    let tail: Vec<_> = new_articles(&db).await.split_off(6);
    assert_eq!(
        tail,
        [
            (a, 5, 10, "a-5".to_owned()),
            (d, 1, 11, "d-1".to_owned()),
            (a, 6, 12, "a-6".to_owned()),
        ]
    );
    assert!(numbers_match(&db).await);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM legacy_articles")
            .fetch_one(&db)
            .await
            .unwrap(),
        9
    );

    // What changed in the new rueder stays.
    let (title, etag): (String, String) =
        sqlx::query_as("SELECT title, etag FROM feeds WHERE id = $1")
            .bind(a)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!((title.as_str(), etag.as_str()), ("Renamed", "new"));
    let tree: Vec<(i32, Option<i32>, f64)> = sqlx::query_as(
        "SELECT feed_id, folder_id, position FROM subscriptions WHERE account_id = $1
         AND folder_id IS NULL ORDER BY position",
    )
    .bind(alice)
    .fetch_all(&db)
    .await
    .unwrap();
    // The new subscription went to the end of the top level.
    assert_eq!(tree.len(), 3);
    assert_eq!((tree[0].0, tree[0].2), (b, 2.5));
    assert_eq!(tree[2].0, d);
    assert!(tree[2].2 > 3.0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM folders WHERE account_id = $1")
            .bind(alice)
            .fetch_one(&db)
            .await
            .unwrap(),
        2
    );

    // What alice read stays, the articles that came for her feeds are unread,
    // and the feed she is new to starts read.
    let unread: Vec<_> = reading::unread_counts(&db, alice)
        .await
        .unwrap()
        .into_iter()
        .map(|feed| (feed.feed_id, feed.unread))
        .filter(|&(_, unread)| unread > 0)
        .collect();
    assert_eq!(unread, [(a, 3)]);

    // The new account has its feed.
    let carol = new_account(&db, "carol").await;
    let list = articles::page(&db, carol, &[a, b, d], ALL).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!((list[0].feed_id, list[0].seq, list[0].read), (d, 1, true));
}

#[sqlx::test]
async fn articles_the_new_rueder_fetched_itself_come_only_once(db: PgPool) {
    let mut old = legacy(&db).await;
    let (old_a, old_b, _) = old_rueder(&mut old).await;
    import::run(&db, &mut old, &rules()).await.unwrap();
    let (a, b) = (new_feed(&db, old_a).await, new_feed(&db, old_b).await);

    // Both run side by side for a while. The new one fetches two articles of
    // feed A; the old one has one of them and another one, and one for feed B.
    let fetched = |guid: &str| {
        let mut article = common::article(7);
        article.guid = guid.to_owned();
        article
    };
    let mut tx = db.begin().await.unwrap();
    articles::insert(&mut tx, a, &[fetched("a-5"), fetched("a-new")])
        .await
        .unwrap();
    tx.commit().await.unwrap();
    old_article(&mut old, 10, old_a, "a-5", text("a-5")).await;
    old_article(&mut old, 11, old_a, "a-6", text("a-6")).await;
    old_article(&mut old, 12, old_b, "b-3", text("b-3")).await;

    let report = import::run(&db, &mut old, &rules()).await.unwrap();
    assert_eq!(
        (
            report.articles_added,
            report.articles_known,
            report.articles_matched
        ),
        (2, 6, 1)
    );
    // The old rueder counts a-6 as the feed's sixth article, here it is the seventh.
    assert_eq!(report.renumbered_feeds, ["Feed A"]);
    assert!(report.to_string().contains("- Feed A\n"));

    let all = new_articles(&db).await;
    let of_a: Vec<_> = all.iter().filter(|article| article.0 == a).collect();
    let guids: Vec<_> = of_a.iter().map(|article| article.3.as_str()).collect();
    assert_eq!(guids, ["a-1", "a-2", "a-3", "a-4", "a-5", "a-new", "a-6"]);
    let seqs: Vec<_> = of_a.iter().map(|article| article.1).collect();
    assert_eq!(seqs, [1, 2, 3, 4, 5, 6, 7]);
    // No two articles share a place in the order of arrival.
    let mut places: Vec<_> = all.iter().map(|article| article.2).collect();
    places.dedup();
    assert_eq!(places.len(), all.len());
    assert_eq!(
        all.iter().filter(|article| article.0 == b).count(),
        3,
        "feed B still got its article"
    );

    // The old article that was already there is known by its old id now, for
    // the labels of the browsers.
    let mapped: (i32, i32) =
        sqlx::query_as("SELECT feed_id, seq FROM legacy_articles WHERE legacy_id = $1")
            .bind(article_id(10))
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(mapped, (a, 5));

    // And once more: nothing left to bring.
    let again = import::run(&db, &mut old, &rules()).await.unwrap();
    assert_eq!(
        (
            again.articles_added,
            again.articles_known,
            again.articles_matched
        ),
        (0, 9, 0)
    );
    assert_eq!(again.renumbered_feeds, ["Feed A"]);
}

#[sqlx::test]
async fn odd_data_does_not_stop_the_import(db: PgPool) {
    let mut old = legacy(&db).await;
    let good = old_feed(&mut old, 1, "a", json!({ "working": true })).await;
    // What the fetcher kept is not what it should be.
    let odd = old_feed(&mut old, 2, "b", json!("broken")).await;
    let nowhere = old_feed(&mut old, 3, "c", json!({})).await;
    sqlx::query("UPDATE feeds SET feed_url = ' ' WHERE id = $1")
        .bind(nowhere)
        .execute(&mut old)
        .await
        .unwrap();
    old_article(&mut old, 1, good, "a-1", json!("just a string")).await;
    old_article(&mut old, 2, nowhere, "c-1", text("c-1")).await;
    old_article(&mut old, 3, odd, "b-1", json!({ "text": 5 })).await;
    old_article(&mut old, 4, good, "a-2", text("a-2")).await;

    // Two users whose names differ only in case, one without a name, one whose
    // folders are not a list.
    let folder = json!([{ "title": "Mine", "feeds": [{ "id": good }, { "id": nowhere }] }]);
    old_user(&mut old, 1, "htpasswd", "Alice", folder).await;
    old_user(&mut old, 2, "oidc", "alice", json!({ "folders": [] })).await;
    old_user(&mut old, 3, "oidc", "", json!([])).await;
    old_subscription(&mut old, 2, good).await;
    old_subscription(&mut old, 2, odd).await;
    sqlx::raw_sql(
        "INSERT INTO user_states
         VALUES ('00000000-0000-0000-0000-000000000001', now(), now(),
                 '00000000-0000-0000-0000-000000000001', '{}');
         INSERT INTO labels
         VALUES ('00000000-0000-0000-0000-000000000001', now(), now(), 'Later', '#ff0000');
         INSERT INTO labels
         VALUES ('00000000-0000-0000-0000-000000000002', now(), now(), 'Never', '#00ff00')",
    )
    .execute(&mut old)
    .await
    .unwrap();

    let report = import::run(&db, &mut old, &rules()).await.unwrap();
    assert_eq!(
        (
            report.feeds_added,
            report.articles_added,
            report.accounts_added
        ),
        (2, 3, 3)
    );
    assert_eq!((report.folders_added, report.subscriptions_added), (1, 3));
    assert_eq!(
        report.notes,
        [
            "1 feeds without an address were left out, with their articles",
            "1 articles of feeds that were left out were left out",
            "2 articles had content the import could not read; they came without text",
            "the folders of alice could not be read; the feeds came without folders",
            "1 subscriptions to feeds that do not exist were left out",
            "login names that were taken or empty got another one: alice is now alice-2,  is \
             now user",
            "the old table user_states has 1 rows; the old rueder never read them, and they were \
             not imported",
            "the old table labels has 2 rows; the old rueder never read them, and they were not \
             imported",
        ]
    );
    let printed = report.to_string();
    assert!(printed.contains("Feeds:         2 added, 0 already there\n"));
    assert!(printed.contains("\nNotes:\n- 1 feeds without an address"));

    let names: Vec<String> = sqlx::query_scalar("SELECT username FROM accounts ORDER BY id")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(names, ["Alice", "alice-2", "user"]);
    // The feed whose state was unreadable is there, and due at once.
    let odd = new_feed(&db, odd).await;
    let second = new_account(&db, "alice-2").await;
    let list = articles::page(&db, second, &[odd], ALL).await.unwrap();
    assert_eq!(list.len(), 1);
    let opened = articles::open(&db, second, odd, 1).await.unwrap().unwrap();
    assert_eq!(opened.html, "");
    assert_eq!(opened.title, "Title of b-1");
}

#[sqlx::test]
async fn an_empty_old_database_brings_nothing(db: PgPool) {
    let mut old = legacy(&db).await;
    assert_eq!(
        import::run(&db, &mut old, &rules()).await.unwrap(),
        Report::default()
    );
    // Without the old rueder's tables there is nothing to import from.
    sqlx::query("DROP TABLE articles")
        .execute(&mut old)
        .await
        .unwrap();
    let failed = import::run(&db, &mut old, &rules()).await.unwrap_err();
    assert!(format!("{failed:#}").contains("reading the old articles"));
    // Tables the old rueder got late may be missing.
    let mut older = {
        sqlx::query("DROP SCHEMA legacy CASCADE")
            .execute(&db)
            .await
            .unwrap();
        legacy(&db).await
    };
    sqlx::query("DROP TABLE user_states")
        .execute(&mut older)
        .await
        .unwrap();
    assert_eq!(
        import::run(&db, &mut older, &rules()).await.unwrap(),
        Report::default()
    );
}
