//! Accounts that make themselves: open registration, invitations, and the
//! limits the operator sets for both.

mod common;

use axum::http::{Method, StatusCode, header};
use common::{call, raw};
use rueder::{
    AppState, accounts,
    registration::{self, Outcome, Refused, Settings},
};
use serde_json::{Value, json};
use sqlx::PgPool;

const PASSWORD: &str = "correct horse battery";

/// Changes what the operator set, as `rueder registration set` does.
async fn set(db: &PgPool, change: impl FnOnce(&mut Settings)) {
    let mut settings = registration::settings(db).await.unwrap();
    change(&mut settings);
    registration::set_settings(db, &settings).await.unwrap();
}

/// Tries to register through the API; the status and what the answer says.
async fn register(state: &AppState, name: &str, invite: Option<&str>) -> (StatusCode, String) {
    let mut body = json!({ "username": name, "password": PASSWORD });
    if let Some(invite) = invite {
        body["invite"] = json!(invite);
    }
    let body = ("application/json", body.to_string().into_bytes());
    let (status, _, answer) = raw(state, Method::POST, "/api/register", None, Some(body)).await;
    (status, String::from_utf8(answer).unwrap())
}

async fn refused(state: &AppState, name: &str, invite: Option<&str>) -> String {
    let (status, word) = register(state, name, invite).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{name}: {word}");
    word
}

async fn registered(state: &AppState, name: &str, invite: Option<&str>) {
    let (status, answer) = register(state, name, invite).await;
    assert_eq!(status, StatusCode::CREATED, "{name}: {answer}");
}

/// Makes an invitation as the browser with this cookie; its token.
async fn invite(state: &AppState, cookie: &str, uses: Option<i32>) -> Result<String, StatusCode> {
    let body = uses.map_or(json!({}), |uses| json!({ "uses": uses }));
    let (status, made) = call(
        state,
        Method::POST,
        "/api/invites",
        Some(cookie),
        Some(body),
    )
    .await;
    match status {
        StatusCode::CREATED => Ok(made["token"].as_str().unwrap().to_owned()),
        status => Err(status),
    }
}

