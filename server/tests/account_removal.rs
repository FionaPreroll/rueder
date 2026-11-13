//! Deleting and disabling an account (work package 5.6): what goes, what
//! stays for the others, and what a disabled account can no longer do.

mod common;

use axum::http::{Method, StatusCode};
use common::{call, raw};
use rueder::{AppState, accounts, login, oidc, reading, second_factor, sessions};
use serde_json::{Value, json};
use sqlx::PgPool;

const PASSWORD: &str = "a long password";

/// The tables that have a column `account_id`, from the schema itself, so that
/// a table added later is among them without anybody thinking of it.
async fn tables_with_accounts(db: &PgPool) -> Vec<String> {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT c.table_name::text FROM information_schema.columns c
         JOIN information_schema.tables t USING (table_schema, table_name)
         WHERE c.table_schema = 'public' AND c.column_name = 'account_id'
           AND t.table_type = 'BASE TABLE'
         ORDER BY 1",
    )
    .fetch_all(db)
    .await
    .unwrap();
    assert!(tables.len() >= 9, "{tables:?}");
    tables
}

/// The tables without one: feeds, articles and what hangs on them, and the
/// few of the server itself.
async fn tables_without_accounts(db: &PgPool) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT t.table_name::text FROM information_schema.tables t
         WHERE t.table_schema = 'public' AND t.table_type = 'BASE TABLE'
           AND NOT EXISTS (
               SELECT FROM information_schema.columns c
               WHERE c.table_schema = t.table_schema AND c.table_name = t.table_name
                 AND c.column_name = 'account_id')
         ORDER BY 1",
    )
    .fetch_all(db)
    .await
    .unwrap()
}

/// Columns other than `account_id` that point at an account, from the
/// foreign keys of the schema: a table that names an account differently
/// must be thought of too.
async fn other_references(db: &PgPool) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT conrelid::regclass::text || '.' || a.attname::text
         FROM pg_constraint k
         JOIN pg_attribute a ON a.attrelid = k.conrelid AND a.attnum = ANY (k.conkey)
         WHERE k.contype = 'f' AND k.confrelid = 'accounts'::regclass
           AND a.attname <> 'account_id'
         ORDER BY 1",
    )
    .fetch_all(db)
    .await
    .unwrap()
}

async fn rows_of(db: &PgPool, table: &str, account_id: i32) -> i64 {
    let query = format!("SELECT count(*) FROM {table} WHERE account_id = $1");
    sqlx::query_scalar(sqlx::AssertSqlSafe(query))
        .bind(account_id)
        .fetch_one(db)
        .await
        .unwrap()
}

