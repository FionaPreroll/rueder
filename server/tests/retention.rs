//! The retention: what goes, what stays, and that lists, unread counts and the
//! search agree with a simple model afterwards, with random input.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{Random, START};
use rueder::{
    articles::{self, NewArticle, Page},
    feeds, reading,
    retention::{self, Articles, Remove},
    search::{self, Outcome},
};
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};

/// Article `n` of a feed, which arrived `day` days after [`START`].
fn arriving(n: usize, day: i64) -> NewArticle {
    let mut article = common::article(n);
    article.fetched_at = START + Duration::days(day);
    article.published_at = article.fetched_at - Duration::hours(1);
    article
}

/// Adds articles that arrived on these days, oldest first, and returns them.
async fn add_arriving(db: &PgPool, feed_id: i32, days: &[i64]) -> Vec<NewArticle> {
    let have: i32 = sqlx::query_scalar("SELECT last_seq FROM feeds WHERE id = $1")
        .bind(feed_id)
        .fetch_one(db)
        .await
        .unwrap();
    let new: Vec<_> = days
        .iter()
        .enumerate()
        .map(|(i, day)| arriving(have as usize + i + 1, *day))
        .collect();
    let mut tx = db.begin().await.unwrap();
    articles::insert(&mut tx, feed_id, &new).await.unwrap();
    tx.commit().await.unwrap();
    new
}

/// A year after the day that splits old from young, with articles kept twelve
/// months: everything that arrived before `day` is old.
fn a_year_after(day: i64) -> OffsetDateTime {
    let split = START + Duration::days(day);
    split.replace_year(split.year() + 1).unwrap()
}

async fn keep(db: &PgPool, settings: Articles) {
    retention::set_articles(db, &settings).await.unwrap();
}

fn articles_after_a_year(what: Remove) -> Articles {
    Articles {
        articles_months: Some(12),
        articles_what: Some(what),
        ..Articles::default()
    }
}

/// What one account keeps of one feed, as sets.
#[derive(Debug, Default, Clone)]
struct Mine {
    read: BTreeSet<i32>,
    hidden: BTreeSet<i32>,
    starred: BTreeSet<i32>,
}

/// The model: the articles there are, and what each reader keeps of them.
#[derive(Debug, Default)]
struct Model {
    /// Of each feed its articles, by number, with the day they arrived.
    feeds: BTreeMap<i32, BTreeMap<i32, i64>>,
    subs: BTreeMap<(i32, i32), Mine>,
    labelled: BTreeSet<(i32, i32)>,
}

impl Model {
    fn kept(&self, feed_id: i32, seq: i32) -> bool {
        self.labelled.contains(&(feed_id, seq))
            || self
                .subs
                .iter()
                .any(|((_, feed), mine)| *feed == feed_id && mine.starred.contains(&seq))
    }

    /// The articles a reader sees in a list: there, and not hidden.
    fn listed(&self, account_id: i32, feed_id: i32, unread_only: bool) -> Vec<i32> {
        let mine = &self.subs[&(account_id, feed_id)];
        self.feeds[&feed_id]
            .keys()
            .rev()
            .copied()
            .filter(|seq| !mine.hidden.contains(seq))
            .filter(|seq| !unread_only || !mine.read.contains(seq))
            .collect()
    }

    fn unread(&self, account_id: i32, feed_id: i32) -> i64 {
        self.listed(account_id, feed_id, true).len() as i64
    }

    /// What the retention leaves at the split day, for whole articles.
    fn cut(&mut self, split: i64) {
        let gone: Vec<(i32, i32)> = self
            .feeds
            .iter()
            .flat_map(|(feed_id, articles)| {
                articles
                    .iter()
                    .take_while(|(_, day)| **day < split)
                    .map(|(seq, _)| (*feed_id, *seq))
                    .collect::<Vec<_>>()
            })
            .filter(|(feed_id, seq)| !self.kept(*feed_id, *seq))
            .collect();
        for (feed_id, seq) in gone {
            self.feeds.get_mut(&feed_id).unwrap().remove(&seq);
            for ((_, feed), mine) in self.subs.iter_mut() {
                if *feed == feed_id {
                    mine.read.remove(&seq);
                    mine.hidden.remove(&seq);
                    mine.starred.remove(&seq);
                }
            }
        }
    }
}

