//! Moving an account from one rueder to another: what a reader made of their
//! account goes out as one JSON file and comes in again on another server.
//!
//! The numbers of feeds and articles are those of the server the file comes
//! from and mean nothing on another one. They stand in the file only so that
//! a person can tell what is what. A feed is found again by its address, an
//! article by its id in its feed (GUID), or by its page where that is the only
//! article with it. What the reader has read is written as the article the
//! mark stands at and the exceptions from it, so that it can be set again on
//! numbers that differ. Starred and labelled articles carry their text, so
//! that they are not lost where the other server never had them.
//!
//! An import adds to what the account has and takes nothing away: feeds it
//! reads already keep their place, their mark only moves up, stars and labels
//! are added, and so are rules that are not there yet. Only the settings are
//! replaced. It happens in one transaction, all of it or nothing.
//!
//! Not in the file: the password, sessions, the identities of an OpenID
//! Connect provider, being an admin, the consent to sending reports of errors
//! (it is given to a server, not taken along), old versions of articles, and
//! what is hidden, which the rules decide again.

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use ts_rs::TS;
use url::Url;

use crate::{
    AppError, AppState,
    articles::{self, NewArticle, Original, guid_hash},
    cleaning::{self, Base, Rules},
    feeds,
    filters::{self, Action, Field, Match},
    labels, reading,
    sessions::Session,
    subscribe::{self, Outline, Step},
    tree::{self, bad},
};

/// What the file says it is.
pub const FORMAT: &str = "rueder-account";
/// The version of the file's layout. A server reads only the versions it knows.
pub const VERSION: u32 = 1;
/// The largest file an import takes, in bytes. The texts of starred and
/// labelled articles make most of it.
pub const MAX_FILE: usize = 32 * 1024 * 1024;
/// How many articles a file can name, in all of its feeds.
pub const MAX_ARTICLES: usize = 50_000;
/// The longest id of an article in its feed, in bytes.
const MAX_GUID: usize = 2000;
/// The longest title and author of an article that is brought along, in
/// characters.
const MAX_TITLE: usize = 1000;
const MAX_AUTHOR: usize = 300;
/// How many tags and files an article brings along at most.
const MAX_TAGS: usize = 20;
const MAX_ENCLOSURES: usize = 20;

/// The types this module sends and reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types.add::<Export>().add::<Imported>();
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/export", get(export_file)).route(
        "/import",
        post(import_file).layer(DefaultBodyLimit::max(MAX_FILE)),
    )
}

/// An account as a file, for another rueder. `GET /api/export` answers with
/// it, and `POST /api/import` reads it.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(rename = "AccountExport")]
pub struct Export {
    /// Always "rueder-account".
    pub format: String,
    /// The layout of the file, 1. A server reads only the layouts it knows.
    pub version: u32,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub exported_at: OffsetDateTime,
    /// The settings of the interface, as `GET /api/settings` gives them.
    #[serde(default)]
    #[ts(type = "Record<string, unknown>")]
    pub settings: serde_json::Value,
    /// Folders and feeds from the top down, in their order: a folder before
    /// what is in it.
    #[serde(default)]
    pub tree: Vec<Node>,
    #[serde(default)]
    pub labels: Vec<ExportedLabel>,
    /// The filter rules in their order.
    #[serde(default)]
    pub rules: Vec<ExportedRule>,
}

/// A folder or a feed of the tree.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[ts(rename = "ExportedNode")]
pub enum Node {
    /// A folder, by its name and the names of the folders it is in, from the
    /// top down.
    Folder {
        path: Vec<String>,
    },
    Feed(Box<ExportedFeed>),
}

/// Who a feed is fetched as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(rename = "ExportedAgent")]
pub enum Agent {
    Rueder,
    Browser,
}

/// A subscription, with what the reader did in it.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExportedFeed {
    /// The feed's number on the server the file comes from.
    pub id: i32,
    /// The address of the feed: how it is found again.
    pub url: String,
    pub site_url: Option<String>,
    /// The feed's own title.
    #[serde(default)]
    pub title: String,
    /// The name the reader gave it.
    pub own_title: Option<String>,
    /// The folder it is in, by its path; empty at the top.
    #[serde(default)]
    pub folder: Vec<String>,
    #[serde(default = "rueder")]
    pub agent: Agent,
    /// The number of the newest article on the server the file comes from.
    #[serde(default)]
    pub last_seq: i32,
    /// The article the mark stands at: it and everything before it is read,
    /// but for the articles below that say otherwise. None if the mark is at
    /// the start.
    pub read_upto: Option<ArticleRef>,
    /// The articles that are read above the mark or unread below it, starred
    /// or labelled.
    #[serde(default)]
    pub articles: Vec<ExportedArticle>,
}

fn rueder() -> Agent {
    Agent::Rueder
}

/// An article, as it can be found on another server.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ArticleRef {
    /// Its number in its feed on the server the file comes from; with the
    /// feed's number its id in that server's API.
    pub seq: i32,
    /// Its id in its feed, as the feed gives it.
    pub guid: String,
    /// Its page.
    pub url: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub published_at: OffsetDateTime,
}

