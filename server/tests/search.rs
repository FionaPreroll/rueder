//! The search: what it finds, held against a simple model with random input,
//! and how its index follows new articles and new versions.

mod common;

use std::collections::{BTreeSet, HashMap, HashSet};

use common::Random;
use rueder::{
    articles::{self, NewArticle},
    reading,
    search::{self, Hits, Outcome, Page},
};
use sqlx::PgPool;

/// Words with shared beginnings, short ones, and letters PostgreSQL has to
/// make small itself.
const WORDS: [&str; 12] = [
    "apfel",
    "apfelbaum",
    "apfelmus",
    "birne",
    "kirsche",
    "ab",
    "x",
    "Über",
    "STRASSE",
    "Straße",
    "wahl",
    "wahlkampf",
];

/// What the model knows of an article.
struct Known {
    feed_id: i32,
    seq: i32,
    words: HashSet<String>,
}

fn written(article: &NewArticle) -> Vec<String> {
    let html = article
        .html
        .trim_start_matches("<p>")
        .trim_end_matches("</p>");
    let mut words: Vec<String> = [article.original.title.as_str(), html]
        .iter()
        .flat_map(|text| text.split_whitespace())
        .chain(article.original.authors.iter().map(String::as_str))
        .chain(article.original.tags.iter().map(String::as_str))
        .map(str::to_lowercase)
        .collect();
    words.sort();
    words
}

fn made_up(random: &mut Random, n: usize) -> NewArticle {
    let mut pick = |count: usize| -> Vec<&str> {
        (0..count)
            .map(|_| WORDS[random.below(WORDS.len())])
            .collect()
    };
    let title = pick(2).join(" ");
    let text = pick(1 + n % 5).join(" ");
    let tag = pick(1)[0].to_owned();
    let mut article = common::article(n);
    article.original.title = title;
    article.original.authors = vec![["ada", "grace"][n % 2].into()];
    article.original.tags = vec![tag];
    article.html = format!("<p>{text}</p>");
    article
}

/// Whether the model's article has every word asked for, or from three letters
/// on a word beginning with it.
fn matches(words: &HashSet<String>, asked: &[String]) -> bool {
    asked.iter().all(|asked| {
        words.iter().any(|word| {
            word == asked || (asked.chars().count() >= 3 && word.starts_with(asked.as_str()))
        })
    })
}

/// All pages of a search, following `next`.
async fn all_pages(
    db: &PgPool,
    account_id: i32,
    feeds: &[i32],
    words: &str,
    limit: i64,
    unread_only: bool,
) -> Vec<(i32, i32)> {
    let mut found = Vec::new();
    let mut before = None;
    loop {
        let page = Page {
            before,
            limit,
            unread_only,
        };
        let outcome = search::search(db, account_id, feeds, words, page)
            .await
            .unwrap();
        let Outcome::Found(Hits { articles, next }) = outcome else {
            panic!("searching for {words}: {outcome:?}");
        };
        assert!(articles.len() as i64 <= limit);
        assert!(articles.windows(2).all(|w| w[0].id > w[1].id));
        found.extend(articles.iter().map(|a| (a.feed_id, a.seq)));
        match next {
            Some(next) => before = Some(next),
            None => break,
        }
    }
    found
}

/// Random articles in three feeds, two accounts that subscribe to two each,
/// random read and hidden articles, and the order of arrival stretched far
/// apart so that a search goes through many windows. Every search, on every
/// page, finds what the model says, in the order of arrival.
#[sqlx::test]
async fn finds_what_a_model_finds(db: PgPool) {
    let mut random = Random::new(13);
    let accounts = [
        common::account(&db, "ada").await,
        common::account(&db, "grace").await,
    ];
    let feeds = [
        common::feed(&db, "one").await,
        common::feed(&db, "two").await,
        common::feed(&db, "three").await,
    ];
    let subscribed = [vec![feeds[0], feeds[1]], vec![feeds[1], feeds[2]]];
    for (account_id, feeds) in accounts.iter().zip(&subscribed) {
        for &feed_id in feeds {
            common::subscribe(&db, *account_id, feed_id).await;
        }
    }

    // Articles arrive in small batches, feed after feed.
    let mut known = Vec::new();
    let mut n = 0;
    for _ in 0..40 {
        let feed_id = feeds[random.below(feeds.len())];
        let batch: Vec<_> = (0..1 + random.below(4))
            .map(|_| {
                n += 1;
                made_up(&mut random, n)
            })
            .collect();
        let mut tx = db.begin().await.unwrap();
        let first = articles::insert(&mut tx, feed_id, &batch).await.unwrap();
        tx.commit().await.unwrap();
        for (article, seq) in batch.iter().zip(first..) {
            known.push(Known {
                feed_id,
                seq,
                words: written(article).into_iter().collect(),
            });
        }
    }
    // A window holds 20,000 articles: these are spread over millions.
    sqlx::query("UPDATE articles SET id = id * 300007")
        .execute(&db)
        .await
        .unwrap();

    // What each account read and hid, in the database and in the model.
    let mut read: HashSet<(i32, i32, i32)> = HashSet::new();
    let mut hidden: HashSet<(i32, i32, i32)> = HashSet::new();
    for (account_id, feeds) in accounts.iter().zip(&subscribed) {
        for article in known.iter().filter(|a| feeds.contains(&a.feed_id)) {
            let key = (*account_id, article.feed_id, article.seq);
            match random.below(6) {
                0 | 1 => {
                    reading::set_read(&db, key.0, key.1, key.2, true)
                        .await
                        .unwrap();
                    read.insert(key);
                }
                2 => {
                    reading::set_hidden(&db, key.0, key.1, key.2, true)
                        .await
                        .unwrap();
                    hidden.insert(key);
                }
                _ => {}
            }
        }
    }

    let order: HashMap<(i32, i32), i64> =
        sqlx::query_as::<_, (i32, i32, i64)>("SELECT feed_id, seq, id FROM articles")
            .fetch_all(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|(feed_id, seq, id)| ((feed_id, seq), id))
            .collect();

    let asked_for = [
        "apfel",
        "apf",
        "APFELBAUM",
        "ab",
        "x",
        "über",
        "straße",
        "strasse",
        "str",
        "wahl",
        "wahlk",
        "wa",
        "birne kirsche",
        "apfel apfel",
        "kirsche, ab!",
        "zitrone",
    ];
    let mut hits = 0;
    for (round, words) in asked_for.iter().cycle().take(64).enumerate() {
        let who = round % 2;
        let (account_id, own) = (accounts[who], &subscribed[who]);
        // All its feeds, or one of them, or all three with one it does not read.
        let scope: Vec<i32> = match random.below(3) {
            0 => own.clone(),
            1 => vec![own[random.below(own.len())]],
            _ => feeds.to_vec(),
        };
        let unread_only = random.below(2) == 0;
        let limit = 1 + random.below(5) as i64;

        let asked: Vec<String> = words
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_lowercase)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut expected: Vec<(i32, i32)> = known
            .iter()
            .filter(|a| scope.contains(&a.feed_id) && own.contains(&a.feed_id))
            .filter(|a| !hidden.contains(&(account_id, a.feed_id, a.seq)))
            .filter(|a| !unread_only || !read.contains(&(account_id, a.feed_id, a.seq)))
            .filter(|a| matches(&a.words, &asked))
            .map(|a| (a.feed_id, a.seq))
            .collect();
        expected.sort_by_key(|key| std::cmp::Reverse(order[key]));

        hits += expected.len();
        let found = all_pages(&db, account_id, &scope, words, limit, unread_only).await;
        assert_eq!(
            found, expected,
            "{words:?} for account {who}, feeds {scope:?}, unread only {unread_only}"
        );
    }
    // The model is not trivially empty.
    assert!(hits > 200, "{hits}");
}