/// Feeds, readers and what they did, at random, in the store and in the model.
async fn made_up(db: &PgPool, random: &mut Random) -> (Model, BTreeMap<i32, Vec<NewArticle>>) {
    let mut model = Model::default();
    let mut sent = BTreeMap::new();
    let mut feeds = Vec::new();
    for f in 0..3 {
        let feed_id = common::feed(db, &format!("feed{f}")).await;
        // Arriving day by day, a few on the same day.
        let mut day = 0;
        let days: Vec<i64> = (0..20 + random.below(30))
            .map(|_| {
                day += random.below(3) as i64;
                day
            })
            .collect();
        let added = add_arriving(db, feed_id, &days).await;
        model.feeds.insert(
            feed_id,
            days.iter()
                .enumerate()
                .map(|(i, day)| (i as i32 + 1, *day))
                .collect(),
        );
        sent.insert(feed_id, added);
        feeds.push(feed_id);
    }
    let mut accounts = Vec::new();
    for a in 0..3 {
        let account_id = common::account(db, &format!("reader{a}")).await;
        let label: i32 = sqlx::query_scalar(
            "INSERT INTO labels (account_id, title, color, position)
             VALUES ($1, 'keep', '#336699', 1) RETURNING id",
        )
        .bind(account_id)
        .fetch_one(db)
        .await
        .unwrap();
        for feed_id in &feeds {
            if a == 0 || random.below(3) > 0 {
                sqlx::query(
                    "INSERT INTO subscriptions (account_id, feed_id, position) VALUES ($1, $2, $2)",
                )
                .bind(account_id)
                .bind(feed_id)
                .execute(db)
                .await
                .unwrap();
                model.subs.insert((account_id, *feed_id), Mine::default());
            }
        }
        accounts.push((account_id, label));
    }

    let subs: Vec<(i32, i32)> = model.subs.keys().copied().collect();
    for _ in 0..120 {
        let (account_id, feed_id) = subs[random.below(subs.len())];
        let last = model.feeds[&feed_id].len() as i32;
        let seq = 1 + random.below(last as usize) as i32;
        let mine = model.subs.get_mut(&(account_id, feed_id)).unwrap();
        match random.below(10) {
            0..=3 => {
                let read = random.below(4) > 0;
                assert!(
                    reading::set_read(db, account_id, feed_id, seq, read)
                        .await
                        .unwrap()
                );
                if read {
                    mine.read.insert(seq);
                } else {
                    mine.read.remove(&seq);
                }
            }
            4 => {
                assert!(
                    reading::mark_all_read(db, account_id, feed_id, seq)
                        .await
                        .unwrap()
                );
                mine.read.extend(1..=seq);
            }
            5 | 6 => {
                let on = random.below(3) > 0;
                assert!(
                    reading::set_starred(db, account_id, feed_id, seq, on)
                        .await
                        .unwrap()
                );
                if on {
                    mine.starred.insert(seq);
                } else {
                    mine.starred.remove(&seq);
                }
            }
            7 => {
                let on = random.below(3) > 0;
                assert!(
                    reading::set_hidden(db, account_id, feed_id, seq, on)
                        .await
                        .unwrap()
                );
                if on {
                    mine.hidden.insert(seq);
                } else {
                    mine.hidden.remove(&seq);
                }
            }
            _ => {
                let (_, label) = accounts.iter().find(|(id, _)| *id == account_id).unwrap();
                sqlx::query(
                    "INSERT INTO article_labels (label_id, feed_id, seq) VALUES ($1, $2, $3)
                     ON CONFLICT DO NOTHING",
                )
                .bind(label)
                .bind(feed_id)
                .bind(seq)
                .execute(db)
                .await
                .unwrap();
                model.labelled.insert((feed_id, seq));
            }
        }
    }
    (model, sent)
}