/// An article the reader did something with.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExportedArticle {
    pub seq: i32,
    pub guid: String,
    pub url: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    pub published_at: OffsetDateTime,
    #[serde(default)]
    pub title: String,
    /// Whether it is read, whatever the mark says.
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub starred: bool,
    /// The names of its labels.
    #[serde(default)]
    pub labels: Vec<String>,
    /// Its text, if it is starred or labelled.
    pub content: Option<Content>,
}

/// The text of an article and what goes with it, as it is shown.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(rename = "ExportedContent")]
pub struct Content {
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub html: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub enclosures: Vec<ExportedEnclosure>,
    pub image_url: Option<String>,
    pub image_title: Option<String>,
    pub thumbnail_url: Option<String>,
}

/// A file of an article, as the feed lists it.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExportedEnclosure {
    pub url: String,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub length: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExportedLabel {
    pub title: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ExportedRule {
    pub enabled: bool,
    pub scope: ExportedScope,
    pub field: Field,
    #[serde(rename = "match")]
    pub matching: Match,
    pub pattern: String,
    pub action: Action,
}

/// Where a rule holds: a folder by its path, a feed by its address.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ExportedScope {
    All,
    Folder { path: Vec<String> },
    Feed { url: String },
}

/// What an import did. `POST /api/import` answers with it.
#[derive(Debug, Default, PartialEq, Eq, Serialize, TS)]
#[ts(rename = "AccountImported")]
pub struct Imported {
    /// Feeds the account reads now and did not before.
    pub feeds: usize,
    /// Feeds of the file the account read already.
    pub known: usize,
    /// Folders that were made.
    pub folders: usize,
    /// Articles of the file that were found here.
    pub articles: usize,
    /// Starred and labelled articles that were not here and came with their
    /// text.
    pub carried: usize,
    /// Articles that were not found and had no text with them: what was read
    /// of them is lost.
    pub missing: usize,
    /// Labels that were made.
    pub labels: usize,
    /// Rules that were added.
    pub rules: usize,
    /// What the file has that could not be taken: feeds without an address
    /// that can be fetched, articles without an id, labels and rules that
    /// break the rules, rules whose folder or feed is not there.
    pub skipped: usize,
    /// Whether the settings came with it.
    pub settings: bool,
}

impl std::fmt::Display for Imported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "feeds:      {} new, {} known", self.feeds, self.known)?;
        writeln!(f, "folders:    {} new", self.folders)?;
        writeln!(
            f,
            "articles:   {} found, {} brought along, {} not found",
            self.articles, self.carried, self.missing
        )?;
        writeln!(f, "labels:     {} new", self.labels)?;
        writeln!(f, "rules:      {} new", self.rules)?;
        writeln!(f, "skipped:    {}", self.skipped)?;
        write!(
            f,
            "settings:   {}",
            if self.settings { "taken" } else { "none" }
        )
    }
}

// ---------------------------------------------------------------- export

