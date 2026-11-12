//! The import, against the old rueder's schema rebuilt from `backend/migrations/`
//! (`fixtures/legacy_schema.sql`). The old database lives in a schema of its
//! own inside the test's database, so no second server is needed.

mod common;

use rueder::{
    articles::{self, Page},
    cleaning::Rules,
    import::{self, Options, Report},
    reading,
};
use serde_json::{Value, json};
use sqlx::{ConnectOptions, PgConnection, PgPool, Row};
use time::{Duration, OffsetDateTime, macros::datetime};
use uuid::Uuid;

const ALL: Page = Page {
    before: None,
    after: None,
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

    let report = import::run(&db, &mut old, &rules(), &Options::default())
        .await
        .unwrap();
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

/// A subscription: feed, folder, place, own title and read mark.
type Placed = (i32, Option<String>, f64, Option<String>, i32);

/// The new rueder in use before the import: what it has, by id.
struct InUse {
    /// Feed A under the old feed's address, with three articles it fetched
    /// itself: `a-4`, which the old rueder has too, `a-new1` and `a-new2`.
    a: i32,
    /// Feed B under an address that only looks different from the old one.
    b: i32,
    /// A feed the old rueder never had.
    x: i32,
    /// The old alice, with her name in capitals.
    alice: i32,
    /// Has the old bob's name, but bob goes to fiona.
    bob: i32,
    fiona: i32,
    /// Has no old user.
    carol: i32,
}

async fn in_use(db: &PgPool) -> InUse {
    let alice = common::account(db, "Alice").await;
    let bob = common::account(db, "bob").await;
    let fiona = common::account(db, "fiona").await;
    let carol = common::account(db, "carol").await;
    let feed = |url: &'static str, title: &'static str| {
        sqlx::query_scalar::<_, i32>(
            "INSERT INTO feeds (feed_url, title) VALUES ($1, $2) RETURNING id",
        )
        .bind(url)
        .bind(title)
        .fetch_one(db)
    };
    let a = feed("https://a.example/feed", "A here").await.unwrap();
    let b = feed("http://B.example/feed/", "Bee").await.unwrap();
    let x = common::feed(db, "x").await;
    let fetched = |guid: &str| {
        let mut article = common::article(7);
        article.guid = guid.to_owned();
        article.original.title = format!("Own {guid}");
        article
    };
    let mut tx = db.begin().await.unwrap();
    let own = [fetched("a-4"), fetched("a-new1"), fetched("a-new2")];
    articles::insert(&mut tx, a, &own).await.unwrap();
    articles::insert(&mut tx, x, &[fetched("x-1")])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // Alice has a folder "news" with X in it, and A on the top level under a
    // name of her own. She read a-4, starred a-new2 and put a label on a-new1.
    let news: i32 = sqlx::query_scalar(
        "INSERT INTO folders (account_id, title, position) VALUES ($1, 'news', 1) RETURNING id",
    )
    .bind(alice)
    .fetch_one(db)
    .await
    .unwrap();
    for (account_id, feed_id, folder_id, position, title, read_upto) in [
        (alice, x, Some(news), 1.0, None, 0),
        (alice, a, None, 2.0, Some("Mine"), 1),
        (carol, a, None, 1.0, None, 0),
    ] {
        sqlx::query(
            "INSERT INTO subscriptions (account_id, feed_id, folder_id, position, title, read_upto)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(account_id)
        .bind(feed_id)
        .bind(folder_id)
        .bind(position)
        .bind(title)
        .bind(read_upto)
        .execute(db)
        .await
        .unwrap();
    }
    sqlx::query(
        "INSERT INTO article_states (account_id, feed_id, seq, starred) VALUES ($1, $2, 3, true)",
    )
    .bind(alice)
    .bind(a)
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "WITH label AS (
             INSERT INTO labels (account_id, title, color, position)
             VALUES ($1, 'Later', '#ff0000', 1) RETURNING id
         )
         INSERT INTO article_labels (label_id, feed_id, seq) SELECT id, $2, 2 FROM label",
    )
    .bind(alice)
    .bind(a)
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO filter_rules (account_id, position, field, match, pattern, action)
         VALUES ($1, 1, 'title', 'contains', 'new1', 'hide')",
    )
    .bind(fiona)
    .execute(db)
    .await
    .unwrap();
    InUse {
        a,
        b,
        x,
        alice,
        bob,
        fiona,
        carol,
    }
}