/// The store agrees with the model: unread counts, both lists with their
/// read and starred marks, and the search.
async fn agrees(db: &PgPool, model: &Model, seed: u64) {
    for ((reader, feed_id), mine) in &model.subs {
        let counts = reading::unread_counts(db, *reader).await.unwrap();
        let count = counts.iter().find(|c| c.feed_id == *feed_id).unwrap();
        assert_eq!(
            count.unread,
            model.unread(*reader, *feed_id),
            "seed {seed}: unread of feed {feed_id} for {reader}"
        );
        for unread_only in [false, true] {
            let page = Page {
                before: None,
                after: None,
                limit: 1000,
                unread_only,
            };
            let items = articles::page(db, *reader, &[*feed_id], page)
                .await
                .unwrap();
            let seqs: Vec<i32> = items.iter().map(|item| item.seq).collect();
            assert_eq!(
                seqs,
                model.listed(*reader, *feed_id, unread_only),
                "seed {seed}: list of feed {feed_id} for {reader}, unread only {unread_only}"
            );
            for item in &items {
                assert_eq!(
                    item.read,
                    mine.read.contains(&item.seq),
                    "seed {seed}: read"
                );
                assert_eq!(
                    item.starred,
                    mine.starred.contains(&item.seq),
                    "seed {seed}: star"
                );
            }
        }
        // Every title has the word, so the search finds what the list shows.
        let mut found = Vec::new();
        let mut before = None;
        loop {
            let page = search::Page {
                before,
                limit: 100,
                unread_only: false,
            };
            let Outcome::Found(hits) = search::search(db, *reader, &[*feed_id], "article", page)
                .await
                .unwrap()
            else {
                panic!("seed {seed}: the search did not search");
            };
            found.extend(hits.articles.iter().map(|item| item.seq));
            match hits.next {
                Some(next) => before = Some(next),
                None => break,
            }
        }
        assert_eq!(
            found,
            model.listed(*reader, *feed_id, false),
            "seed {seed}: search in feed {feed_id} for {reader}"
        );
    }
}

#[sqlx::test]
async fn whole_articles_go_and_lists_counts_and_search_agree(db: PgPool) {
    for seed in 1..=8u64 {
        // A database of its own for each seed would be slow; clear it instead.
        sqlx::query("TRUNCATE accounts, feeds, article_tombstones CASCADE")
            .execute(&db)
            .await
            .unwrap();
        let mut random = Random::new(seed);
        let (mut model, sent) = made_up(&db, &mut random).await;
        agrees(&db, &model, seed).await;

        keep(&db, articles_after_a_year(Remove::Article)).await;
        let split = 5 + random.below(40) as i64;
        let swept = retention::sweep(&db, None, a_year_after(split), None)
            .await
            .unwrap();
        let before: usize = model.feeds.values().map(BTreeMap::len).sum();
        model.cut(split);
        let after: usize = model.feeds.values().map(BTreeMap::len).sum();
        assert_eq!(
            swept.articles as usize,
            before - after,
            "seed {seed}: how many went"
        );
        agrees(&db, &model, seed).await;

        // Marked articles stay, whatever their age.
        for (feed_id, seq) in &model.labelled {
            assert!(
                model.feeds[feed_id].contains_key(seq),
                "seed {seed}: label kept"
            );
        }

        // The feed lists everything again, the gone ones too, and two new ones:
        // only these come in, with the next numbers, and nothing comes back.
        for (feed_id, articles) in &sent {
            let last: i32 = sqlx::query_scalar("SELECT last_seq FROM feeds WHERE id = $1")
                .bind(feed_id)
                .fetch_one(&db)
                .await
                .unwrap();
            let mut listed = articles.clone();
            listed.push(arriving(1000, 400));
            listed.push(arriving(1001, 400));
            let mut tx = db.begin().await.unwrap();
            let stored = feeds::store(&mut tx, *feed_id, &listed).await.unwrap();
            tx.commit().await.unwrap();
            assert_eq!(stored.new, 2, "seed {seed}: only the new ones are new");
            assert_eq!(
                stored.first_new,
                Some(last + 1),
                "seed {seed}: numbers go on"
            );
            let model_feed = model.feeds.get_mut(feed_id).unwrap();
            model_feed.insert(last + 1, 400);
            model_feed.insert(last + 2, 400);
        }
        let twice: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM (SELECT feed_id, guid_hash FROM article_content
                                   GROUP BY feed_id, guid_hash HAVING count(*) > 1) d",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(twice, 0, "seed {seed}: no article twice");
        agrees(&db, &model, seed).await;

        // A new reader starts at the cut: nothing that went counts for them.
        let newcomer = common::account(&db, "newcomer").await;
        for feed_id in model.feeds.keys() {
            common::subscribe(&db, newcomer, *feed_id).await;
            let cut: i32 = sqlx::query_scalar("SELECT cut_seq FROM feeds WHERE id = $1")
                .bind(feed_id)
                .fetch_one(&db)
                .await
                .unwrap();
            let counts = reading::unread_counts(&db, newcomer).await.unwrap();
            let count = counts.iter().find(|c| c.feed_id == *feed_id).unwrap();
            let above = model.feeds[feed_id]
                .keys()
                .filter(|seq| **seq > cut)
                .count();
            assert_eq!(
                count.unread, above as i64,
                "seed {seed}: the newcomer's count"
            );
        }
        keep(&db, Articles::default()).await;
    }
}

