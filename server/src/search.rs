//! The search over the articles of the feeds an account subscribes to.
//!
//! An index over `article_content` holds the words of every article's current
//! version: title, author, tags and text (`migrations/0004_search.sql`). A
//! search asks for articles that have all the words, or words that begin with
//! them, newest first, and pages through them with the same anchor as the lists.
//!
//! Words are taken as PostgreSQL's parser splits them, without stems: "wahl"
//! finds "Wahl", "Wahlen" and "Wahlkampf". Words of one or two letters are
//! only found whole, as there would be too many beginning with them.
//!
//! There are two ways to find the newest hits, and which one is quicker
//! depends on how common the words are:
//!
//! - Looking at the articles themselves, newest first, and working out the
//!   words of each. That costs the same for every article looked at, so it is
//!   quick when every few articles is a hit, and slow for a rare word.
//! - Asking the index. It knows which articles have a word, not in which order
//!   they came, and reading what it holds for a word costs as much as there
//!   are articles with it, in all feeds and all years. That is quick for a
//!   rare word and slow for a common one. So the index is asked in windows of
//!   the order of arrival, first the newest 20,000 articles, then twice as
//!   many before them, and so on, and only the hits inside a window are read.
//!
//! PostgreSQL's planner can choose between the two, but guesses badly how
//! often a word begins other words, and was measured to pick the slow way for
//! common words as often as not (`docs/implementation.md`, "Messung"). So the
//! search chooses itself: it starts by looking at the newest few hundred
//! articles, which shows how common the words are among them, and goes on with
//! whichever way its costs say is quicker. Each step is bounded, and after half
//! a second a search stops and says where to go on.

use std::time::{Duration, Instant};

use sqlx::PgPool;

use crate::articles::ListItem;

/// The longest search, in characters, and the most words in it.
pub const MAX_CHARS: usize = 200;
pub const MAX_WORDS: usize = 10;
/// After this long a search stops at the end of a step and says where to go
/// on, so that no request holds a connection for long.
pub const BUDGET: Duration = Duration::from_millis(500);
/// A step that takes longer than this is given up.
pub const TIMEOUT: Duration = Duration::from_secs(3);

/// How many articles the first look at the newest ones takes in, and the most
/// that one step looks at.
const PROBE: i64 = 500;
const MOST_LOOKED_AT: i64 = 4000;
/// How far back in the order of arrival a step that looks at the articles
/// goes at first, and at the most, when the feeds of the search are few of
/// all and their articles lie far apart.
const FIRST_SPAN: i64 = 20_000;
const MOST_SPAN: i64 = 320_000;
/// How many of the newest articles the first window of the index holds.
const FIRST_WINDOW: i64 = 20_000;
/// What reading one article from the index costs, in microseconds, measured
/// with 10 million made-up articles (`docs/implementation.md`, "Messung").
/// What it costs to look at an article is measured by every search itself, as
/// it depends on how long the texts are and how far apart the articles of the
/// search lie.
const INDEX_HIT_COST: f64 = 1.3;
/// The index is not asked if one window is expected to take longer than this,
/// in microseconds: looking at the articles goes on instead, in steps that
/// each stay short.
const MOST_INDEX_COST: f64 = 1_000_000.0;

/// Which part of the hits to read.
#[derive(Debug, Clone, Copy)]
pub struct Page {
    /// Only articles that arrived before this one (`ListItem::id`, or `next`
    /// of the page before); `None` starts at the newest.
    pub before: Option<i64>,
    pub limit: i64,
    /// Leave out what the account has read.
    pub unread_only: bool,
}

/// A page of hits, newest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hits {
    pub articles: Vec<ListItem>,
    /// Where the next page begins (its `before`); `None` at the oldest
    /// article. A page may have fewer hits than asked for, even none, and
    /// still go on: then the search ran out of time.
    pub next: Option<i64>,
}

/// What came of a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Found(Hits),
    /// The words were too many or too long, or there was no word in them.
    Unusable(&'static str),
    /// A step took longer than [`TIMEOUT`].
    TooSlow,
}

/// The query PostgreSQL searches with: every word, and from three letters on
/// every word beginning with it. `None` if there is no word in the text.
pub async fn query(db: &PgPool, words: &str) -> anyhow::Result<Option<String>> {
    let terms = terms(db, words).await?;
    Ok((!terms.is_empty()).then(|| terms.join(" & ")))
}