fn bob_to_fiona() -> Options {
    Options {
        accounts: vec![("bob".into(), "fiona".into())],
        dry_run: false,
    }
}

/// The lines that say what went together, for the data of `old_rueder` and `in_use`.
fn merged_lines(new: &InUse) -> Vec<String> {
    vec![
        format!(
            "feed https://a.example/feed with 4 articles: into feed {} \"A here\"",
            new.a
        ),
        format!(
            "feed https://b.example/feed with 2 articles: into feed {} \"Bee\", \
             which is at http://B.example/feed/",
            new.b
        ),
        "user alice: into account Alice of the same name".into(),
        "user bob: into account fiona, as the call says".into(),
    ]
}

/// What the import of `old_rueder` into `in_use` comes to, whether in one run
/// or in several.
async fn check_merged_articles(db: &PgPool, new: &InUse, c: i32) {
    let InUse { a, b, x, .. } = *new;
    // The old articles come first, in their old order and with their old
    // places; what the new rueder fetched follows, a-4 only once.
    let guid = |name: &str| name.to_owned();
    assert_eq!(
        new_articles(db).await,
        [
            (a, 1, 1, guid("a-1")),
            (b, 1, 2, guid("b-1")),
            (a, 2, 3, guid("a-2")),
            (a, 3, 5, guid("a-3")),
            (b, 2, 6, guid("b-2")),
            (a, 4, 10, guid("a-4")),
            (a, 5, 11, guid("a-new1")),
            (a, 6, 12, guid("a-new2")),
            (x, 1, 13, guid("x-1")),
        ]
    );
    let feeds: Vec<(i32, i32, String, String, Option<Uuid>)> =
        sqlx::query_as("SELECT id, last_seq, feed_url, title, legacy_id FROM feeds ORDER BY id")
            .fetch_all(db)
            .await
            .unwrap();
    let url = |text: &str| text.to_owned();
    assert_eq!(
        feeds,
        [
            (
                a,
                6,
                url("https://a.example/feed"),
                url("A here"),
                Some(feed_id(1))
            ),
            (
                b,
                2,
                url("http://B.example/feed/"),
                url("Bee"),
                Some(feed_id(2))
            ),
            (x, 1, url("https://x.example/feed.xml"), url("x"), None),
            (
                c,
                0,
                url("https://c.example/feed"),
                url("Feed C"),
                Some(feed_id(3))
            ),
        ]
    );
    // Each old article knows where it went and which number the old rueder
    // gave it; a-4 is the one the new rueder had.
    let mapped: Vec<(Uuid, i32, i32, i32, bool)> = sqlx::query_as(
        "SELECT legacy_id, feed_id, seq, legacy_seq, matched FROM legacy_articles
         ORDER BY legacy_id",
    )
    .fetch_all(db)
    .await
    .unwrap();
    assert_eq!(
        mapped,
        [
            (article_id(1), a, 1, 1, false),
            (article_id(2), b, 1, 1, false),
            (article_id(3), a, 2, 2, false),
            (article_id(5), a, 3, 3, false),
            (article_id(6), b, 2, 2, false),
            (article_id(9), a, 4, 4, true),
        ]
    );
    // Text, versions and the copy for the search moved with their articles.
    let off: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM articles a FULL JOIN article_content c USING (feed_id, seq)
                 WHERE a.id IS DISTINCT FROM c.id OR a.title IS DISTINCT FROM c.title)
              + (SELECT count(*) FROM articles a FULL JOIN article_versions v USING (feed_id, seq)
                 WHERE a.id IS NULL OR v.raw IS NULL)",
    )
    .fetch_one(db)
    .await
    .unwrap();
    assert_eq!(off, 0);
}

