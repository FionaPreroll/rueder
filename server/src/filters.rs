//! A reader's filter rules: hide articles, or mark them read, by words in the
//! title or the text, by author or by tag.
//!
//! A rule looks at an article when it arrives ([`apply_new`], called by the
//! fetch) and at what is there when the rules change ([`reapply`]). Whether an
//! article is hidden is the rules' word alone: it is hidden exactly if a rule
//! that is switched on and in whose scope its feed lies matches it. "Read"
//! only ever marks; a rule that goes does not make anything unread again.
//!
//! The decisions are the ones of `web/src/lib/filters.ts`, with one engine
//! for all patterns: the `regex` crate, which runs in time linear in the text
//! whatever the pattern.

use std::collections::{HashMap, HashSet};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};

use crate::{AppError, AppState, reading, sessions::Session, tree::bad};

/// How many rules an account can have.
pub const MAX_RULES: i64 = 200;
/// The longest pattern, in characters.
pub const MAX_PATTERN: usize = 200;
/// How many of a feed's newest articles a changed rule looks at. Older ones
/// stay as they are, except hidden ones, which are always looked at again.
pub const RECENT: i32 = 200;
/// How large a compiled pattern may get, in bytes.
const MAX_PROGRAM: usize = 1024 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/rules", get(rules).post(add))
        .route("/rules/order", post(order))
        .route("/rules/preview", post(preview))
        .route("/rules/{id}", put(change).delete(remove))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Field {
    Any,
    Title,
    Content,
    Author,
    Tag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Match {
    Contains,
    Word,
    Regex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Hide,
    Read,
}

/// Where a rule holds: everywhere, in a folder with all that is in it, or in
/// one feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "lowercase")]
pub enum Scope {
    All,
    Folder(i32),
    Feed(i32),
}

impl Field {
    fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Title => "title",
            Self::Content => "content",
            Self::Author => "author",
            Self::Tag => "tag",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        [
            Self::Any,
            Self::Title,
            Self::Content,
            Self::Author,
            Self::Tag,
        ]
        .into_iter()
        .find(|field| field.as_str() == text)
    }
}

impl Match {
    fn as_str(self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::Word => "word",
            Self::Regex => "regex",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        [Self::Contains, Self::Word, Self::Regex]
            .into_iter()
            .find(|matching| matching.as_str() == text)
    }
}

impl Action {
    fn as_str(self) -> &'static str {
        match self {
            Self::Hide => "hide",
            Self::Read => "read",
        }
    }
}

/// The test a pattern stands for, or why it is none. Letters match whatever
/// their case.
pub fn compile(matching: Match, pattern: &str) -> Result<Regex, String> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return Err("the pattern is empty".into());
    }
    if pattern.chars().count() > MAX_PATTERN {
        return Err(format!(
            "the pattern is longer than {MAX_PATTERN} characters"
        ));
    }
    let source = match matching {
        Match::Contains => regex::escape(pattern),
        // A whole word: no letter or digit right before or after it.
        Match::Word => format!(
            r"(?:^|[^\p{{L}}\p{{N}}]){}(?:$|[^\p{{L}}\p{{N}}])",
            regex::escape(pattern)
        ),
        Match::Regex => pattern.to_owned(),
    };
    RegexBuilder::new(&source)
        .case_insensitive(true)
        .size_limit(MAX_PROGRAM)
        .dfa_size_limit(MAX_PROGRAM)
        .build()
        .map_err(|err| match err {
            regex::Error::CompiledTooBig(_) => "the pattern is too costly".into(),
            other => other.to_string().chars().take(300).collect(),
        })
}

