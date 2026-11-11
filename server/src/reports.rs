//! Reports of errors in the browsers of readers who agreed to send them.
//!
//! A reader switches it on for their own account (`PUT /api/reports/consent`);
//! nobody else can, and switching it off deletes what the account sent. The
//! browser sends its reports to `POST /api/reports`, to this server and no
//! other. The browser leaves addresses and numbers out of them already; the
//! server does so once more, as it trusts no browser, and keeps only the
//! lines of a stack that name the files of rueder's own frontend. The same
//! error counts up in one row, and a row goes 30 days after it last happened.
//!
//! Admins see the errors on the status page with how many accounts they hit,
//! never which ones ([`summary`]). `CLIENT_REPORTS=off` makes the server take
//! none at all.

use std::{sync::LazyLock, time::Instant};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    routing::{get, post},
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use time::OffsetDateTime;
use ts_rs::TS;

use crate::{AppError, AppState, sessions::Session};

/// The largest body of `POST /api/reports`, in bytes.
pub const MAX_BODY: usize = 32 * 1024;
/// How many reports one request may bring.
pub const MAX_REPORTS: usize = 20;
/// The longest message, in characters.
pub const MAX_MESSAGE: usize = 500;
/// How many lines of a stack are kept, and how long one may be.
const MAX_FRAMES: usize = 10;
const MAX_FRAME: usize = 160;
/// The longest version and name of a browser.
const MAX_LABEL: usize = 40;
/// How many errors the status page shows.
const SHOWN: i64 = 20;

/// The types this module sends and reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types
        .add::<Consent>()
        .add::<ConsentChange>()
        .add::<Reports>();
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/reports",
            post(receive).layer(DefaultBodyLimit::max(MAX_BODY)),
        )
        .route("/reports/consent", get(consent).put(set_consent))
}

/// What `GET /api/reports/consent` answers.
#[derive(Serialize, TS)]
#[ts(rename = "ReportsConsent")]
struct Consent {
    /// Whether this server takes reports at all.
    available: bool,
    /// Whether this account sends them.
    on: bool,
}

/// What `PUT /api/reports/consent` reads. Off deletes what the account sent.
#[derive(Deserialize, TS)]
#[ts(rename = "ReportsConsentChange")]
struct ConsentChange {
    on: bool,
}

/// What kind of error a report is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename = "ReportKind")]
pub enum Kind {
    /// An error nothing caught.
    Error,
    /// A promise that failed and that nothing caught.
    Rejection,
    /// A component of the page that failed to draw.
    Component,
    /// A request to the API that failed.
    Request,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Rejection => "rejection",
            Self::Component => "component",
            Self::Request => "request",
        }
    }

    fn named(name: &str) -> Self {
        match name {
            "rejection" => Self::Rejection,
            "component" => Self::Component,
            "request" => Self::Request,
            _ => Self::Error,
        }
    }
}

/// One error, as the browser saw it.
#[derive(Deserialize, TS)]
#[ts(rename = "ClientReport")]
struct Report {
    kind: Kind,
    message: String,
    /// The lines of the stack in the files of rueder's frontend, as
    /// "_app/immutable/chunks/x.js:12:34", one a line.
    #[serde(default)]
    #[ts(optional, as = "Option<String>")]
    stack: String,
    /// The version of the frontend.
    version: String,
    /// The browser and its major version, such as "Firefox 140".
    browser: String,
}

/// What `POST /api/reports` reads: at most twenty reports. It answers 204.
#[derive(Deserialize, TS)]
#[ts(rename = "ClientReports")]
struct Reports {
    reports: Vec<Report>,
}

