//! Reports of what feeds bring, and feeds blocked for everybody.
//!
//! A reader reports a feed they subscribe to, or one of its articles, with a
//! reason from a fixed list and a short text (`POST /api/feeds/{id}/report`,
//! `POST /api/feeds/{id}/articles/{seq}/report`). One account has one report
//! per feed or article; reporting again replaces it. How many an account may
//! send is limited like the reports of errors ([`crate::rate::CONTENT_REPORTS`])
//! and how many it may have at once too ([`MAX_PER_ACCOUNT`]).
//!
//! Admins see on the status page the address of every reported feed, how
//! often and by how many accounts it was reported, the reasons and the latest
//! texts, never who reported ([`summary`]). That a feed is named there at all
//! is the one exception to the status page naming no feeds: an admin cannot
//! judge a report without knowing what it is about.
//!
//! Blocking is for the operator, on the command line (`rueder feed block`,
//! [`block`]): a blocked feed is not fetched, its articles are shown to nobody,
//! nobody can subscribe to it anew, and its images leave the cache of the image
//! proxy, which does not fetch them again. The subscriptions stay, so that
//! `rueder feed unblock` ([`unblock`]) brings everything back as it was.

use std::{collections::HashSet, path::Path, time::Instant};

use anyhow::Context;
use axum::{
    Json, Router,
    extract::{Path as UrlPath, State},
    http::StatusCode,
    routing::post,
};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::OffsetDateTime;
use ts_rs::TS;

use crate::{
    AppError, AppState, cleaning,
    images::{self, Cache},
    sessions::Session,
    subscribe,
};

/// The longest text of a report, in characters.
pub const MAX_TEXT: usize = 500;
/// How many reports one account may have at once. Reports go when an admin
/// dismisses them (`rueder feed dismiss`).
pub const MAX_PER_ACCOUNT: i64 = 200;
/// How many reported feeds the status page shows, and how many texts of each.
const SHOWN: i64 = 50;
const SHOWN_TEXTS: usize = 5;

/// The types this module sends and reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types.add::<NewReport>();
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/feeds/{feed_id}/report", post(report_feed))
        .route(
            "/feeds/{feed_id}/articles/{seq}/report",
            post(report_article),
        )
}

/// Why a reader reports a feed or an article.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename = "ReportReason")]
pub enum Reason {
    /// Advertising or other unwanted mass content.
    Spam,
    /// Content that is against the law.
    Illegal,
    /// Hatred, harassment or threats.
    Hate,
    /// Somebody else's work without their leave.
    Copyright,
    /// Software that does harm, or a page that fishes for passwords.
    Malware,
    /// Anything else; the text says what.
    Other,
}

impl Reason {
    const ALL: [Self; 6] = [
        Self::Spam,
        Self::Illegal,
        Self::Hate,
        Self::Copyright,
        Self::Malware,
        Self::Other,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Spam => "spam",
            Self::Illegal => "illegal",
            Self::Hate => "hate",
            Self::Copyright => "copyright",
            Self::Malware => "malware",
            Self::Other => "other",
        }
    }

    #[cfg(test)]
    fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|reason| reason.name() == name)
    }
}

/// What `POST /api/feeds/{id}/report` and
/// `POST /api/feeds/{id}/articles/{seq}/report` read. They answer 204.
#[derive(Deserialize, TS)]
#[ts(rename = "NewReport")]
struct NewReport {
    reason: Reason,
    /// At most 500 characters; it may be left out.
    #[serde(default)]
    #[ts(optional, as = "Option<String>")]
    text: String,
}

async fn report_feed(
    State(state): State<AppState>,
    session: Session,
    UrlPath(feed_id): UrlPath<i32>,
    Json(report): Json<NewReport>,
) -> Result<StatusCode, AppError> {
    receive(&state, &session, feed_id, None, report).await
}

async fn report_article(
    State(state): State<AppState>,
    session: Session,
    UrlPath((feed_id, seq)): UrlPath<(i32, i32)>,
    Json(report): Json<NewReport>,
) -> Result<StatusCode, AppError> {
    receive(&state, &session, feed_id, Some(seq), report).await
}

