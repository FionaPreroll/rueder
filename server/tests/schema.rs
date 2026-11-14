//! The rules the schema itself keeps, whatever the program above it does.

mod common;

use common::{account, add, feed, reader, subscribe};
use sqlx::PgPool;

/// The constraint a statement was refused for.
async fn refused(db: &PgPool, sql: &str) -> String {
    let err = sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
        .execute(db)
        .await
        .expect_err(sql);
    err.as_database_error()
        .and_then(|err| err.constraint())
        .unwrap_or_else(|| panic!("{sql}: {err}"))
        .to_owned()
}

async fn run(db: &PgPool, sql: &str) {
    sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
        .execute(db)
        .await
        .unwrap_or_else(|err| panic!("{sql}: {err}"));
}

async fn folder(db: &PgPool, account_id: i32, parent_id: Option<i32>, title: &str) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO folders (account_id, parent_id, title, position) VALUES ($1, $2, $3, 1)
         RETURNING id",
    )
    .bind(account_id)
    .bind(parent_id)
    .bind(title)
    .fetch_one(db)
    .await
    .unwrap()
}

/// A chain of folders, each inside the one before.
async fn chain(db: &PgPool, account_id: i32, name: &str, levels: usize) -> Vec<i32> {
    let mut ids: Vec<i32> = Vec::new();
    for level in 1..=levels {
        let parent = ids.last().copied();
        ids.push(folder(db, account_id, parent, &format!("{name}{level}")).await);
    }
    ids
}

#[sqlx::test]
async fn folders_nest_five_levels_deep_and_no_deeper(db: PgPool) {
    let reader = account(&db, "reader").await;
    let five = chain(&db, reader, "a", 5).await;
    let sixth = format!(
        "INSERT INTO folders (account_id, parent_id, title, position)
         VALUES ({reader}, {}, 'too deep', 1)",
        five[4]
    );
    assert_eq!(refused(&db, &sixth).await, "folders_max_depth");

    // A move counts the folder with all that is in it.
    let three = chain(&db, reader, "b", 3).await;
    let into = |folder: i32, parent: i32| {
        format!("UPDATE folders SET parent_id = {parent} WHERE id = {folder}")
    };
    // Three levels under the third level would make six.
    assert_eq!(
        refused(&db, &into(three[0], five[2])).await,
        "folders_max_depth"
    );
    // Under the second level they make five.
    run(&db, &into(three[0], five[1])).await;
    // Now nothing fits under the last of them.
    let under = format!(
        "INSERT INTO folders (account_id, parent_id, title, position)
         VALUES ({reader}, {}, 'too deep', 1)",
        three[2]
    );
    assert_eq!(refused(&db, &under).await, "folders_max_depth");
    // Back to the top level is always fine.
    run(
        &db,
        &format!(
            "UPDATE folders SET parent_id = NULL WHERE id = {}",
            three[0]
        ),
    )
    .await;
    // Renaming and reordering do not ask about the depth.
    run(
        &db,
        &format!(
            "UPDATE folders SET title = 'deepest', position = 7 WHERE id = {}",
            five[4]
        ),
    )
    .await;
}

#[sqlx::test]
async fn a_folder_never_ends_up_inside_itself(db: PgPool) {
    let reader = account(&db, "reader").await;
    let ids = chain(&db, reader, "a", 3).await;
    let into = |folder: i32, parent: i32| {
        format!("UPDATE folders SET parent_id = {parent} WHERE id = {folder}")
    };
    // Into itself: the trigger sees it before the table's own check does.
    assert_eq!(
        refused(&db, &into(ids[0], ids[0])).await,
        "folders_no_circle"
    );
    assert_eq!(
        refused(&db, &into(ids[0], ids[1])).await,
        "folders_no_circle"
    );
    assert_eq!(
        refused(&db, &into(ids[0], ids[2])).await,
        "folders_no_circle"
    );
    // The other way round is an ordinary move.
    run(&db, &into(ids[2], ids[0])).await;
}