/// Everything of an account that moves, as one file.
pub async fn export(db: &PgPool, account_id: i32) -> anyhow::Result<Export> {
    let mut tx = db.begin().await?;
    // One picture of the account, however long the reading takes.
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let settings = sqlx::query_scalar!("SELECT settings FROM accounts WHERE id = $1", account_id)
        .fetch_one(&mut *tx)
        .await?;

    let folders = sqlx::query!(
        "SELECT id, parent_id, title, position FROM folders WHERE account_id = $1",
        account_id
    )
    .fetch_all(&mut *tx)
    .await?;
    let subscriptions = sqlx::query!(
        r#"SELECT s.feed_id, s.folder_id, s.position, s.title AS own_title,
                  f.feed_url, f.site_url, f.title, f.fetch_agent, f.last_seq,
                  a.seq AS "mark_seq?", a.url AS mark_url, a.published_at AS "mark_at?",
                  c.guid AS "mark_guid?"
           FROM subscriptions s
           JOIN feeds f ON f.id = s.feed_id
           LEFT JOIN articles a ON a.feed_id = s.feed_id AND a.seq = s.read_upto
           LEFT JOIN article_content c ON c.feed_id = a.feed_id AND c.seq = a.seq
           WHERE s.account_id = $1"#,
        account_id
    )
    .fetch_all(&mut *tx)
    .await?;
    let marked = sqlx::query!(
        r#"WITH own AS (
               SELECT feed_id, seq FROM article_states
               WHERE account_id = $1 AND (read OR kept_unread OR starred)
               UNION
               SELECT al.feed_id, al.seq FROM article_labels al
               JOIN labels l ON l.id = al.label_id WHERE l.account_id = $1
           )
           SELECT o.feed_id AS "feed_id!", o.seq AS "seq!", a.title, a.url, a.published_at,
                  a.author, a.thumbnail_url, c.guid, c.tags, c.enclosures, c.image_url,
                  c.image_title,
                  CASE WHEN a.seq <= s.read_upto THEN NOT coalesce(st.kept_unread, false)
                       ELSE coalesce(st.read, false) END AS "read!",
                  coalesce(st.starred, false) AS "starred!",
                  ARRAY(SELECT l.title FROM article_labels al JOIN labels l ON l.id = al.label_id
                        WHERE l.account_id = $1 AND al.feed_id = o.feed_id AND al.seq = o.seq
                        ORDER BY l.position, l.id) AS "labels!",
                  CASE WHEN coalesce(st.starred, false)
                         OR EXISTS (SELECT 1 FROM article_labels al
                                    JOIN labels l ON l.id = al.label_id
                                    WHERE l.account_id = $1 AND al.feed_id = o.feed_id
                                      AND al.seq = o.seq)
                       THEN c.html END AS html
           FROM own o
           JOIN subscriptions s ON s.account_id = $1 AND s.feed_id = o.feed_id
           JOIN articles a ON a.feed_id = o.feed_id AND a.seq = o.seq
           JOIN article_content c ON c.feed_id = o.feed_id AND c.seq = o.seq
           LEFT JOIN article_states st
               ON st.account_id = $1 AND st.feed_id = o.feed_id AND st.seq = o.seq
           ORDER BY o.feed_id, o.seq"#,
        account_id
    )
    .fetch_all(&mut *tx)
    .await?;
    let labels = sqlx::query_as!(
        ExportedLabel,
        "SELECT title, color FROM labels WHERE account_id = $1 ORDER BY position, id",
        account_id
    )
    .fetch_all(&mut *tx)
    .await?;
    let rules = sqlx::query!(
        "SELECT enabled, folder_id, feed_id, field, match AS matching, pattern, action
         FROM filter_rules WHERE account_id = $1 ORDER BY position, id",
        account_id
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    let mut articles: HashMap<i32, Vec<ExportedArticle>> = HashMap::new();
    for row in marked {
        let content = row.html.map(|html| Content {
            author: row.author,
            html,
            tags: row.tags,
            enclosures: enclosures_of(&row.enclosures),
            image_url: row.image_url,
            image_title: row.image_title,
            thumbnail_url: row.thumbnail_url,
        });
        articles
            .entry(row.feed_id)
            .or_default()
            .push(ExportedArticle {
                seq: row.seq,
                guid: row.guid,
                url: row.url,
                published_at: row.published_at,
                title: row.title,
                read: row.read,
                starred: row.starred,
                labels: row.labels,
                content,
            });
    }

    // The tree from the top down: each level in its order, folders before
    // feeds at the same place, and a folder's insides right after it.
    enum Entry {
        Folder(i32, String),
        Feed(Box<ExportedFeed>),
    }
    // A level: by position, folder or feed, number, and what it is.
    type Level = Vec<(f64, bool, i32, Entry)>;
    let mut levels: HashMap<Option<i32>, Level> = HashMap::new();
    for folder in folders {
        let entry = Entry::Folder(folder.id, folder.title);
        let level = levels.entry(folder.parent_id).or_default();
        level.push((folder.position, false, folder.id, entry));
    }
    for sub in subscriptions {
        let read_upto = match (sub.mark_seq, sub.mark_guid, sub.mark_at) {
            (Some(seq), Some(guid), Some(published_at)) => Some(ArticleRef {
                seq,
                guid,
                url: sub.mark_url,
                published_at,
            }),
            _ => None,
        };
        let feed = ExportedFeed {
            id: sub.feed_id,
            url: sub.feed_url,
            site_url: sub.site_url,
            title: sub.title,
            own_title: sub.own_title,
            folder: Vec::new(),
            agent: if sub.fetch_agent == "browser" {
                Agent::Browser
            } else {
                Agent::Rueder
            },
            last_seq: sub.last_seq,
            read_upto,
            articles: articles.remove(&sub.feed_id).unwrap_or_default(),
        };
        let level = levels.entry(sub.folder_id).or_default();
        level.push((sub.position, true, sub.feed_id, Entry::Feed(Box::new(feed))));
    }
    for level in levels.values_mut() {
        level.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    }
    let mut tree = Vec::new();
    let mut paths: HashMap<i32, Vec<String>> = HashMap::new();
    // What is still to write, the last first, with the path it is in.
    let mut waiting: Vec<(Vec<String>, Entry)> = Vec::new();
    let mut push = |waiting: &mut Vec<_>, parent: Option<i32>, path: &Vec<String>| {
        let level = levels.remove(&parent).unwrap_or_default();
        let entries = level
            .into_iter()
            .rev()
            .map(|(.., entry)| (path.clone(), entry));
        waiting.extend(entries);
    };
    push(&mut waiting, None, &Vec::new());
    while let Some((path, entry)) = waiting.pop() {
        match entry {
            Entry::Folder(id, title) => {
                let mut inside = path;
                inside.push(title);
                tree.push(Node::Folder {
                    path: inside.clone(),
                });
                push(&mut waiting, Some(id), &inside);
                paths.insert(id, inside);
            }
            Entry::Feed(mut feed) => {
                feed.folder = path;
                tree.push(Node::Feed(feed));
            }
        }
    }

    let urls: HashMap<i32, String> = tree
        .iter()
        .filter_map(|node| match node {
            Node::Feed(feed) => Some((feed.id, feed.url.clone())),
            Node::Folder { .. } => None,
        })
        .collect();
    let rules = rules
        .into_iter()
        .filter_map(|rule| {
            let scope = match (rule.folder_id, rule.feed_id) {
                (Some(folder), _) => ExportedScope::Folder {
                    path: paths.get(&folder)?.clone(),
                },
                (None, Some(feed)) => ExportedScope::Feed {
                    url: urls.get(&feed)?.clone(),
                },
                (None, None) => ExportedScope::All,
            };
            Some(ExportedRule {
                enabled: rule.enabled,
                scope,
                field: Field::parse(&rule.field)?,
                matching: Match::parse(&rule.matching)?,
                pattern: rule.pattern,
                action: if rule.action == "hide" {
                    Action::Hide
                } else {
                    Action::Read
                },
            })
        })
        .collect();

    Ok(Export {
        format: FORMAT.into(),
        version: VERSION,
        exported_at: OffsetDateTime::now_utc(),
        settings,
        tree,
        labels,
        rules,
    })
}