async fn consent(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<Consent>, AppError> {
    Ok(Json(Consent {
        available: state.reports,
        on: sends(&state.db, session.account_id).await?,
    }))
}

async fn sends(db: &PgPool, account_id: i32) -> anyhow::Result<bool> {
    let on = sqlx::query_scalar!(
        "SELECT sends_reports FROM accounts WHERE id = $1",
        account_id
    )
    .fetch_optional(db)
    .await?;
    Ok(on.unwrap_or(false))
}

async fn set_consent(
    State(state): State<AppState>,
    session: Session,
    Json(change): Json<ConsentChange>,
) -> Result<StatusCode, AppError> {
    if change.on && !state.reports {
        return Err(not_taken());
    }
    let mut tx = state.db.begin().await?;
    sqlx::query!(
        "UPDATE accounts SET sends_reports = $2 WHERE id = $1",
        session.account_id,
        change.on,
    )
    .execute(&mut *tx)
    .await?;
    if !change.on {
        sqlx::query!(
            "DELETE FROM client_reports WHERE account_id = $1",
            session.account_id
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

fn not_taken() -> AppError {
    let message = "this server takes no reports of errors";
    AppError::Status(StatusCode::NOT_FOUND, message.into())
}

async fn receive(
    State(state): State<AppState>,
    session: Session,
    Json(sent): Json<Reports>,
) -> Result<StatusCode, AppError> {
    if !state.reports {
        return Err(not_taken());
    }
    if sent.reports.len() > MAX_REPORTS {
        let message = format!("at most {MAX_REPORTS} reports at once");
        return Err(AppError::BadRequest(message));
    }
    if !sends(&state.db, session.account_id).await? {
        let message = "this account does not send reports of errors";
        return Err(AppError::Status(StatusCode::FORBIDDEN, message.into()));
    }
    for report in sent.reports {
        state.rates.report(session.account_id, Instant::now())?;
        let report = cleaned(report);
        sqlx::query!(
            "INSERT INTO client_reports (account_id, fingerprint, kind, message, stack, version, browser)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (account_id, fingerprint, version, browser) DO UPDATE
             SET count = least(client_reports.count::bigint + 1, 2147483647)::integer,
                 last_at = now()",
            session.account_id,
            &fingerprint(&report)[..],
            report.kind.name(),
            report.message,
            report.stack,
            report.version,
            report.browser,
        )
        .execute(&state.db)
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Addresses: anything with a scheme, and what looks like a mail address.
static ADDRESS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z][A-Za-z0-9+.-]*://[^\s'\x22)>\]]+|[^\s@'\x22]+@[^\s@'\x22]+\.[A-Za-z]{2,}")
        .expect("a valid pattern")
});
/// Numbers that name something: after a slash, an equals sign, a hash or a
/// colon in a path, and any of four digits or more.
static NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([/=#])\d+|\d{4,}").expect("a valid pattern"));
/// A line of a stack in a file of rueder's own frontend.
static FRAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^_app/[A-Za-z0-9._/-]+\.js:\d{1,7}:\d{1,7}$").expect("a valid pattern")
});

/// A text of a browser as it may be kept: one line, without addresses and
/// numbers that name something, and not too long.
pub fn scrubbed(text: &str, at_most: usize) -> String {
    let line: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let line = line.split_whitespace().collect::<Vec<_>>().join(" ");
    let line = ADDRESS.replace_all(&line, "<address>");
    let line = NUMBER.replace_all(&line, |found: &regex::Captures| match found.get(1) {
        Some(before) => format!("{}<n>", before.as_str()),
        None => "<n>".to_owned(),
    });
    line.chars().take(at_most).collect()
}

/// The lines of a stack that name files of rueder's frontend, at most ten.
pub fn own_frames(stack: &str) -> String {
    stack
        .lines()
        .map(str::trim)
        .filter(|line| line.len() <= MAX_FRAME && FRAME.is_match(line))
        .take(MAX_FRAMES)
        .collect::<Vec<_>>()
        .join("\n")
}

/// A version or a browser: letters, digits, spaces, dots and dashes.
fn label(text: &str) -> String {
    let kept: String = text
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '-' | '_'))
        .take(MAX_LABEL)
        .collect();
    match kept.trim() {
        "" => "unknown".to_owned(),
        kept => kept.to_owned(),
    }
}

fn cleaned(report: Report) -> Report {
    Report {
        kind: report.kind,
        message: scrubbed(&report.message, MAX_MESSAGE),
        stack: own_frames(&report.stack),
        version: label(&report.version),
        browser: label(&report.browser),
    }
}

/// The same error has the same fingerprint, from whichever browser.
fn fingerprint(report: &Report) -> Vec<u8> {
    let first = report.stack.lines().next().unwrap_or_default();
    Sha256::new()
        .chain_update(report.kind.name())
        .chain_update(b"\n")
        .chain_update(&report.message)
        .chain_update(b"\n")
        .chain_update(first)
        .finalize()
        .to_vec()
}

/// Deletes the reports that last happened more than 30 days ago, and says how
/// many.
pub async fn sweep(db: &PgPool) -> anyhow::Result<u64> {
    let done =
        sqlx::query!("DELETE FROM client_reports WHERE last_at < now() - interval '30 days'")
            .execute(db)
            .await?;
    Ok(done.rows_affected())
}

/// The errors in browsers, for the status page.
#[derive(Serialize, TS)]
#[ts(rename = "StatusBrowserErrors")]
pub struct Summary {
    /// Whether the server takes reports at all.
    pub available: bool,
    /// How many accounts send reports.
    pub consenting: i64,
    /// The errors, the one that happened last first. No account is named.
    pub errors: Vec<BrowserError>,
}

#[derive(Serialize, TS)]
#[ts(rename = "StatusBrowserError")]
pub struct BrowserError {
    pub kind: Kind,
    pub message: String,
    pub stack: String,
    /// How often it happened, in all browsers.
    pub count: i64,
    /// In how many accounts.
    pub accounts: i64,
    /// The versions of the frontend and the browsers it happened in.
    pub versions: Vec<String>,
    pub browsers: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub first: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub last: OffsetDateTime,
}

pub async fn summary(db: &PgPool, available: bool) -> anyhow::Result<Summary> {
    let consenting =
        sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM accounts WHERE sends_reports"#)
            .fetch_one(db)
            .await?;
    let rows = sqlx::query!(
        r#"SELECT min(kind) AS "kind!", min(message) AS "message!", min(stack) AS "stack!",
                  sum(count)::bigint AS "count!", count(DISTINCT account_id) AS "accounts!",
                  array_agg(DISTINCT version ORDER BY version) AS "versions!",
                  array_agg(DISTINCT browser ORDER BY browser) AS "browsers!",
                  min(first_at) AS "first!", max(last_at) AS "last!"
           FROM client_reports
           GROUP BY fingerprint
           ORDER BY max(last_at) DESC, fingerprint
           LIMIT $1"#,
        SHOWN,
    )
    .fetch_all(db)
    .await?;
    let errors = rows
        .into_iter()
        .map(|row| BrowserError {
            kind: Kind::named(&row.kind),
            message: row.message,
            stack: row.stack,
            count: row.count,
            accounts: row.accounts,
            versions: row.versions,
            browsers: row.browsers,
            first: row.first,
            last: row.last,
        })
        .collect();
    Ok(Summary {
        available,
        consenting,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::{own_frames, scrubbed};

    #[test]
    fn a_message_keeps_no_address_and_no_number_that_names_something() {
        assert_eq!(
            scrubbed(
                "Failed to fetch https://example.org/feed?id=12 for ada@example.org",
                500
            ),
            "Failed to fetch <address> for <address>"
        );
        assert_eq!(
            scrubbed("GET /feeds/12/articles/3456: 502", 500),
            "GET /feeds/<n>/articles/<n>: 502"
        );
        assert_eq!(
            scrubbed(
                "Cannot read properties of undefined (reading 'x') at 1731340800",
                500
            ),
            "Cannot read properties of undefined (reading 'x') at <n>"
        );
        assert_eq!(
            scrubbed("two\nlines\t and  spaces", 500),
            "two lines and spaces"
        );
        assert_eq!(scrubbed(&"ä".repeat(600), 500).chars().count(), 500);
    }

    #[test]
    fn a_stack_keeps_only_the_files_of_the_frontend() {
        let stack = "_app/immutable/chunks/abc.js:12:34\n\
                     https://evil.example/x.js:1:2\n\
                     _app/immutable/nodes/2.js:1:99\n\
                     at something else\n\
                     _app/../../etc/passwd:1:1";
        assert_eq!(
            own_frames(stack),
            "_app/immutable/chunks/abc.js:12:34\n_app/immutable/nodes/2.js:1:99"
        );
        let many = "_app/a.js:1:1\n".repeat(30);
        assert_eq!(own_frames(&many).lines().count(), 10);
    }
}
