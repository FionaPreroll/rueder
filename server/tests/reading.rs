mod common;

use std::collections::BTreeSet;

use common::{Random, START, account, add, reader};
use rueder::{
    articles::{self, Page},
    reading::{self, FeedUnread},
};
use sqlx::PgPool;
use time::Duration;

async fn unread(db: &PgPool, account_id: i32) -> FeedUnread {
    let counts = reading::unread_counts(db, account_id).await.unwrap();
    assert_eq!(counts.len(), 1);
    counts[0]
}

/// The exceptions from the mark, as (number, read, kept unread, hidden, starred).
async fn states(db: &PgPool, account_id: i32) -> Vec<(i32, bool, bool, bool, bool)> {
    sqlx::query_as(
        "SELECT seq, read, kept_unread, hidden, starred FROM article_states
         WHERE account_id = $1 ORDER BY seq",
    )
    .bind(account_id)
    .fetch_all(db)
    .await
    .unwrap()
}

#[sqlx::test]
async fn a_new_subscription_has_everything_unread(db: PgPool) {
    let (account_id, feed_id) = reader(&db, "a", 7).await;
    let expected = FeedUnread {
        feed_id,
        last_seq: 7,
        read_upto: 0,
        unread: 7,
    };
    assert_eq!(unread(&db, account_id).await, expected);

    // New articles count as they arrive.
    add(&db, feed_id, 3).await;
    assert_eq!(unread(&db, account_id).await.unread, 10);
    // Somebody without subscriptions has no counts.
    let other = account(&db, "other").await;
    assert!(reading::unread_counts(&db, other).await.unwrap().is_empty());
}

#[sqlx::test]
async fn the_mark_moves_over_what_is_read_right_above_it(db: PgPool) {
    let (account_id, feed_id) = reader(&db, "a", 10).await;
    for seq in [3, 2, 5] {
        assert!(
            reading::set_read(&db, account_id, feed_id, seq, true)
                .await
                .unwrap()
        );
    }
    let now = unread(&db, account_id).await;
    assert_eq!((now.read_upto, now.unread), (0, 7));
    assert_eq!(states(&db, account_id).await.len(), 3);

    // Reading 1 closes the gap: the mark passes 1, 2 and 3 and stops before 4.
    reading::set_read(&db, account_id, feed_id, 1, true)
        .await
        .unwrap();
    let now = unread(&db, account_id).await;
    assert_eq!((now.read_upto, now.unread), (3, 6));
    assert_eq!(
        states(&db, account_id).await,
        [(5, true, false, false, false)]
    );

    // Reading it again changes nothing.
    reading::set_read(&db, account_id, feed_id, 1, true)
        .await
        .unwrap();
    assert_eq!(unread(&db, account_id).await.unread, 6);
}

#[sqlx::test]
async fn an_article_under_the_mark_can_be_kept_unread(db: PgPool) {
    let (account_id, feed_id) = reader(&db, "a", 10).await;
    reading::mark_all_read(&db, account_id, feed_id, 6)
        .await
        .unwrap();
    assert_eq!(unread(&db, account_id).await.unread, 4);

    reading::set_read(&db, account_id, feed_id, 2, false)
        .await
        .unwrap();
    assert_eq!(unread(&db, account_id).await.unread, 5);
    assert_eq!(
        states(&db, account_id).await,
        [(2, false, true, false, false)]
    );
    // Above the mark, setting an unread article back to unread changes nothing.
    reading::set_read(&db, account_id, feed_id, 8, false)
        .await
        .unwrap();
    assert_eq!(states(&db, account_id).await.len(), 1);

    reading::set_read(&db, account_id, feed_id, 2, true)
        .await
        .unwrap();
    assert_eq!(unread(&db, account_id).await.unread, 4);
    assert!(states(&db, account_id).await.is_empty());
}

#[sqlx::test]
async fn marking_all_keeps_what_is_newer_and_never_moves_back(db: PgPool) {
    let (account_id, feed_id) = reader(&db, "a", 10).await;
    reading::set_read(&db, account_id, feed_id, 4, true)
        .await
        .unwrap();
    reading::set_read(&db, account_id, feed_id, 8, true)
        .await
        .unwrap();

    // Up to 6, as the list showed it; 7, 9 and 10 arrived later and stay unread.
    assert!(
        reading::mark_all_read(&db, account_id, feed_id, 6)
            .await
            .unwrap()
    );
    let now = unread(&db, account_id).await;
    assert_eq!((now.read_upto, now.unread), (6, 3));
    assert_eq!(
        states(&db, account_id).await,
        [(8, true, false, false, false)]
    );

    // Marking up to an older article reads what was kept unread up to there.
    reading::set_read(&db, account_id, feed_id, 2, false)
        .await
        .unwrap();
    reading::set_read(&db, account_id, feed_id, 5, false)
        .await
        .unwrap();
    reading::mark_all_read(&db, account_id, feed_id, 3)
        .await
        .unwrap();
    let now = unread(&db, account_id).await;
    assert_eq!((now.read_upto, now.unread), (6, 4));
    assert_eq!(states(&db, account_id).await.len(), 2);

    // Beyond the newest article there is nothing to mark; 7 read, the mark runs on over 8.
    reading::mark_all_read(&db, account_id, feed_id, 7)
        .await
        .unwrap();
    assert_eq!(unread(&db, account_id).await.read_upto, 8);
    reading::mark_all_read(&db, account_id, feed_id, 99)
        .await
        .unwrap();
    let now = unread(&db, account_id).await;
    assert_eq!((now.read_upto, now.unread), (10, 0));
    assert!(states(&db, account_id).await.is_empty());
}

