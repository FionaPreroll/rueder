mod common;

use std::collections::{HashMap, HashSet};

use common::{Random, account, add, article, feed, reader, subscribe};
use rueder::{
    articles::{self, ListItem, MAX_VERSIONS, Page},
    reading,
};
use sqlx::PgPool;
use time::Duration;

const ALL: Page = Page {
    before: None,
    limit: 50,
    unread_only: false,
};

fn numbers(items: &[ListItem]) -> Vec<(i32, i32)> {
    items.iter().map(|item| (item.feed_id, item.seq)).collect()
}

#[sqlx::test]
async fn numbers_come_in_order_without_gaps(db: PgPool) {
    let (account_id, feed_id) = reader(&db, "a", 0).await;
    assert_eq!(add(&db, feed_id, 3).await, 1);
    assert_eq!(add(&db, feed_id, 0).await, 4);
    assert_eq!(add(&db, feed_id, 2).await, 4);

    let last: i32 = sqlx::query_scalar("SELECT last_seq FROM feeds WHERE id = $1")
        .bind(feed_id)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(last, 5);

    let items = articles::page(&db, account_id, &[feed_id], ALL)
        .await
        .unwrap();
    assert_eq!(numbers(&items), [5, 4, 3, 2, 1].map(|seq| (feed_id, seq)));
    // The order of arrival rises with the number.
    assert!(items.windows(2).all(|pair| pair[0].id > pair[1].id));
    assert_eq!(items[4].title, "Article 1");
    assert_eq!(items[4].author, "Ada, Grace");
    assert_eq!(items[4].version, 1);
    assert_eq!(items[4].updated_at, None);
}

#[sqlx::test]
async fn an_unknown_feed_takes_no_articles(db: PgPool) {
    let mut tx = db.begin().await.unwrap();
    assert!(articles::insert(&mut tx, 404, &[article(1)]).await.is_err());
}

#[sqlx::test]
async fn an_article_comes_only_once_per_feed(db: PgPool) {
    let feed_id = feed(&db, "a").await;
    add(&db, feed_id, 2).await;
    let mut tx = db.begin().await.unwrap();
    // Article 2 again, under the same id of the feed.
    assert!(
        articles::insert(&mut tx, feed_id, &[article(2)])
            .await
            .is_err()
    );
}