/// The files of an article as they are stored, without the ones that stood
/// in its text: those come again when the text is cleaned.
fn enclosures_of(stored: &serde_json::Value) -> Vec<ExportedEnclosure> {
    let stored = stored.as_array().map(Vec::as_slice).unwrap_or_default();
    stored
        .iter()
        .filter(|item| item.get("inline").and_then(|i| i.as_bool()) != Some(true))
        .filter_map(|item| {
            let url = item.get("url")?.as_str()?.to_owned();
            let kind = item
                .get("type")
                .and_then(|kind| kind.as_str())
                .map(str::to_owned);
            let length = item.get("length").and_then(|length| match length {
                serde_json::Value::String(text) => text.trim().parse().ok(),
                number => number.as_u64(),
            });
            Some(ExportedEnclosure { url, kind, length })
        })
        .collect()
}

/// `GET /api/export`: the account as a file to save.
async fn export_file(
    State(state): State<AppState>,
    session: Session,
) -> Result<Response, AppError> {
    let file = export(&state.db, session.account_id).await?;
    let day = file.exported_at.date();
    let body = serde_json::to_vec_pretty(&file)?;
    let name = format!("attachment; filename=\"rueder-{day}.json\"");
    let headers = [
        (header::CONTENT_TYPE, "application/json".to_owned()),
        (header::CONTENT_DISPOSITION, name),
    ];
    Ok((headers, body).into_response())
}

// ---------------------------------------------------------------- import

/// An article of the file, checked.
struct Wanted {
    guid: String,
    url: Option<String>,
    published_at: OffsetDateTime,
    title: String,
    read: bool,
    starred: bool,
    labels: Vec<String>,
    content: Option<Content>,
}

/// A feed of the file, checked; the n-th feed of the outline.
struct WantedFeed {
    url: String,
    site_url: Option<String>,
    own_title: Option<String>,
    agent: Agent,
    mark: Option<(String, Option<String>, OffsetDateTime)>,
    articles: Vec<Wanted>,
}

/// A file, checked and ready to be taken.
struct Plan {
    settings: Option<serde_json::Value>,
    outline: Outline,
    feeds: Vec<WantedFeed>,
    labels: Vec<(String, String)>,
    rules: Vec<ExportedRule>,
    skipped: usize,
}

/// A path of folders as names the tree takes, at most as deep as folders go.
fn folder_path(path: &[String]) -> Vec<String> {
    path.iter()
        .filter_map(|name| subscribe::name(name))
        .take(MAX_FOLDER_DEPTH)
        .collect()
}

const MAX_FOLDER_DEPTH: usize = 5;

/// Adds a folder and the folders it is in to the outline, each once.
fn add_folder(outline: &mut Outline, path: &[String]) -> Result<(), AppError> {
    for depth in 1..=path.len() {
        let above = &path[..depth];
        if !outline.folders.iter().any(|known| known == above) {
            if outline.folders.len() as i64 >= tree::MAX_FOLDERS {
                return Err(bad("this file has too many folders"));
            }
            outline.order.push(Step::Folder(outline.folders.len()));
            outline.folders.push(above.to_vec());
        }
    }
    Ok(())
}

/// A web address, or none.
fn web(url: Option<String>, rules: &Rules) -> Option<String> {
    let url = url?;
    let url = url.trim();
    if url.len() > subscribe::MAX_URL {
        return None;
    }
    let parsed = Url::parse(url).ok()?;
    let fits = matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some();
    fits.then(|| rules.clean_url(url))
}

fn cut(text: &str, at: usize) -> String {
    let text: String = text.chars().filter(|c| !c.is_control()).collect();
    text.trim().chars().take(at).collect()
}