#[sqlx::test]
async fn the_tree_of_an_account_holds_only_its_own(db: PgPool) {
    let (reader, feed_id) = reader(&db, "reader", 1).await;
    let stranger = account(&db, "stranger").await;
    let theirs = folder(&db, stranger, None, "Theirs").await;

    let nested = format!(
        "INSERT INTO folders (account_id, parent_id, title, position)
         VALUES ({reader}, {theirs}, 'Mine', 1)"
    );
    assert_eq!(
        refused(&db, &nested).await,
        "folders_account_id_parent_id_fkey"
    );
    let moved = format!(
        "UPDATE subscriptions SET folder_id = {theirs}
         WHERE account_id = {reader} AND feed_id = {feed_id}"
    );
    assert_eq!(
        refused(&db, &moved).await,
        "subscriptions_account_id_folder_id_fkey"
    );
    let rule = format!(
        "INSERT INTO filter_rules (account_id, position, folder_id, field, match, pattern, action)
         VALUES ({reader}, 1, {theirs}, 'title', 'contains', 'ad', 'hide')"
    );
    assert_eq!(
        refused(&db, &rule).await,
        "filter_rules_account_id_folder_id_fkey"
    );
    // A rule for a feed wants the account to subscribe to it.
    let other_feed = feed(&db, "other").await;
    let rule = format!(
        "INSERT INTO filter_rules (account_id, position, feed_id, field, match, pattern, action)
         VALUES ({reader}, 1, {other_feed}, 'title', 'contains', 'ad', 'hide')"
    );
    assert_eq!(
        refused(&db, &rule).await,
        "filter_rules_account_id_feed_id_fkey"
    );
}

#[sqlx::test]
async fn a_deleted_folder_leaves_what_it_held_on_the_top_level(db: PgPool) {
    let (reader, feed_id) = reader(&db, "reader", 1).await;
    let ids = chain(&db, reader, "a", 2).await;
    run(
        &db,
        &format!(
            "UPDATE subscriptions SET folder_id = {} WHERE account_id = {reader}",
            ids[0]
        ),
    )
    .await;
    run(
        &db,
        &format!(
            "INSERT INTO filter_rules (account_id, position, folder_id, field, match, pattern, action)
             VALUES ({reader}, 1, {}, 'any', 'word', 'ad', 'read')",
            ids[0]
        ),
    )
    .await;

    run(&db, &format!("DELETE FROM folders WHERE id = {}", ids[0])).await;
    let left: Vec<(i32, Option<i32>, i32)> =
        sqlx::query_as("SELECT id, parent_id, account_id FROM folders")
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(left, [(ids[1], None, reader)]);
    let subscription: (i32, Option<i32>) =
        sqlx::query_as("SELECT feed_id, folder_id FROM subscriptions WHERE account_id = $1")
            .bind(reader)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(subscription, (feed_id, None));
    // The folder's rules go with it.
    let rules: i64 = sqlx::query_scalar("SELECT count(*) FROM filter_rules")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(rules, 0);
}

#[sqlx::test]
async fn numbers_of_a_feed_are_given_out_once(db: PgPool) {
    let feed_id = feed(&db, "a").await;
    add(&db, feed_id, 2).await;
    let again = format!(
        "INSERT INTO articles (feed_id, seq, published_at, title) VALUES ({feed_id}, 2, now(), 'x')"
    );
    assert_eq!(refused(&db, &again).await, "articles_pkey");
    let zero = format!(
        "INSERT INTO articles (feed_id, seq, published_at, title) VALUES ({feed_id}, 0, now(), 'x')"
    );
    assert_eq!(refused(&db, &zero).await, "articles_seq_check");
    // Two articles of a feed never share a place in the order of arrival.
    let same_place = format!(
        "INSERT INTO articles (feed_id, seq, id, published_at, title)
         SELECT {feed_id}, 3, id, now(), 'x' FROM articles WHERE feed_id = {feed_id} AND seq = 1"
    );
    assert_eq!(refused(&db, &same_place).await, "articles_arrival");
    // Text and versions belong to an article.
    let text = format!(
        "INSERT INTO article_content (feed_id, seq, guid_hash, checksum, guid, html)
         VALUES ({feed_id}, 9, decode(repeat('00', 16), 'hex'), '', 'g', '')"
    );
    assert_eq!(
        refused(&db, &text).await,
        "article_content_feed_id_seq_fkey"
    );
    assert_eq!(
        refused(
            &db,
            &format!("UPDATE feeds SET last_seq = -1 WHERE id = {feed_id}")
        )
        .await,
        "feeds_last_seq_check"
    );

    // A feed takes its articles with it, in all three tables.
    run(&db, &format!("DELETE FROM feeds WHERE id = {feed_id}")).await;
    for table in ["articles", "article_content", "article_versions"] {
        let rows: i64 =
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(rows, 0, "{table}");
    }
}