#[sqlx::test]
async fn hidden_articles_never_count(db: PgPool) {
    let (account_id, feed_id) = reader(&db, "a", 6).await;
    assert!(
        reading::set_hidden(&db, account_id, feed_id, 4, true)
            .await
            .unwrap()
    );
    reading::set_read(&db, account_id, feed_id, 5, true)
        .await
        .unwrap();
    reading::set_hidden(&db, account_id, feed_id, 5, true)
        .await
        .unwrap();
    assert_eq!(unread(&db, account_id).await.unread, 4);

    // The mark passes hidden articles too, so they cannot hold it back for good.
    reading::mark_all_read(&db, account_id, feed_id, 3)
        .await
        .unwrap();
    let now = unread(&db, account_id).await;
    assert_eq!((now.read_upto, now.unread), (5, 1));

    // 4 was hidden unread, 5 hidden read: shown again, 4 is unread and 5 is read.
    reading::set_hidden(&db, account_id, feed_id, 4, false)
        .await
        .unwrap();
    reading::set_hidden(&db, account_id, feed_id, 5, false)
        .await
        .unwrap();
    assert_eq!(unread(&db, account_id).await.unread, 2);
    assert_eq!(
        states(&db, account_id).await,
        [(4, false, true, false, false)]
    );
}

#[sqlx::test]
async fn stars_are_apart_from_reading(db: PgPool) {
    let (account_id, feed_id) = reader(&db, "a", 3).await;
    assert!(
        reading::set_starred(&db, account_id, feed_id, 2, true)
            .await
            .unwrap()
    );
    assert_eq!(unread(&db, account_id).await.unread, 3);

    reading::mark_all_read(&db, account_id, feed_id, 3)
        .await
        .unwrap();
    assert_eq!(
        states(&db, account_id).await,
        [(2, false, false, false, true)]
    );
    let page = Page {
        before: None,
        after: None,
        limit: 10,
        unread_only: false,
    };
    let items = articles::page(&db, account_id, &[feed_id], page)
        .await
        .unwrap();
    assert_eq!(
        items.iter().map(|item| item.starred).collect::<Vec<_>>(),
        [false, true, false]
    );

    reading::set_starred(&db, account_id, feed_id, 2, false)
        .await
        .unwrap();
    assert!(states(&db, account_id).await.is_empty());
}

#[sqlx::test]
async fn only_ones_own_articles_can_be_marked(db: PgPool) {
    let (account_id, feed_id) = reader(&db, "a", 3).await;
    let stranger = account(&db, "stranger").await;

    assert!(
        !reading::set_read(&db, stranger, feed_id, 1, true)
            .await
            .unwrap()
    );
    assert!(
        !reading::mark_all_read(&db, stranger, feed_id, 3)
            .await
            .unwrap()
    );
    assert!(
        !reading::mark_older_read(&db, stranger, feed_id, 3, time::OffsetDateTime::now_utc())
            .await
            .unwrap()
    );
    assert!(
        !reading::set_starred(&db, stranger, feed_id, 1, true)
            .await
            .unwrap()
    );
    assert!(
        !reading::set_hidden(&db, stranger, feed_id, 1, true)
            .await
            .unwrap()
    );
    // Numbers the feed has not given out.
    for seq in [0, 4, -1] {
        assert!(
            !reading::set_read(&db, account_id, feed_id, seq, true)
                .await
                .unwrap()
        );
        assert!(
            !reading::set_starred(&db, account_id, feed_id, seq, true)
                .await
                .unwrap()
        );
        assert!(
            !reading::set_hidden(&db, account_id, feed_id, seq, true)
                .await
                .unwrap()
        );
    }
    assert_eq!(unread(&db, account_id).await.unread, 3);
    assert!(states(&db, account_id).await.is_empty());
}