/// The text of a report as it is kept: without control characters, with its
/// spaces at the ends cut.
fn cleaned_text(text: &str) -> String {
    let kept: String = text
        .chars()
        .map(|c| if c.is_control() && c != '\n' { ' ' } else { c })
        .collect();
    kept.trim().to_owned()
}

async fn receive(
    state: &AppState,
    session: &Session,
    feed_id: i32,
    seq: Option<i32>,
    report: NewReport,
) -> Result<StatusCode, AppError> {
    let account_id = session.account_id;
    let text = cleaned_text(&report.text);
    if text.chars().count() > MAX_TEXT {
        let message = format!("a report has at most {MAX_TEXT} characters");
        return Err(AppError::BadRequest(message));
    }
    // Only what the account reads, and of a blocked feed no article: nobody
    // sees those.
    let readable = sqlx::query_scalar!(
        r#"SELECT f.blocked_at IS NULL AS "open!"
           FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
           WHERE s.account_id = $1 AND s.feed_id = $2"#,
        account_id,
        feed_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    if let Some(seq) = seq {
        let there = sqlx::query_scalar!(
            "SELECT seq FROM articles WHERE feed_id = $1 AND seq = $2",
            feed_id,
            seq
        )
        .fetch_optional(&state.db)
        .await?;
        if there.is_none() || !readable {
            return Err(AppError::NotFound);
        }
    }
    state.rates.content_report(account_id, Instant::now())?;
    let mut tx = state.db.begin().await?;
    // One at a time per account, so that two at once do not pass the limit
    // together.
    sqlx::query!(
        "SELECT id FROM accounts WHERE id = $1 FOR UPDATE",
        account_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let others = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM feed_reports
           WHERE account_id = $1 AND NOT (feed_id = $2 AND seq IS NOT DISTINCT FROM $3)"#,
        account_id,
        feed_id,
        seq,
    )
    .fetch_one(&mut *tx)
    .await?;
    if others >= MAX_PER_ACCOUNT {
        let message = format!("an account has at most {MAX_PER_ACCOUNT} reports at once");
        return Err(AppError::Status(StatusCode::FORBIDDEN, message));
    }
    sqlx::query!(
        "INSERT INTO feed_reports (account_id, feed_id, seq, reason, text)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (account_id, feed_id, seq) DO UPDATE
         SET reason = excluded.reason, text = excluded.text, created_at = now()",
        account_id,
        feed_id,
        seq,
        report.reason.name(),
        text,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The reported feeds, for the status page. No account is named.
#[derive(Serialize, TS)]
#[ts(rename = "StatusReports")]
pub struct Summary {
    /// The feeds with reports, the one reported last first.
    pub feeds: Vec<ReportedFeed>,
    /// How many addresses are blocked.
    pub blocked: i64,
}

#[derive(Serialize, TS)]
#[ts(rename = "StatusReportedFeed")]
pub struct ReportedFeed {
    /// The address of the feed, which is what `rueder feed block` takes.
    pub feed_url: String,
    /// How many reports there are, of the feed and of its articles.
    pub reports: i64,
    /// From how many accounts.
    pub accounts: i64,
    /// How many of them are about an article and not the whole feed.
    pub articles: i64,
    /// How many reports give each reason, the most given first.
    pub reasons: Vec<ReasonCount>,
    /// The latest texts that are not empty, at most five, the latest first.
    pub texts: Vec<String>,
    pub blocked: bool,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub last: OffsetDateTime,
}

#[derive(Serialize, TS)]
#[ts(rename = "StatusReportReason")]
pub struct ReasonCount {
    pub reason: Reason,
    pub count: i64,
}

pub async fn summary(db: &PgPool) -> anyhow::Result<Summary> {
    let rows = sqlx::query!(
        r#"SELECT f.feed_url, bool_or(f.blocked_at IS NOT NULL) AS "blocked!",
                  count(*) AS "reports!", count(DISTINCT r.account_id) AS "accounts!",
                  count(r.seq) AS "articles!", max(r.created_at) AS "last!",
                  array_agg(r.reason) AS "reasons!",
                  coalesce(array_agg(r.text ORDER BY r.created_at DESC, r.id DESC)
                           FILTER (WHERE r.text <> ''), '{}') AS "texts!"
           FROM feed_reports r JOIN feeds f ON f.id = r.feed_id
           GROUP BY f.feed_url
           ORDER BY max(r.created_at) DESC, f.feed_url
           LIMIT $1"#,
        SHOWN,
    )
    .fetch_all(db)
    .await?;
    let blocked = sqlx::query_scalar!(
        r#"SELECT count(DISTINCT feed_url) AS "n!" FROM feeds WHERE blocked_at IS NOT NULL"#
    )
    .fetch_one(db)
    .await?;
    let feeds = rows
        .into_iter()
        .map(|row| {
            let mut reasons: Vec<ReasonCount> = Reason::ALL
                .into_iter()
                .map(|reason| ReasonCount {
                    reason,
                    count: row.reasons.iter().filter(|r| *r == reason.name()).count() as i64,
                })
                .filter(|counted| counted.count > 0)
                .collect();
            reasons.sort_by_key(|counted| std::cmp::Reverse(counted.count));
            let mut texts = row.texts;
            texts.truncate(SHOWN_TEXTS);
            ReportedFeed {
                feed_url: row.feed_url,
                reports: row.reports,
                accounts: row.accounts,
                articles: row.articles,
                reasons,
                texts,
                blocked: row.blocked,
                last: row.last,
            }
        })
        .collect();
    Ok(Summary { feeds, blocked })
}

/// How many reported feeds wait for an admin: reported and not blocked.
pub fn waiting(summary: &Summary) -> usize {
    summary.feeds.iter().filter(|feed| !feed.blocked).count()
}

/// The ways an address may be written down as a feed's: as it is given, and
/// as subscribing writes it, which comes last.
fn spellings(address: &str) -> anyhow::Result<Vec<String>> {
    let given = address.trim().to_owned();
    if given.is_empty() {
        anyhow::bail!("the address is empty");
    }
    let mut spellings = vec![given];
    // What the old rueder kept may be no address subscribing takes.
    if let Ok(url) = subscribe::address(&spellings[0])
        && spellings[0] != url.as_str()
    {
        spellings.push(url.to_string());
    }
    Ok(spellings)
}

/// What [`block`] did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Blocked {
    /// The feeds at the address that are blocked now; more than one where an
    /// import brought the same address twice.
    pub feeds: usize,
    /// No feed had the address: one was made, blocked, so that nobody
    /// subscribes to it.
    pub made: bool,
    /// How many images of the feeds are listed as blocked.
    pub images: usize,
    /// How many files left the cache of the image proxy.
    pub removed: usize,
    /// How many accounts subscribe to the feeds.
    pub subscribers: i64,
}

impl std::fmt::Display for Blocked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.made {
            write!(f, "No feed had this address; it is blocked from now on.")?;
        } else {
            write!(
                f,
                "Blocked {} feed(s) with {} subscriber(s). {} image(s) are blocked, \
                 {} file(s) left the image cache.",
                self.feeds, self.subscribers, self.images, self.removed
            )?;
        }
        Ok(())
    }
}