#[sqlx::test]
async fn a_state_says_something_about_a_subscribed_feed(db: PgPool) {
    let (reader, feed_id) = reader(&db, "reader", 3).await;
    let state = |flags: &str| {
        format!(
            "INSERT INTO article_states (account_id, feed_id, seq, {flags})
             VALUES ({reader}, {feed_id}, 1, true, true)"
        )
    };
    assert_eq!(
        refused(&db, &state("read, kept_unread")).await,
        "article_states_read_or_kept"
    );
    let nothing = format!(
        "INSERT INTO article_states (account_id, feed_id, seq) VALUES ({reader}, {feed_id}, 1)"
    );
    assert_eq!(refused(&db, &nothing).await, "article_states_not_empty");
    run(&db, &state("read, starred")).await;
    assert_eq!(
        refused(&db, &state("hidden, starred")).await,
        "article_states_pkey"
    );
    let cleared = format!(
        "UPDATE article_states SET read = false, starred = false WHERE account_id = {reader}"
    );
    assert_eq!(refused(&db, &cleared).await, "article_states_not_empty");

    // Not without a subscription, and gone with it.
    let other = feed(&db, "other").await;
    let unsubscribed = format!(
        "INSERT INTO article_states (account_id, feed_id, seq, starred)
         VALUES ({reader}, {other}, 1, true)"
    );
    assert_eq!(
        refused(&db, &unsubscribed).await,
        "article_states_account_id_feed_id_fkey"
    );
    run(
        &db,
        &format!("DELETE FROM subscriptions WHERE account_id = {reader}"),
    )
    .await;
    let states: i64 = sqlx::query_scalar("SELECT count(*) FROM article_states")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(states, 0);
    assert_eq!(
        refused(
            &db,
            &format!(
                "INSERT INTO subscriptions (account_id, feed_id, position, read_upto)
                 VALUES ({reader}, {feed_id}, 1, -1)"
            )
        )
        .await,
        "subscriptions_read_upto_check"
    );
}

#[sqlx::test]
async fn rules_labels_and_names_are_checked(db: PgPool) {
    let (reader, feed_id) = reader(&db, "reader", 0).await;
    let folder_id = folder(&db, reader, None, "News").await;
    let rule = |scope: &str, field: &str, kind: &str, pattern: &str, action: &str| {
        format!(
            "INSERT INTO filter_rules
                 (account_id, position, folder_id, feed_id, field, match, pattern, action)
             VALUES ({reader}, 1, {scope}, '{field}', '{kind}', '{pattern}', '{action}')"
        )
    };
    run(&db, &rule("NULL, NULL", "any", "contains", "ad", "hide")).await;
    run(
        &db,
        &rule(&format!("{folder_id}, NULL"), "tag", "word", "ad", "read"),
    )
    .await;
    run(
        &db,
        &rule(
            &format!("NULL, {feed_id}"),
            "author",
            "regex",
            "^ad",
            "hide",
        ),
    )
    .await;
    let both = format!("{folder_id}, {feed_id}");
    assert_eq!(
        refused(&db, &rule(&both, "any", "contains", "ad", "hide")).await,
        "filter_rules_one_scope"
    );
    for (field, kind, pattern, action, constraint) in [
        ("body", "contains", "ad", "hide", "filter_rules_field_check"),
        ("title", "glob", "ad", "hide", "filter_rules_match_check"),
        ("title", "word", "", "hide", "filter_rules_pattern_check"),
        (
            "title",
            "word",
            &"x".repeat(201),
            "hide",
            "filter_rules_pattern_check",
        ),
        ("title", "word", "ad", "delete", "filter_rules_action_check"),
    ] {
        assert_eq!(
            refused(&db, &rule("NULL, NULL", field, kind, pattern, action)).await,
            constraint
        );
    }
    run(
        &db,
        &rule("NULL, NULL", "title", "word", &"x".repeat(200), "hide"),
    )
    .await;

    let label = |title: &str, color: &str| {
        format!(
            "INSERT INTO labels (account_id, title, color, position)
             VALUES ({reader}, '{title}', '{color}', 1)"
        )
    };
    run(&db, &label("Later", "#1a2B3c")).await;
    assert_eq!(
        refused(&db, &label("Later", "#000000")).await,
        "labels_account_id_title_key"
    );
    for color in ["red", "#12345", "#1234567", "#12345g"] {
        assert_eq!(
            refused(&db, &label("Soon", color)).await,
            "labels_color_check"
        );
    }

    // Login names do not tell upper from lower case.
    assert_eq!(
        refused(&db, "INSERT INTO accounts (username) VALUES ('READER')").await,
        "accounts_username"
    );
    assert_eq!(
        refused(&db, "INSERT INTO accounts (username) VALUES ('')").await,
        "accounts_username_check"
    );
    assert_eq!(
        refused(
            &db,
            &format!("UPDATE feeds SET fetch_agent = 'curl' WHERE id = {feed_id}")
        )
        .await,
        "feeds_fetch_agent_check"
    );
    // Every account has its own subscription to a feed, once.
    let stranger = account(&db, "stranger").await;
    subscribe(&db, stranger, feed_id).await;
    let twice = format!(
        "INSERT INTO subscriptions (account_id, feed_id, position) VALUES ({stranger}, {feed_id}, 2)"
    );
    assert_eq!(refused(&db, &twice).await, "subscriptions_pkey");
}