#[sqlx::test]
async fn the_mark_runs_over_long_stretches(db: PgPool) {
    let (account_id, feed_id) = reader(&db, "a", 0).await;
    sqlx::query("UPDATE feeds SET last_seq = 3000 WHERE id = $1")
        .bind(feed_id)
        .execute(&db)
        .await
        .unwrap();
    // Everything from 2 to 2500 read one by one, as after years without "mark all".
    sqlx::query(
        "INSERT INTO article_states (account_id, feed_id, seq, read, starred)
         SELECT $1, $2, n, true, n % 500 = 0 FROM generate_series(2, 2500) n",
    )
    .bind(account_id)
    .bind(feed_id)
    .execute(&db)
    .await
    .unwrap();
    assert_eq!(unread(&db, account_id).await.unread, 501);

    reading::set_read(&db, account_id, feed_id, 1, true)
        .await
        .unwrap();
    let now = unread(&db, account_id).await;
    assert_eq!((now.read_upto, now.unread), (2500, 500));
    // Only the stars are left.
    assert_eq!(states(&db, account_id).await.len(), 5);
}

/// The obvious model: the set of read articles, the set of hidden ones.
#[derive(Default)]
struct Model {
    last: i32,
    /// When each article was published, in minutes after the start, by number.
    published: Vec<i64>,
    read: BTreeSet<i32>,
    hidden: BTreeSet<i32>,
    starred: BTreeSet<i32>,
}

impl Model {
    fn unread(&self) -> Vec<i32> {
        (1..=self.last)
            .rev()
            .filter(|seq| !self.read.contains(seq) && !self.hidden.contains(seq))
            .collect()
    }
}

/// Random reading, compared after every step with the obvious model: the count,
/// the lists, and that the stored state stays as small as it can be.
#[sqlx::test]
async fn reading_matches_the_obvious_model(db: PgPool) {
    for seed in 1..=25 {
        let mut random = Random::new(seed);
        let (account_id, feed_id) = reader(&db, &format!("reader-{seed}"), 0).await;
        let mut model = Model::default();
        let mut log = Vec::new();

        for _ in 0..40 {
            let seq = 1 + random.below(model.last as usize + 2) as i32;
            let exists = seq <= model.last;
            let step = match random.below(14) {
                0 | 1 => {
                    let count = 1 + random.below(6);
                    add(&db, feed_id, count).await;
                    // Most are published in the order they arrive, some earlier
                    // or later than their neighbours.
                    for seq in model.last + 1..=model.last + count as i32 {
                        let mut minute = i64::from(seq);
                        if random.below(3) == 0 {
                            minute = random.below(model.last as usize + 8) as i64;
                            sqlx::query(
                                "UPDATE articles SET published_at = $3
                                 WHERE feed_id = $1 AND seq = $2",
                            )
                            .bind(feed_id)
                            .bind(seq)
                            .bind(START + Duration::minutes(minute))
                            .execute(&db)
                            .await
                            .unwrap();
                        }
                        model.published.push(minute);
                    }
                    model.last += count as i32;
                    format!("{count} arrive")
                }
                12 | 13 => {
                    let minute = random.below(model.last as usize + 8) as i64;
                    // Published in that minute or before; an article is never
                    // published between two minutes.
                    let cutoff = START + Duration::minutes(minute) + Duration::seconds(30);
                    reading::mark_older_read(&db, account_id, feed_id, seq, cutoff)
                        .await
                        .unwrap();
                    let old = (1..=seq.min(model.last))
                        .filter(|seq| model.published[*seq as usize - 1] <= minute);
                    model.read.extend(old);
                    format!("older than minute {minute} read up to {seq}")
                }
                2..=4 => {
                    let done = reading::set_read(&db, account_id, feed_id, seq, true)
                        .await
                        .unwrap();
                    assert_eq!(done, exists);
                    if exists {
                        model.read.insert(seq);
                    }
                    format!("read {seq}")
                }
                5 => {
                    reading::set_read(&db, account_id, feed_id, seq, false)
                        .await
                        .unwrap();
                    model.read.remove(&seq);
                    format!("unread {seq}")
                }
                6 | 7 => {
                    reading::mark_all_read(&db, account_id, feed_id, seq)
                        .await
                        .unwrap();
                    model.read.extend(1..=seq.min(model.last));
                    format!("all read up to {seq}")
                }
                8 | 9 => {
                    let on = random.below(3) > 0;
                    reading::set_hidden(&db, account_id, feed_id, seq, on)
                        .await
                        .unwrap();
                    if on && exists {
                        model.hidden.insert(seq);
                    } else if !on {
                        model.hidden.remove(&seq);
                    }
                    format!("hidden {seq} {on}")
                }
                _ => {
                    let on = random.below(3) > 0;
                    reading::set_starred(&db, account_id, feed_id, seq, on)
                        .await
                        .unwrap();
                    if on && exists {
                        model.starred.insert(seq);
                    } else if !on {
                        model.starred.remove(&seq);
                    }
                    format!("starred {seq} {on}")
                }
            };
            log.push(step);
            let context = format!("seed {seed}: {}", log.join(", "));

            let counts = reading::unread_counts(&db, account_id).await.unwrap();
            let now = counts[0];
            assert_eq!(now.last_seq, model.last, "{context}");
            assert_eq!(now.unread as usize, model.unread().len(), "{context}");

            let page = |unread_only| Page {
                before: None,
                after: None,
                limit: 1000,
                unread_only,
            };
            let all = articles::page(&db, account_id, &[feed_id], page(false))
                .await
                .unwrap();
            let shown: Vec<_> = all
                .iter()
                .map(|item| (item.seq, item.read, item.starred))
                .collect();
            let expected: Vec<_> = (1..=model.last)
                .rev()
                .filter(|seq| !model.hidden.contains(seq))
                .map(|seq| (seq, model.read.contains(&seq), model.starred.contains(&seq)))
                .collect();
            assert_eq!(shown, expected, "{context}");
            let unread_only = articles::page(&db, account_id, &[feed_id], page(true))
                .await
                .unwrap();
            let shown: Vec<_> = unread_only.iter().map(|item| item.seq).collect();
            assert_eq!(shown, model.unread(), "{context}");

            // "Read" is only stored above the mark, "kept unread" only at or under
            // it, and the mark has passed everything it can pass.
            let mark = now.read_upto;
            assert!(mark <= model.last, "{context}");
            for (seq, read, kept_unread, hidden, _) in states(&db, account_id).await {
                assert!(!read || seq > mark, "{context}");
                assert!(!kept_unread || seq <= mark, "{context}");
                assert!(seq != mark + 1 || !(read || hidden), "{context}");
            }
        }
    }
}