#[sqlx::test]
async fn a_list_mixes_feeds_in_the_order_of_arrival(db: PgPool) {
    let account_id = account(&db, "reader").await;
    let (a, b, c) = (
        feed(&db, "a").await,
        feed(&db, "b").await,
        feed(&db, "c").await,
    );
    for feed_id in [a, b] {
        subscribe(&db, account_id, feed_id).await;
    }
    add(&db, a, 2).await;
    add(&db, b, 1).await;
    add(&db, c, 1).await;
    add(&db, a, 1).await;
    add(&db, b, 2).await;

    let page = |before, limit| Page {
        before,
        limit,
        unread_only: false,
    };
    let all = articles::page(&db, account_id, &[a, b, c], page(None, 50))
        .await
        .unwrap();
    // Feed c is not subscribed to, so its article stays out.
    assert_eq!(
        numbers(&all),
        [(b, 3), (b, 2), (a, 3), (b, 1), (a, 2), (a, 1)]
    );

    let first = articles::page(&db, account_id, &[a, b], page(None, 2))
        .await
        .unwrap();
    assert_eq!(numbers(&first), [(b, 3), (b, 2)]);
    let anchor = first.last().map(|item| item.id);
    let second = articles::page(&db, account_id, &[a, b], page(anchor, 3))
        .await
        .unwrap();
    assert_eq!(numbers(&second), [(a, 3), (b, 1), (a, 2)]);

    let only_a = articles::page(&db, account_id, &[a], page(None, 50))
        .await
        .unwrap();
    assert_eq!(numbers(&only_a), [(a, 3), (a, 2), (a, 1)]);
    assert!(
        articles::page(&db, account_id, &[], page(None, 50))
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test]
async fn nobody_reads_feeds_of_others(db: PgPool) {
    let (_, feed_id) = reader(&db, "subscriber", 3).await;
    let stranger = account(&db, "stranger").await;

    assert!(
        articles::page(&db, stranger, &[feed_id], ALL)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        articles::open(&db, stranger, feed_id, 1).await.unwrap(),
        None
    );
    assert!(
        articles::versions(&db, stranger, feed_id, 1)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test]
async fn opening_gives_the_cleaned_text(db: PgPool) {
    let (account_id, feed_id) = reader(&db, "a", 2).await;

    let opened = articles::open(&db, account_id, feed_id, 2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(opened.title, "Article 2");
    assert_eq!(opened.html, "<p>Text of article 2.</p>");
    assert_eq!(opened.tags, ["news", "tag-2"]);
    assert_eq!(opened.url.as_deref(), Some("https://blog.example/2"));
    assert_eq!(opened.enclosures, serde_json::json!([]));

    assert_eq!(
        articles::open(&db, account_id, feed_id, 3).await.unwrap(),
        None
    );
}

#[sqlx::test]
async fn a_changed_article_gets_a_version_and_stays_read(db: PgPool) {
    let (account_id, feed_id) = reader(&db, "a", 2).await;
    reading::mark_all_read(&db, account_id, feed_id, 2)
        .await
        .unwrap();

    let mut changed = article(2);
    changed.original.title = "Article 2, corrected".into();
    changed.original.content = "<p>Corrected text.</p>".into();
    changed.html = "<p>Corrected text.</p>".into();
    changed.fetched_at += Duration::days(40);

    let mut tx = db.begin().await.unwrap();
    // The same text with other line breaks is no new version.
    let mut reflowed = article(2);
    reflowed.original.content = reflowed.original.content.replace(' ', "\n ");
    assert_eq!(
        articles::add_version(&mut tx, feed_id, 2, &reflowed)
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        articles::add_version(&mut tx, feed_id, 2, &changed)
            .await
            .unwrap(),
        Some(2)
    );
    assert_eq!(
        articles::add_version(&mut tx, feed_id, 2, &changed)
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        articles::add_version(&mut tx, feed_id, 9, &changed)
            .await
            .unwrap(),
        None
    );
    tx.commit().await.unwrap();

    let items = articles::page(&db, account_id, &[feed_id], ALL)
        .await
        .unwrap();
    assert_eq!(items[0].title, "Article 2, corrected");
    assert_eq!(items[0].version, 2);
    assert_eq!(items[0].updated_at, Some(changed.fetched_at));
    assert!(items[0].read, "an updated article stays read");
    let unread = reading::unread_counts(&db, account_id).await.unwrap();
    assert_eq!(unread[0].unread, 0);

    let opened = articles::open(&db, account_id, feed_id, 2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(opened.html, "<p>Corrected text.</p>");

    let versions = articles::versions(&db, account_id, feed_id, 2)
        .await
        .unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version, 1);
    assert_eq!(versions[0].original, article(2).original);
    assert_eq!(versions[1].original, changed.original);
    assert_eq!(versions[1].fetched_at, changed.fetched_at);

    // Each version lies in the partition of the month it was fetched in.
    let partitions: Vec<String> = sqlx::query_scalar(
        "SELECT tableoid::regclass::text FROM article_versions
         WHERE feed_id = $1 AND seq = 2 ORDER BY version",
    )
    .bind(feed_id)
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(
        partitions,
        ["article_versions_2026_03", "article_versions_2026_04"]
    );
}

#[sqlx::test]
async fn an_article_keeps_its_first_and_its_newest_versions(db: PgPool) {
    let (account_id, feed_id) = reader(&db, "a", 1).await;
    let mut tx = db.begin().await.unwrap();
    for n in 2..=25 {
        let mut changed = article(1);
        changed.original.content = format!("<p>Version {n}.</p>");
        changed.fetched_at += Duration::hours(n);
        let version = articles::add_version(&mut tx, feed_id, 1, &changed)
            .await
            .unwrap();
        assert_eq!(version, Some(n as i16));
    }
    tx.commit().await.unwrap();

    let versions = articles::versions(&db, account_id, feed_id, 1)
        .await
        .unwrap();
    let kept: Vec<_> = versions.iter().map(|v| v.version).collect();
    let expected: Vec<i16> = std::iter::once(1).chain(7..=25).collect();
    assert_eq!(kept.len() as i64, MAX_VERSIONS);
    assert_eq!(kept, expected);
    assert_eq!(versions[0].original, article(1).original);
}

#[sqlx::test]
async fn the_server_makes_the_partitions_it_will_need(db: PgPool) {
    articles::ensure_current_partitions(&db).await.unwrap();
    articles::ensure_current_partitions(&db).await.unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_inherits WHERE inhparent = 'article_versions'::regclass",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(count, 2);
}

/// What the page should be, worked out the slow and obvious way.
async fn expected_page(
    db: &PgPool,
    account_id: i32,
    feed_ids: &[i32],
    page: Page,
) -> Vec<(i32, i32, bool, bool)> {
    let marks: HashMap<i32, i32> =
        sqlx::query_as("SELECT feed_id, read_upto FROM subscriptions WHERE account_id = $1")
            .bind(account_id)
            .fetch_all(db)
            .await
            .unwrap()
            .into_iter()
            .collect();
    let states: HashMap<(i32, i32), (bool, bool, bool, bool)> = sqlx::query_as(
        "SELECT feed_id, seq, read, kept_unread, hidden, starred FROM article_states
         WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_all(db)
    .await
    .unwrap()
    .into_iter()
    .map(
        |(feed, seq, read, kept, hidden, starred): (i32, i32, bool, bool, bool, bool)| {
            ((feed, seq), (read, kept, hidden, starred))
        },
    )
    .collect();
    let rows: Vec<(i32, i32, i64)> =
        sqlx::query_as("SELECT feed_id, seq, id FROM articles ORDER BY id DESC")
            .fetch_all(db)
            .await
            .unwrap();

    let wanted: HashSet<i32> = feed_ids.iter().copied().collect();
    rows.into_iter()
        .filter(|(feed, _, id)| {
            wanted.contains(feed) && page.before.is_none_or(|before| *id < before)
        })
        .filter_map(|(feed, seq, _)| {
            let mark = *marks.get(&feed)?;
            let (read, kept, hidden, starred) =
                states.get(&(feed, seq)).copied().unwrap_or_default();
            let is_read = if seq <= mark { !kept } else { read };
            (!hidden && !(page.unread_only && is_read)).then_some((feed, seq, is_read, starred))
        })
        .take(page.limit as usize)
        .collect()
}

/// Many feeds of very different sizes, readers in every state, and every page
/// compared with the obvious way of working it out.
#[sqlx::test]
async fn pages_match_the_obvious_way_of_working_them_out(db: PgPool) {
    let mut random = Random::new(7);
    let mut feeds = Vec::new();
    for n in 0..9 {
        feeds.push(feed(&db, &format!("feed-{n}")).await);
    }
    // Arrivals mixed across feeds; the first feeds get most, the last one nothing.
    for _ in 0..120 {
        let feed_id = feeds[random.below(8).min(random.below(8))];
        add(&db, feed_id, 1 + random.below(4)).await;
    }

    for name in ["ann", "ben", "cy"] {
        let account_id = account(&db, name).await;
        let mine: Vec<i32> = feeds
            .iter()
            .copied()
            .filter(|_| random.below(4) > 0)
            .collect();
        for &feed_id in &mine {
            subscribe(&db, account_id, feed_id).await;
            let last: i32 = sqlx::query_scalar("SELECT last_seq FROM feeds WHERE id = $1")
                .bind(feed_id)
                .fetch_one(&db)
                .await
                .unwrap();
            if last == 0 {
                continue;
            }
            let seq = |random: &mut Random| 1 + random.below(last as usize) as i32;
            if random.below(3) > 0 {
                let upto = seq(&mut random);
                reading::mark_all_read(&db, account_id, feed_id, upto)
                    .await
                    .unwrap();
            }
            for _ in 0..random.below(12) {
                let (seq, on) = (seq(&mut random), random.below(4) > 0);
                match random.below(4) {
                    0 | 1 => reading::set_read(&db, account_id, feed_id, seq, on)
                        .await
                        .unwrap(),
                    2 => reading::set_hidden(&db, account_id, feed_id, seq, on)
                        .await
                        .unwrap(),
                    _ => reading::set_starred(&db, account_id, feed_id, seq, on)
                        .await
                        .unwrap(),
                };
            }
        }

        let total: i64 = sqlx::query_scalar("SELECT max(id) FROM articles")
            .fetch_one(&db)
            .await
            .unwrap();
        for round in 0..60 {
            // One feed, a few of them, or all, also ones the account does not have.
            let scope: Vec<i32> = match round % 3 {
                0 => feeds.clone(),
                1 => vec![feeds[random.below(feeds.len())]],
                _ => feeds
                    .iter()
                    .copied()
                    .filter(|_| random.below(2) == 0)
                    .collect(),
            };
            let page = Page {
                before: (random.below(3) > 0).then(|| 1 + random.below(total as usize + 2) as i64),
                limit: [1, 2, 5, 20, 50, 500][random.below(6)],
                unread_only: random.below(2) == 0,
            };
            let got: Vec<_> = articles::page(&db, account_id, &scope, page)
                .await
                .unwrap()
                .into_iter()
                .map(|item| (item.feed_id, item.seq, item.read, item.starred))
                .collect();
            let expected = expected_page(&db, account_id, &scope, page).await;
            assert_eq!(got, expected, "{name}, feeds {scope:?}, {page:?}");
        }
    }
}