/// The words of a cleaned text, without its markup. This is for texts as the
/// cleaning left them, whose markup is well-formed.
pub fn text_of(html: &str) -> String {
    let mut plain = String::with_capacity(html.len());
    let (mut in_tag, mut in_quotes) = (false, false);
    for letter in html.chars() {
        match letter {
            '<' if !in_tag => {
                in_tag = true;
                plain.push(' ');
            }
            '"' if in_tag => in_quotes = !in_quotes,
            '>' if in_tag && !in_quotes => in_tag = false,
            _ if in_tag => {}
            _ => plain.push(letter),
        }
    }
    let plain = plain
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&");
    plain.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What of an article a rule can look at.
pub struct Texts {
    pub title: String,
    /// The text without markup, see [`text_of`].
    pub text: String,
    pub author: String,
    pub tags: Vec<String>,
}

/// A rule ready to decide.
#[derive(Clone)]
pub struct Test {
    field: Field,
    action: Action,
    regex: Regex,
}

impl Test {
    pub fn new(field: Field, matching: Match, pattern: &str, action: Action) -> Option<Self> {
        let regex = compile(matching, pattern).ok()?;
        Some(Self {
            field,
            action,
            regex,
        })
    }

    pub fn matches(&self, article: &Texts) -> bool {
        let found = |text: &str| self.regex.is_match(text);
        let in_tags = || article.tags.iter().any(|tag| found(tag));
        match self.field {
            Field::Title => found(&article.title),
            Field::Content => found(&article.text),
            Field::Author => found(&article.author),
            Field::Tag => in_tags(),
            Field::Any => {
                found(&article.title) || found(&article.text) || found(&article.author) || in_tags()
            }
        }
    }
}

/// What the rules say about an article: hidden wins over read.
fn decide<'a>(tests: impl IntoIterator<Item = &'a Test>, article: &Texts) -> (bool, bool) {
    let (mut hide, mut read) = (false, false);
    for test in tests {
        if test.matches(article) {
            match test.action {
                Action::Hide => hide = true,
                Action::Read => read = true,
            }
        }
    }
    (hide, read && !hide)
}

/// A stored rule that is switched on, ready to decide, with its scope.
struct Stored {
    scope: Scope,
    test: Test,
}

async fn stored(conn: &mut PgConnection, account_id: i32) -> anyhow::Result<Vec<Stored>> {
    let rows = sqlx::query!(
        "SELECT folder_id, feed_id, field, match AS matching, pattern, action
         FROM filter_rules WHERE account_id = $1 AND enabled ORDER BY position, id",
        account_id
    )
    .fetch_all(conn)
    .await?;
    let rules = rows.into_iter().filter_map(|row| {
        let scope = match (row.folder_id, row.feed_id) {
            (Some(folder), _) => Scope::Folder(folder),
            (None, Some(feed)) => Scope::Feed(feed),
            (None, None) => Scope::All,
        };
        let action = if row.action == "hide" {
            Action::Hide
        } else {
            Action::Read
        };
        // A pattern that no longer compiles decides nothing.
        let test = Test::new(
            Field::parse(&row.field)?,
            Match::parse(&row.matching)?,
            &row.pattern,
            action,
        )?;
        Some(Stored { scope, test })
    });
    Ok(rules.collect())
}

/// The folders each of the account's feeds is in, from the inside out.
async fn folders_of(
    conn: &mut PgConnection,
    account_id: i32,
    feed_id: Option<i32>,
) -> anyhow::Result<HashMap<i32, HashSet<i32>>> {
    let rows = sqlx::query!(
        r#"WITH RECURSIVE chain AS (
               SELECT s.feed_id, s.folder_id AS folder FROM subscriptions s
               WHERE s.account_id = $1 AND ($2::int4 IS NULL OR s.feed_id = $2)
             UNION ALL
               SELECT c.feed_id, f.parent_id FROM chain c
               JOIN folders f ON f.account_id = $1 AND f.id = c.folder
           )
           SELECT feed_id AS "feed_id!", folder FROM chain"#,
        account_id,
        feed_id,
    )
    .fetch_all(conn)
    .await?;
    let mut feeds: HashMap<i32, HashSet<i32>> = HashMap::new();
    for row in rows {
        let folders = feeds.entry(row.feed_id).or_default();
        folders.extend(row.folder);
    }
    Ok(feeds)
}

fn in_scope(scope: Scope, feed_id: i32, folders: &HashSet<i32>) -> bool {
    match scope {
        Scope::All => true,
        Scope::Feed(feed) => feed == feed_id,
        Scope::Folder(folder) => folders.contains(&folder),
    }
}