#[sqlx::test]
async fn what_cannot_be_searched_for(db: PgPool) {
    let (account_id, feed_id) = common::reader(&db, "ada", 3).await;
    let page = Page {
        before: None,
        limit: 10,
        unread_only: false,
    };
    let search = |words: String| {
        let db = db.clone();
        async move {
            search::search(&db, account_id, &[feed_id], &words, page)
                .await
                .unwrap()
        }
    };
    for words in ["", "   ", "!?", "<p></p>"] {
        assert!(
            matches!(search(words.into()).await, Outcome::Unusable(_)),
            "{words:?}"
        );
    }
    assert!(matches!(
        search("article ".repeat(40)).await,
        Outcome::Unusable(_)
    ));
    let eleven: Vec<String> = (0..11).map(|n| format!("w{n}")).collect();
    assert!(matches!(
        search(eleven.join(" ")).await,
        Outcome::Unusable(_)
    ));
    // Quotes, backslashes and the signs of PostgreSQL's queries are only words.
    for words in [
        "it's",
        r"back\slash",
        "a & b | !c",
        "'article':*",
        "(text) <-> of",
    ] {
        assert!(
            matches!(search(words.into()).await, Outcome::Found(_)),
            "{words:?}"
        );
    }
    let Outcome::Found(hits) = search("TEXT of".into()).await else {
        panic!()
    };
    assert_eq!(hits.articles.len(), 3);
    assert_eq!(hits.next, None);
    // Nothing for a feed nobody here subscribes to, and nothing in no feeds.
    let other = common::feed(&db, "other").await;
    common::add(&db, other, 2).await;
    for feeds in [vec![other], vec![]] {
        let outcome = search::search(&db, account_id, &feeds, "text", page)
            .await
            .unwrap();
        assert_eq!(
            outcome,
            Outcome::Found(Hits {
                articles: vec![],
                next: None
            })
        );
    }
}

/// The words come from the current version: a new title, author, text or tag
/// is found, the old one no longer.
#[sqlx::test]
async fn follows_new_versions(db: PgPool) {
    let (account_id, feed_id) = common::reader(&db, "ada", 2).await;
    let found = |words: &'static str| {
        let db = db.clone();
        async move { all_pages(&db, account_id, &[feed_id], words, 10, false).await }
    };
    assert_eq!(found("article").await, [(feed_id, 2), (feed_id, 1)]);
    assert_eq!(found("grace news tag-1").await, [(feed_id, 1)]);

    let mut changed = common::article(1);
    changed.original.title = "Erdbeeren".into();
    changed.original.authors = vec!["Hedy".into()];
    changed.original.tags = vec!["Obst".into()];
    changed.html = "<p>Frisch vom Feld.</p>".into();
    let mut tx = db.begin().await.unwrap();
    articles::add_version(&mut tx, feed_id, 1, &changed)
        .await
        .unwrap()
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(found("article").await, [(feed_id, 2)]);
    assert_eq!(found("tag-1").await, []);
    for words in ["erdbeer", "hedy", "obst", "feld", "frisch erdbeeren"] {
        assert_eq!(found(words).await, [(feed_id, 1)], "{words}");
    }
    // The copy the index reads stays in step with the article.
    let copies: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM articles a JOIN article_content c USING (feed_id, seq)
         WHERE (a.id, a.title, a.author) IS DISTINCT FROM (c.id, c.title, c.author)",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(copies, 0);
}