/// Checks a file before anything is written: what breaks the limits refuses
/// the whole file, what is merely wrong is left out and counted.
fn plan(file: Export, rules: &Rules) -> Result<Plan, AppError> {
    if file.format != FORMAT {
        return Err(bad("this is not a file of a rueder account"));
    }
    if file.version != VERSION {
        return Err(bad("this file has a layout this rueder does not know"));
    }
    let settings = match file.settings {
        serde_json::Value::Null => None,
        serde_json::Value::Object(map) if map.is_empty() => None,
        settings @ serde_json::Value::Object(_) => {
            if settings.to_string().len() > tree::MAX_SETTINGS {
                return Err(bad("the settings are too large"));
            }
            Some(settings)
        }
        _ => return Err(bad("settings are an object")),
    };
    if file.labels.len() as i64 > labels::MAX_LABELS {
        return Err(bad("this file has too many labels"));
    }
    if file.rules.len() as i64 > filters::MAX_RULES {
        return Err(bad("this file has too many rules"));
    }
    let mut skipped = 0;
    let mut outline = Outline::default();
    let mut feeds = Vec::new();
    let mut seen = HashSet::new();
    let mut count = 0;
    for node in file.tree {
        match node {
            Node::Folder { path } => {
                let path = folder_path(&path);
                add_folder(&mut outline, &path)?;
            }
            Node::Feed(feed) => {
                let Ok(url) = subscribe::address(&feed.url) else {
                    skipped += 1;
                    continue;
                };
                if !seen.insert(url.clone()) {
                    skipped += 1;
                    continue;
                }
                if feeds.len() as i64 >= tree::MAX_FEEDS {
                    return Err(bad("this file has too many feeds"));
                }
                let path = folder_path(&feed.folder);
                add_folder(&mut outline, &path)?;
                count += feed.articles.len();
                if count > MAX_ARTICLES {
                    return Err(bad("this file has too many articles"));
                }
                let mut guids = HashSet::new();
                let mut articles = Vec::new();
                for article in feed.articles {
                    let fits = !article.guid.is_empty() && article.guid.len() <= MAX_GUID;
                    if !fits || !guids.insert(article.guid.clone()) {
                        skipped += 1;
                        continue;
                    }
                    let content = article
                        .content
                        .filter(|content| content.html.len() <= feeds::MAX_CONTENT);
                    articles.push(Wanted {
                        guid: article.guid,
                        url: web(article.url, rules),
                        published_at: article.published_at,
                        title: cut(&article.title, MAX_TITLE),
                        read: article.read,
                        starred: article.starred,
                        labels: article.labels,
                        content,
                    });
                }
                let mark = feed
                    .read_upto
                    .filter(|mark| !mark.guid.is_empty() && mark.guid.len() <= MAX_GUID)
                    .map(|mark| (mark.guid, web(mark.url, rules), mark.published_at));
                outline.order.push(Step::Feed(outline.feeds.len()));
                let title = subscribe::name(&feed.title);
                outline.feeds.push((path, url.clone(), title));
                feeds.push(WantedFeed {
                    url: url.into(),
                    site_url: web(feed.site_url, rules),
                    own_title: feed.own_title.as_deref().and_then(subscribe::name),
                    agent: feed.agent,
                    mark,
                    articles,
                });
            }
        }
    }
    let mut labels = Vec::new();
    for label in file.labels {
        match (tree::title(&label.title), labels::color(&label.color)) {
            (Ok(title), Ok(color)) => labels.push((title.to_owned(), color.to_owned())),
            _ => skipped += 1,
        }
    }
    Ok(Plan {
        settings,
        outline,
        feeds,
        labels,
        rules: file.rules,
        skipped,
    })
}

