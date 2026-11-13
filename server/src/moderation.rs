//! Reports of feeds and articles by their readers, and blocking a feed.
//!
//! - **Reporting** (`POST /api/feeds/{id}/report`,
//!   `POST /api/feeds/{id}/articles/{seq}/report`) takes a reason from a fixed
//!   list and a short note, from a reader of the feed, within a limit per
//!   account. It is on only where the operator says so
//!   (`CONTENT_REPORTS=on`). An account reports a feed or an article once; a
//!   second report replaces the first.
//! - **The status page** lists the reported feeds with their address and the
//!   number of reports by reason, never who reported ([`reported`]).
//! - **Blocking** is for the operator on the command line (`rueder feed block
//!   <address>`, [`block`]): the feed is not fetched any more, nobody sees its
//!   articles, nobody subscribes to it anew, and its images go from the cache
//!   ([`images_of`]). They are listed in `blocked_images`, so that the proxy
//!   does not fetch them back for a browser that still knows their addresses.
//!   `rueder feed unblock` lifts it ([`unblock`]).

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::OffsetDateTime;
use ts_rs::TS;

use crate::{AppError, AppState, cleaning, images, sessions::Session};

/// The longest note of a report, in characters.
pub const MAX_NOTE: usize = 500;
/// How many reported feeds the status page lists, the latest first.
pub const LISTED: i64 = 50;

/// Why something is reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename = "ReportReason")]
pub enum Reason {
    /// Against the law: hate, violence, abuse of children, terror.
    Illegal,
    /// Somebody else's work without their leave.
    Copyright,
    /// Malware, phishing, fraud.
    Malicious,
    Spam,
    Other,
}

impl Reason {
    fn name(self) -> &'static str {
        match self {
            Self::Illegal => "illegal",
            Self::Copyright => "copyright",
            Self::Malicious => "malicious",
            Self::Spam => "spam",
            Self::Other => "other",
        }
    }

    fn named(name: &str) -> Option<Self> {
        [
            Self::Illegal,
            Self::Copyright,
            Self::Malicious,
            Self::Spam,
            Self::Other,
        ]
        .into_iter()
        .find(|reason| reason.name() == name)
    }
}

/// A report of a feed or an article.
///
/// `POST /api/feeds/{id}/report` and `POST /api/feeds/{id}/articles/{seq}/report`
/// read it and answer 204.
#[derive(Deserialize, TS)]
#[ts(rename = "ContentReport")]
pub struct Report {
    reason: Reason,
    /// What the reader adds, at most 500 characters.
    #[serde(default)]
    #[ts(as = "Option<String>", optional)]
    note: String,
}

/// A feed readers reported, for the status page: its address, never who.
#[derive(Debug, Serialize, TS)]
#[ts(rename = "ReportedFeed")]
pub struct Reported {
    pub feed_id: i32,
    pub url: String,
    pub title: String,
    pub reports: i64,
    /// How many reports gave each reason, the most first.
    pub reasons: Vec<ReasonCount>,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub last_at: OffsetDateTime,
    pub blocked: bool,
}

#[derive(Debug, Serialize, TS)]
#[ts(rename = "ReportReasonCount")]
pub struct ReasonCount {
    pub reason: Reason,
    pub reports: i64,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/feeds/{feed_id}/report", post(report_feed))
        .route(
            "/feeds/{feed_id}/articles/{seq}/report",
            post(report_article),
        )
}

/// The types this module sends and reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types.add::<Report>().add::<Reported>();
}

async fn report_feed(
    State(state): State<AppState>,
    session: Session,
    Path(feed_id): Path<i32>,
    Json(report): Json<Report>,
) -> Result<StatusCode, AppError> {
    file(&state, &session, feed_id, None, report).await
}

async fn report_article(
    State(state): State<AppState>,
    session: Session,
    Path((feed_id, seq)): Path<(i32, i32)>,
    Json(report): Json<Report>,
) -> Result<StatusCode, AppError> {
    file(&state, &session, feed_id, Some(seq), report).await
}

/// The note as it is kept: without control characters but line breaks, and
/// trimmed; `None` if it is too long.
fn note_of(note: &str) -> Option<String> {
    let note: String = note
        .trim()
        .chars()
        .filter(|c| *c == '\n' || !c.is_control())
        .collect();
    (note.chars().count() <= MAX_NOTE).then_some(note)
}