/// An article as the rules see it, with what the account's state says now.
struct Candidate {
    seq: i32,
    title: String,
    html: String,
    author: String,
    tags: Vec<String>,
    hidden: bool,
    read: bool,
}

/// What the rules say about a candidate.
struct Verdict {
    seq: i32,
    hidden: bool,
    read: bool,
    hide: bool,
    mark_read: bool,
}

/// The articles of a feed that rules look at: from number `from` on, and,
/// with `and_hidden`, the ones hidden for the account wherever they are.
async fn candidates(
    conn: &mut PgConnection,
    account_id: i32,
    feed_id: i32,
    from: i32,
    and_hidden: bool,
) -> anyhow::Result<Vec<Candidate>> {
    let rows = sqlx::query!(
        r#"SELECT a.seq, a.title, a.author, c.html, c.tags,
                  coalesce(st.hidden, false) AS "hidden!",
                  CASE WHEN a.seq <= s.read_upto THEN NOT coalesce(st.kept_unread, false)
                       ELSE coalesce(st.read, false) END AS "read!"
           FROM subscriptions s
           JOIN articles a ON a.feed_id = s.feed_id
           JOIN article_content c ON c.feed_id = a.feed_id AND c.seq = a.seq
           LEFT JOIN article_states st
               ON st.account_id = s.account_id AND st.feed_id = a.feed_id AND st.seq = a.seq
           WHERE s.account_id = $1 AND s.feed_id = $2
             AND (a.seq >= $3 OR ($4 AND a.seq IN (
                 SELECT x.seq FROM article_states x
                 WHERE x.account_id = $1 AND x.feed_id = $2 AND x.hidden)))
           ORDER BY a.seq"#,
        account_id,
        feed_id,
        from,
        and_hidden,
    )
    .fetch_all(conn)
    .await?;
    let candidates = rows.into_iter().map(|row| Candidate {
        seq: row.seq,
        title: row.title,
        html: row.html,
        author: row.author,
        tags: row.tags,
        hidden: row.hidden,
        read: row.read,
    });
    Ok(candidates.collect())
}

/// Lets the rules look at the candidates. Reading the texts takes time that
/// grows with them, so this happens away from the threads that serve requests.
async fn verdicts(tests: Vec<Test>, candidates: Vec<Candidate>) -> anyhow::Result<Vec<Verdict>> {
    let work = move || {
        let verdict = |article: Candidate| {
            let texts = Texts {
                title: article.title,
                text: text_of(&article.html),
                author: article.author,
                tags: article.tags,
            };
            let (hide, mark_read) = decide(&tests, &texts);
            Verdict {
                seq: article.seq,
                hidden: article.hidden,
                read: article.read,
                hide,
                mark_read,
            }
        };
        candidates.into_iter().map(verdict).collect()
    };
    Ok(tokio::task::spawn_blocking(work).await?)
}

/// What a look at the articles changed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Changed {
    /// Articles that are hidden now and were not.
    pub hidden: usize,
    /// Articles that were hidden and are back.
    pub shown: usize,
    /// Articles marked read.
    pub read: usize,
}

/// Brings one feed's articles in line with the rules that hold for it.
async fn settle(
    conn: &mut PgConnection,
    account_id: i32,
    feed_id: i32,
    tests: Vec<Test>,
    from: i32,
    and_hidden: bool,
    changed: &mut Changed,
) -> anyhow::Result<()> {
    let articles = candidates(conn, account_id, feed_id, from, and_hidden).await?;
    for article in verdicts(tests, articles).await? {
        if article.hide != article.hidden {
            reading::set_hidden_on(conn, account_id, feed_id, article.seq, article.hide).await?;
            if article.hide {
                changed.hidden += 1;
            } else {
                changed.shown += 1;
            }
        }
        if article.mark_read && !article.read {
            reading::set_read_on(conn, account_id, feed_id, article.seq, true).await?;
            changed.read += 1;
        }
    }
    Ok(())
}