#[sqlx::test]
async fn only_texts_go_and_the_lists_stay(db: PgPool) {
    for seed in 1..=6u64 {
        sqlx::query("TRUNCATE accounts, feeds, article_tombstones CASCADE")
            .execute(&db)
            .await
            .unwrap();
        let mut random = Random::new(seed);
        let (model, _) = made_up(&db, &mut random).await;
        keep(&db, articles_after_a_year(Remove::Text)).await;
        let split = 5 + random.below(40) as i64;
        let swept = retention::sweep(&db, None, a_year_after(split), None)
            .await
            .unwrap();
        assert_eq!(swept.articles, 0, "seed {seed}: no article went whole");
        // Nothing changed in lists, counts or the search by title.
        agrees(&db, &model, seed).await;

        let mut dropped = 0;
        for (feed_id, articles) in &model.feeds {
            for (seq, day) in articles {
                let (html, gone, versions): (String, bool, i64) = sqlx::query_as(
                    "SELECT c.html, c.text_dropped,
                            (SELECT count(*) FROM article_versions v
                             WHERE v.feed_id = c.feed_id AND v.seq = c.seq)
                     FROM article_content c WHERE c.feed_id = $1 AND c.seq = $2",
                )
                .bind(feed_id)
                .bind(seq)
                .fetch_one(&db)
                .await
                .unwrap();
                let goes = *day < split && !model.kept(*feed_id, *seq);
                assert_eq!(gone, goes, "seed {seed}: text of {feed_id}/{seq}");
                assert_eq!(
                    html.is_empty(),
                    goes,
                    "seed {seed}: html of {feed_id}/{seq}"
                );
                assert_eq!(
                    versions == 0,
                    goes,
                    "seed {seed}: versions of {feed_id}/{seq}"
                );
                dropped += u64::from(goes);
            }
        }
        assert_eq!(swept.texts, dropped, "seed {seed}: how many texts went");

        // What was only in the text is no longer found.
        for ((reader, feed_id), mine) in &model.subs {
            let page = search::Page {
                before: None,
                limit: 1000,
                unread_only: false,
            };
            let Outcome::Found(hits) = search::search(&db, *reader, &[*feed_id], "text", page)
                .await
                .unwrap()
            else {
                panic!("seed {seed}: the search did not search");
            };
            let found: Vec<i32> = hits.articles.iter().map(|item| item.seq).collect();
            let expected: Vec<i32> = model.feeds[feed_id]
                .iter()
                .rev()
                .filter(|(seq, day)| **day >= split || model.kept(*feed_id, **seq))
                .map(|(seq, _)| *seq)
                .filter(|seq| !mine.hidden.contains(seq))
                .collect();
            assert_eq!(found, expected, "seed {seed}: search in the text");
        }
        keep(&db, Articles::default()).await;
    }
}

#[sqlx::test]
async fn as_rueder_comes_nothing_goes(db: PgPool) {
    let feed_id = common::feed(&db, "old").await;
    add_arriving(&db, feed_id, &[0, 1, 2]).await;
    common::subscribe(&db, common::account(&db, "ada").await, feed_id).await;
    sqlx::query("UPDATE feeds SET orphaned_at = '2000-01-01' WHERE id = $1")
        .bind(feed_id)
        .execute(&db)
        .await
        .unwrap();
    let far = START + Duration::days(36500);
    let swept = retention::sweep(&db, None, far, None).await.unwrap();
    assert!(swept.is_empty());
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM articles")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(left, 3);
}

#[sqlx::test]
async fn settings_are_stored_and_checked(db: PgPool) {
    assert_eq!(retention::articles(&db).await.unwrap(), Articles::default());
    let set = Articles {
        articles_months: Some(24),
        articles_what: Some(Remove::Text),
        versions_months: Some(12),
        orphans_days: Some(90),
    };
    keep(&db, set).await;
    assert_eq!(retention::articles(&db).await.unwrap(), set);
    // Without saying what goes, nothing is stored.
    let unsaid = Articles {
        articles_what: None,
        ..set
    };
    assert!(retention::set_articles(&db, &unsaid).await.is_err());
    assert_eq!(retention::articles(&db).await.unwrap(), set);
}

