//! Cleans the text of articles again, from the versions as they were fetched.
//!
//! The cleaning has rules and lists that get better over time. Because every
//! version is kept as it came, a better cleaning can be had for old articles
//! too: this reads the current version of each article, cleans it with the
//! rules of today and puts the result in place. Versions are not touched, and
//! nothing about what anybody has read changes.

use std::{fmt, time::Instant};

use sqlx::PgPool;

use crate::{
    articles::{self, Original},
    cleaning::{self, Base, Rules},
    feeds::add_inline,
};

/// Articles read and written in one go.
const BATCH: i32 = 1000;

/// What a run did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    /// Articles looked at.
    pub articles: u64,
    /// Articles whose text or enclosures came out differently.
    pub changed: u64,
    /// Articles whose version could not be read. They stay as they are.
    pub unreadable: u64,
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} articles cleaned again, {} of them came out differently",
            self.articles, self.changed
        )?;
        if self.unreadable > 0 {
            write!(
                f,
                "; {} have a version that cannot be read and stay as they are",
                self.unreadable
            )?;
        }
        Ok(())
    }
}

/// Cleans the articles of all feeds again, or of one.
pub async fn run(db: &PgPool, rules: &Rules, feed: Option<i32>) -> anyhow::Result<Report> {
    let started = Instant::now();
    let mut report = Report::default();
    let feeds = sqlx::query!(
        "SELECT id, feed_url, last_seq FROM feeds WHERE $1::integer IS NULL OR id = $1 ORDER BY id",
        feed
    )
    .fetch_all(db)
    .await?;
    for feed in feeds {
        let mut after = 0;
        while after < feed.last_seq {
            let upto = after.saturating_add(BATCH);
            // The versions lie in a table per month. Asking for a stretch of
            // numbers looks into each of them once, not once per article.
            let rows = sqlx::query!(
                "SELECT c.seq, c.html, c.checksum, c.enclosures, v.raw
                 FROM article_content c
                 JOIN articles a ON a.feed_id = c.feed_id AND a.seq = c.seq
                 JOIN article_versions v
                     ON v.feed_id = a.feed_id AND v.seq = a.seq AND v.version = a.version
                 WHERE c.feed_id = $1 AND c.seq > $2 AND c.seq <= $3
                   AND v.feed_id = $1 AND v.seq > $2 AND v.seq <= $3
                 ORDER BY c.seq",
                feed.id,
                after,
                upto,
            )
            .fetch_all(db)
            .await?;
            after = upto;

            let (mut seqs, mut texts, mut checksums, mut all_enclosures) =
                (Vec::new(), Vec::new(), Vec::new(), Vec::new());
            for row in rows {
                report.articles += 1;
                let Ok(original) = Original::decompress(&row.raw) else {
                    report.unreadable += 1;
                    continue;
                };
                let base = Base {
                    article: original.url.as_deref(),
                    feed: Some(&feed.feed_url),
                };
                let cleaned = cleaning::clean(&original.content, base, rules);
                // What the feed itself lists stays; what stood in the text is
                // whatever stands there now.
                let mut enclosures = row.enclosures.as_array().cloned().unwrap_or_default();
                enclosures.retain(|enclosure| enclosure["inline"] != true);
                add_inline(&mut enclosures, cleaned.enclosures);
                let enclosures = serde_json::Value::from(enclosures);
                let checksum = articles::checksum(&original.title, &cleaned.html);
                let same = cleaned.html == row.html && checksum == row.checksum;
                if same && enclosures == row.enclosures {
                    continue;
                }
                seqs.push(row.seq);
                texts.push(cleaned.html);
                checksums.push(checksum);
                all_enclosures.push(enclosures.to_string());
            }
            if seqs.is_empty() {
                continue;
            }
            sqlx::query!(
                "UPDATE article_content c
                 SET html = n.html, checksum = n.checksum, enclosures = n.enclosures::jsonb
                 FROM unnest($2::integer[], $3::text[], $4::bytea[], $5::text[])
                      AS n (seq, html, checksum, enclosures)
                 WHERE c.feed_id = $1 AND c.seq = n.seq",
                feed.id,
                &seqs,
                &texts,
                &checksums,
                &all_enclosures,
            )
            .execute(db)
            .await?;
            report.changed += seqs.len() as u64;
        }
        tracing::info!(
            "feed {}: {} articles so far, {} changed, after {} s",
            feed.id,
            report.articles,
            report.changed,
            started.elapsed().as_secs()
        );
    }
    Ok(report)
}
