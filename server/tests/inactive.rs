//! Accounts nobody uses (E13): two mails, and only then the account goes;
//! using it takes the mails back; who never goes; and the share of the day's
//! mail.

use std::time::Duration;

use rueder::{
    accounts, app_passwords, inactive,
    mail::{Mailer, Outbox, Sent},
    rate::{Limits, Rate, Rates},
    sessions,
};
use sqlx::PgPool;
use time::OffsetDateTime;

const SITE: &str = "https://reader.example";

/// Room for the mails of a test; the test of the share sets its own.
fn rates() -> Rates {
    let room = Rate::new(100, Duration::from_secs(3600));
    Rates::new(Limits {
        mails_per_account: room,
        mails_per_address: room,
        mails_per_day: room,
        inactive_mails_per_day: room,
        ..Limits::default()
    })
}

/// An account with a confirmed address, last used `days` days before the end
/// of twelve months counted back from `now`.
async fn account(db: &PgPool, name: &str, now: OffsetDateTime, days: i32) -> i32 {
    let id = accounts::create(db, name, Some("correct horse battery"), false)
        .await
        .unwrap()
        .unwrap();
    sqlx::query(
        "UPDATE accounts
         SET email = $2, active_at = $3 - interval '12 months' + make_interval(days => $4)
         WHERE id = $1",
    )
    .bind(id)
    .bind(format!("{name}@example.org"))
    .bind(now)
    .bind(days)
    .execute(db)
    .await
    .unwrap();
    id
}

async fn exists(db: &PgPool, id: i32) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM accounts WHERE id = $1)")
        .bind(id)
        .fetch_one(db)
        .await
        .unwrap()
}

fn taken(outbox: &Outbox) -> Vec<Sent> {
    std::mem::take(&mut *outbox.lock().unwrap())
}

#[sqlx::test]
async fn an_account_nobody_uses_goes_after_two_mails(db: PgPool) {
    let (mailer, outbox) = Mailer::in_memory(SITE).unwrap();
    let rates = rates();
    let now = OffsetDateTime::now_utc();
    let days = time::Duration::days;
    let ada = account(&db, "ada", now, 30).await;
    sqlx::query("UPDATE accounts SET language = 'de' WHERE id = $1")
        .bind(ada)
        .execute(&db)
        .await
        .unwrap();
    // When its twelve months are over, as PostgreSQL counts months.
    let due: OffsetDateTime =
        sqlx::query_scalar("SELECT active_at + interval '12 months' FROM accounts WHERE id = $1")
            .bind(ada)
            .fetch_one(&db)
            .await
            .unwrap();
    let goes = due.date().to_string();

    // As rueder comes, nothing goes.
    let swept = inactive::sweep(&db, &mailer, &rates, due + days(400))
        .await
        .unwrap();
    assert!(swept.is_empty(), "{swept:?}");
    assert!(taken(&outbox).is_empty());

    inactive::set_months(&db, Some(12)).await.unwrap();
    let swept = inactive::sweep(&db, &mailer, &rates, due - days(31))
        .await
        .unwrap();
    assert!(swept.is_empty(), "{swept:?}");
    let swept = inactive::sweep(&db, &mailer, &rates, due - days(30))
        .await
        .unwrap();
    assert_eq!(swept.warned, 1);
    let first = taken(&outbox);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].to, "ada@example.org");
    assert_eq!(first[0].subject, "Dein rueder-Konto wird gelöscht");
    for said in [&goes, "ada", "12 Monate", "https://reader.example/"] {
        assert!(first[0].body.contains(said), "{said}: {}", first[0].body);
    }

    // Once is enough, and the second comes a week before.
    for later in [due - days(29), due - days(8)] {
        let swept = inactive::sweep(&db, &mailer, &rates, later).await.unwrap();
        assert!(swept.is_empty(), "{swept:?}");
    }
    let swept = inactive::sweep(&db, &mailer, &rates, due - days(7))
        .await
        .unwrap();
    assert_eq!(swept.reminded, 1);
    let second = taken(&outbox);
    assert_eq!(second.len(), 1);
    assert!(
        second[0].subject.starts_with("Erinnerung"),
        "{}",
        second[0].subject
    );
    assert!(second[0].body.contains(&goes), "{}", second[0].body);

    // It goes on the day the mails named, not before.
    inactive::sweep(&db, &mailer, &rates, due - time::Duration::hours(1))
        .await
        .unwrap();
    assert!(exists(&db, ada).await);
    let swept = inactive::sweep(&db, &mailer, &rates, due).await.unwrap();
    assert_eq!(swept.removed, 1);
    assert!(!exists(&db, ada).await);
    assert!(taken(&outbox).is_empty());
}