async fn left(state: &AppState, cookie: &str) -> Value {
    let (status, invites) = call(state, Method::GET, "/api/invites", Some(cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    invites["left"].clone()
}

async fn origin(db: &PgPool, name: &str) -> String {
    sqlx::query_scalar("SELECT origin FROM accounts WHERE username = $1")
        .bind(name)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn admin(db: &PgPool, name: &str) -> String {
    let id = common::account(db, name).await;
    accounts::set_admin(db, name, true).await.unwrap();
    common::session(db, id).await
}

async fn reader(db: &PgPool, name: &str) -> String {
    let id = common::account(db, name).await;
    common::session(db, id).await
}

#[sqlx::test]
async fn nothing_is_open_until_the_operator_says_so(db: PgPool) {
    let state = AppState::new(db.clone());
    assert_eq!(
        registration::settings(&db).await.unwrap(),
        Settings {
            open: false,
            max_accounts: None,
            per_day: None,
            invites_per_account: 0,
            invite_days: 7,
            admin_invite_accounts: None,
        }
    );
    let (_, ways) = call(&state, Method::GET, "/api/login/ways", None, None).await;
    assert_eq!(
        (&ways["oidc"], &ways["registration"]),
        (&json!(false), &json!(false))
    );
    let (_, asked) = call(&state, Method::GET, "/api/register", None, None).await;
    assert_eq!(asked, json!({ "open": false }));

    assert_eq!(refused(&state, "ada", None).await, "closed");
    assert_eq!(refused(&state, "ada", Some("made up")).await, "invite");
    // A reader has no invitations to give.
    let cookie = reader(&db, "grace").await;
    assert_eq!(left(&state, &cookie).await, 0);
    assert_eq!(
        invite(&state, &cookie, None).await,
        Err(StatusCode::FORBIDDEN)
    );
    let accounts: i64 = sqlx::query_scalar("SELECT count(*) FROM accounts")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(accounts, 1);
}

#[sqlx::test]
async fn open_registration_makes_an_account_and_logs_it_in(db: PgPool) {
    let state = AppState::new(db.clone());
    set(&db, |settings| settings.open = true).await;
    let (_, ways) = call(&state, Method::GET, "/api/login/ways", None, None).await;
    assert_eq!(ways["registration"], true);

    let body = json!({ "username": "  Ada ", "password": PASSWORD }).to_string();
    let body = ("application/json", body.into_bytes());
    let (status, headers, answer) =
        raw(&state, Method::POST, "/api/register", None, Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
    let me: Value = serde_json::from_slice(&answer).unwrap();
    assert_eq!(me["username"], "Ada");
    assert_eq!(me["is_admin"], false);

    // The answer carries a session, as a login's does.
    let cookie = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
    assert!(cookie.starts_with("__Host-rueder=") && cookie.contains("HttpOnly"));
    let cookie = cookie.split(';').next().unwrap();
    let (status, who) = call(&state, Method::GET, "/api/me", Some(cookie), None).await;
    assert_eq!((status, &who["username"]), (StatusCode::OK, &json!("Ada")));
    assert_eq!(origin(&db, "Ada").await, "registration");
    // The password is kept as its hash, and logs in.
    assert!(
        accounts::login(&db, "ada", PASSWORD)
            .await
            .unwrap()
            .is_some()
    );

    // A name is taken whatever its case; a name or a password that cannot be
    // one is said to be so.
    assert_eq!(register(&state, "ADA", None).await.0, StatusCode::CONFLICT);
    assert_eq!(register(&state, " ", None).await.0, StatusCode::BAD_REQUEST);
    let short = json!({ "username": "grace", "password": "short" }).to_string();
    let short = ("application/json", short.into_bytes());
    let (status, _, why) = raw(&state, Method::POST, "/api/register", None, Some(short)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8(why).unwrap().contains("shorter than 8"));
    // What is no registration at all is refused before anything is looked at.
    let nothing = ("application/json", b"{}".to_vec());
    let (status, _, _) = raw(&state, Method::POST, "/api/register", None, Some(nothing)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    set(&db, |settings| settings.open = false).await;
    assert_eq!(refused(&state, "grace", None).await, "closed");
}

#[sqlx::test]
async fn there_are_only_so_many_accounts_and_so_many_a_day(db: PgPool) {
    let state = AppState::new(db.clone());
    // Two accounts the operator made count for the total, not for the day.
    common::account(&db, "one").await;
    common::account(&db, "two").await;
    set(&db, |settings| {
        settings.open = true;
        settings.max_accounts = Some(4);
        settings.per_day = Some(1);
    })
    .await;

    registered(&state, "three", None).await;
    assert_eq!(refused(&state, "four", None).await, "today");
    // The page is not offered a registration that cannot work.
    let (_, ways) = call(&state, Method::GET, "/api/login/ways", None, None).await;
    assert_eq!(ways["registration"], false);

    // A day later there is room for one more, and then none.
    sqlx::query("UPDATE accounts SET created_at = now() - interval '25 hours'")
        .execute(&db)
        .await
        .unwrap();
    registered(&state, "four", None).await;
    set(&db, |settings| settings.per_day = None).await;
    assert_eq!(refused(&state, "five", None).await, "full");
    // Whoever cannot get in does not learn which names are taken.
    assert_eq!(refused(&state, "four", None).await, "full");

    set(&db, |settings| settings.max_accounts = Some(0)).await;
    assert_eq!(refused(&state, "five", None).await, "full");
    set(&db, |settings| settings.max_accounts = None).await;
    registered(&state, "five", None).await;
}

#[sqlx::test]
async fn a_reader_invites_as_many_as_their_quota_says(db: PgPool) {
    let state = AppState::new(db.clone());
    let ada = reader(&db, "ada").await;
    set(&db, |settings| settings.invites_per_account = 3).await;
    assert_eq!(left(&state, &ada).await, 3);

    // One link for one account, one for two: the quota is used up.
    let single = invite(&state, &ada, None).await.unwrap();
    let double = invite(&state, &ada, Some(2)).await.unwrap();
    assert_eq!(left(&state, &ada).await, 0);
    assert_eq!(invite(&state, &ada, None).await, Err(StatusCode::FORBIDDEN));

    // Registration is closed; the links are the way in.
    registered(&state, "bob", Some(&single)).await;
    assert_eq!(refused(&state, "carol", Some(&single)).await, "invite");
    registered(&state, "carol", Some(&double)).await;
    registered(&state, "dave", Some(&double)).await;
    assert_eq!(refused(&state, "erin", Some(&double)).await, "invite");
    assert_eq!(origin(&db, "bob").await, "invite");
    // The new accounts are readers with a quota of their own.
    let invited_by: Vec<String> = sqlx::query_scalar(
        "SELECT i.username FROM accounts a JOIN invites l ON l.id = a.invite_id
         JOIN accounts i ON i.id = l.account_id WHERE a.username IN ('bob', 'carol', 'dave')",
    )
    .fetch_all(&db)
    .await
    .unwrap();
    assert_eq!(invited_by, ["ada", "ada", "ada"]);

    let (_, list) = call(&state, Method::GET, "/api/invites", Some(&ada), None).await;
    let uses: Vec<_> = list["invites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|invite| {
            (
                invite["uses"].clone(),
                invite["max_uses"].clone(),
                invite["usable"].clone(),
            )
        })
        .collect();
    assert_eq!(
        uses,
        [
            (json!(2), json!(2), json!(false)),
            (json!(1), json!(1), json!(false))
        ]
    );
    assert_eq!(list["days"], 7);

    // A quota of the account's own counts before the one for everybody.
    assert_eq!(left(&state, &ada).await, 0);
    assert!(registration::set_quota(&db, "ADA", Some(5)).await.unwrap());
    assert_eq!(left(&state, &ada).await, 2);
    assert!(registration::set_quota(&db, "ada", None).await.unwrap());
    assert_eq!(left(&state, &ada).await, 0);
    assert!(
        !registration::set_quota(&db, "nobody", Some(1))
            .await
            .unwrap()
    );

    // What a link may be for.
    for uses in [0, -1, 1001] {
        assert_eq!(
            invite(&state, &ada, Some(uses)).await,
            Err(StatusCode::BAD_REQUEST)
        );
    }
}

#[sqlx::test]
async fn a_link_that_ran_out_or_was_taken_back_gives_its_place_back(db: PgPool) {
    let state = AppState::new(db.clone());
    let ada = reader(&db, "ada").await;
    set(&db, |settings| {
        settings.invites_per_account = 3;
        settings.invite_days = 2;
    })
    .await;

    let link = invite(&state, &ada, Some(3)).await.unwrap();
    // It is good for as many days as the operator set.
    let days: f64 = sqlx::query_scalar(
        "SELECT (extract(epoch FROM expires_at - created_at) / 86400)::float8 FROM invites",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert!((days - 2.0).abs() < 0.01, "{days}");
    registered(&state, "bob", Some(&link)).await;
    assert_eq!(left(&state, &ada).await, 0);
    let asked = format!("/api/register?invite={link}");
    let (_, answer) = call(&state, Method::GET, &asked, None, None).await;
    assert_eq!(answer, json!({ "open": false, "invite": true }));

    // Run out: nobody gets in with it, and the two places it did not use are
    // the reader's again. The one it used stays used.
    sqlx::query("UPDATE invites SET expires_at = now() - interval '1 minute'")
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(refused(&state, "carol", Some(&link)).await, "invite");
    let (_, answer) = call(&state, Method::GET, &asked, None, None).await;
    assert_eq!(answer, json!({ "open": false, "invite": false }));
    assert_eq!(left(&state, &ada).await, 2);

    // Taken back: the same.
    let second = invite(&state, &ada, Some(2)).await.unwrap();
    assert_eq!(left(&state, &ada).await, 0);
    let (_, list) = call(&state, Method::GET, "/api/invites", Some(&ada), None).await;
    let id = list["invites"][0]["id"].as_i64().unwrap();
    let path = format!("/api/invites/{id}");
    let (status, _) = call(&state, Method::DELETE, &path, Some(&ada), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(refused(&state, "carol", Some(&second)).await, "invite");
    assert_eq!(left(&state, &ada).await, 2);
}

#[sqlx::test]
async fn readers_invitations_keep_to_the_limits(db: PgPool) {
    let state = AppState::new(db.clone());
    let ada = reader(&db, "ada").await;
    set(&db, |settings| {
        settings.invites_per_account = 10;
        settings.max_accounts = Some(3);
        settings.per_day = Some(1);
    })
    .await;
    let link = invite(&state, &ada, Some(10)).await.unwrap();

    registered(&state, "bob", Some(&link)).await;
    assert_eq!(refused(&state, "carol", Some(&link)).await, "today");
    set(&db, |settings| settings.per_day = None).await;
    registered(&state, "carol", Some(&link)).await;
    assert_eq!(refused(&state, "dave", Some(&link)).await, "full");
    // The link itself is still good; there is only no room right now.
    let asked = format!("/api/register?invite={link}");
    let (_, answer) = call(&state, Method::GET, &asked, None, None).await;
    assert_eq!(answer["invite"], true);
    // A refusal uses nothing of the link.
    let uses: i32 = sqlx::query_scalar("SELECT uses FROM invites")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(uses, 2);
    // Open registration and invitations share the day's count.
    set(&db, |settings| {
        settings.open = true;
        settings.max_accounts = None;
        settings.per_day = Some(2);
    })
    .await;
    assert_eq!(refused(&state, "dave", None).await, "today");
}

#[sqlx::test]
async fn an_admin_always_invites_into_a_pool_of_its_own(db: PgPool) {
    let state = AppState::new(db.clone());
    let root = admin(&db, "root").await;
    // The general pool is full and nobody has a quota.
    set(&db, |settings| {
        settings.max_accounts = Some(1);
        settings.per_day = Some(0);
        settings.admin_invite_accounts = Some(2);
    })
    .await;
    assert_eq!(left(&state, &root).await, Value::Null);

    let link = invite(&state, &root, Some(5)).await.unwrap();
    registered(&state, "bob", Some(&link)).await;
    registered(&state, "carol", Some(&link)).await;
    assert_eq!(origin(&db, "bob").await, "admin_invite");
    // The admins' pool has a limit of its own.
    assert_eq!(refused(&state, "dave", Some(&link)).await, "full");
    set(&db, |settings| settings.admin_invite_accounts = None).await;
    registered(&state, "dave", Some(&link)).await;

    // What admins invited takes no place in the general pool.
    let mut conn = db.acquire().await.unwrap();
    let counts = registration::counts(&mut conn).await.unwrap();
    assert_eq!((counts.general, counts.today, counts.by_admins), (1, 0, 3));
    set(&db, |settings| {
        settings.open = true;
        settings.max_accounts = Some(2);
        settings.per_day = None;
    })
    .await;
    registered(&state, "erin", None).await;
    assert_eq!(refused(&state, "frank", None).await, "full");

    // A link stays an admin's when its maker is no admin any more, and what
    // that account invites from then on takes from its quota.
    accounts::set_admin(&db, "root", false).await.unwrap();
    registered(&state, "frank", Some(&link)).await;
    assert_eq!(left(&state, &root).await, 0);
    assert_eq!(
        invite(&state, &root, None).await,
        Err(StatusCode::FORBIDDEN)
    );
}

#[sqlx::test]
async fn invitations_are_their_makers_alone(db: PgPool) {
    let state = AppState::new(db.clone());
    let ada = reader(&db, "ada").await;
    let grace = reader(&db, "grace").await;
    set(&db, |settings| settings.invites_per_account = 2).await;
    let link = invite(&state, &ada, None).await.unwrap();

    // The token is shown once and kept as its hash.
    let stored: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM invites")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(stored.len(), 32);
    assert!(
        !stored
            .windows(link.len())
            .any(|part| part == link.as_bytes())
    );
    let (_, list) = call(&state, Method::GET, "/api/invites", Some(&ada), None).await;
    assert!(!list.to_string().contains(&link));
    let id = list["invites"][0]["id"].as_i64().unwrap();

    // Grace sees none of it, and cannot take it back.
    let (_, theirs) = call(&state, Method::GET, "/api/invites", Some(&grace), None).await;
    assert_eq!(theirs["invites"], json!([]));
    assert_eq!(theirs["left"], 2);
    let path = format!("/api/invites/{id}");
    let (status, _) = call(&state, Method::DELETE, &path, Some(&grace), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    registered(&state, "bob", Some(&link)).await;

    // Without a session there are no invitations to see, make or take back.
    for (method, path) in [
        (Method::GET, "/api/invites"),
        (Method::POST, "/api/invites"),
        (Method::DELETE, path.as_str()),
    ] {
        let (status, _) = call(&state, method, path, None, Some(json!({}))).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    // Who goes takes their links along; the accounts they brought stay.
    sqlx::query("DELETE FROM accounts WHERE username = 'ada'")
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(origin(&db, "bob").await, "invite");
}

#[sqlx::test]
async fn many_at_once_get_no_more_places_than_there_are(db: PgPool) {
    let state = AppState::new(db.clone());
    let root = admin(&db, "root").await;
    set(&db, |settings| {
        settings.open = true;
        settings.max_accounts = Some(4);
    })
    .await;
    let link = invite(&state, &root, Some(2)).await.unwrap();

    // Ten try the open door, ten the link for two.
    let mut tries = Vec::new();
    for n in 0..20 {
        let (db, link) = (db.clone(), link.clone());
        tries.push(tokio::spawn(async move {
            let token = (n % 2 == 0).then_some(link.as_str());
            registration::make(&db, &format!("reader-{n}"), PASSWORD, token)
                .await
                .unwrap()
        }));
    }
    let mut made = 0;
    for outcome in tries {
        match outcome.await.unwrap() {
            Outcome::Made(_) => made += 1,
            Outcome::Refused(Refused::Full | Refused::Invite) => {}
            other => panic!("{other:?}"),
        }
    }
    // Three more in the general pool next to root, two through the link.
    assert_eq!(made, 5);
    let mut conn = db.acquire().await.unwrap();
    let counts = registration::counts(&mut conn).await.unwrap();
    assert_eq!((counts.general, counts.by_admins), (4, 2));
    let uses: i32 = sqlx::query_scalar("SELECT uses FROM invites")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(uses, 2);
}