#[sqlx::test]
async fn merges_into_a_rueder_that_is_in_use(db: PgPool) {
    let new = in_use(&db).await;
    let InUse {
        a,
        b,
        x,
        alice,
        bob,
        fiona,
        carol,
    } = new;
    let mut old = legacy(&db).await;
    let (_, _, old_c) = old_rueder(&mut old).await;

    let report = import::run(&db, &mut old, &rules(), &bob_to_fiona())
        .await
        .unwrap();
    assert_eq!(
        report,
        Report {
            feeds_added: 1,
            feeds_merged: 2,
            articles_added: 5,
            articles_matched: 1,
            accounts_merged: 2,
            subscriptions_added: 3,
            merged: merged_lines(&new),
            notes: vec!["1 subscriptions to feeds that do not exist were left out".into()],
            ..Report::default()
        }
    );
    let printed = report.to_string();
    assert!(printed.contains("Feeds:         1 added, 2 merged, 0 already there\n"));
    assert!(printed.contains("\nMerged with what the new rueder has:\n- feed https://a.example"));
    let c = new_feed(&db, old_c).await;
    check_merged_articles(&db, &new, c).await;

    // Accounts: the old users went into the accounts that were there; the
    // account with the name of the one that went elsewhere stays as it was.
    let accounts: Vec<(i32, Option<Uuid>)> =
        sqlx::query_as("SELECT id, legacy_id FROM accounts ORDER BY id")
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(
        accounts,
        [
            (alice, Some(user_id(1))),
            (bob, None),
            (fiona, Some(user_id(2))),
            (carol, None)
        ]
    );
    let identities: Vec<(String, i32)> =
        sqlx::query_as("SELECT subject, account_id FROM account_identities ORDER BY subject")
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(
        identities,
        [("alice".to_owned(), alice), ("bob".to_owned(), fiona)]
    );

    // Alice's tree: what she set up stays. Of her old feeds, B went into her
    // folder of the same title, with her old name for it; C, which was in no
    // folder, to the end of the top level. Her old second folder only held A,
    // which she has, so it did not come.
    let tree: Vec<Placed> = sqlx::query_as(
        "SELECT s.feed_id, f.title, s.position, s.title, s.read_upto
         FROM subscriptions s LEFT JOIN folders f ON f.id = s.folder_id
         WHERE s.account_id = $1 ORDER BY f.position NULLS LAST, s.position",
    )
    .bind(alice)
    .fetch_all(&db)
    .await
    .unwrap();
    let news = Some("news".to_owned());
    assert_eq!(
        tree,
        [
            (x, news.clone(), 1.0, None, 0),
            (b, news, 2.0, Some("My B".to_owned()), 2),
            (a, None, 2.0, Some("Mine".to_owned()), 4),
            (c, None, 3.0, None, 0),
        ]
    );
    let folders: i64 = sqlx::query_scalar("SELECT count(*) FROM folders")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(folders, 1);

    // What accounts noted on articles of A moved with them: the read mark past
    // the old articles, alice's star on a-new2 and her label on a-new1.
    let starred: Vec<(i32, i32, i32)> =
        sqlx::query_as("SELECT account_id, feed_id, seq FROM article_states WHERE starred")
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(starred, [(alice, a, 6)]);
    let labelled: Vec<(i32, i32)> = sqlx::query_as("SELECT feed_id, seq FROM article_labels")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(labelled, [(a, 5)]);
    let opened = articles::open(&db, alice, a, 5).await.unwrap().unwrap();
    assert_eq!(opened.title, "Own a-new1");
    let opened = articles::open(&db, alice, a, 1).await.unwrap().unwrap();
    assert_eq!(opened.title, "Title of a-1");

    // Unread is what the new rueder fetched and the account had not read; the
    // old articles count as read. For fiona, who is new to A, a-4 was in the old
    // rueder too, and her rule hides a-new1.
    let pool = &db;
    let unread = |account_id| async move {
        let mut counts: Vec<_> = reading::unread_counts(pool, account_id)
            .await
            .unwrap()
            .into_iter()
            .map(|feed| (feed.feed_id, feed.unread))
            .collect();
        counts.sort_unstable();
        counts
    };
    assert_eq!(unread(alice).await, [(a, 2), (b, 0), (x, 1), (c, 0)]);
    assert_eq!(unread(carol).await, [(a, 3)]);
    assert_eq!(unread(fiona).await, [(a, 1)]);
    assert!(unread(bob).await.is_empty());
    let list = articles::page(&db, fiona, &[a], ALL).await.unwrap();
    let listed: Vec<_> = list.iter().map(|item| (item.seq, item.read)).collect();
    assert_eq!(
        listed,
        [(6, false), (4, true), (3, true), (2, true), (1, true)]
    );

    // What is fetched from now on comes behind everything.
    assert_eq!(common::add(&db, a, 1).await, 7);
    let newest = &articles::page(&db, carol, &[a], ALL).await.unwrap()[0];
    assert_eq!((newest.seq, newest.read), (7, false));
    assert!(newest.id > 13);
}