async fn file(
    state: &AppState,
    session: &Session,
    feed_id: i32,
    seq: Option<i32>,
    report: Report,
) -> Result<StatusCode, AppError> {
    if !state.content_reports {
        return Err(AppError::NotFound);
    }
    let Some(note) = note_of(&report.note) else {
        let message = format!("a note has at most {MAX_NOTE} characters");
        return Err(AppError::BadRequest(message));
    };
    // Only a reader of the feed reports it, and only an article it has.
    let known = match seq {
        None => sqlx::query_scalar!(
            r#"SELECT true AS "known!" FROM subscriptions WHERE account_id = $1 AND feed_id = $2"#,
            session.account_id,
            feed_id
        )
        .fetch_optional(&state.db)
        .await?
        .is_some(),
        Some(seq) => sqlx::query_scalar!(
            r#"SELECT true AS "known!" FROM subscriptions s
               JOIN articles a ON a.feed_id = s.feed_id AND a.seq = $3
               WHERE s.account_id = $1 AND s.feed_id = $2"#,
            session.account_id,
            feed_id,
            seq
        )
        .fetch_optional(&state.db)
        .await?
        .is_some(),
    };
    if !known {
        return Err(AppError::NotFound);
    }
    state
        .rates
        .content_report(session.account_id, std::time::Instant::now())?;
    sqlx::query!(
        "INSERT INTO content_reports (account_id, feed_id, seq, reason, note)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (account_id, feed_id, seq)
         DO UPDATE SET reason = EXCLUDED.reason, note = EXCLUDED.note, created_at = now()",
        session.account_id,
        feed_id,
        seq,
        report.reason.name(),
        note
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// The reported feeds, the latest report first, with their address and the
/// number of reports by reason. Nothing says who reported.
pub async fn reported(db: &PgPool) -> anyhow::Result<Vec<Reported>> {
    let rows = sqlx::query!(
        r#"SELECT f.id, f.feed_url, f.title, f.blocked_at IS NOT NULL AS "blocked!",
                  count(*) AS "reports!", max(r.created_at) AS "last_at!",
                  array_agg(r.reason) AS "reasons!"
           FROM content_reports r JOIN feeds f ON f.id = r.feed_id
           GROUP BY f.id
           ORDER BY max(r.created_at) DESC
           LIMIT $1"#,
        LISTED
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let mut reasons: Vec<ReasonCount> = Vec::new();
            for name in &row.reasons {
                let Some(reason) = Reason::named(name) else {
                    continue;
                };
                match reasons.iter_mut().find(|count| count.reason == reason) {
                    Some(count) => count.reports += 1,
                    None => reasons.push(ReasonCount { reason, reports: 1 }),
                }
            }
            reasons.sort_by_key(|count| std::cmp::Reverse(count.reports));
            Reported {
                feed_id: row.id,
                url: row.feed_url,
                title: row.title,
                reports: row.reports,
                reasons,
                last_at: row.last_at,
                blocked: row.blocked,
            }
        })
        .collect())
}

/// Blocks the feeds at this address and says which; none if there is no feed
/// there. A feed blocked already stays as it was blocked. Its images are
/// listed as blocked, so that the image proxy does not fetch them; taking them
/// out of the cache is the caller's, after this.
pub async fn block(db: &PgPool, url: &str) -> anyhow::Result<Vec<i32>> {
    let blocked = sqlx::query_scalar!(
        "UPDATE feeds SET blocked_at = coalesce(blocked_at, now())
         WHERE feed_url = $1 RETURNING id",
        url.trim()
    )
    .fetch_all(db)
    .await?;
    for &feed_id in &blocked {
        let hashes: Vec<Vec<u8>> = images_of(db, feed_id)
            .await?
            .iter()
            .map(|url| images::source_hash(url))
            .collect();
        sqlx::query!(
            "INSERT INTO blocked_images (url_hash, feed_id)
             SELECT h, $2 FROM unnest($1::bytea[]) AS h
             ON CONFLICT DO NOTHING",
            &hashes,
            feed_id,
        )
        .execute(db)
        .await?;
    }
    Ok(blocked)
}

/// Lifts the block of the feeds at this address, which are fetched again at
/// once and whose images the proxy fetches again; how many there were.
pub async fn unblock(db: &PgPool, url: &str) -> anyhow::Result<u64> {
    let mut tx = db.begin().await?;
    let lifted = sqlx::query_scalar!(
        "UPDATE feeds SET blocked_at = NULL, next_fetch_at = now()
         WHERE feed_url = $1 AND blocked_at IS NOT NULL RETURNING id",
        url.trim()
    )
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM blocked_images WHERE feed_id = ANY($1)",
        &lifted
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(lifted.len() as u64)
}

/// How many articles [`images_of`] reads at a time.
const PAGE: i64 = 1000;

/// The addresses of every image of a feed, as they are stored: its icon, the
/// thumbnails and lead images of its articles, and the images in their texts.
pub async fn images_of(db: &PgPool, feed_id: i32) -> anyhow::Result<Vec<String>> {
    let mut found: Vec<String> = sqlx::query_scalar!(
        "SELECT icon_url FROM feeds WHERE id = $1 AND icon_url IS NOT NULL",
        feed_id
    )
    .fetch_all(db)
    .await?
    .into_iter()
    .flatten()
    .collect();
    let mut after = 0;
    loop {
        let page = sqlx::query!(
            "SELECT a.seq, a.thumbnail_url, c.image_url, c.html
             FROM articles a JOIN article_content c ON c.feed_id = a.feed_id AND c.seq = a.seq
             WHERE a.feed_id = $1 AND a.seq > $2
             ORDER BY a.seq LIMIT $3",
            feed_id,
            after,
            PAGE
        )
        .fetch_all(db)
        .await?;
        let Some(last) = page.last() else {
            break;
        };
        after = last.seq;
        for article in page {
            found.extend(article.thumbnail_url);
            found.extend(article.image_url);
            found.extend(cleaning::images_in(&article.html));
        }
    }
    found.sort();
    found.dedup();
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_kept_short_and_plain() {
        assert_eq!(
            note_of("  ads\u{0}\nand\tmore "),
            Some("ads\nandmore".to_owned())
        );
        assert_eq!(
            note_of(&"ä".repeat(MAX_NOTE)).map(|n| n.chars().count()),
            Some(MAX_NOTE)
        );
        assert_eq!(note_of(&"ä".repeat(MAX_NOTE + 1)), None);
        for reason in [
            Reason::Illegal,
            Reason::Copyright,
            Reason::Malicious,
            Reason::Spam,
            Reason::Other,
        ] {
            assert_eq!(Reason::named(reason.name()), Some(reason));
        }
        assert_eq!(Reason::named("nonsense"), None);
    }
}