/// The words of the query one by one, each as a query of its own.
async fn terms(db: &PgPool, words: &str) -> anyhow::Result<Vec<String>> {
    let terms = sqlx::query_scalar!(
        r#"SELECT '''' || replace(replace(w, '\', '\\'), '''', '''''') || ''''
                  || CASE WHEN char_length(w) >= 3 THEN ':*' ELSE '' END AS "term!"
           FROM unnest(tsvector_to_array(to_tsvector('simple', $1))) w
           ORDER BY w"#,
        words
    )
    .fetch_all(db)
    .await?;
    Ok(terms)
}

/// What the steps so far have seen of the articles they looked at.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Seen {
    /// Articles of the search looked at, and how many of them were hits.
    looked_at: i64,
    hits: i64,
    /// How many had each word of the search.
    with_word: [i64; MAX_WORDS],
    /// How far back in the order of arrival that took.
    span: i64,
    /// How long it took, in microseconds.
    micros: f64,
}

/// What to do next.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Next {
    /// Look at this many articles, going back at most this far.
    LookAt { articles: i64, span: i64 },
    /// Ask the index from now on, for this word of the search: the rarest.
    /// The others are looked for in what it finds.
    AskIndex { word: usize },
}

/// The next step after what was seen, for `wanted` more hits of a search for
/// `words` words among `total` articles.
fn next_step(seen: Seen, words: usize, wanted: i64, total: i64) -> Next {
    // The word the fewest articles had: the index has to read all of them.
    let word = (0..words.clamp(1, MAX_WORDS))
        .min_by_key(|&w| seen.with_word[w])
        .unwrap_or(0);
    if seen.looked_at < PROBE && seen.span < MOST_SPAN {
        // Too few to tell yet: look further back.
        let span = (seen.span.max(FIRST_SPAN / 2) * 2).min(MOST_SPAN);
        return Next::LookAt {
            articles: PROBE,
            span,
        };
    }
    if seen.looked_at == 0 {
        // The feeds of the search have nothing for long stretches.
        return Next::AskIndex { word };
    }
    let share = seen.hits as f64 / seen.looked_at as f64;
    let rarest = seen.with_word[word] as f64 / seen.looked_at as f64;
    // What one article costs to look at, stepping past the others included.
    let per_article = seen.micros / seen.looked_at as f64;
    let to_look_at = wanted as f64 / share;
    let looking = to_look_at * per_article;
    let asking = rarest * total as f64 * INDEX_HIT_COST;
    if asking < looking && asking < MOST_INDEX_COST {
        return Next::AskIndex { word };
    }
    // The articles of the search among all, to know how far back to go.
    let density = seen.looked_at as f64 / seen.span.max(1) as f64;
    let articles = ((to_look_at * 1.5).ceil() as i64).clamp(PROBE, MOST_LOOKED_AT);
    let span = ((articles as f64 / density * 1.5).ceil() as i64).clamp(FIRST_SPAN, MOST_SPAN);
    Next::LookAt { articles, span }
}

/// An article found, before it is read for the list.
#[derive(Debug, Clone, Copy)]
struct Key {
    feed_id: i32,
    seq: i32,
    id: i64,
}