#[sqlx::test]
async fn a_dry_run_and_a_wrong_call_write_nothing(db: PgPool) {
    let new = in_use(&db).await;
    let mut old = legacy(&db).await;
    old_rueder(&mut old).await;
    let state = async || {
        let feeds: Vec<(i32, i32, Option<Uuid>)> =
            sqlx::query_as("SELECT id, last_seq, legacy_id FROM feeds ORDER BY id")
                .fetch_all(&db)
                .await
                .unwrap();
        let accounts: Vec<(i32, Option<Uuid>)> =
            sqlx::query_as("SELECT id, legacy_id FROM accounts ORDER BY id")
                .fetch_all(&db)
                .await
                .unwrap();
        (feeds, accounts, new_articles(&db).await)
    };
    let before = state().await;

    let dry = Options {
        dry_run: true,
        ..bob_to_fiona()
    };
    let report = import::run(&db, &mut old, &rules(), &dry).await.unwrap();
    assert_eq!(
        report,
        Report {
            dry_run: true,
            feeds_added: 1,
            feeds_merged: 2,
            articles_added: 6,
            accounts_merged: 2,
            merged: merged_lines(&new),
            ..Report::default()
        }
    );
    let printed = report.to_string();
    assert!(printed.starts_with("Dry run: this is what the import would do."));
    assert!(printed.contains("Articles:      6 to bring, 0 already there\n"));
    assert!(!printed.contains("Subscriptions:"));
    assert_eq!(state().await, before);

    for (accounts, wrong) in [
        (
            vec![("nobody", "fiona")],
            "there is not exactly one old user nobody",
        ),
        (vec![("bob", "nobody")], "there is no account nobody"),
        (
            vec![("alice", "fiona"), ("bob", "fiona")],
            "fiona is named twice",
        ),
        (
            vec![("bob", "fiona"), ("bob", "carol")],
            "bob is named twice",
        ),
    ] {
        let options = Options {
            accounts: accounts
                .into_iter()
                .map(|(old, new)| (old.to_owned(), new.to_owned()))
                .collect(),
            dry_run: false,
        };
        let failed = import::run(&db, &mut old, &rules(), &options)
            .await
            .unwrap_err();
        assert!(failed.to_string().contains(wrong), "{failed}");
        assert_eq!(state().await, before);
    }
}