/// All rows of a table but those of the account, as JSON in a fixed order.
async fn rows_but(db: &PgPool, table: &str, account_id: i32) -> Value {
    let query = format!(
        "SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text), '[]')
         FROM {table} t WHERE account_id <> $1"
    );
    sqlx::query_scalar(sqlx::AssertSqlSafe(query))
        .bind(account_id)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn all_rows(db: &PgPool, table: &str) -> Value {
    let query = format!(
        "SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text), '[]') FROM {table} t"
    );
    sqlx::query_scalar(sqlx::AssertSqlSafe(query))
        .fetch_one(db)
        .await
        .unwrap()
}

/// The other accounts, without the invitation they came by, which goes with
/// the account that made it.
async fn other_accounts(db: &PgPool, account_id: i32) -> Value {
    sqlx::query_scalar(
        "SELECT jsonb_agg(to_jsonb(a) - 'invite_id' ORDER BY a.id) FROM accounts a WHERE id <> $1",
    )
    .bind(account_id)
    .fetch_one(db)
    .await
    .unwrap()
}

/// The labels on articles, by whose label they are.
async fn labelled(db: &PgPool, account_id: i32) -> Value {
    sqlx::query_scalar(
        "SELECT coalesce(jsonb_agg(to_jsonb(al) ORDER BY to_jsonb(al)::text), '[]')
         FROM article_labels al JOIN labels l ON l.id = al.label_id WHERE l.account_id = $1",
    )
    .bind(account_id)
    .fetch_one(db)
    .await
    .unwrap()
}

/// Gives an account a bit of everything an account can have: a folder with a
/// feed it shares and one of its own, what it read and starred, a label on an
/// article, a rule, settings, a report of an error and an identity at a
/// provider. The feeds are the shared one and the account's own.
async fn fill(db: &PgPool, account_id: i32, name: &str, shared: i32) -> i32 {
    let own = common::feed(db, &format!("{name}-own")).await;
    common::add(db, own, 4).await;
    let folder = common::folder(db, account_id, None, "News").await;
    for feed in [shared, own] {
        common::subscribe(db, account_id, feed).await;
        common::move_to(db, account_id, feed, folder).await;
        reading::set_read(db, account_id, feed, 1, true)
            .await
            .unwrap();
        reading::set_starred(db, account_id, feed, 2, true)
            .await
            .unwrap();
    }
    let label: i32 = sqlx::query_scalar(
        "INSERT INTO labels (account_id, title, color, position)
         VALUES ($1, 'Later', '#c8402e', 1) RETURNING id",
    )
    .bind(account_id)
    .fetch_one(db)
    .await
    .unwrap();
    for feed in [shared, own] {
        sqlx::query("INSERT INTO article_labels (label_id, feed_id, seq) VALUES ($1, $2, 3)")
            .bind(label)
            .bind(feed)
            .execute(db)
            .await
            .unwrap();
    }
    sqlx::query(
        "INSERT INTO filter_rules (account_id, position, folder_id, field, match, pattern, action)
         VALUES ($1, 1, $2, 'title', 'contains', 'sponsored', 'hide')",
    )
    .bind(account_id)
    .bind(folder)
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE accounts SET settings = '{\"theme\": \"dark\"}', sends_reports = true
         WHERE id = $1",
    )
    .bind(account_id)
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO client_reports (account_id, fingerprint, kind, message, version, browser)
         VALUES ($1, '\\x01', 'error', 'broken', '1', 'firefox')",
    )
    .bind(account_id)
    .execute(db)
    .await
    .unwrap();
    // A report of one of its feeds.
    sqlx::query(
        "INSERT INTO content_reports (account_id, feed_id, reason)
         SELECT $1, feed_id, 'spam' FROM subscriptions WHERE account_id = $1 LIMIT 1",
    )
    .bind(account_id)
    .execute(db)
    .await
    .unwrap();
    // A recovery code of a second factor.
    sqlx::query("INSERT INTO recovery_codes (account_id, code_hash) VALUES ($1, '\\x03')")
        .bind(account_id)
        .execute(db)
        .await
        .unwrap();
    assert!(
        oidc::link(db, name, "https://id.example", &format!("{name}-subject"))
            .await
            .unwrap()
    );
    own
}

async fn make(db: &PgPool, name: &str, admin: bool) -> i32 {
    accounts::create(db, name, Some(PASSWORD), admin)
        .await
        .unwrap()
        .unwrap()
}

async fn log_in(state: &AppState, name: &str, password: &str) -> (StatusCode, String) {
    let body = json!({ "username": name, "password": password }).to_string();
    let body = ("application/json", body.into_bytes());
    let (status, _, answer) = raw(state, Method::POST, "/api/login", None, Some(body)).await;
    (status, String::from_utf8_lossy(&answer).into_owned())
}

async fn delete_own(state: &AppState, cookie: &str, body: Value) -> StatusCode {
    call(
        state,
        Method::DELETE,
        "/api/account",
        Some(cookie),
        Some(body),
    )
    .await
    .0
}

async fn logged_in(state: &AppState, cookie: &str) -> bool {
    let (status, _) = call(state, Method::GET, "/api/me", Some(cookie), None).await;
    status == StatusCode::OK
}