/// A page of the articles of these feeds that have the words, as the account
/// sees them: only feeds it subscribes to, without what its rules hid, each
/// article with whether it is read and starred.
pub async fn search(
    db: &PgPool,
    account_id: i32,
    feed_ids: &[i32],
    words: &str,
    page: Page,
) -> anyhow::Result<Outcome> {
    if words.chars().count() > MAX_CHARS {
        return Ok(Outcome::Unusable("the search is too long"));
    }
    let terms = terms(db, words).await?;
    if terms.is_empty() {
        return Ok(Outcome::Unusable("there is no word to search for"));
    }
    if terms.len() > MAX_WORDS {
        return Ok(Outcome::Unusable("the search has too many words"));
    }
    let query = terms.join(" & ");
    let started = Instant::now();

    let mut tx = db.begin().await?;
    // A plan of its own for every step, made with the words and the bounds in
    // it. Reading all of the table is never the quicker way.
    sqlx::query!(
        "SELECT set_config('plan_cache_mode', 'force_custom_plan', true) AS plan,
                set_config('enable_seqscan', 'off', true) AS seq,
                set_config('statement_timeout', $1, true) AS timeout",
        TIMEOUT.as_millis().to_string()
    )
    .fetch_one(&mut *tx)
    .await?;
    let bounds = sqlx::query!("SELECT min(id) AS first, max(id) AS last FROM article_content")
        .fetch_one(&mut *tx)
        .await?;
    let (Some(first), Some(last)) = (bounds.first, bounds.last) else {
        return Ok(Outcome::Found(Hits {
            articles: Vec::new(),
            next: None,
        }));
    };
    let total = last - first + 1;

    let mut found: Vec<Key> = Vec::new();
    let mut upto = page.before.unwrap_or(i64::MAX).min(last + 1);
    let mut seen = Seen::default();
    let mut next = next_step(seen, terms.len(), page.limit, total);
    let mut width = FIRST_WINDOW;
    let more = loop {
        if upto <= first {
            break None;
        }
        let wanted = page.limit - found.len() as i64;
        let step = Instant::now();
        let outcome = match next {
            Next::LookAt { articles, span } => {
                let from = upto.saturating_sub(span).max(first);
                let looked = look_at(
                    &mut tx, account_id, feed_ids, &query, &terms, from, upto, articles, page,
                )
                .await;
                looked.map(|looked| {
                    let complete = looked.len() as i64 == articles;
                    let oldest = looked.last().map(|article| article.key.id);
                    let hits: Vec<Key> = looked
                        .iter()
                        .filter(|article| article.hit)
                        .map(|article| article.key)
                        .collect();
                    for article in &looked {
                        for (count, has) in seen.with_word.iter_mut().zip(&article.with_word) {
                            *count += i64::from(*has);
                        }
                    }
                    seen.looked_at += looked.len() as i64;
                    seen.hits += hits.len() as i64;
                    // Where the step ended: after the oldest article it looked
                    // at if it looked at all it should, else at its span.
                    let ended = match (complete, oldest) {
                        (true, Some(oldest)) => oldest,
                        _ => from,
                    };
                    seen.span += upto - ended;
                    (hits, ended)
                })
            }
            Next::AskIndex { word } => {
                let from = upto.saturating_sub(width).max(first);
                width = width.saturating_mul(2);
                let asked = Asked {
                    term: &terms[word.min(terms.len() - 1)],
                    query: (terms.len() > 1).then_some(query.as_str()),
                };
                ask_index(
                    &mut tx, account_id, feed_ids, asked, from, upto, wanted, page,
                )
                .await
                .map(|hits| (hits, from))
            }
        };
        let (hits, ended) = match outcome {
            Ok(outcome) => outcome,
            Err(sqlx::Error::Database(err)) if err.code().as_deref() == Some("57014") => {
                // 57014: canceled, here by the statement timeout.
                return Ok(Outcome::TooSlow);
            }
            Err(err) => return Err(err.into()),
        };
        if let Next::LookAt { .. } = next {
            seen.micros += step.elapsed().as_secs_f64() * 1e6;
        }
        if hits.len() as i64 >= wanted {
            found.extend(hits.into_iter().take(wanted as usize));
            break found.last().map(|key| key.id);
        }
        found.extend(hits);
        upto = ended;
        if let Next::LookAt { .. } = next {
            next = next_step(seen, terms.len(), page.limit - found.len() as i64, total);
            if let Next::AskIndex { .. } = next {
                // From here on the index; the planner must not walk through a
                // window article by article, which is what was just decided
                // against.
                sqlx::query!("SELECT set_config('enable_indexscan', 'off', true) AS scan")
                    .fetch_one(&mut *tx)
                    .await?;
            }
        }
        if upto > first && started.elapsed() > BUDGET {
            break Some(upto);
        }
    };
    let articles = read(&mut tx, account_id, &found).await?;
    tx.commit().await?;
    Ok(Outcome::Found(Hits {
        articles,
        next: more,
    }))
}

/// An article looked at.
struct Looked {
    key: Key,
    /// Whether it has all the words, and which of them.
    hit: bool,
    with_word: Vec<bool>,
}