#[sqlx::test]
async fn a_late_mail_gives_the_whole_time_before_the_account_goes(db: PgPool) {
    let (mailer, outbox) = Mailer::in_memory(SITE).unwrap();
    let rates = rates();
    let now = OffsetDateTime::now_utc();
    let days = time::Duration::days;
    inactive::set_months(&db, Some(12)).await.unwrap();
    // Its months were over ten days ago, and it had no mail yet: maybe the
    // setting is new, maybe the day's mail ran out.
    let grace = account(&db, "grace", now, -10).await;

    inactive::sweep(&db, &mailer, &rates, now).await.unwrap();
    let first = taken(&outbox);
    assert!(first[0].body.contains(&(now + days(30)).date().to_string()));
    inactive::sweep(&db, &mailer, &rates, now + days(23))
        .await
        .unwrap();
    assert_eq!(taken(&outbox).len(), 1);
    inactive::sweep(&db, &mailer, &rates, now + days(29))
        .await
        .unwrap();
    assert!(exists(&db, grace).await);
    inactive::sweep(&db, &mailer, &rates, now + days(30))
        .await
        .unwrap();
    assert!(!exists(&db, grace).await);
}

#[sqlx::test]
async fn using_an_account_takes_its_mails_back(db: PgPool) {
    let (mailer, outbox) = Mailer::in_memory(SITE).unwrap();
    let rates = rates();
    let now = OffsetDateTime::now_utc();
    let days = time::Duration::days;
    inactive::set_months(&db, Some(12)).await.unwrap();
    let linus = account(&db, "linus", now, 30).await;
    let ken = account(&db, "ken", now, 30).await;
    let app = account(&db, "app", now, 30).await;
    inactive::sweep(&db, &mailer, &rates, now).await.unwrap();
    assert_eq!(taken(&outbox).len(), 3);

    // A login, a session that was used, an app that logs in.
    sessions::start(&db, linus, false).await.unwrap();
    let token = sessions::start(&db, ken, false).await.unwrap();
    sqlx::query(
        "UPDATE accounts SET active_at = now() - interval '11 months', inactive_warned_at = now()
         WHERE id = $1",
    )
    .bind(ken)
    .execute(&db)
    .await
    .unwrap();
    sqlx::query("UPDATE sessions SET last_seen_at = now() - interval '2 hours'")
        .execute(&db)
        .await
        .unwrap();
    assert!(sessions::find(&db, &token).await.unwrap().is_some());
    let secret = "a".repeat(app_passwords::SECRET_LEN);
    sqlx::query(
        "INSERT INTO app_passwords (account_id, name, secret_hash)
         VALUES ($1, 'NetNewsWire', sha256(convert_to($2, 'UTF8')))",
    )
    .bind(app)
    .bind(&secret)
    .execute(&db)
    .await
    .unwrap();
    assert!(app_passwords::find(&db, &secret).await.unwrap().is_some());

    let warned: i64 =
        sqlx::query_scalar("SELECT count(*) FROM accounts WHERE inactive_warned_at IS NOT NULL")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(warned, 0);
    let swept = inactive::sweep(&db, &mailer, &rates, now + days(31))
        .await
        .unwrap();
    assert!(swept.is_empty(), "{swept:?}");
    for id in [linus, ken, app] {
        assert!(exists(&db, id).await);
    }
}

#[sqlx::test]
async fn no_address_no_admin_and_no_disabled_account_goes(db: PgPool) {
    let (mailer, outbox) = Mailer::in_memory(SITE).unwrap();
    let rates = rates();
    let now = OffsetDateTime::now_utc();
    inactive::set_months(&db, Some(12)).await.unwrap();
    let unknown = account(&db, "unknown", now, -400).await;
    sqlx::query("UPDATE accounts SET email = NULL WHERE id = $1")
        .bind(unknown)
        .execute(&db)
        .await
        .unwrap();
    let root = account(&db, "root", now, -400).await;
    accounts::set_admin(&db, "root", true).await.unwrap();
    let banned = account(&db, "banned", now, -400).await;
    accounts::set_disabled(&db, "banned", true).await.unwrap();

    let far = now + time::Duration::days(1000);
    for at in [now, far] {
        let swept = inactive::sweep(&db, &mailer, &rates, at).await.unwrap();
        assert!(swept.is_empty(), "{swept:?}");
    }
    assert!(taken(&outbox).is_empty());
    for id in [unknown, root, banned] {
        assert!(exists(&db, id).await);
    }
    // The one without an address is counted for the operator.
    let standing = inactive::standing(&db, 12, now).await.unwrap();
    assert_eq!(standing.without_address, 1);
}

#[sqlx::test]
async fn the_mails_keep_to_their_share_of_the_day(db: PgPool) {
    let (mailer, outbox) = Mailer::in_memory(SITE).unwrap();
    let room = Rate::new(100, Duration::from_secs(3600));
    let rates = Rates::new(Limits {
        mails_per_account: room,
        mails_per_address: room,
        mails_per_day: room,
        inactive_mails_per_day: Rate::new(1, Duration::from_secs(24 * 3600)),
        ..Limits::default()
    });
    let now = OffsetDateTime::now_utc();
    inactive::set_months(&db, Some(12)).await.unwrap();
    account(&db, "first", now, 30).await;
    let second = account(&db, "second", now, 30).await;

    let swept = inactive::sweep(&db, &mailer, &rates, now).await.unwrap();
    assert_eq!((swept.warned, swept.waiting), (1, true));
    assert_eq!(taken(&outbox).len(), 1);
    // Without its mails it does not go, however late it is.
    inactive::sweep(&db, &mailer, &rates, now + time::Duration::days(400))
        .await
        .unwrap();
    assert!(exists(&db, second).await);
    let bounds = inactive::set_months(&db, Some(0)).await;
    assert!(bounds.is_err());
}