/// The rules look at articles that just arrived in a feed, numbers `first` and
/// up, for everybody who subscribes to it. Runs in the transaction that stored
/// them, so nobody sees them unfiltered.
pub async fn apply_new(conn: &mut PgConnection, feed_id: i32, first: i32) -> anyhow::Result<()> {
    let accounts = sqlx::query_scalar!(
        "SELECT DISTINCT r.account_id FROM filter_rules r
         JOIN subscriptions s ON s.account_id = r.account_id AND s.feed_id = $1
         WHERE r.enabled",
        feed_id
    )
    .fetch_all(&mut *conn)
    .await?;
    for account_id in accounts {
        let rules = stored(conn, account_id).await?;
        let folders = folders_of(conn, account_id, Some(feed_id)).await?;
        let none = HashSet::new();
        let folders = folders.get(&feed_id).unwrap_or(&none);
        let tests: Vec<Test> = rules
            .iter()
            .filter(|rule| in_scope(rule.scope, feed_id, folders))
            .map(|rule| rule.test.clone())
            .collect();
        if !tests.is_empty() {
            let mut changed = Changed::default();
            settle(conn, account_id, feed_id, tests, first, false, &mut changed).await?;
        }
    }
    Ok(())
}

/// The rules of an account look again at what is there: each feed's newest
/// [`RECENT`] articles and everything that is hidden. For after a rule
/// changed, or the tree did.
pub async fn reapply(db: &PgPool, account_id: i32) -> anyhow::Result<Changed> {
    let mut tx = db.begin().await?;
    let rules = stored(&mut tx, account_id).await?;
    let folders = folders_of(&mut tx, account_id, None).await?;
    let feeds = sqlx::query!(
        "SELECT s.feed_id, f.last_seq FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
         WHERE s.account_id = $1 ORDER BY s.feed_id",
        account_id
    )
    .fetch_all(&mut *tx)
    .await?;
    let none = HashSet::new();
    let mut changed = Changed::default();
    for feed in feeds {
        let folders = folders.get(&feed.feed_id).unwrap_or(&none);
        let tests: Vec<Test> = rules
            .iter()
            .filter(|rule| in_scope(rule.scope, feed.feed_id, folders))
            .map(|rule| rule.test.clone())
            .collect();
        // Without rules there is nothing to read but what is hidden.
        let from = if tests.is_empty() {
            i32::MAX
        } else {
            feed.last_seq - RECENT + 1
        };
        settle(
            &mut tx,
            account_id,
            feed.feed_id,
            tests,
            from,
            true,
            &mut changed,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(changed)
}

/// Folders decide which rules hold for a feed, so after a move or after a
/// folder went, the rules look again. Not needed while no rule names a folder
/// and nothing is hidden.
pub async fn tree_changed(db: &PgPool, account_id: i32) -> anyhow::Result<()> {
    let needed = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM filter_rules
                          WHERE account_id = $1 AND enabled AND folder_id IS NOT NULL)
               OR EXISTS (SELECT 1 FROM article_states WHERE account_id = $1 AND hidden)
               AS "needed!""#,
        account_id
    )
    .fetch_one(db)
    .await?;
    if needed {
        reapply(db, account_id).await?;
    }
    Ok(())
}

/// A rule as the API takes and gives it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    #[serde(default, skip_deserializing)]
    pub id: i32,
    #[serde(default = "yes")]
    pub enabled: bool,
    pub scope: Scope,
    pub field: Field,
    #[serde(rename = "match")]
    pub matching: Match,
    pub pattern: String,
    pub action: Action,
}

fn yes() -> bool {
    true
}

impl Rule {
    /// The rule with its pattern as it is stored, if it can be a rule of
    /// this account.
    async fn checked(mut self, state: &AppState, session: &Session) -> Result<Self, AppError> {
        self.pattern = self.pattern.trim().to_owned();
        compile(self.matching, &self.pattern).map_err(AppError::BadRequest)?;
        let own = match self.scope {
            Scope::All => true,
            Scope::Folder(id) => sqlx::query_scalar!(
                "SELECT id FROM folders WHERE account_id = $1 AND id = $2",
                session.account_id,
                id
            )
            .fetch_optional(&state.db)
            .await?
            .is_some(),
            Scope::Feed(id) => sqlx::query_scalar!(
                "SELECT feed_id FROM subscriptions WHERE account_id = $1 AND feed_id = $2",
                session.account_id,
                id
            )
            .fetch_optional(&state.db)
            .await?
            .is_some(),
        };
        if own {
            Ok(self)
        } else {
            Err(AppError::NotFound)
        }
    }