#[sqlx::test]
async fn a_run_that_stopped_halfway_carries_on(db: PgPool) {
    let new = in_use(&db).await;
    let mut old = legacy(&db).await;
    let (_, _, old_c) = old_rueder(&mut old).await;

    // The disk fills up in the middle of feed A, the first one.
    sqlx::raw_sql(
        "CREATE FUNCTION fail_on_a3() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
             IF NEW.guid = 'a-3' THEN RAISE EXCEPTION 'disk full'; END IF;
             RETURN NEW;
         END $$;
         CREATE TRIGGER fail_on_a3 BEFORE INSERT ON article_content
             FOR EACH ROW EXECUTE FUNCTION fail_on_a3()",
    )
    .execute(&db)
    .await
    .unwrap();
    let failed = import::run(&db, &mut old, &rules(), &bob_to_fiona())
        .await
        .unwrap_err();
    assert!(format!("{failed:#}").contains("disk full"));
    // Feed A is as it was, but for the places of its articles, which made room.
    let of_a: Vec<(i32, i64)> =
        sqlx::query_as("SELECT seq, id FROM articles WHERE feed_id = $1 ORDER BY seq")
            .bind(new.a)
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(of_a, [(1, 10), (2, 11), (3, 12)]);
    let brought: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM legacy_articles)
              + (SELECT count(*) FROM feeds WHERE legacy_id IS NOT NULL)
              + (SELECT count(*) FROM accounts WHERE legacy_id IS NOT NULL)",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(brought, 0);

    // Run again, and it comes out as if it had gone through at once.
    sqlx::query("DROP TRIGGER fail_on_a3 ON article_content")
        .execute(&db)
        .await
        .unwrap();
    let report = import::run(&db, &mut old, &rules(), &bob_to_fiona())
        .await
        .unwrap();
    assert_eq!(
        (
            report.feeds_merged,
            report.articles_added,
            report.accounts_merged
        ),
        (2, 5, 2)
    );
    let c = new_feed(&db, old_c).await;
    check_merged_articles(&db, &new, c).await;

    // A run that stops later, after feed A: the rest follows, and feed A keeps
    // what it got the first time.
    sqlx::query("UPDATE feeds SET legacy_id = NULL, last_seq = 0 WHERE id = $1")
        .bind(c)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("DELETE FROM feeds WHERE id = $1")
        .bind(c)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("UPDATE accounts SET legacy_id = NULL WHERE id = $1")
        .bind(new.fiona)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("DELETE FROM subscriptions WHERE account_id = $1")
        .bind(new.fiona)
        .execute(&db)
        .await
        .unwrap();
    let report = import::run(&db, &mut old, &rules(), &bob_to_fiona())
        .await
        .unwrap();
    assert_eq!(
        report,
        Report {
            feeds_added: 1,
            feeds_known: 2,
            articles_known: 6,
            accounts_merged: 1,
            accounts_known: 1,
            subscriptions_added: 1,
            merged: vec![merged_lines(&new)[3].clone()],
            ..Report::default()
        }
    );
    let c = new_feed(&db, old_c).await;
    // She has A again, and a-4 counts as read for her: it was in the old
    // rueder. Her rule hides a-new1 again.
    let unread: Vec<_> = reading::unread_counts(&db, new.fiona)
        .await
        .unwrap()
        .into_iter()
        .map(|feed| (feed.feed_id, feed.unread))
        .collect();
    assert_eq!(unread, [(new.a, 1)]);
    assert!(c > new.x);

    // And once more: nothing left to bring.
    let again = import::run(&db, &mut old, &rules(), &bob_to_fiona())
        .await
        .unwrap();
    assert_eq!(
        again,
        Report {
            feeds_known: 3,
            articles_known: 6,
            accounts_known: 2,
            ..Report::default()
        }
    );
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

    let report = import::run(&db, &mut old, &rules(), &Options::default())
        .await
        .unwrap();
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
            "login names that were taken or empty got another one: alice is now alice-2,  is \
             now user",
            "2 articles had content the import could not read; they came without text",
            "the folders of alice could not be read; the feeds came without folders",
            "1 subscriptions to feeds that do not exist were left out",
            "the old table user_states has 1 rows; the old rueder never read them, and they were \
             not imported",
            "the old table labels has 2 rows; the old rueder never read them, and they were not \
             imported",
        ]
    );
    let printed = report.to_string();
    assert!(printed.contains("Feeds:         2 added, 0 merged, 0 already there\n"));
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
async fn texts_are_cleaned_on_the_way_in(db: PgPool) {
    let mut old = legacy(&db).await;
    let feed = old_feed(&mut old, 1, "a", json!({ "working": true })).await;
    // The old rueder took scripts out when it fetched; the rest it left to the browser.
    let content = json!({
        "text": "<p>Text. <a href=\"/more?utm_source=feed\">more</a> <a href=\"#top\">up</a></p>\
                 <img src=\"https://a.fsdn.com/sd/count.gif\"><video src=\"clip.mp4\"></video>",
        "enclosures": [{ "url": "https://a.example/one.mp3", "length": "9", "type": "audio/mpeg" }]
    });
    old_article(&mut old, 1, feed, "a-1", content).await;
    sqlx::query(
        "UPDATE articles SET link = 'https://blog.example/posts/a-1?utm_campaign=x&id=7',
                             thumbnail = 'https://blog.example/a-1.jpg?utm_source=feed'",
    )
    .execute(&mut old)
    .await
    .unwrap();
    old_user(&mut old, 1, "htpasswd", "alice", json!([])).await;
    old_subscription(&mut old, 1, feed).await;
    import::run(&db, &mut old, &rules(), &Options::default())
        .await
        .unwrap();

    let (feed, alice) = (new_feed(&db, feed).await, new_account(&db, "alice").await);
    let opened = articles::open(&db, alice, feed, 1).await.unwrap().unwrap();
    assert_eq!(
        opened.html,
        "<p>Text. <a href=\"https://blog.example/more\" target=\"_blank\" \
         rel=\"nofollow noopener noreferrer\">more</a> \
         <a href=\"https://blog.example/posts/a-1?id=7#top\" target=\"_blank\" \
         rel=\"nofollow noopener noreferrer\">up</a></p>"
    );
    assert_eq!(
        opened.url.as_deref(),
        Some("https://blog.example/posts/a-1?id=7")
    );
    assert_eq!(
        opened.enclosures,
        json!([
            { "url": "https://a.example/one.mp3", "length": "9", "type": "audio/mpeg" },
            { "url": "https://blog.example/posts/clip.mp4", "type": null, "inline": true },
        ])
    );
    let list = articles::page(&db, alice, &[feed], ALL).await.unwrap();
    assert_eq!(
        list[0].thumbnail_url.as_deref(),
        Some("https://blog.example/a-1.jpg")
    );
    // The version keeps the text as the old rueder had it.
    let versions = articles::versions(&db, alice, feed, 1).await.unwrap();
    assert!(
        versions[0]
            .original
            .content
            .contains("/more?utm_source=feed")
    );
}