#[sqlx::test]
async fn a_mark_on_an_article_that_went_is_refused(db: PgPool) {
    let (account_id, feed_id) = (
        common::account(&db, "ada").await,
        common::feed(&db, "f").await,
    );
    common::subscribe(&db, account_id, feed_id).await;
    add_arriving(&db, feed_id, &[0, 0, 0, 100]).await;
    // Article 2 has a star and stays below the cut.
    assert!(
        reading::set_starred(&db, account_id, feed_id, 2, true)
            .await
            .unwrap()
    );
    keep(&db, articles_after_a_year(Remove::Article)).await;
    let swept = retention::sweep(&db, None, a_year_after(50), None)
        .await
        .unwrap();
    assert_eq!(swept.articles, 2);

    // Unread again, on one that went: it would count for nothing.
    assert!(
        !reading::set_read(&db, account_id, feed_id, 1, false)
            .await
            .unwrap()
    );
    assert!(
        !reading::set_starred(&db, account_id, feed_id, 3, true)
            .await
            .unwrap()
    );
    let none = reading::set_many_on(
        &mut db.acquire().await.unwrap(),
        account_id,
        feed_id,
        &[1, 3],
        reading::Many::Read(false),
    )
    .await
    .unwrap();
    assert!(none);
    // The starred one that stayed was unread, and is.
    let count = |db: PgPool| async move {
        reading::unread_counts(&db, account_id).await.unwrap()[0].unread
    };
    assert_eq!(count(db.clone()).await, 2);
    assert!(
        reading::set_read(&db, account_id, feed_id, 2, true)
            .await
            .unwrap()
    );
    assert_eq!(count(db.clone()).await, 1);
    assert!(
        reading::set_read(&db, account_id, feed_id, 2, false)
            .await
            .unwrap()
    );
    assert_eq!(count(db.clone()).await, 2);
    let rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM article_states WHERE feed_id = $1 AND seq IN (1, 3)",
    )
    .bind(feed_id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(rows, 0);
}

#[sqlx::test]
async fn months_of_versions_go_whole(db: PgPool) {
    let (account_id, feed_id) = common::reader(&db, "ada", 0).await;
    // Articles in March, April and May 2026.
    add_arriving(&db, feed_id, &[0, 31, 62]).await;
    keep(
        &db,
        Articles {
            versions_months: Some(12),
            ..Articles::default()
        },
    )
    .await;
    // A year after the middle of May: March and April lie wholly before it.
    let now = START + Duration::days(75);
    let now = now.replace_year(now.year() + 1).unwrap();
    let swept = retention::sweep(&db, None, now, None).await.unwrap();
    assert_eq!(swept.months, ["2026_03", "2026_04"]);
    for (seq, versions) in [(1, 0), (2, 0), (3, 1)] {
        let kept = articles::versions(&db, account_id, feed_id, seq)
            .await
            .unwrap();
        assert_eq!(kept.len(), versions, "versions of {seq}");
    }
    // The articles themselves stay, with their text.
    let opened = articles::open(&db, account_id, feed_id, 1)
        .await
        .unwrap()
        .unwrap();
    assert!(!opened.html.is_empty());
    assert!(!opened.text_dropped);

    // Without a version to compare with, a text that reads differently is put
    // in place, but is no new version; a new headline is.
    let mut changed = arriving(1, 0);
    changed.html = "<p>Cleaned another way.</p>".into();
    let mut renamed = arriving(2, 31);
    renamed.original.title = "A new headline".into();
    let mut tx = db.begin().await.unwrap();
    let stored = feeds::store(&mut tx, feed_id, &[changed, renamed])
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(stored.updated, 1);
    let opened = articles::open(&db, account_id, feed_id, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (opened.version, opened.html.as_str()),
        (1, "<p>Cleaned another way.</p>")
    );
    let opened = articles::open(&db, account_id, feed_id, 2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (opened.version, opened.title.as_str()),
        (2, "A new headline")
    );
}