    fn scope_ids(&self) -> (Option<i32>, Option<i32>) {
        match self.scope {
            Scope::All => (None, None),
            Scope::Folder(id) => (Some(id), None),
            Scope::Feed(id) => (None, Some(id)),
        }
    }
}

async fn rules(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<Vec<Rule>>, AppError> {
    let rows = sqlx::query!(
        "SELECT id, enabled, folder_id, feed_id, field, match AS matching, pattern, action
         FROM filter_rules WHERE account_id = $1 ORDER BY position, id",
        session.account_id
    )
    .fetch_all(&state.db)
    .await?;
    let rules = rows.into_iter().filter_map(|row| {
        Some(Rule {
            id: row.id,
            enabled: row.enabled,
            scope: match (row.folder_id, row.feed_id) {
                (Some(folder), _) => Scope::Folder(folder),
                (None, Some(feed)) => Scope::Feed(feed),
                (None, None) => Scope::All,
            },
            field: Field::parse(&row.field)?,
            matching: Match::parse(&row.matching)?,
            pattern: row.pattern,
            action: if row.action == "hide" {
                Action::Hide
            } else {
                Action::Read
            },
        })
    });
    Ok(Json(rules.collect()))
}

/// What a change to the rules did to the articles that are there.
#[derive(Serialize)]
struct Outcome {
    id: i32,
    changed: Changed,
}

async fn add(
    State(state): State<AppState>,
    session: Session,
    Json(rule): Json<Rule>,
) -> Result<(StatusCode, Json<Outcome>), AppError> {
    let rule = rule.checked(&state, &session).await?;
    let (folder_id, feed_id) = rule.scope_ids();
    let id = sqlx::query_scalar!(
        "INSERT INTO filter_rules
             (account_id, position, enabled, folder_id, feed_id, field, match, pattern, action)
         SELECT $1, coalesce(max(position), 0) + 1, $2, $3, $4, $5, $6, $7, $8
         FROM filter_rules WHERE account_id = $1
         HAVING count(*) < $9
         RETURNING id",
        session.account_id,
        rule.enabled,
        folder_id,
        feed_id,
        rule.field.as_str(),
        rule.matching.as_str(),
        rule.pattern,
        rule.action.as_str(),
        MAX_RULES,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| bad("too many rules"))?;
    let changed = reapply(&state.db, session.account_id).await?;
    Ok((StatusCode::CREATED, Json(Outcome { id, changed })))
}

async fn change(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i32>,
    Json(rule): Json<Rule>,
) -> Result<Json<Outcome>, AppError> {
    let rule = rule.checked(&state, &session).await?;
    let (folder_id, feed_id) = rule.scope_ids();
    let updated = sqlx::query!(
        "UPDATE filter_rules
         SET enabled = $3, folder_id = $4, feed_id = $5, field = $6, match = $7, pattern = $8,
             action = $9
         WHERE account_id = $1 AND id = $2",
        session.account_id,
        id,
        rule.enabled,
        folder_id,
        feed_id,
        rule.field.as_str(),
        rule.matching.as_str(),
        rule.pattern,
        rule.action.as_str(),
    )
    .execute(&state.db)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    let changed = reapply(&state.db, session.account_id).await?;
    Ok(Json(Outcome { id, changed }))
}

async fn remove(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i32>,
) -> Result<Json<Outcome>, AppError> {
    let gone = sqlx::query!(
        "DELETE FROM filter_rules WHERE account_id = $1 AND id = $2",
        session.account_id,
        id
    )
    .execute(&state.db)
    .await?;
    if gone.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    let changed = reapply(&state.db, session.account_id).await?;
    Ok(Json(Outcome { id, changed }))
}

#[derive(Deserialize)]
struct Order {
    ids: Vec<i32>,
}

/// Puts the rules in this order, for the list of them: what they decide does
/// not depend on it. Rules that are not named come after the named ones.
async fn order(
    State(state): State<AppState>,
    session: Session,
    Json(order): Json<Order>,
) -> Result<StatusCode, AppError> {
    if order.ids.len() > MAX_RULES as usize {
        return Err(bad("too many rules"));
    }
    sqlx::query!(
        "UPDATE filter_rules r SET position = n.position
         FROM (
             SELECT k.id, row_number() OVER (ORDER BY t.at, k.position, k.id) AS position
             FROM filter_rules k
             LEFT JOIN (
                 SELECT u.id, min(u.at) AS at
                 FROM unnest($2::int4[]) WITH ORDINALITY AS u(id, at) GROUP BY u.id
             ) t ON t.id = k.id
             WHERE k.account_id = $1
         ) n
         WHERE r.id = n.id",
        session.account_id,
        &order.ids,
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// How many articles a rule would catch.
#[derive(Serialize)]
struct Preview {
    /// The articles the rule matches,
    matches: usize,
    /// of this many it looked at: the newest of each feed in its scope.
    of: usize,
}

/// Tries a rule on what is there without storing it or changing anything.
/// This is also how the interface learns whether a pattern is one.
async fn preview(
    State(state): State<AppState>,
    session: Session,
    Json(rule): Json<Rule>,
) -> Result<Json<Preview>, AppError> {
    let rule = rule.checked(&state, &session).await?;
    let test = Test::new(rule.field, rule.matching, &rule.pattern, rule.action)
        .ok_or_else(|| bad("the pattern cannot be used"))?;
    let mut conn = state.db.acquire().await?;
    let folders = folders_of(&mut conn, session.account_id, None).await?;
    let feeds = sqlx::query!(
        "SELECT s.feed_id, f.last_seq FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
         WHERE s.account_id = $1",
        session.account_id
    )
    .fetch_all(&mut *conn)
    .await?;
    let none = HashSet::new();
    let mut preview = Preview { matches: 0, of: 0 };
    for feed in feeds {
        let folders = folders.get(&feed.feed_id).unwrap_or(&none);
        if !in_scope(rule.scope, feed.feed_id, folders) {
            continue;
        }
        let from = feed.last_seq - RECENT + 1;
        let articles = candidates(&mut conn, session.account_id, feed.feed_id, from, false).await?;
        preview.of += articles.len();
        let verdicts = verdicts(vec![test.clone()], articles).await?;
        let matching = verdicts.iter().filter(|v| v.hide || v.mark_read);
        preview.matches += matching.count();
    }
    Ok(Json(preview))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn article() -> Texts {
        Texts {
            title: "Anzeige: Der neue Lötkolben".into(),
            text: text_of("<p>Jetzt <b>zugreifen</b>, nur diese Woche.</p>"),
            author: "Werbeteam".into(),
            tags: vec!["werbung".into(), "hardware".into()],
        }
    }

    fn matches(field: Field, matching: Match, pattern: &str) -> bool {
        Test::new(field, matching, pattern, Action::Hide)
            .is_some_and(|test| test.matches(&article()))
    }

    fn found(matching: Match, pattern: &str, text: &str) -> bool {
        compile(matching, pattern).unwrap().is_match(text)
    }

    #[test]
    fn the_text_is_without_tags_and_with_single_blanks() {
        assert_eq!(
            text_of("<p>Jetzt <b>zugreifen</b>,\n nur</p><p>heute</p>"),
            "Jetzt zugreifen , nur heute"
        );
        assert_eq!(
            text_of(
                r#"<a href="https://a.example/?a=1&amp;b=>2" title="x > y">Fish &amp; Chips</a>&nbsp;&lt;3"#
            ),
            "Fish & Chips <3"
        );
        assert_eq!(text_of(""), "");
        // Markup that never closes ends the text; nothing fails.
        assert_eq!(text_of("one <b two"), "one");
    }

    #[test]
    fn an_empty_a_too_long_or_a_broken_pattern_is_none() {
        assert!(compile(Match::Contains, "  ").is_err());
        assert!(compile(Match::Contains, &"x".repeat(MAX_PATTERN + 1)).is_err());
        assert!(compile(Match::Contains, &"ä".repeat(MAX_PATTERN)).is_ok());
        assert!(compile(Match::Regex, "(").is_err());
        // What the frontend's engine knows and this one does not: no rule.
        assert!(compile(Match::Regex, r"(a)\1").is_err());
        assert!(compile(Match::Regex, "a(?=b)").is_err());
    }

    #[test]
    fn a_costly_pattern_is_refused_and_a_nested_one_is_no_danger() {
        assert_eq!(
            compile(Match::Regex, r"(\pL{50}){50}").unwrap_err(),
            "the pattern is too costly"
        );
        // Where a backtracking engine would never come back.
        let slow = compile(Match::Regex, "(a+)+$").unwrap();
        assert!(!slow.is_match(&format!("{}b", "a".repeat(50_000))));
    }

    #[test]
    fn a_pattern_is_taken_as_it_is_unless_it_is_a_regular_expression() {
        assert!(!found(Match::Contains, "a.c", "abc"));
        assert!(found(Match::Contains, "a.c", "A.C"));
        assert!(found(Match::Regex, "a.c", "abc"));
        assert!(found(Match::Contains, " Ärger ", "viel ärger heute"));
    }

    #[test]
    fn a_word_is_found_whole_only_with_umlauts_as_letters() {
        assert!(found(Match::Word, "ende", "Am Ende."));
        assert!(found(Match::Word, "ende", "Ende"));
        assert!(!found(Match::Word, "ende", "Wochenende"));
        assert!(!found(Match::Word, "ende", "endet"));
        assert!(!found(Match::Word, "öl", "Möl"));
        assert!(!found(Match::Word, "7", "2027"));
        assert!(found(Match::Word, "c++", "Mehr C++ wagen"));
    }

    #[test]
    fn a_rule_looks_at_the_field_it_names() {
        assert!(matches(Field::Title, Match::Contains, "anzeige"));
        assert!(!matches(Field::Title, Match::Contains, "zugreifen"));
        assert!(matches(Field::Content, Match::Contains, "zugreifen"));
        assert!(matches(Field::Author, Match::Contains, "werbe"));
        assert!(matches(Field::Tag, Match::Word, "hardware"));
        assert!(!matches(Field::Tag, Match::Contains, "software"));
        // Tags are looked at one by one, not as one text.
        assert!(!matches(Field::Tag, Match::Regex, "werbung.*hardware"));
    }

    #[test]
    fn any_looks_everywhere() {
        for pattern in ["Lötkolben", "diese Woche", "Werbeteam", "werbung"] {
            assert!(matches(Field::Any, Match::Contains, pattern), "{pattern}");
        }
        assert!(!matches(Field::Any, Match::Contains, "Teleskop"));
        assert!(!matches(Field::Any, Match::Regex, "["));
    }

    #[test]
    fn hiding_wins_over_marking_read() {
        let hide = Test::new(Field::Title, Match::Contains, "anzeige", Action::Hide).unwrap();
        let read = Test::new(Field::Author, Match::Contains, "werbe", Action::Read).unwrap();
        let other = Test::new(Field::Title, Match::Contains, "teleskop", Action::Hide).unwrap();
        assert_eq!(decide([&read], &article()), (false, true));
        assert_eq!(decide([&hide, &read], &article()), (true, false));
        assert_eq!(decide([&other], &article()), (false, false));
        assert_eq!(decide([], &article()), (false, false));
    }

    #[test]
    fn a_scope_is_written_with_its_kind() {
        let scope = |json: &str| serde_json::from_str::<Scope>(json).ok();
        assert_eq!(scope(r#"{"kind":"all"}"#), Some(Scope::All));
        assert_eq!(scope(r#"{"kind":"folder","id":3}"#), Some(Scope::Folder(3)));
        assert_eq!(scope(r#"{"kind":"feed","id":4}"#), Some(Scope::Feed(4)));
        assert_eq!(scope(r#"{"kind":"folder"}"#), None);
        assert_eq!(scope(r#"{"kind":"site","id":1}"#), None);
        assert_eq!(
            serde_json::to_string(&Scope::Feed(4)).unwrap(),
            r#"{"kind":"feed","id":4}"#
        );
    }
}