#[sqlx::test]
async fn a_deleted_account_leaves_nothing_and_the_others_keep_everything(db: PgPool) {
    let state = AppState::new(db.clone());
    let shared = common::feed(&db, "shared").await;
    common::add(&db, shared, 5).await;
    let ada = make(&db, "ada", true).await;
    let ada_cookie = common::session(&db, ada).await;
    common::session(&db, ada).await;
    let ada_own = fill(&db, ada, "ada", shared).await;

    // Grace came in by an invitation of Ada's, reads the same feed and has
    // the same kinds of things.
    let (status, made) = call(
        &state,
        Method::POST,
        "/api/invites",
        Some(&ada_cookie),
        Some(json!({ "uses": 2 })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let token = made["token"].as_str().unwrap();
    let body = json!({ "username": "grace", "password": PASSWORD, "invite": token }).to_string();
    let body = ("application/json", body.into_bytes());
    let (status, _, _) = raw(&state, Method::POST, "/api/register", None, Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
    let grace = accounts::id_of(&db, "grace").await.unwrap().unwrap();
    let grace_cookie = common::session(&db, grace).await;
    let grace_own = fill(&db, grace, "grace", shared).await;
    let (status, made) = call(
        &state,
        Method::POST,
        "/api/invites",
        Some(&grace_cookie),
        Some(json!({})),
    )
    .await;
    // Readers have no invitations unless the operator gives them some; one
    // of Grace's own is made here directly.
    assert!(status.is_client_error(), "{made}");
    sqlx::query(
        "INSERT INTO invites (token_hash, account_id, by_admin, max_uses, expires_at)
         VALUES ('\\x02', $1, false, 1, now() + interval '1 day')",
    )
    .bind(grace)
    .execute(&db)
    .await
    .unwrap();

    // Every table with an account in it has something of Ada's, so the test
    // shows that it goes. A new table that this test does not fill fails
    // here: fill it in `fill` above.
    let tables = tables_with_accounts(&db).await;
    for table in &tables {
        assert!(
            rows_of(&db, table, ada).await > 0,
            "nothing of Ada's in {table}"
        );
        assert!(
            rows_of(&db, table, grace).await > 0,
            "nothing of Grace's in {table}"
        );
    }
    // An account is named by `account_id` everywhere, but for the invitation
    // an account came by.
    assert_eq!(other_references(&db).await, Vec::<String>::new());

    let mut others = Vec::new();
    for table in &tables {
        others.push(rows_but(&db, table, ada).await);
    }
    let rest = tables_without_accounts(&db).await;
    assert!(rest.contains(&"feeds".to_owned()) && rest.contains(&"articles".to_owned()));
    let mut kept = Vec::new();
    let kept_as_they_are =
        |table: &&String| !["accounts", "article_labels"].contains(&table.as_str());
    for table in rest.iter().filter(kept_as_they_are) {
        kept.push(all_rows(&db, table).await);
    }
    let accounts_before = other_accounts(&db, ada).await;
    let graces_labels = labelled(&db, grace).await;
    assert_eq!(labelled(&db, ada).await.as_array().unwrap().len(), 2);

    // Wrong proof, or none, deletes nothing.
    assert_eq!(
        delete_own(&state, &ada_cookie, json!({ "password": "not it at all" })).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        delete_own(&state, &ada_cookie, json!({ "name": "ada" })).await,
        StatusCode::FORBIDDEN
    );
    let (status, _) = call(
        &state,
        Method::DELETE,
        "/api/account",
        None,
        Some(json!({ "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(logged_in(&state, &ada_cookie).await);

    let (status, headers, _) = raw(
        &state,
        Method::DELETE,
        "/api/account",
        Some(&ada_cookie),
        Some((
            "application/json",
            json!({ "password": PASSWORD }).to_string().into_bytes(),
        )),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let cookie = headers["set-cookie"].to_str().unwrap();
    assert!(cookie.contains("Max-Age=0"), "{cookie}");

    // Nothing of Ada's is left, in any table.
    for table in &tables {
        assert_eq!(rows_of(&db, table, ada).await, 0, "left in {table}");
    }
    assert_eq!(accounts::id_of(&db, "ada").await.unwrap(), None);
    assert_eq!(labelled(&db, ada).await, json!([]));
    assert!(!logged_in(&state, &ada_cookie).await);
    assert_eq!(
        log_in(&state, "ada", PASSWORD).await.0,
        StatusCode::UNAUTHORIZED
    );

    // Grace has all she had, the feeds and their articles are as they were,
    // Ada's own feed included, which nobody reads now.
    for (table, before) in tables.iter().zip(&others) {
        assert_eq!(&rows_but(&db, table, ada).await, before, "{table}");
    }
    for (table, before) in rest.iter().filter(kept_as_they_are).zip(&kept) {
        assert_eq!(&all_rows(&db, table).await, before, "{table}");
    }
    assert_eq!(other_accounts(&db, ada).await, accounts_before);
    assert_eq!(labelled(&db, grace).await, graces_labels);
    let feeds: Vec<i32> = sqlx::query_scalar("SELECT id FROM feeds ORDER BY id")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(feeds, vec![shared, ada_own, grace_own]);
    // Grace no longer names the invitation she came by, as it is gone.
    let invite: Option<i32> = sqlx::query_scalar("SELECT invite_id FROM accounts WHERE id = $1")
        .bind(grace)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(invite, None);
    assert!(logged_in(&state, &grace_cookie).await);
    assert_eq!(log_in(&state, "grace", PASSWORD).await.0, StatusCode::OK);
}

#[sqlx::test]
async fn an_account_without_a_password_is_deleted_by_its_name(db: PgPool) {
    let state = AppState::new(db.clone());
    // As an account that logs in through OpenID Connect only.
    let ada = common::account(&db, "Ada").await;
    let cookie = common::session(&db, ada).await;
    let (_, me) = call(&state, Method::GET, "/api/me", Some(&cookie), None).await;
    assert_eq!(me["has_password"], false);
    for wrong in [
        json!({}),
        json!({ "name": "grace" }),
        json!({ "password": "" }),
    ] {
        assert_eq!(
            delete_own(&state, &cookie, wrong).await,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        delete_own(&state, &cookie, json!({ "name": " ada " })).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(accounts::id_of(&db, "ada").await.unwrap(), None);
}

#[sqlx::test]
async fn wrong_passwords_for_deleting_count_as_at_login(db: PgPool) {
    let state = AppState::new(db.clone());
    let ada = make(&db, "ada", false).await;
    let cookie = common::session(&db, ada).await;
    let (_, me) = call(&state, Method::GET, "/api/me", Some(&cookie), None).await;
    assert_eq!(me["has_password"], true);
    for _ in 0..login::PER_NAME {
        assert_eq!(
            delete_own(&state, &cookie, json!({ "password": "wrong guess" })).await,
            StatusCode::FORBIDDEN
        );
    }
    // Used up: not even the right one is tried now.
    assert_eq!(
        delete_own(&state, &cookie, json!({ "password": PASSWORD })).await,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert!(accounts::id_of(&db, "ada").await.unwrap().is_some());
}

#[sqlx::test]
async fn the_command_line_deletes_what_the_settings_do(db: PgPool) {
    let shared = common::feed(&db, "shared").await;
    common::add(&db, shared, 5).await;
    let ada = make(&db, "ada", false).await;
    common::session(&db, ada).await;
    fill(&db, ada, "ada", shared).await;
    let grace = make(&db, "grace", false).await;
    fill(&db, grace, "grace", shared).await;

    assert!(accounts::remove(&db, ada).await.unwrap());
    for table in tables_with_accounts(&db).await {
        assert_eq!(rows_of(&db, &table, ada).await, 0, "left in {table}");
        if table != "sessions" && table != "invites" {
            assert!(rows_of(&db, &table, grace).await > 0, "{table}");
        }
    }
    // Twice is nothing.
    assert!(!accounts::remove(&db, ada).await.unwrap());
}

#[sqlx::test]
async fn a_disabled_account_cannot_log_in_and_is_told_so(db: PgPool) {
    let state = AppState::new(db.clone());
    let ada = make(&db, "ada", true).await;
    let laptop = common::session(&db, ada).await;
    let phone = common::session(&db, ada).await;
    let (status, made) = call(
        &state,
        Method::POST,
        "/api/invites",
        Some(&laptop),
        Some(json!({})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let token = made["token"].as_str().unwrap().to_owned();
    let invite_works = |state: AppState, token: String| async move {
        let path = format!("/api/register?invite={token}");
        let (_, asked) = call(&state, Method::GET, &path, None, None).await;
        asked["invite"] == true
    };
    assert!(invite_works(state.clone(), token.clone()).await);
    let grace = make(&db, "grace", false).await;
    let graces = common::session(&db, grace).await;

    assert!(accounts::set_disabled(&db, " ADA", true).await.unwrap());
    assert!(!accounts::set_disabled(&db, "nobody", true).await.unwrap());

    // Its sessions are over, and a new one would be no session either.
    assert!(!logged_in(&state, &laptop).await && !logged_in(&state, &phone).await);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions WHERE account_id = $1")
        .bind(ada)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let token_after = sessions::start(&db, ada, false).await.unwrap();
    assert_eq!(sessions::find(&db, &token_after).await.unwrap(), None);

    // Who knows the password hears that the account is disabled; who does
    // not hears what they hear for any wrong password.
    assert_eq!(
        log_in(&state, "ada", PASSWORD).await,
        (StatusCode::FORBIDDEN, "disabled".to_owned())
    );
    assert_eq!(
        log_in(&state, "ada", "not the password").await.0,
        StatusCode::UNAUTHORIZED
    );
    // Its invitations take nobody in.
    assert!(!invite_works(state.clone(), token.clone()).await);
    // Nothing else is touched: Grace goes on, and Ada keeps what she has.
    assert!(logged_in(&state, &graces).await);
    let listed = accounts::list(&db).await.unwrap();
    let disabled: Vec<_> = listed
        .iter()
        .map(|a| (a.username.as_str(), a.disabled))
        .collect();
    assert_eq!(disabled, [("ada", true), ("grace", false)]);
    // Disabled twice keeps when it was first.
    let first: Option<time::OffsetDateTime> =
        sqlx::query_scalar("SELECT disabled_at FROM accounts WHERE id = $1")
            .bind(ada)
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(accounts::set_disabled(&db, "ada", true).await.unwrap());
    let again: Option<time::OffsetDateTime> =
        sqlx::query_scalar("SELECT disabled_at FROM accounts WHERE id = $1")
            .bind(ada)
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(first.is_some());
    assert_eq!(first, again);

    // Enabled, it logs in again, and its invitation works again.
    assert!(accounts::set_disabled(&db, "ada", false).await.unwrap());
    assert_eq!(log_in(&state, "ada", PASSWORD).await.0, StatusCode::OK);
    assert!(invite_works(state.clone(), token).await);
    // What came about while it was disabled does not come alive with it.
    assert_eq!(sessions::find(&db, &token_after).await.unwrap(), None);
    assert!(!accounts::is_disabled(&db, ada).await.unwrap());
}

#[sqlx::test]
async fn an_admin_sees_which_accounts_are_disabled(db: PgPool) {
    let state = AppState::new(db.clone());
    let ada = make(&db, "ada", true).await;
    make(&db, "grace", false).await;
    let cookie = common::session(&db, ada).await;
    accounts::set_disabled(&db, "grace", true).await.unwrap();
    let (status, listed) = call(&state, Method::GET, "/api/accounts", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    let disabled: Vec<_> = listed
        .as_array()
        .unwrap()
        .iter()
        .map(|account| (account["username"].clone(), account["disabled"].clone()))
        .collect();
    assert_eq!(
        disabled,
        [(json!("ada"), json!(false)), (json!("grace"), json!(true))]
    );
}

#[sqlx::test]
async fn a_second_factor_is_asked_for_before_deleting(db: PgPool) {
    let state = AppState::new(db.clone());
    let ada = make(&db, "ada", false).await;
    let cookie = common::session(&db, ada).await;
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/second-factor/setup",
        Some(&cookie),
        Some(json!({ "password": PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let pending: Vec<u8> = sqlx::query_scalar("SELECT totp_pending FROM accounts WHERE id = $1")
        .bind(ada)
        .fetch_one(&db)
        .await
        .unwrap();
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let code = format!(
        "{:06}",
        second_factor::code_at(&pending, second_factor::step_of(unix))
    );
    let (status, codes) = call(
        &state,
        Method::POST,
        "/api/second-factor/confirm",
        Some(&cookie),
        Some(json!({ "code": code })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let recovery = codes["codes"][0].as_str().unwrap().to_owned();

    // The password alone is not enough, and says what is missing.
    let (status, _, said) = raw(
        &state,
        Method::DELETE,
        "/api/account",
        Some(&cookie),
        Some((
            "application/json",
            json!({ "password": PASSWORD }).to_string().into_bytes(),
        )),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(String::from_utf8_lossy(&said), login::CODE_NEEDED);
    // A wrong code with the right password deletes nothing either.
    assert_eq!(
        delete_own(
            &state,
            &cookie,
            json!({ "password": PASSWORD, "code": "000000" })
        )
        .await,
        StatusCode::FORBIDDEN
    );
    // Nor a right code with a wrong password.
    assert_eq!(
        delete_own(
            &state,
            &cookie,
            json!({ "password": "not the password", "code": recovery })
        )
        .await,
        StatusCode::FORBIDDEN
    );
    assert!(accounts::id_of(&db, "ada").await.unwrap().is_some());
    assert_eq!(
        delete_own(
            &state,
            &cookie,
            json!({ "password": PASSWORD, "code": recovery })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(accounts::id_of(&db, "ada").await.unwrap(), None);
}

#[sqlx::test]
async fn a_disabled_account_with_a_second_factor_is_told_so_without_a_code(db: PgPool) {
    let state = AppState::new(db.clone());
    make(&db, "ada", false).await;
    sqlx::query("UPDATE accounts SET totp_secret = '\\x0102' WHERE username = 'ada'")
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(
        log_in(&state, "ada", PASSWORD).await.0,
        StatusCode::UNAUTHORIZED
    );
    accounts::set_disabled(&db, "ada", true).await.unwrap();
    assert_eq!(
        log_in(&state, "ada", PASSWORD).await,
        (StatusCode::FORBIDDEN, "disabled".to_owned())
    );
}