/// The newest articles of the search from `from` up to before `upto` in the
/// order of arrival, at most `limit`, newest first, each with whether it has
/// the words.
#[allow(clippy::too_many_arguments)]
async fn look_at(
    conn: &mut sqlx::PgConnection,
    account_id: i32,
    feed_ids: &[i32],
    query: &str,
    terms: &[String],
    from: i64,
    upto: i64,
    limit: i64,
    page: Page,
) -> Result<Vec<Looked>, sqlx::Error> {
    // The inner query finds the articles, the outer one works out their
    // words: only for those, as a query with a limit is planned on its own,
    // and once for each, as the lateral query is too.
    let rows = sqlx::query!(
        r#"
        SELECT z.feed_id, z.seq, z.id, d.doc @@ $3::text::tsquery AS "hit!",
               ARRAY(SELECT d.doc @@ t::tsquery FROM unnest($8::text[]) t) AS "with_word!"
        FROM (
            SELECT x.feed_id, x.seq, x.id, x.title, x.author, x.tags, x.html
            FROM article_content x
            JOIN subscriptions s ON s.account_id = $1 AND s.feed_id = x.feed_id
            WHERE x.id >= $4 AND x.id < $5 AND x.feed_id = ANY($2)
              AND NOT EXISTS (
                  SELECT 1 FROM article_states h
                  WHERE h.account_id = $1 AND h.feed_id = x.feed_id AND h.seq = x.seq
                    AND (h.hidden OR ($6 AND h.read)))
              AND ($6 IS NOT TRUE OR x.seq > s.read_upto OR EXISTS (
                  SELECT 1 FROM article_states k
                  WHERE k.account_id = $1 AND k.feed_id = x.feed_id AND k.seq = x.seq
                    AND k.kept_unread))
            ORDER BY x.id DESC
            LIMIT $7
        ) z
        CROSS JOIN LATERAL (
            SELECT article_search_doc(z.title, z.author, z.tags, z.html) AS doc OFFSET 0
        ) d
        ORDER BY z.id DESC
        "#,
        account_id,
        feed_ids,
        query,
        from,
        upto,
        page.unread_only,
        limit,
        terms,
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Looked {
            key: Key {
                feed_id: r.feed_id,
                seq: r.seq,
                id: r.id,
            },
            hit: r.hit,
            with_word: r.with_word,
        })
        .collect())
}

/// What the index is asked for.
#[derive(Debug, Clone, Copy)]
struct Asked<'a> {
    /// The word it is asked for.
    term: &'a str,
    /// All the words, if there are others, which the articles it finds must
    /// also have.
    query: Option<&'a str>,
}

/// The hits the index has among the articles from `from` up to before `upto`
/// in the order of arrival, at most `limit`, newest first.
#[allow(clippy::too_many_arguments)]
async fn ask_index(
    conn: &mut sqlx::PgConnection,
    account_id: i32,
    feed_ids: &[i32],
    asked: Asked<'_>,
    from: i64,
    upto: i64,
    limit: i64,
    page: Page,
) -> Result<Vec<Key>, sqlx::Error> {
    // The index is asked for one word only: for all of them, it would read
    // every article with any of them, the common ones too. The inner query
    // is planned on its own, so the other words are only looked for in what
    // it found.
    sqlx::query_as!(
        Key,
        r#"
        SELECT c.feed_id, c.seq, c.id
        FROM (
        SELECT x.feed_id, x.seq, x.id, x.title, x.author, x.tags, x.html
        FROM article_content x
        JOIN subscriptions s ON s.account_id = $1 AND s.feed_id = x.feed_id
        WHERE article_search_doc(x.title, x.author, x.tags, x.html) @@ $3::text::tsquery
          AND x.id >= $4 AND x.id < $5 AND x.feed_id = ANY($2)
          AND NOT EXISTS (
              SELECT 1 FROM article_states h
              WHERE h.account_id = $1 AND h.feed_id = x.feed_id AND h.seq = x.seq
                AND (h.hidden OR ($6 AND h.read)))
          AND ($6 IS NOT TRUE OR x.seq > s.read_upto OR EXISTS (
              SELECT 1 FROM article_states k
              WHERE k.account_id = $1 AND k.feed_id = x.feed_id AND k.seq = x.seq
                AND k.kept_unread))
        OFFSET 0
        ) c
        WHERE $8::text IS NULL
           OR article_search_doc(c.title, c.author, c.tags, c.html) @@ $8::text::tsquery
        ORDER BY c.id DESC
        LIMIT $7
        "#,
        account_id,
        feed_ids,
        asked.term,
        from,
        upto,
        page.unread_only,
        limit,
        asked.query,
    )
    .fetch_all(&mut *conn)
    .await
}

