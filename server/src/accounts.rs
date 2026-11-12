//! Accounts and their passwords.
//!
//! An account is made on the command line (see `rueder account`), by an admin,
//! or by registering where the operator allows it (see `registration`). A
//! password is kept as its Argon2id hash only, and whoever sets a new one ends
//! every session of the account.

use std::{fmt, sync::LazyLock};

use anyhow::Context;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use sqlx::PgPool;
use time::OffsetDateTime;
use tokio::sync::Semaphore;

/// The longest login name, in characters.
pub const MAX_NAME: usize = 64;
/// The shortest password, in characters, and the longest, in bytes.
pub const MIN_PASSWORD: usize = 8;
pub const MAX_PASSWORD: usize = 1024;

/// Hashing takes 19 MiB and a few dozen milliseconds each time, on purpose.
/// No more than this many run at once, whoever asks.
static HASHING: Semaphore = Semaphore::const_new(4);

/// A hash nobody knows the password of. A login for a name that does not exist,
/// or has no password, is checked against it, so it takes as long as any other
/// and does not tell which names exist.
static NOBODY: LazyLock<Option<String>> = LazyLock::new(|| {
    let mut secret = [0u8; 32];
    getrandom::fill(&mut secret).ok()?;
    Some(Argon2::default().hash_password(&secret).ok()?.to_string())
});

/// Why a name or a password is not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalid {
    NameEmpty,
    NameTooLong,
    NameCharacters,
    NameTaken,
    PasswordTooShort,
    PasswordTooLong,
}

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NameEmpty => write!(f, "the name is empty"),
            Self::NameTooLong => write!(f, "the name is longer than {MAX_NAME} characters"),
            Self::NameCharacters => write!(f, "the name has characters that cannot be typed"),
            Self::NameTaken => write!(f, "there is an account with this name already"),
            Self::PasswordTooShort => {
                write!(f, "the password is shorter than {MIN_PASSWORD} characters")
            }
            Self::PasswordTooLong => {
                write!(f, "the password is longer than {MAX_PASSWORD} bytes")
            }
        }
    }
}

impl std::error::Error for Invalid {}

/// The name as it is stored: without blanks around it.
pub fn check_name(name: &str) -> Result<&str, Invalid> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Invalid::NameEmpty);
    }
    if name.chars().count() > MAX_NAME {
        return Err(Invalid::NameTooLong);
    }
    if name.chars().any(char::is_control) {
        return Err(Invalid::NameCharacters);
    }
    Ok(name)
}

pub fn check_password(password: &str) -> Result<(), Invalid> {
    if password.chars().count() < MIN_PASSWORD {
        return Err(Invalid::PasswordTooShort);
    }
    if password.len() > MAX_PASSWORD {
        return Err(Invalid::PasswordTooLong);
    }
    Ok(())
}

/// The Argon2id hash of a password, as the text that goes into the database.
pub async fn hash(password: &str) -> anyhow::Result<String> {
    let password = password.to_owned();
    let _permit = HASHING.acquire().await?;
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(password.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|err| anyhow::anyhow!("hashing a password: {err}"))
    })
    .await?
}

/// Whether the password is the one the hash was made of. Without a hash the
/// answer is no, after the same work.
pub async fn verify(password: &str, hash: Option<String>) -> anyhow::Result<bool> {
    // Longer than any password that could have been set: no need to hash it.
    if password.len() > MAX_PASSWORD {
        return Ok(false);
    }
    let password = password.to_owned();
    let _permit = HASHING.acquire().await?;
    let matches = tokio::task::spawn_blocking(move || {
        let known = hash.is_some();
        let Some(hash) = hash.or_else(|| NOBODY.clone()) else {
            return false;
        };
        let Ok(parsed) = PasswordHash::new(&hash) else {
            return false;
        };
        let matches = Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok();
        matches && known
    })
    .await?;
    Ok(matches)
}

/// An account, as far as a list of accounts shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: i32,
    pub username: String,
    pub is_admin: bool,
    pub has_password: bool,
    pub created_at: OffsetDateTime,
}

/// Makes an account. Without a password nobody can log in to it with one.
pub async fn create(
    db: &PgPool,
    name: &str,
    password: Option<&str>,
    is_admin: bool,
) -> anyhow::Result<Result<i32, Invalid>> {
    let name = match check_name(name) {
        Ok(name) => name,
        Err(invalid) => return Ok(Err(invalid)),
    };
    let password_hash = match password {
        None => None,
        Some(password) => {
            if let Err(invalid) = check_password(password) {
                return Ok(Err(invalid));
            }
            Some(hash(password).await?)
        }
    };
    let created = sqlx::query_scalar!(
        "INSERT INTO accounts (username, password_hash, is_admin) VALUES ($1, $2, $3) RETURNING id",
        name,
        password_hash,
        is_admin,
    )
    .fetch_one(db)
    .await;
    match created {
        Ok(id) => Ok(Ok(id)),
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => Ok(Err(Invalid::NameTaken)),
        Err(err) => Err(err).context("creating an account"),
    }
}

/// Gives the account a new password and ends its sessions: whoever knew the old
/// one is out. `Ok(Ok(false))` if there is no account of that name.
pub async fn set_password(
    db: &PgPool,
    name: &str,
    password: &str,
) -> anyhow::Result<Result<bool, Invalid>> {
    if let Err(invalid) = check_password(password) {
        return Ok(Err(invalid));
    }
    let password_hash = hash(password).await?;
    let mut tx = db.begin().await?;
    let account_id = sqlx::query_scalar!(
        "UPDATE accounts SET password_hash = $2 WHERE lower(username) = lower($1) RETURNING id",
        name.trim(),
        password_hash,
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(account_id) = account_id else {
        return Ok(Ok(false));
    };
    sqlx::query!("DELETE FROM sessions WHERE account_id = $1", account_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Ok(true))
}

/// Makes an account an admin, or an admin a reader again. False if there is
/// no account of that name. What an admin may do counts from the next request
/// on, as every request reads it anew.
pub async fn set_admin(db: &PgPool, name: &str, is_admin: bool) -> anyhow::Result<bool> {
    let changed = sqlx::query!(
        "UPDATE accounts SET is_admin = $2 WHERE lower(username) = lower($1)",
        name.trim(),
        is_admin,
    )
    .execute(db)
    .await?;
    Ok(changed.rows_affected() > 0)
}

/// The account that name and password belong to, if they do.
pub async fn login(db: &PgPool, name: &str, password: &str) -> anyhow::Result<Option<Account>> {
    let name = name.trim();
    let account = if name.chars().count() > MAX_NAME {
        None
    } else {
        sqlx::query!(
            "SELECT id, username, is_admin, password_hash, created_at
             FROM accounts WHERE lower(username) = lower($1)",
            name
        )
        .fetch_optional(db)
        .await?
    };
    let hash = account
        .as_ref()
        .and_then(|account| account.password_hash.clone());
    if !verify(password, hash).await? {
        return Ok(None);
    }
    Ok(account.map(|account| Account {
        id: account.id,
        username: account.username,
        is_admin: account.is_admin,
        has_password: true,
        created_at: account.created_at,
    }))
}

/// All accounts, by name.
pub async fn list(db: &PgPool) -> anyhow::Result<Vec<Account>> {
    let accounts = sqlx::query_as!(
        Account,
        r#"SELECT id, username, is_admin, password_hash IS NOT NULL AS "has_password!", created_at
           FROM accounts ORDER BY lower(username)"#
    )
    .fetch_all(db)
    .await?;
    Ok(accounts)
}