#[sqlx::test]
async fn a_text_that_went_stays_gone_when_the_feed_lists_it(db: PgPool) {
    let (account_id, feed_id) = common::reader(&db, "ada", 0).await;
    let sent = add_arriving(&db, feed_id, &[0, 100]).await;
    keep(&db, articles_after_a_year(Remove::Text)).await;
    let swept = retention::sweep(&db, None, a_year_after(50), None)
        .await
        .unwrap();
    assert_eq!(swept.texts, 1);
    let opened = articles::open(&db, account_id, feed_id, 1)
        .await
        .unwrap()
        .unwrap();
    assert!(opened.text_dropped);
    assert_eq!(opened.html, "");
    assert_eq!(opened.title, "Article 1");

    // Changed at its source, or cleaned anew: it is left as it is.
    let mut again = sent.clone();
    again[0].html = "<p>Another text.</p>".into();
    again[0].original.title = "Another title".into();
    let mut tx = db.begin().await.unwrap();
    let stored = feeds::store(&mut tx, feed_id, &again).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!((stored.new, stored.updated), (0, 0));
    let opened = articles::open(&db, account_id, feed_id, 1)
        .await
        .unwrap()
        .unwrap();
    assert!(opened.text_dropped);
    assert_eq!(opened.title, "Article 1");
}