/// The lines of the list for what was found, newest first.
async fn read(
    conn: &mut sqlx::PgConnection,
    account_id: i32,
    found: &[Key],
) -> Result<Vec<ListItem>, sqlx::Error> {
    let feeds: Vec<i32> = found.iter().map(|key| key.feed_id).collect();
    let seqs: Vec<i32> = found.iter().map(|key| key.seq).collect();
    sqlx::query_as!(
        ListItem,
        r#"
        SELECT a.feed_id, a.seq, a.id, a.published_at, a.updated_at, a.version, a.title,
               a.teaser, a.author, a.url, a.thumbnail_url,
               CASE WHEN a.seq <= s.read_upto THEN NOT coalesce(st.kept_unread, false)
                    ELSE coalesce(st.read, false) END AS "read!",
               coalesce(st.starred, false) AS "starred!"
        FROM unnest($2::int[], $3::int[]) AS k (feed_id, seq)
        JOIN subscriptions s ON s.account_id = $1 AND s.feed_id = k.feed_id
        JOIN articles a ON a.feed_id = k.feed_id AND a.seq = k.seq
        LEFT JOIN article_states st
            ON st.account_id = $1 AND st.feed_id = a.feed_id AND st.seq = a.seq
        ORDER BY a.id DESC
        "#,
        account_id,
        &feeds,
        &seqs,
    )
    .fetch_all(&mut *conn)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOTAL: i64 = 10_000_000;

    /// What was seen for a search for one word.
    fn seen(looked_at: i64, hits: i64, span: i64, micros_each: f64) -> Seen {
        let mut with_word = [0; MAX_WORDS];
        with_word[0] = hits;
        Seen {
            looked_at,
            hits,
            with_word,
            span,
            micros: looked_at as f64 * micros_each,
        }
    }

    #[test]
    fn starts_by_looking_at_the_newest() {
        let first = next_step(Seen::default(), 1, 50, TOTAL);
        assert_eq!(
            first,
            Next::LookAt {
                articles: PROBE,
                span: FIRST_SPAN
            }
        );
        // Few articles of the search so far: further back, up to a limit.
        let few = next_step(seen(30, 10, FIRST_SPAN, 60.0), 1, 40, TOTAL);
        assert_eq!(
            few,
            Next::LookAt {
                articles: PROBE,
                span: 2 * FIRST_SPAN
            }
        );
        // Nothing of the search that far back: the index.
        let none = Seen {
            span: MOST_SPAN,
            ..Seen::default()
        };
        assert_eq!(next_step(none, 1, 50, TOTAL), Next::AskIndex { word: 0 });
        // A few, and then the costs decide.
        let common = seen(20, 10, MOST_SPAN, 400.0);
        assert!(matches!(
            next_step(common, 1, 40, TOTAL),
            Next::LookAt { .. }
        ));
        let rare = seen(20, 0, MOST_SPAN, 400.0);
        assert_eq!(next_step(rare, 1, 50, TOTAL), Next::AskIndex { word: 0 });
    }

    #[test]
    fn a_common_word_is_looked_for_in_the_articles() {
        // One in ten is a hit: 500 more articles to look at.
        let next = next_step(seen(500, 50, 600, 60.0), 1, 45, TOTAL);
        assert!(
            matches!(next, Next::LookAt { articles, .. } if (500..=1000).contains(&articles)),
            "{next:?}"
        );
        // Even in a few feeds, far apart: the index would read a million.
        let next = next_step(seen(500, 50, 200_000, 60.0), 1, 45, TOTAL);
        assert!(matches!(next, Next::LookAt { .. }), "{next:?}");
    }

    #[test]
    fn a_rare_word_is_asked_of_the_index() {
        let rare = Next::AskIndex { word: 0 };
        assert_eq!(next_step(seen(500, 0, 600, 60.0), 1, 50, TOTAL), rare);
        assert_eq!(next_step(seen(500, 1, 600, 60.0), 1, 49, TOTAL), rare);
        // Also when the texts are short and looking is cheap.
        assert_eq!(next_step(seen(500, 1, 600, 5.0), 1, 49, TOTAL), rare);
    }

    #[test]
    fn of_several_words_the_index_is_asked_for_the_rarest() {
        // A common word and a rare one: together they are rare, and the
        // index reads only the articles with the rare one.
        let mut both = seen(500, 0, 600, 60.0);
        both.with_word[..3].copy_from_slice(&[60, 1, 200]);
        assert_eq!(next_step(both, 3, 50, TOTAL), Next::AskIndex { word: 1 });
        // Two common words that are seldom together: asking the index for
        // either would read a million articles, so they are looked for.
        let mut common = seen(500, 5, 600, 60.0);
        common.with_word[..2].copy_from_slice(&[60, 70]);
        assert!(matches!(
            next_step(common, 2, 45, TOTAL),
            Next::LookAt { .. }
        ));
    }

    #[test]
    fn looking_stays_within_bounds() {
        for hits in [1, 10, 100, 499, 500] {
            for span in [500, 5_000, 50_000, MOST_SPAN] {
                if let Next::LookAt { articles, span } =
                    next_step(seen(500, hits, span, 60.0), 1, 50, TOTAL)
                {
                    assert!((PROBE..=MOST_LOOKED_AT).contains(&articles));
                    assert!((FIRST_SPAN..=MOST_SPAN).contains(&span));
                }
            }
        }
    }
}
