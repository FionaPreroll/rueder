//! Cleaning stored articles again, from their versions.

mod common;

use rueder::{
    articles,
    cleaning::Rules,
    feeds::{self, Stored},
    reclean::{self, Report},
};
use sqlx::PgPool;
use time::{OffsetDateTime, macros::datetime};

const NOW: OffsetDateTime = datetime!(2026-11-10 12:00 UTC);

/// A feed with two entries: one that stays as it is under any rules and, older
/// and so the feed's first article, one that other rules clean differently.
fn body(link: &str) -> Vec<u8> {
    format!(
        "<?xml version=\"1.0\"?><rss version=\"2.0\"><channel><title>Example</title>\
         <item><guid>plain</guid><title>Plain</title><link>{link}/plain</link>\
         <description><![CDATA[<p>Nothing to clean.</p>]]></description></item>\
         <item><guid>busy</guid><title>Busy</title><link>{link}/busy</link>\
         <enclosure url=\"{link}/busy.mp3\" length=\"9\" type=\"audio/mpeg\"/>\
         <description><![CDATA[<p>Text. <a href=\"more?ref=feed\">more</a></p>\
         <img src=\"https://ads.example.net/banner.png\">\
         <video src=\"https://ads.example.net/clip.mp4\"></video>\
         <video src=\"clip.mp4\"></video>]]></description></item>\
         </channel></rss>"
    )
    .into_bytes()
}

async fn fetch(db: &PgPool, feed: i32, link: &str, rules: &Rules) -> Stored {
    let url = format!("{link}/feed.xml");
    let parsed = feeds::parse(&body(link), &url, NOW, rules).unwrap();
    let mut tx = db.begin().await.unwrap();
    let stored = feeds::store(&mut tx, feed, &parsed.articles).await.unwrap();
    tx.commit().await.unwrap();
    stored
}

/// The text and the enclosures of the feed's busy article.
async fn busy(db: &PgPool, account: i32, feed: i32) -> (String, serde_json::Value) {
    let opened = articles::open(db, account, feed, 1).await.unwrap().unwrap();
    (opened.html, opened.enclosures)
}

#[sqlx::test]
async fn articles_are_cleaned_again_from_their_versions(db: PgPool) {
    let (account, feed) = common::reader(&db, "ada", 0).await;
    let (other_account, other_feed) = common::reader(&db, "bob", 0).await;
    sqlx::query("UPDATE feeds SET feed_url = 'https://' || title || '.example/feed.xml'")
        .execute(&db)
        .await
        .unwrap();
    let built_in = Rules::built_in().unwrap();
    fetch(&db, feed, "https://ada.example", &built_in).await;
    fetch(&db, other_feed, "https://bob.example", &built_in).await;
    let (before, _) = busy(&db, account, feed).await;
    assert!(before.contains("banner.png") && before.contains("?ref=feed"));

    // With the rules they were cleaned by, nothing comes out differently.
    let same = reclean::run(&db, &built_in, None).await.unwrap();
    assert_eq!(
        same,
        Report {
            articles: 4,
            changed: 0,
            unreadable: 0
        }
    );
    assert_eq!(
        same.to_string(),
        "4 articles cleaned again, 0 of them came out differently"
    );

    // With stricter lists, the article they speak of does, in the one feed asked for.
    let stricter = Rules::parse("ads.example.net\n", "ref\n").unwrap();
    let report = reclean::run(&db, &stricter, Some(feed)).await.unwrap();
    assert_eq!(
        report,
        Report {
            articles: 2,
            changed: 1,
            unreadable: 0
        }
    );
    assert_eq!(
        busy(&db, account, feed).await,
        (
            "<p>Text. <a href=\"https://ada.example/more\" target=\"_blank\" \
             rel=\"nofollow noopener noreferrer\">more</a></p>"
                .to_owned(),
            // The feed's own enclosure stays; of those in the text, the one
            // the lists now block is gone.
            serde_json::json!([
                { "url": "https://ada.example/busy.mp3", "type": "audio/mpeg", "length": 9 },
                { "url": "https://ada.example/clip.mp4", "type": null, "inline": true },
            ])
        )
    );
    let (untouched, enclosures) = busy(&db, other_account, other_feed).await;
    assert!(untouched.contains("banner.png"));
    assert_eq!(enclosures.as_array().unwrap().len(), 3);

    // Nothing but the text for reading changed: no new version, nothing unread.
    assert_eq!(
        articles::versions(&db, account, feed, 1)
            .await
            .unwrap()
            .len(),
        1
    );
    let (version, updated): (i16, Option<OffsetDateTime>) =
        sqlx::query_as("SELECT version, updated_at FROM articles WHERE feed_id = $1 AND seq = 1")
            .bind(feed)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!((version, updated), (1, None));
    // The next fetch under the same rules finds everything as it expects it.
    assert_eq!(
        fetch(&db, feed, "https://ada.example", &stricter).await,
        Stored::default()
    );
    assert_eq!(
        reclean::run(&db, &stricter, Some(feed))
            .await
            .unwrap()
            .changed,
        0
    );

    // All feeds, and a version nobody can read any more.
    sqlx::query("UPDATE article_versions SET raw = 'broken' WHERE feed_id = $1 AND seq = 2")
        .bind(other_feed)
        .execute(&db)
        .await
        .unwrap();
    let report = reclean::run(&db, &stricter, None).await.unwrap();
    assert_eq!(
        report,
        Report {
            articles: 4,
            changed: 1,
            unreadable: 1
        }
    );
    assert!(
        report
            .to_string()
            .ends_with("; 1 have a version that cannot be read and stay as they are")
    );
    assert!(
        !busy(&db, other_account, other_feed)
            .await
            .0
            .contains("banner.png")
    );
    // A feed that is not there has nothing to clean.
    assert_eq!(
        reclean::run(&db, &stricter, Some(-1)).await.unwrap(),
        Report::default()
    );
}