/// What apps send, many articles at once, ends where one after the other
/// ends: two accounts read the same feed, one in batches, one an article at a
/// time, and their states are the same after every step.
#[sqlx::test]
async fn many_at_once_is_one_after_the_other(db: PgPool) {
    use reading::Many;
    for seed in 1..=20 {
        let mut random = Random::new(seed);
        let (batched, feed_id) = reader(&db, &format!("batched-{seed}"), 30).await;
        let single = account(&db, &format!("single-{seed}")).await;
        common::subscribe(&db, single, feed_id).await;
        for _ in 0..25 {
            let last: i32 = sqlx::query_scalar("SELECT last_seq FROM feeds WHERE id = $1")
                .bind(feed_id)
                .fetch_one(&db)
                .await
                .unwrap();
            // Numbers the feed has, some twice, and some it has not.
            let seqs: Vec<i32> = (0..random.below(12))
                .map(|_| 1 + random.below(last as usize + 3) as i32)
                .collect();
            let change = match random.below(5) {
                0 | 1 => Many::Read(true),
                2 => Many::Read(false),
                3 => Many::Starred(true),
                _ => Many::Starred(false),
            };
            if random.below(6) == 0 {
                let seq = 1 + random.below(last as usize) as i32;
                for account in [batched, single] {
                    reading::set_hidden(&db, account, feed_id, seq, true)
                        .await
                        .unwrap();
                }
            }
            if random.below(8) == 0 {
                add(&db, feed_id, 1 + random.below(3)).await;
            }
            let mut tx = db.begin().await.unwrap();
            assert!(
                reading::set_many_on(&mut tx, batched, feed_id, &seqs, change)
                    .await
                    .unwrap()
            );
            tx.commit().await.unwrap();
            for seq in &seqs {
                match change {
                    Many::Read(read) => {
                        reading::set_read(&db, single, feed_id, *seq, read)
                            .await
                            .unwrap();
                    }
                    Many::Starred(on) => {
                        reading::set_starred(&db, single, feed_id, *seq, on)
                            .await
                            .unwrap();
                    }
                }
            }
            let (a, b) = (unread(&db, batched).await, unread(&db, single).await);
            assert_eq!(
                (a.read_upto, a.unread),
                (b.read_upto, b.unread),
                "seed {seed}, {change:?} {seqs:?}"
            );
            assert_eq!(
                states(&db, batched).await,
                states(&db, single).await,
                "seed {seed}, {change:?} {seqs:?}"
            );
        }
        // Not the account's feed: nothing.
        let stranger = account(&db, &format!("stranger-{seed}")).await;
        let mut tx = db.begin().await.unwrap();
        assert!(
            !reading::set_many_on(&mut tx, stranger, feed_id, &[1], Many::Read(true))
                .await
                .unwrap()
        );
    }
}