async fn orphaned_since(db: &PgPool, feed_id: i32) -> Option<OffsetDateTime> {
    sqlx::query_scalar("SELECT orphaned_at FROM feeds WHERE id = $1")
        .bind(feed_id)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn feed_exists(db: &PgPool, feed_id: i32) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM feeds WHERE id = $1)")
        .bind(feed_id)
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test]
async fn feeds_nobody_reads_go_after_their_days(db: PgPool) {
    let lonely = common::feed(&db, "lonely").await;
    let read = common::feed(&db, "read").await;
    let blocked = common::feed(&db, "blocked").await;
    let back = common::feed(&db, "back").await;
    for feed_id in [lonely, read, blocked, back] {
        add_arriving(&db, feed_id, &[0, 1, 2]).await;
    }
    let ada = common::account(&db, "ada").await;
    common::subscribe(&db, ada, read).await;
    sqlx::query("UPDATE feeds SET blocked_at = now() WHERE id = $1")
        .bind(blocked)
        .execute(&db)
        .await
        .unwrap();
    // Labelled by somebody who left: it does not keep the feed.
    let label: i32 = sqlx::query_scalar(
        "INSERT INTO labels (account_id, title, color, position) VALUES ($1, 'l', '#336699', 1)
         RETURNING id",
    )
    .bind(ada)
    .fetch_one(&db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO article_labels (label_id, feed_id, seq) VALUES ($1, $2, 1)")
        .bind(label)
        .bind(lonely)
        .execute(&db)
        .await
        .unwrap();
    keep(
        &db,
        Articles {
            orphans_days: Some(90),
            ..Articles::default()
        },
    )
    .await;

    // The first run sees which have no readers, and counts from then.
    let first = START + Duration::days(10);
    let swept = retention::sweep(&db, None, first, None).await.unwrap();
    assert!(swept.is_empty());
    assert_eq!(orphaned_since(&db, lonely).await, Some(first));
    assert_eq!(orphaned_since(&db, read).await, None);

    // One comes back before its time: it is counted anew.
    common::subscribe(&db, ada, back).await;
    retention::sweep(&db, None, first + Duration::days(1), None)
        .await
        .unwrap();
    assert_eq!(orphaned_since(&db, back).await, None);

    let swept = retention::sweep(&db, None, first + Duration::days(91), None)
        .await
        .unwrap();
    assert_eq!(swept.feeds, 1);
    assert_eq!(swept.feed_articles, 6);
    assert!(!feed_exists(&db, lonely).await);
    // The blocked one keeps its row and its block, without its articles.
    assert!(feed_exists(&db, blocked).await);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM articles WHERE feed_id = $1")
        .bind(blocked)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(left, 0);
    for feed_id in [read, back] {
        let left: i64 = sqlx::query_scalar("SELECT count(*) FROM articles WHERE feed_id = $1")
            .bind(feed_id)
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(left, 3, "feed {feed_id} keeps its articles");
    }
}

#[sqlx::test]
async fn a_new_subscription_starts_at_the_cut(db: PgPool) {
    let feed_id = common::feed(&db, "half").await;
    add_arriving(&db, feed_id, &[0, 0, 0, 0, 100, 100]).await;
    // Half of it went while nobody read it.
    sqlx::query("DELETE FROM articles WHERE feed_id = $1 AND seq <= 4")
        .bind(feed_id)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("UPDATE feeds SET cut_seq = 4 WHERE id = $1")
        .bind(feed_id)
        .execute(&db)
        .await
        .unwrap();
    let ada = common::account(&db, "ada").await;
    common::subscribe(&db, ada, feed_id).await;
    let counts = reading::unread_counts(&db, ada).await.unwrap();
    assert_eq!((counts[0].read_upto, counts[0].unread), (4, 2));
}

#[sqlx::test]
async fn a_long_feed_goes_in_portions(db: PgPool) {
    let (ada, feed_id) = (
        common::account(&db, "ada").await,
        common::feed(&db, "long").await,
    );
    common::subscribe(&db, ada, feed_id).await;
    let portion = retention::PORTION as usize;
    let mut days = vec![0; 2 * portion + 100];
    days.extend([100, 100]);
    add_arriving(&db, feed_id, &days).await;
    // One in the second portion has a star, one in the third was read.
    let starred = portion as i32 + 7;
    assert!(
        reading::set_starred(&db, ada, feed_id, starred, true)
            .await
            .unwrap()
    );
    let read = 2 * portion as i32 + 50;
    assert!(
        reading::set_read(&db, ada, feed_id, read, true)
            .await
            .unwrap()
    );
    keep(&db, articles_after_a_year(Remove::Article)).await;
    let swept = retention::sweep(&db, None, a_year_after(50), None)
        .await
        .unwrap();
    let old = 2 * portion + 100;
    assert_eq!(swept.articles as usize, old - 1);
    assert!(!swept.more);
    let counts = reading::unread_counts(&db, ada).await.unwrap();
    // The two young ones and the starred one, which nobody read.
    assert_eq!((counts[0].read_upto, counts[0].unread), (old as i32, 3));
    let left: Vec<i32> =
        sqlx::query_scalar("SELECT seq FROM articles WHERE feed_id = $1 ORDER BY seq")
            .bind(feed_id)
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(left, [starred, old as i32 + 1, old as i32 + 2]);
    let tombstones: i64 = sqlx::query_scalar("SELECT count(*) FROM article_tombstones")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(tombstones as usize, old - 1);
}

#[sqlx::test]
async fn a_run_stops_when_its_time_is_up(db: PgPool) {
    let (_, feed_id) = common::reader(&db, "ada", 0).await;
    let portion = retention::PORTION as usize;
    add_arriving(&db, feed_id, &vec![0; 2 * portion]).await;
    keep(&db, articles_after_a_year(Remove::Article)).await;
    // The time is up after the first portion: the rest is for the next run.
    let swept = retention::sweep(&db, None, a_year_after(50), Some(std::time::Instant::now()))
        .await
        .unwrap();
    assert!(swept.more);
    assert!(swept.articles as usize <= portion);
    let swept = retention::sweep(&db, None, a_year_after(50), None)
        .await
        .unwrap();
    assert!(!swept.more);
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM articles WHERE feed_id = $1")
        .bind(feed_id)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(left, 0);
}

#[sqlx::test]
async fn a_move_still_says_where_the_mark_is_after_a_cut(db: PgPool) {
    use rueder::transfer::{self, Node};

    let (ada, feed_id) = (
        common::account(&db, "ada").await,
        common::feed(&db, "moved").await,
    );
    common::subscribe(&db, ada, feed_id).await;
    let sent = add_arriving(&db, feed_id, &[0, 0, 0, 100]).await;
    keep(&db, articles_after_a_year(Remove::Article)).await;
    retention::sweep(&db, None, a_year_after(50), None)
        .await
        .unwrap();
    // The mark is at the cut, on an article that went.
    let counts = reading::unread_counts(&db, ada).await.unwrap();
    assert_eq!((counts[0].read_upto, counts[0].unread), (3, 1));

    let file = transfer::export(&db, ada).await.unwrap();
    let feed = file
        .tree
        .iter()
        .find_map(|node| match node {
            Node::Feed(feed) => Some(feed),
            _ => None,
        })
        .unwrap();
    let mark = feed
        .read_upto
        .as_ref()
        .expect("the mark goes with the account");
    assert_eq!(mark.seq, 3);
    assert_eq!(mark.guid, sent[2].guid);
    assert_eq!(mark.published_at, sent[2].published_at);
}