#[sqlx::test]
async fn versions_lie_in_the_month_they_were_fetched_in(db: PgPool) {
    let feed_id = feed(&db, "a").await;
    add(&db, feed_id, 1).await;
    // Asking twice for the same month is fine.
    for day in ["2031-01-31", "2031-01-01", "2031-02-01"] {
        run(&db, &format!("SELECT ensure_versions_partition('{day}')")).await;
    }
    // Months are cut in UTC.
    run(
        &db,
        &format!(
            "INSERT INTO article_versions (feed_id, seq, version, fetched_at, checksum, raw)
             VALUES ({feed_id}, 1, 2, '2031-01-31 23:59:59+00', '', ''),
                    ({feed_id}, 1, 3, '2031-02-01 00:30:00+01', '', ''),
                    ({feed_id}, 1, 4, '2031-02-01 00:00:00+00', '', '')"
        ),
    )
    .await;
    let partitions: Vec<(String, i64)> = sqlx::query_as(
        "SELECT tableoid::regclass::text, count(*) FROM article_versions
         WHERE version > 1 GROUP BY 1 ORDER BY 1",
    )
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(
        partitions,
        [
            ("article_versions_2031_01".to_owned(), 2),
            ("article_versions_2031_02".to_owned(), 1)
        ]
    );
    // A month nobody asked for has no place to put a version.
    let homeless = sqlx::query(
        "INSERT INTO article_versions (feed_id, seq, version, fetched_at, checksum, raw)
         VALUES ($1, 1, 5, '2031-03-01 00:00:00+00', '', '')",
    )
    .bind(feed_id)
    .execute(&db)
    .await;
    assert!(homeless.is_err());
}

/// The migration of 2026-11-14 takes the brackets off the names of authors that
/// were stored before the fetch did so itself, in the article and in the copy
/// the search reads, and leaves every other name as it is.
#[sqlx::test]
async fn stored_authors_lose_their_brackets(db: PgPool) {
    let (_, feed_id) = reader(&db, "ada", 6).await;
    let names = [
        ("(Jonas Erbe)", "Jonas Erbe"),
        ("<Jonas Erbe>", "Jonas Erbe"),
        ("( Jonas Erbe )", "Jonas Erbe"),
        ("(Klammern) im Namen", "(Klammern) im Namen"),
        ("Jonas Erbe", "Jonas Erbe"),
        ("", ""),
    ];
    for (seq, (stored, _)) in (1..).zip(names) {
        sqlx::query("UPDATE articles SET author = $1 WHERE feed_id = $2 AND seq = $3")
            .bind(stored)
            .bind(feed_id)
            .bind(seq)
            .execute(&db)
            .await
            .unwrap();
    }
    sqlx::raw_sql(include_str!("../migrations/0018_author_names.sql"))
        .execute(&db)
        .await
        .unwrap();
    for (seq, (stored, name)) in (1..).zip(names) {
        let (author, copied): (String, String) = sqlx::query_as(
            "SELECT a.author, c.author FROM articles a JOIN article_content c USING (feed_id, seq)
             WHERE feed_id = $1 AND seq = $2",
        )
        .bind(feed_id)
        .bind(seq)
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(author, name, "{stored}");
        assert_eq!(copied, name, "{stored}");
    }
}