/// Blocks the feeds at an address for everybody: they are no longer fetched,
/// their articles are shown to nobody, nobody can subscribe to them, and their
/// images leave `cache`, if there is one, and are not fetched again. Blocking
/// what is blocked already lists its images again.
pub async fn block(db: &PgPool, address: &str, cache: Option<&Cache>) -> anyhow::Result<Blocked> {
    let spellings = spellings(address)?;
    let mut tx = db.begin().await?;
    let mut ids = sqlx::query_scalar!(
        "UPDATE feeds SET blocked_at = coalesce(blocked_at, now())
         WHERE feed_url = ANY($1) RETURNING id",
        &spellings,
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut done = Blocked::default();
    if ids.is_empty() {
        let url = spellings.last().context("an address has a spelling")?;
        let id = sqlx::query_scalar!(
            "INSERT INTO feeds (feed_url, blocked_at) VALUES ($1, now()) RETURNING id",
            url,
        )
        .fetch_one(&mut *tx)
        .await?;
        ids.push(id);
        done.made = true;
    }
    done.feeds = ids.len();
    done.subscribers = sqlx::query_scalar!(
        r#"SELECT count(DISTINCT account_id) AS "n!" FROM subscriptions WHERE feed_id = ANY($1)"#,
        &ids,
    )
    .fetch_one(&mut *tx)
    .await?;
    let mut sources = HashSet::new();
    for &id in &ids {
        let found = images_of(&mut tx, id).await?;
        let hashes: Vec<Vec<u8>> = found.iter().map(|url| images::source_hash(url)).collect();
        sqlx::query!(
            "INSERT INTO blocked_images (url_hash, feed_id)
             SELECT h, $2 FROM unnest($1::bytea[]) AS h
             ON CONFLICT DO NOTHING",
            &hashes,
            id,
        )
        .execute(&mut *tx)
        .await?;
        sources.extend(found);
    }
    tx.commit().await?;
    done.images = sources.len();
    // Only now: until the block holds, the proxy could fetch them again.
    if let Some(cache) = cache {
        done.removed = sources.iter().map(|url| cache.remove(url)).sum();
    }
    Ok(done)
}

/// Every image a feed has delivered: its icon, the pictures of its articles
/// and the images in their texts. The images of older versions of an article
/// are not looked at, as those versions are cleaned only when they are shown.
async fn images_of(conn: &mut sqlx::PgConnection, feed_id: i32) -> anyhow::Result<HashSet<String>> {
    let mut found = HashSet::new();
    let icon = sqlx::query_scalar!("SELECT icon_url FROM feeds WHERE id = $1", feed_id)
        .fetch_one(&mut *conn)
        .await?;
    found.extend(icon);
    let mut rows = sqlx::query!(
        "SELECT a.thumbnail_url, c.image_url, c.html
         FROM articles a LEFT JOIN article_content c ON c.feed_id = a.feed_id AND c.seq = a.seq
         WHERE a.feed_id = $1",
        feed_id
    )
    .fetch(&mut *conn);
    while let Some(row) = rows.try_next().await? {
        found.extend(row.thumbnail_url);
        found.extend(row.image_url);
        if let Some(html) = row.html {
            found.extend(cleaning::image_sources(&html));
        }
    }
    Ok(found)
}

/// Lifts the block of the feeds at an address: they are fetched again at
/// once, and their articles and images are there again. Says how many feeds
/// were blocked.
pub async fn unblock(db: &PgPool, address: &str) -> anyhow::Result<usize> {
    let spellings = spellings(address)?;
    let mut tx = db.begin().await?;
    let ids = sqlx::query_scalar!(
        "UPDATE feeds SET blocked_at = NULL, next_fetch_at = now()
         WHERE feed_url = ANY($1) AND blocked_at IS NOT NULL RETURNING id",
        &spellings,
    )
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query!("DELETE FROM blocked_images WHERE feed_id = ANY($1)", &ids)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(ids.len())
}

/// Deletes the reports of the feeds at an address and of their articles, once
/// an admin has dealt with them. Says how many there were.
pub async fn dismiss(db: &PgPool, address: &str) -> anyhow::Result<u64> {
    let spellings = spellings(address)?;
    let done = sqlx::query!(
        "DELETE FROM feed_reports
         WHERE feed_id IN (SELECT id FROM feeds WHERE feed_url = ANY($1))",
        &spellings,
    )
    .execute(db)
    .await?;
    Ok(done.rows_affected())
}

/// The cache of the image proxy for the command line, if there is one at
/// `dir`; the server makes it, and a block does not need to.
pub fn cache_at(dir: &Path) -> anyhow::Result<Option<Cache>> {
    if !dir.is_dir() {
        return Ok(None);
    }
    Ok(Some(Cache::open(dir, u64::MAX)?))
}

#[cfg(test)]
mod tests {
    use super::{Reason, cleaned_text};

    #[test]
    fn every_reason_has_its_name() {
        for reason in Reason::ALL {
            assert_eq!(Reason::named(reason.name()), Some(reason));
        }
        assert_eq!(Reason::named("nonsense"), None);
    }

    #[test]
    fn a_text_loses_its_control_characters() {
        assert_eq!(cleaned_text("  a\u{0}b\nc\u{1b}[31m "), "a b\nc [31m");
    }
}
