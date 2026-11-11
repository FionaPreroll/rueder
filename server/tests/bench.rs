use rueder::bench::{self, Size};
use sqlx::PgPool;

/// The made-up data keeps the rules of the real thing, and every query that is
/// timed runs on it.
#[sqlx::test]
async fn made_up_data_can_be_measured(db: PgPool) {
    let size = Size {
        accounts: 3,
        feeds: 12,
        articles: 3000,
    };
    bench::generate(&db, size).await.unwrap();
    // Only into an empty database.
    assert!(bench::generate(&db, size).await.is_err());

    // Numbers without gaps, rising with the order of arrival.
    let broken: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM feeds f
         WHERE f.last_seq <> (SELECT count(*) FROM articles a WHERE a.feed_id = f.id)
            OR f.last_seq <> (SELECT coalesce(max(a.seq), 0) FROM articles a WHERE a.feed_id = f.id)",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(broken, 0);
    let out_of_order: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM (
             SELECT id, lag(id) OVER (PARTITION BY feed_id ORDER BY seq) AS before FROM articles
         ) a WHERE a.before >= a.id",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(out_of_order, 0);
    // What was read is stored the way the reading functions leave it.
    let misplaced: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM article_states st JOIN subscriptions s USING (account_id, feed_id)
         WHERE (st.read AND st.seq <= s.read_upto) OR (st.kept_unread AND st.seq > s.read_upto)
            OR (st.seq = s.read_upto + 1 AND (st.read OR st.hidden))",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(misplaced, 0);

    let read = || async {
        let marks: Vec<(i32, i32, i32)> = sqlx::query_as(
            "SELECT account_id, feed_id, read_upto FROM subscriptions ORDER BY 1, 2",
        )
        .fetch_all(&db)
        .await
        .unwrap();
        let states: Vec<(i32, i32, i32, bool, bool, bool, bool)> =
            sqlx::query_as("SELECT * FROM article_states ORDER BY 1, 2, 3")
                .fetch_all(&db)
                .await
                .unwrap();
        (marks, states)
    };
    let before = read().await;
    let report = bench::measure(&db, 3).await.unwrap();
    // Measuring leaves what was read as it found it, so it can be repeated.
    assert!(read().await == before);
    assert_eq!(
        (report.accounts, report.feeds, report.articles),
        (3, 12, 3000)
    );
    assert_eq!(report.timings.len(), 23);
    assert!(
        report
            .timings
            .iter()
            .all(|t| t.runs == 3 && t.median <= t.max)
    );
    let printed = report.to_string();
    assert!(printed.contains("| All feeds, unread only |"));
    assert!(printed.contains("| articles_pkey |"));
    assert!(printed.contains("| Search, a rare word |"));
    assert!(printed.contains("| article_content_search |"));
    // The most common word is in most articles, and the search finds it.
    let (_, word, articles) = &report.searched[0];
    assert!(*articles > 1000, "{word}: {articles}");
    assert_eq!(bench::word(1), "er");
    assert_eq!(bench::word(32 * 32 + 3), "kaener");
}

/// The made-up texts are words, and the search index is there afterwards,
/// with the statistics the migration gave it.
#[sqlx::test]
async fn made_up_texts_are_words(db: PgPool) {
    let size = Size {
        accounts: 2,
        feeds: 3,
        articles: 200,
    };
    bench::generate(&db, size).await.unwrap();
    let (title, html): (String, String) =
        sqlx::query_as("SELECT title, html FROM article_content ORDER BY id LIMIT 1")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(title.split(' ').count(), 9, "{title}");
    assert!(html.starts_with("<p>") && html.ends_with(".</p>"), "{html}");
    let statistics: i16 = sqlx::query_scalar(
        "SELECT attstattarget FROM pg_attribute
         WHERE attrelid = 'article_content_search'::regclass AND attnum = 1",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(statistics, 1000);
    let copies: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM articles a JOIN article_content c USING (feed_id, seq)
         WHERE (a.id, a.title, a.author) IS DISTINCT FROM (c.id, c.title, c.author)",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(copies, 0);
}

#[sqlx::test]
async fn nothing_to_measure_in_an_empty_database(db: PgPool) {
    assert!(bench::measure(&db, 3).await.is_err());
    let none = Size {
        accounts: 2,
        feeds: 0,
        articles: 10,
    };
    assert!(bench::generate(&db, none).await.is_err());
}