/// `POST /api/import`: a file of `GET /api/export`, from this rueder or
/// another, into the account.
async fn import_file(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Imported>, AppError> {
    // Said to be JSON: a form on somebody else's page cannot say that.
    let kind = headers.get(header::CONTENT_TYPE);
    let kind = kind.and_then(|kind| kind.to_str().ok()).unwrap_or_default();
    let kind = kind.split(';').next().unwrap_or_default().trim();
    if !kind.eq_ignore_ascii_case("application/json") {
        let message = "the file of an account comes as application/json";
        return Err(AppError::Status(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            message.into(),
        ));
    }
    state
        .rates
        .import(session.account_id, std::time::Instant::now())?;
    let read = tokio::task::spawn_blocking(move || serde_json::from_slice::<Export>(&body));
    let file = read
        .await?
        .map_err(|_| bad("this is not a file of a rueder account"))?;
    let rules = match &state.rules {
        Some(rules) => rules.clone(),
        None => std::sync::Arc::new(Rules::built_in()?),
    };
    let imported = import(&state.db, &rules, session.account_id, file).await?;
    Ok(Json(imported))
}

/// Takes a file into an account. See the head of this module for what that
/// means.
pub async fn import(
    db: &PgPool,
    rules: &Rules,
    account_id: i32,
    file: Export,
) -> Result<Imported, AppError> {
    let plan = plan(file, rules)?;
    let mut imported = Imported {
        skipped: plan.skipped,
        ..Imported::default()
    };
    let mut tx = db.begin().await?;
    let taken = subscribe::take(&mut tx, account_id, &plan.outline).await?;
    imported.folders = taken.folders_made;

    if let Some(settings) = &plan.settings {
        sqlx::query!(
            "UPDATE accounts SET settings = $2 WHERE id = $1",
            account_id,
            settings,
        )
        .execute(&mut *tx)
        .await?;
        imported.settings = true;
    }

    // A label is the one of that name, or a new one at the end.
    let mut label_ids: HashMap<String, i32> = sqlx::query!(
        "SELECT id, title FROM labels WHERE account_id = $1",
        account_id
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|row| (row.title, row.id))
    .collect();
    for (title, color) in &plan.labels {
        if label_ids.contains_key(title) {
            continue;
        }
        let id = sqlx::query_scalar!(
            "INSERT INTO labels (account_id, title, color, position)
             SELECT $1, $2, $3, coalesce(max(position), 0) + 1 FROM labels WHERE account_id = $1
             HAVING count(*) < $4
             RETURNING id",
            account_id,
            title,
            color,
            labels::MAX_LABELS,
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| bad("too many labels"))?;
        label_ids.insert(title.clone(), id);
        imported.labels += 1;
    }

    let mut feed_ids: HashMap<String, i32> = HashMap::new();
    for (feed, found) in plan.feeds.into_iter().zip(&taken.feeds) {
        if found.added {
            imported.feeds += 1;
        } else {
            imported.known += 1;
        }
        feed_ids.insert(feed.url.clone(), found.feed_id);
        take_feed(
            &mut tx,
            rules,
            account_id,
            found,
            feed,
            &label_ids,
            &mut imported,
        )
        .await?;
    }

    take_rules(
        &mut tx,
        account_id,
        plan.rules,
        &taken.folders,
        &feed_ids,
        &mut imported,
    )
    .await?;
    tx.commit().await?;
    // The rules decide again what is hidden, the new ones included.
    filters::reapply(db, account_id).await?;
    Ok(imported)
}

/// Where an article of the file is in a feed here, if it is.
async fn find(
    tx: &mut PgConnection,
    feed_id: i32,
    wanted: &[(&str, Option<&str>)],
) -> anyhow::Result<Vec<Option<i32>>> {
    let hashes: Vec<Vec<u8>> = wanted.iter().map(|(guid, _)| guid_hash(guid)).collect();
    let by_guid: HashMap<Vec<u8>, i32> = sqlx::query!(
        "SELECT guid_hash, seq FROM article_content WHERE feed_id = $1 AND guid_hash = ANY($2)",
        feed_id,
        &hashes,
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|row| (row.guid_hash, row.seq))
    .collect();
    // By its page only where no other article of the feed has the same.
    let urls: Vec<String> = wanted
        .iter()
        .zip(&hashes)
        .filter(|(_, hash)| !by_guid.contains_key(*hash))
        .filter_map(|((_, url), _)| url.map(str::to_owned))
        .collect();
    let by_url: HashMap<String, i32> = if urls.is_empty() {
        HashMap::new()
    } else {
        sqlx::query!(
            r#"SELECT url AS "url!", min(seq) AS "seq!" FROM articles
               WHERE feed_id = $1 AND url = ANY($2)
               GROUP BY url HAVING count(*) = 1"#,
            feed_id,
            &urls,
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|row| (row.url, row.seq))
        .collect()
    };
    Ok(wanted
        .iter()
        .zip(&hashes)
        .map(|((_, url), hash)| {
            by_guid
                .get(hash)
                .or_else(|| url.and_then(|url| by_url.get(url)))
                .copied()
        })
        .collect())
}

/// An article of the file as one to store, for one that is not here.
fn carried(wanted: &Wanted, content: &Content, feed_url: &str, rules: &Rules) -> NewArticle {
    let base = Base {
        article: wanted.url.as_deref(),
        feed: Some(feed_url),
    };
    // The text was cleaned where it comes from, but that was another server.
    let cleaned = cleaning::clean(&content.html, base, rules);
    let mut enclosures: Vec<serde_json::Value> = content
        .enclosures
        .iter()
        .take(MAX_ENCLOSURES)
        .filter_map(|file| {
            let url = web(Some(file.url.clone()), rules)?;
            let kind = file.kind.as_deref().map(|kind| cut(kind, 100));
            Some(serde_json::json!({ "url": url, "type": kind, "length": file.length }))
        })
        .collect();
    feeds::add_inline(&mut enclosures, cleaned.enclosures);
    let author = cut(&content.author, MAX_AUTHOR);
    let mut tags: Vec<String> = Vec::new();
    for tag in &content.tags {
        let tag = cut(tag, 100);
        if !tag.is_empty() && !tags.contains(&tag) && tags.len() < MAX_TAGS {
            tags.push(tag);
        }
    }
    NewArticle {
        guid: wanted.guid.clone(),
        published_at: wanted.published_at,
        fetched_at: OffsetDateTime::now_utc(),
        teaser: feeds::teaser(&cleaned.html),
        thumbnail_url: web(content.thumbnail_url.clone(), rules),
        html: cleaned.html,
        enclosures: enclosures.into(),
        image_url: web(content.image_url.clone(), rules),
        image_title: content
            .image_title
            .as_deref()
            .map(|title| cut(title, MAX_TITLE)),
        original: Original {
            title: wanted.title.clone(),
            content: content.html.clone(),
            authors: if author.is_empty() {
                Vec::new()
            } else {
                vec![author]
            },
            tags,
            url: wanted.url.clone(),
        },
    }
}

/// What the account did in one feed of the file.
async fn take_feed(
    tx: &mut PgConnection,
    rules: &Rules,
    account_id: i32,
    found: &subscribe::TakenFeed,
    feed: WantedFeed,
    label_ids: &HashMap<String, i32>,
    imported: &mut Imported,
) -> Result<(), AppError> {
    let feed_id = found.feed_id;
    if found.created {
        let agent = match feed.agent {
            Agent::Rueder => "rueder",
            Agent::Browser => "browser",
        };
        sqlx::query!(
            "UPDATE feeds SET fetch_agent = $2, site_url = coalesce(site_url, $3) WHERE id = $1",
            feed_id,
            agent,
            feed.site_url,
        )
        .execute(&mut *tx)
        .await?;
    }
    if found.added && feed.own_title.is_some() {
        sqlx::query!(
            "UPDATE subscriptions SET title = $3 WHERE account_id = $1 AND feed_id = $2",
            account_id,
            feed_id,
            feed.own_title,
        )
        .execute(&mut *tx)
        .await?;
    }

    // Where the articles are here, and the ones that are not but bring
    // their text along.
    let keys: Vec<(&str, Option<&str>)> = feed
        .articles
        .iter()
        .map(|article| (article.guid.as_str(), article.url.as_deref()))
        .collect();
    let mut seqs = find(&mut *tx, feed_id, &keys).await?;
    let mut new: Vec<(usize, NewArticle)> = Vec::new();
    for (at, article) in feed.articles.iter().enumerate() {
        match (&seqs[at], &article.content) {
            (Some(_), _) => imported.articles += 1,
            (None, Some(content)) => new.push((at, carried(article, content, &feed.url, rules))),
            (None, None) => imported.missing += 1,
        }
    }
    if !new.is_empty() {
        new.sort_by_key(|(_, article)| article.published_at);
        let articles: Vec<NewArticle> = new.iter().map(|(_, article)| article.clone()).collect();
        let first = articles::insert(&mut *tx, feed_id, &articles).await?;
        for (n, (at, _)) in new.iter().enumerate() {
            seqs[*at] = Some(first + n as i32);
        }
        imported.carried += new.len();
        let last = first + new.len() as i32 - 1;
        // To the others who read the feed they are old news, not new.
        let others = sqlx::query!(
            "SELECT account_id, read_upto FROM subscriptions
             WHERE feed_id = $1 AND account_id <> $2",
            feed_id,
            account_id,
        )
        .fetch_all(&mut *tx)
        .await?;
        for other in others {
            sqlx::query!(
                "INSERT INTO article_states (account_id, feed_id, seq, read)
                 SELECT $1, $2, g.seq, true FROM generate_series($3::int4, $4::int4) AS g(seq)
                 WHERE g.seq > $5
                 ON CONFLICT DO NOTHING",
                other.account_id,
                feed_id,
                first,
                last,
                other.read_upto,
            )
            .execute(&mut *tx)
            .await?;
            reading::compact(&mut *tx, other.account_id, feed_id, other.read_upto).await?;
        }
        filters::apply_new(&mut *tx, feed_id, first).await?;
    }

    // The mark: at the article it stood at, or else by its time.
    let state = sqlx::query!(
        "SELECT f.last_seq, f.last_success_at IS NULL AS \"never!\", s.read_upto
         FROM feeds f JOIN subscriptions s ON s.feed_id = f.id AND s.account_id = $2
         WHERE f.id = $1",
        feed_id,
        account_id,
    )
    .fetch_one(&mut *tx)
    .await?;
    let mark = match &feed.mark {
        None => 0,
        Some((guid, url, at)) => {
            let at_mark = find(&mut *tx, feed_id, &[(guid.as_str(), url.as_deref())]).await?;
            match at_mark.first().copied().flatten() {
                Some(seq) => seq,
                None => {
                    let young = sqlx::query_scalar!(
                        "SELECT min(seq) FROM articles WHERE feed_id = $1 AND published_at > $2",
                        feed_id,
                        at,
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    young.map_or(state.last_seq, |young| young - 1)
                }
            }
        }
    };
    if found.added {
        // What the subscription started with gives way to what the file says;
        // the rules decide again at the end what is hidden.
        sqlx::query!(
            "DELETE FROM article_states WHERE account_id = $1 AND feed_id = $2",
            account_id,
            feed_id,
        )
        .execute(&mut *tx)
        .await?;
        // A feed that was never fetched here keeps the time of the mark for
        // its first fetch.
        let before = feed
            .mark
            .as_ref()
            .filter(|_| state.never)
            .map(|(_, _, at)| *at);
        sqlx::query!(
            "UPDATE subscriptions SET read_upto = $3, read_before = $4
             WHERE account_id = $1 AND feed_id = $2",
            account_id,
            feed_id,
            mark,
            before,
        )
        .execute(&mut *tx)
        .await?;
    } else if mark > state.read_upto {
        reading::mark_all_read_on(&mut *tx, account_id, feed_id, mark).await?;
    }
    let mark = sqlx::query_scalar!(
        "SELECT read_upto FROM subscriptions WHERE account_id = $1 AND feed_id = $2",
        account_id,
        feed_id,
    )
    .fetch_one(&mut *tx)
    .await?;

    // The exceptions from the mark, the stars and the labels. A feed the
    // account read already keeps what it has and only gains.
    let mut flags: BTreeMap<i32, (bool, bool, bool)> = BTreeMap::new();
    let mut labelled: HashSet<(i32, i32)> = HashSet::new();
    for (article, seq) in feed.articles.iter().zip(&seqs) {
        let Some(seq) = *seq else { continue };
        let read = article.read && seq > mark;
        let kept_unread = !article.read && seq <= mark && found.added;
        let entry = flags.entry(seq).or_default();
        *entry = (
            entry.0 || read,
            entry.1 || kept_unread,
            entry.2 || article.starred,
        );
        for label in &article.labels {
            match label_ids.get(label) {
                Some(&id) => {
                    labelled.insert((id, seq));
                }
                None => imported.skipped += 1,
            }
        }
    }
    flags.retain(|_, (read, kept, starred)| *read || *kept || *starred);
    let (mut at, mut read, mut kept, mut starred) = (vec![], vec![], vec![], vec![]);
    for (seq, flag) in flags {
        at.push(seq);
        // Read wins where two articles of the file turned out to be one.
        read.push(flag.0);
        kept.push(flag.1 && !flag.0);
        starred.push(flag.2);
    }
    if !at.is_empty() {
        sqlx::query!(
            "INSERT INTO article_states (account_id, feed_id, seq, read, kept_unread, starred)
             SELECT $1, $2, u.seq, u.read, u.kept, u.starred
             FROM unnest($3::int4[], $4::bool[], $5::bool[], $6::bool[])
                  AS u(seq, read, kept, starred)
             ON CONFLICT (account_id, feed_id, seq) DO UPDATE
             SET read = article_states.read OR excluded.read,
                 kept_unread = (article_states.kept_unread OR excluded.kept_unread)
                               AND NOT (article_states.read OR excluded.read),
                 starred = article_states.starred OR excluded.starred",
            account_id,
            feed_id,
            &at,
            &read,
            &kept,
            &starred,
        )
        .execute(&mut *tx)
        .await?;
        reading::compact(&mut *tx, account_id, feed_id, mark).await?;
    }
    if !labelled.is_empty() {
        let (ids, seqs): (Vec<i32>, Vec<i32>) = labelled.into_iter().unzip();
        sqlx::query!(
            "INSERT INTO article_labels (label_id, feed_id, seq)
             SELECT u.id, $1, u.seq FROM unnest($2::int4[], $3::int4[]) AS u(id, seq)
             ON CONFLICT DO NOTHING",
            feed_id,
            &ids,
            &seqs,
        )
        .execute(&mut *tx)
        .await?;
    }
    Ok(())
}

/// The rules of the file that the account does not have yet, after its own.
async fn take_rules(
    tx: &mut PgConnection,
    account_id: i32,
    rules: Vec<ExportedRule>,
    folders: &HashMap<Vec<String>, i32>,
    feed_ids: &HashMap<String, i32>,
    imported: &mut Imported,
) -> Result<(), AppError> {
    if rules.is_empty() {
        return Ok(());
    }
    type Key = (Option<i32>, Option<i32>, String, String, String, String);
    let mut have: HashSet<Key> = sqlx::query!(
        "SELECT folder_id, feed_id, field, match AS matching, pattern, action
         FROM filter_rules WHERE account_id = $1",
        account_id
    )
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|row| {
        let key = (row.field, row.matching, row.pattern, row.action);
        (row.folder_id, row.feed_id, key.0, key.1, key.2, key.3)
    })
    .collect();
    for rule in rules {
        let pattern = rule.pattern.trim().to_owned();
        if filters::compile(rule.matching, &pattern).is_err() {
            imported.skipped += 1;
            continue;
        }
        let scope = match &rule.scope {
            ExportedScope::All => Some((None, None)),
            ExportedScope::Folder { path } => {
                folders.get(&folder_path(path)).map(|id| (Some(*id), None))
            }
            ExportedScope::Feed { url } => subscribe::address(url)
                .ok()
                .and_then(|url| feed_ids.get(url.as_str()))
                .map(|id| (None, Some(*id))),
        };
        let Some((folder_id, feed_id)) = scope else {
            imported.skipped += 1;
            continue;
        };
        let key: Key = (
            folder_id,
            feed_id,
            rule.field.as_str().to_owned(),
            rule.matching.as_str().to_owned(),
            pattern.clone(),
            rule.action.as_str().to_owned(),
        );
        if !have.insert(key) {
            continue;
        }
        sqlx::query_scalar!(
            "INSERT INTO filter_rules
                 (account_id, position, enabled, folder_id, feed_id, field, match, pattern, action)
             SELECT $1, coalesce(max(position), 0) + 1, $2, $3, $4, $5, $6, $7, $8
             FROM filter_rules WHERE account_id = $1
             HAVING count(*) < $9
             RETURNING id",
            account_id,
            rule.enabled,
            folder_id,
            feed_id,
            rule.field.as_str(),
            rule.matching.as_str(),
            pattern,
            rule.action.as_str(),
            filters::MAX_RULES,
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| bad("too many rules"))?;
        imported.rules += 1;
    }
    Ok(())
}