#[sqlx::test]
async fn an_empty_old_database_brings_nothing(db: PgPool) {
    let mut old = legacy(&db).await;
    assert_eq!(
        import::run(&db, &mut old, &rules(), &Options::default())
            .await
            .unwrap(),
        Report::default()
    );
    // Without the old rueder's tables there is nothing to import from.
    sqlx::query("DROP TABLE articles")
        .execute(&mut old)
        .await
        .unwrap();
    let failed = import::run(&db, &mut old, &rules(), &Options::default())
        .await
        .unwrap_err();
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
        import::run(&db, &mut older, &rules(), &Options::default())
            .await
            .unwrap(),
        Report::default()
    );
}

/// Random feeds on both sides, compared with the obvious model of a merge: the
/// old articles that only the old rueder has come first in their order, then
/// all the new rueder has in its order; what an account of the new rueder read
/// or starred there stays with the article, and old articles count as read.
#[sqlx::test]
async fn merging_matches_the_obvious_model(db: PgPool) {
    use common::Random;
    for seed in 1..=30 {
        let mut random = Random::new(seed);
        let name = |what: &str| format!("{what}-{seed}");
        let url = format!("https://feed-{seed}.example/feed");
        // Every round imports once into a rueder of its own.
        sqlx::query("TRUNCATE feeds, accounts RESTART IDENTITY CASCADE")
            .execute(&db)
            .await
            .unwrap();

        // The new rueder: a feed with some articles, one reader who read and
        // starred some of them.
        let feed: i32 = sqlx::query_scalar("INSERT INTO feeds (feed_url) VALUES ($1) RETURNING id")
            .bind(&url)
            .fetch_one(&db)
            .await
            .unwrap();
        let reader = common::account(&db, &name("reader")).await;
        common::subscribe(&db, reader, feed).await;
        let own: Vec<String> = (0..random.below(6))
            .map(|n| {
                // Some of them the old rueder has too.
                if random.below(3) == 0 {
                    format!("old-{}", random.below(8))
                } else {
                    format!("own-{n}")
                }
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut tx = db.begin().await.unwrap();
        let fetched: Vec<_> = own
            .iter()
            .map(|guid| {
                let mut article = common::article(1);
                article.guid = guid.clone();
                article
            })
            .collect();
        articles::insert(&mut tx, feed, &fetched).await.unwrap();
        tx.commit().await.unwrap();
        // (read, starred) of what the reader has.
        let mut model: Vec<(String, bool, bool)> = Vec::new();
        for (n, guid) in own.iter().enumerate() {
            let seq = n as i32 + 1;
            let (read, starred) = (random.below(2) == 0, random.below(3) == 0);
            reading::set_read(&db, reader, feed, seq, read)
                .await
                .unwrap();
            reading::set_starred(&db, reader, feed, seq, starred)
                .await
                .unwrap();
            model.push((guid.clone(), read, starred));
        }

        // The old rueder: the same feed with articles of its own, numbered with
        // holes, read by the reader and by somebody new.
        sqlx::query("DROP SCHEMA IF EXISTS legacy CASCADE")
            .execute(&db)
            .await
            .unwrap();
        let mut old = legacy(&db).await;
        // The ids of the old rueder are its own in each round, as the new
        // rueder remembers them.
        let old_feed_id = old_feed(&mut old, seed.into(), &format!("feed-{seed}"), json!({})).await;
        let mut old_guids = Vec::new();
        let mut seq = seed as i64 * 100;
        for n in 0..random.below(9) {
            seq += 1 + random.below(3) as i64;
            let guid = format!("old-{n}");
            old_article(&mut old, seq, old_feed_id, &guid, text(&guid)).await;
            old_guids.push(guid);
        }
        for (n, user) in [name("reader"), name("newbie")].iter().enumerate() {
            let id = u128::from(seed) * 2 + n as u128;
            old_user(&mut old, id, "htpasswd", user, json!([])).await;
            old_subscription(&mut old, id, old_feed_id).await;
        }

        let report = import::run(&db, &mut old, &rules(), &Options::default())
            .await
            .unwrap();
        let log = format!("seed {seed}: own {own:?}, old {old_guids:?}");
        assert_eq!(report.feeds_merged, 1, "{log}");
        assert_eq!(report.accounts_merged, 1, "{log}");

        let only_old: Vec<_> = old_guids.iter().filter(|g| !own.contains(g)).collect();
        let mut wanted: Vec<(String, bool, bool)> = only_old
            .iter()
            .map(|guid| (guid.to_string(), true, false))
            .collect();
        wanted.extend(model);
        let listed = |account_id| {
            let (db, log) = (&db, &log);
            async move {
                let page = Page { limit: 100, ..ALL };
                let mut list = articles::page(db, account_id, &[feed], page).await.unwrap();
                list.reverse();
                let mut seen = Vec::new();
                for (n, item) in list.iter().enumerate() {
                    assert_eq!(item.seq, n as i32 + 1, "{log}");
                    let guid: String = sqlx::query_scalar(
                        "SELECT guid FROM article_content WHERE feed_id = $1 AND seq = $2",
                    )
                    .bind(feed)
                    .bind(item.seq)
                    .fetch_one(db)
                    .await
                    .unwrap();
                    seen.push((guid, item.read, item.starred));
                }
                assert!(list.windows(2).all(|pair| pair[0].id < pair[1].id), "{log}");
                seen
            }
        };
        assert_eq!(listed(reader).await, wanted, "{log}");
        let unread = |account_id| {
            let db = &db;
            async move {
                reading::unread_counts(db, account_id)
                    .await
                    .unwrap()
                    .iter()
                    .map(|feed| feed.unread)
                    .sum::<i64>()
            }
        };
        let reader_unread = wanted.iter().filter(|w| !w.1).count() as i64;
        assert_eq!(unread(reader).await, reader_unread, "{log}");

        // Somebody new to the feed has read what the old rueder had, and
        // nothing of what only the new one fetched.
        let newbie = new_account(&db, &name("newbie")).await;
        let newbie_wanted: Vec<_> = wanted
            .iter()
            .map(|(guid, _, _)| (guid.clone(), old_guids.contains(guid), false))
            .collect();
        assert_eq!(listed(newbie).await, newbie_wanted, "{log}");
        let newbie_unread = newbie_wanted.iter().filter(|w| !w.1).count() as i64;
        assert_eq!(unread(newbie).await, newbie_unread, "{log}");
    }
}
