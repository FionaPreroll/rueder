//! Subscribing: finding the feeds of an address, subscribing to one, and
//! taking a list of subscriptions in and out as OPML.
//!
//! An address a reader types is fetched by the hardened HTTP client and by
//! nothing else. A feed that is new is fetched before anybody subscribes to
//! it, so no address becomes a feed without being one. An OPML file is the
//! exception: its feeds are taken on trust and fetched by the next round,
//! and the ones that are none show their error in the tree.

use std::{collections::HashMap, sync::Arc};

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use quick_xml::{Reader, XmlVersion, events::BytesStart, events::Event};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use time::OffsetDateTime;
use url::Url;

use crate::{
    AppError, AppState,
    cleaning::{self, Rules},
    feeds,
    fetch::{self, Outcome},
    filters,
    http_client::{self, HttpClient, Request},
    sessions::Session,
    tree::{self, bad},
};

/// The longest address of a feed or a page, in bytes.
pub const MAX_URL: usize = 2000;
/// How many feeds one OPML file can bring.
pub const MAX_OPML_FEEDS: usize = 2000;
/// Who subscribes to a feed that others read already finds this many of its
/// newest articles unread, and the rest read.
pub const NEWEST_UNREAD: i32 = 20;
/// How much of a web page the search for feeds reads, in bytes. The links to
/// feeds are in the head of a page.
pub const MAX_PAGE: usize = 512 * 1024;
/// The largest OPML file, in bytes.
pub const MAX_OPML: usize = 1024 * 1024;
/// How deep the outlines of an OPML file may nest. Folders nest less deep;
/// what is deeper than they go lands in the deepest folder.
const MAX_OPML_DEPTH: usize = 32;
/// How deep folders nest; the database has the last word.
const MAX_FOLDER_DEPTH: usize = 5;

const ACCEPT: &str = "application/atom+xml, application/rss+xml, application/feed+json, \
                      application/xml;q=0.9, text/html;q=0.8, */*;q=0.5";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/feeds", post(subscribe))
        .route("/feeds/find", post(find))
        .route(
            "/opml",
            get(export)
                .post(import)
                .layer(DefaultBodyLimit::max(MAX_OPML)),
        )
}

/// The address a reader typed, as one that can be fetched. Without a scheme
/// it is taken as https.
fn address(text: &str) -> Result<Url, AppError> {
    let text = text.trim();
    if text.is_empty() || text.len() > MAX_URL || text.chars().any(char::is_control) {
        return Err(bad("this is not an address"));
    }
    let url = match text.split_once("://") {
        Some(_) => Url::parse(text),
        None => Url::parse(&format!("https://{text}")),
    };
    let url = url.map_err(|_| bad("this is not an address"))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(bad("this is not an address of a web page or a feed"));
    }
    // A feed belongs to everybody who reads it, and so would a password in
    // its address.
    if !url.username().is_empty() || url.password().is_some() {
        return Err(bad(
            "an address with a name and a password cannot be subscribed to",
        ));
    }
    Ok(url)
}

fn client(state: &AppState) -> Result<&HttpClient, AppError> {
    let none = || {
        let message = "this server does not fetch feeds";
        AppError::Status(StatusCode::SERVICE_UNAVAILABLE, message.into())
    };
    state.client.as_ref().ok_or_else(none)
}

fn rules(state: &AppState) -> anyhow::Result<Arc<Rules>> {
    match &state.rules {
        Some(rules) => Ok(rules.clone()),
        None => Ok(Arc::new(Rules::built_in()?)),
    }
}

#[derive(Deserialize)]
struct Find {
    url: String,
}

/// A feed found at an address.
#[derive(Debug, PartialEq, Eq, Serialize)]
struct Candidate {
    url: String,
    title: String,
}

#[derive(Serialize)]
struct Found {
    feeds: Vec<Candidate>,
}

/// What is at an address: the feed itself, or the feeds a web page points to.
fn candidates(response: &http_client::Response, rules: &Rules) -> Vec<Candidate> {
    let now = OffsetDateTime::now_utc();
    if let Ok(feed) = feeds::parse(&response.body, response.url.as_str(), now, rules) {
        return vec![Candidate {
            url: response.url.to_string(),
            title: feed.title.unwrap_or_default(),
        }];
    }
    let page = &response.body[..response.body.len().min(MAX_PAGE)];
    let links = cleaning::feed_links(&String::from_utf8_lossy(page), &response.url);
    let candidate = |link: cleaning::FeedLink| Candidate {
        url: link.url.into(),
        title: link.title,
    };
    links.into_iter().map(candidate).collect()
}

/// Looks at an address for feeds. An address that is a feed is the one feed;
/// a web page has the feeds it names, which nobody has looked at yet.
async fn find(
    State(state): State<AppState>,
    _session: Session,
    Json(find): Json<Find>,
) -> Result<Json<Found>, AppError> {
    let url = address(&find.url)?;
    let request = Request {
        accept: Some(ACCEPT),
        ..Request::new(url.as_str())
    };
    let response = client(&state)?
        .get(request)
        .await
        .map_err(|err| AppError::BadRequest(format!("the address cannot be fetched: {err}")))?;
    if !response.status.is_success() {
        let message = format!("the site answers with {}", response.status);
        return Err(AppError::BadRequest(message));
    }
    let rules = rules(&state)?;
    // Reading a feed or a page takes a while.
    let feeds = tokio::task::spawn_blocking(move || candidates(&response, &rules)).await?;
    Ok(Json(Found { feeds }))
}

#[derive(Deserialize)]
struct New {
    url: String,
    folder_id: Option<i32>,
}

#[derive(Serialize)]
struct Subscribed {
    id: i32,
}

/// The feed at this address that is there already: the oldest, as the store
/// does not rule out two.
async fn known(
    conn: &mut PgConnection,
    url: &str,
    before: Option<i32>,
) -> sqlx::Result<Option<i32>> {
    sqlx::query_scalar!(
        "SELECT id FROM feeds WHERE feed_url = $1 AND ($2::int4 IS NULL OR id < $2)
         ORDER BY id LIMIT 1",
        url,
        before,
    )
    .fetch_optional(conn)
    .await
}

/// Makes a feed of an address: fetches it, and keeps it only if that worked.
async fn create(state: &AppState, url: &Url) -> Result<i32, AppError> {
    let (client, rules) = (client(state)?, rules(state)?);
    let id = sqlx::query_scalar!(
        "INSERT INTO feeds (feed_url) VALUES ($1) RETURNING id",
        url.as_str()
    )
    .fetch_one(&state.db)
    .await?;
    let outcome = fetch::fetch_feed(&state.db, client, &rules, id).await;
    let mut conn = state.db.acquire().await?;
    // Somebody may have made the same feed meanwhile: the older one stays.
    let older = match &outcome {
        Ok(Outcome::Fetched(_) | Outcome::NotModified) => {
            known(&mut conn, url.as_str(), Some(id)).await?
        }
        _ => None,
    };
    if older.is_some() || !matches!(outcome, Ok(Outcome::Fetched(_) | Outcome::NotModified)) {
        sqlx::query!(
            "DELETE FROM feeds WHERE id = $1
             AND NOT EXISTS (SELECT 1 FROM subscriptions WHERE feed_id = $1)",
            id
        )
        .execute(&mut *conn)
        .await?;
    }
    match (outcome?, older) {
        (Outcome::Failed(reason), _) => {
            let message = format!("this is not a feed that can be read: {reason}");
            Err(AppError::BadRequest(message))
        }
        (_, Some(older)) => Ok(older),
        (_, None) => Ok(id),
    }
}

/// Subscribes an account to a feed, at the end of a folder or of the top
/// level. `false` if it reads the feed already. The caller holds the lock of
/// the account's tree.
///
/// Of a feed with a history only the newest articles are unread; a feed that
/// is `fresh` brings what its first fetch finds.
async fn add(
    conn: &mut PgConnection,
    account_id: i32,
    feed_id: i32,
    folder: Option<i32>,
    fresh: bool,
) -> Result<bool, AppError> {
    let reads = sqlx::query_scalar!(
        "SELECT feed_id FROM subscriptions WHERE account_id = $1 AND feed_id = $2",
        account_id,
        feed_id
    )
    .fetch_optional(&mut *conn)
    .await?;
    if reads.is_some() {
        return Ok(false);
    }
    tree::room_for_feed(conn, account_id, folder).await?;
    let position = tree::end_of(conn, account_id, folder).await?;
    sqlx::query!(
        "INSERT INTO subscriptions (account_id, feed_id, folder_id, position, read_upto)
         SELECT $1, f.id, $3, $4, CASE WHEN $5 THEN 0 ELSE greatest(f.last_seq - $6, 0) END
         FROM feeds f
         WHERE f.id = $2",
        account_id,
        feed_id,
        folder,
        position,
        fresh,
        NEWEST_UNREAD,
    )
    .execute(&mut *conn)
    .await?;
    filters::subscribed(conn, account_id, feed_id).await?;
    Ok(true)
}

async fn own_folder(conn: &mut PgConnection, account_id: i32, id: i32) -> Result<(), AppError> {
    let own = sqlx::query_scalar!(
        "SELECT id FROM folders WHERE account_id = $1 AND id = $2",
        account_id,
        id
    )
    .fetch_optional(conn)
    .await?;
    own.map(|_| ()).ok_or(AppError::NotFound)
}

/// Subscribes to the feed at an address: 201 with its number, or 200 if the
/// reader has it already. The address has to be the feed itself; `find` says
/// which feeds a web page has.
async fn subscribe(
    State(state): State<AppState>,
    session: Session,
    Json(new): Json<New>,
) -> Result<(StatusCode, Json<Subscribed>), AppError> {
    let account_id = session.account_id;
    let url = address(&new.url)?;
    let mut conn = state.db.acquire().await?;
    if let Some(folder) = new.folder_id {
        own_folder(&mut conn, account_id, folder).await?;
    }
    // A feed is known by its address as it is written here, or as the old
    // rueder wrote it down.
    let mut feed = known(&mut conn, url.as_str(), None).await?;
    if feed.is_none() && new.url.trim() != url.as_str() {
        feed = known(&mut conn, new.url.trim(), None).await?;
    }
    drop(conn);
    let (id, fresh) = match feed {
        Some(id) => (id, false),
        None => (create(&state, &url).await?, true),
    };
    let mut tx = state.db.begin().await?;
    tree::lock(&mut tx, account_id).await?;
    if let Some(folder) = new.folder_id {
        own_folder(&mut tx, account_id, folder).await?;
    }
    let added = add(&mut tx, account_id, id, new.folder_id, fresh).await?;
    tx.commit().await?;
    let status = if added {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(Subscribed { id })))
}

/// A text as it stands in an XML attribute. Characters XML has no way to
/// write are left out.
fn xml(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for letter in text.chars() {
        match letter {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\t' | '\n' | '\r' => escaped.push(' '),
            '\u{FFFE}' | '\u{FFFF}' => {}
            _ if letter.is_control() => {}
            _ => escaped.push(letter),
        }
    }
    escaped
}

/// A node of the tree on its way into an OPML file.
enum Entry {
    Folder(i32),
    Feed { url: String, site: Option<String> },
}

/// The account's subscriptions as an OPML file, folders as outlines around
/// their feeds.
async fn export(State(state): State<AppState>, session: Session) -> Result<Response, AppError> {
    let rows = sqlx::query!(
        r#"SELECT true AS "folder!", id AS "id!", parent_id AS parent, title AS "title!",
                  position AS "position!", NULL::text AS feed_url, NULL::text AS site_url
           FROM folders WHERE account_id = $1
           UNION ALL
           SELECT false, s.feed_id, s.folder_id, coalesce(s.title, nullif(f.title, ''), f.feed_url),
                  s.position, f.feed_url, f.site_url
           FROM subscriptions s JOIN feeds f ON f.id = s.feed_id WHERE s.account_id = $1
           ORDER BY 5, 1 DESC, 2"#,
        session.account_id
    )
    .fetch_all(&state.db)
    .await?;
    let mut levels: HashMap<Option<i32>, Vec<(String, Entry)>> = HashMap::new();
    for row in rows {
        let entry = match (row.folder, row.feed_url) {
            (false, Some(url)) => Entry::Feed {
                url,
                site: row.site_url,
            },
            _ => Entry::Folder(row.id),
        };
        levels
            .entry(row.parent)
            .or_default()
            .push((row.title, entry));
    }

    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<opml version=\"2.0\">\n  <head>\n    \
         <title>rueder</title>\n  </head>\n  <body>\n",
    );
    // What is still to write, the last first; `None` closes a folder.
    let mut waiting: Vec<(usize, Option<(String, Entry)>)> = Vec::new();
    let mut push = |waiting: &mut Vec<_>, parent: Option<i32>, depth: usize| {
        let level = levels.remove(&parent).unwrap_or_default();
        waiting.extend(level.into_iter().rev().map(|node| (depth, Some(node))));
    };
    push(&mut waiting, None, 0);
    while let Some((depth, node)) = waiting.pop() {
        let indent = "  ".repeat(depth + 2);
        match node {
            None => out.push_str(&format!("{indent}</outline>\n")),
            Some((title, Entry::Folder(id))) => {
                let title = xml(&title);
                out.push_str(&format!(
                    "{indent}<outline text=\"{title}\" title=\"{title}\">\n"
                ));
                waiting.push((depth, None));
                push(&mut waiting, Some(id), depth + 1);
            }
            Some((title, Entry::Feed { url, site })) => {
                let (title, url) = (xml(&title), xml(&url));
                let site = site.map(|site| format!(" htmlUrl=\"{}\"", xml(&site)));
                let site = site.unwrap_or_default();
                out.push_str(&format!(
                    "{indent}<outline type=\"rss\" text=\"{title}\" title=\"{title}\" \
                     xmlUrl=\"{url}\"{site}/>\n"
                ));
            }
        }
    }
    out.push_str("  </body>\n</opml>\n");
    let headers = [
        (header::CONTENT_TYPE, "text/x-opml; charset=utf-8"),
        (
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"rueder.opml\"",
        ),
    ];
    Ok((headers, out).into_response())
}

/// What an OPML file asks for: folders, and feeds in them.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outline {
    /// The folders by the names from the top level down, each once, a folder
    /// after the one it is in.
    pub folders: Vec<Vec<String>>,
    /// The feeds in the order of the file, each address once: the folder it
    /// is in (empty at the top level), its address and its name.
    pub feeds: Vec<(Vec<String>, Url, Option<String>)>,
    /// Both in the order of the file, by their place in the lists above.
    pub order: Vec<Step>,
    /// Outlines that were left out: a feed without an address that can be
    /// fetched, or one beyond [`MAX_OPML_FEEDS`].
    pub skipped: usize,
}

/// A folder or a feed of an [`Outline`], by its place in the list of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Folder(usize),
    Feed(usize),
}

/// A name from an OPML file as a name in the tree: within the length, without
/// control characters, or none.
fn name(text: &str) -> Option<String> {
    let name: String = text.chars().filter(|c| !c.is_control()).collect();
    let name: String = name.trim().chars().take(tree::MAX_TITLE).collect();
    let name = name.trim_end();
    (!name.is_empty()).then(|| name.to_owned())
}

fn attributes(outline: &BytesStart<'_>) -> HashMap<String, String> {
    let mut found = HashMap::new();
    // An attribute that cannot be read is as good as not there.
    for attribute in outline.attributes().flatten() {
        let key = attribute.key.as_ref().to_ascii_lowercase();
        if let Ok(value) = attribute.normalized_value(XmlVersion::Implicit1_0) {
            found.entry(key).or_insert_with(|| value.into_owned());
        }
    }
    found
}

/// Reads an OPML file. An outline with an address (`xmlUrl`) is a feed; one
/// without is a folder, named by `text` or `title`, and holds what is in it.
/// Folders deeper than the tree allows give what they hold to the deepest
/// folder that fits.
pub fn outline(file: &str) -> Result<Outline, &'static str> {
    let mut reader = Reader::from_str(file);
    let mut result = Outline::default();
    // For every outline that is open: whether it added a folder to the path.
    let mut open: Vec<bool> = Vec::new();
    let mut path: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    loop {
        let event = reader
            .read_event()
            .map_err(|_| "this is not an OPML file")?;
        let (start, closed) = match event {
            Event::Start(start) => (start, false),
            Event::Empty(start) => (start, true),
            Event::End(end) => {
                if end.name().as_ref().eq_ignore_ascii_case("outline") && open.pop() == Some(true) {
                    path.pop();
                }
                continue;
            }
            Event::Eof => break,
            _ => continue,
        };
        if !start.name().as_ref().eq_ignore_ascii_case("outline") {
            continue;
        }
        if open.len() >= MAX_OPML_DEPTH {
            return Err("the outlines of this file nest too deep");
        }
        let attributes = attributes(&start);
        let title = ["text", "title"]
            .iter()
            .find_map(|key| attributes.get(*key).and_then(|text| name(text)));
        let mut is_folder = false;
        match attributes.get("xmlurl") {
            Some(url) => match address(url) {
                Ok(url) if result.feeds.len() >= MAX_OPML_FEEDS => {
                    drop(url);
                    result.skipped += 1;
                }
                Ok(url) => {
                    if seen.insert(url.clone()) {
                        result.order.push(Step::Feed(result.feeds.len()));
                        result.feeds.push((path.clone(), url, title));
                    }
                }
                Err(_) => result.skipped += 1,
            },
            None => {
                if let Some(title) = title
                    && !closed
                    && path.len() < MAX_FOLDER_DEPTH
                {
                    path.push(title);
                    is_folder = true;
                    if !result.folders.contains(&path) {
                        if result.folders.len() as i64 >= tree::MAX_FOLDERS {
                            return Err("this file has too many folders");
                        }
                        result.order.push(Step::Folder(result.folders.len()));
                        result.folders.push(path.clone());
                    }
                }
            }
        }
        if !closed {
            open.push(is_folder);
        }
    }
    Ok(result)
}

/// What an import did.
#[derive(Serialize)]
struct Imported {
    /// Feeds the account reads now and did not before.
    feeds: usize,
    /// Folders that were made.
    folders: usize,
    /// Feeds of the file the account read already.
    known: usize,
    /// Outlines that were left out, see [`Outline::skipped`].
    skipped: usize,
}

/// Takes the subscriptions of an OPML file, sent as the body. Folders are
/// found by their name or made; feeds the account reads already stay where
/// they are. The feeds are not fetched here: the next round does that.
async fn import(
    State(state): State<AppState>,
    session: Session,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Imported>, AppError> {
    let account_id = session.account_id;
    // Said to be XML: a form on somebody else's page cannot say that.
    let kind = headers.get(header::CONTENT_TYPE);
    let kind = kind.and_then(|kind| kind.to_str().ok()).unwrap_or_default();
    let kind = kind.split(';').next().unwrap_or_default().trim();
    if !["text/x-opml", "text/xml", "application/xml"]
        .iter()
        .any(|known| kind.eq_ignore_ascii_case(known))
    {
        let message = "an OPML file comes as text/x-opml or XML";
        return Err(AppError::Status(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            message.into(),
        ));
    }
    let read = tokio::task::spawn_blocking(move || outline(&String::from_utf8_lossy(&body)));
    let outline = read.await?.map_err(bad)?;
    let mut tx = state.db.begin().await?;
    tree::lock(&mut tx, account_id).await?;
    let mut imported = Imported {
        feeds: 0,
        folders: 0,
        known: 0,
        skipped: outline.skipped,
    };

    // A folder is the one of that name in that place, or a new one.
    let mut ids: HashMap<Vec<String>, i32> = HashMap::new();
    let have = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM folders WHERE account_id = $1"#,
        account_id
    )
    .fetch_one(&mut *tx)
    .await?;
    for step in &outline.order {
        match *step {
            Step::Folder(at) => {
                let Some((title, above)) = outline.folders.get(at).and_then(|p| p.split_last())
                else {
                    continue;
                };
                let parent = ids.get(above).copied();
                let existing = sqlx::query_scalar!(
                    "SELECT id FROM folders
                     WHERE account_id = $1 AND parent_id IS NOT DISTINCT FROM $2 AND title = $3
                     ORDER BY position, id LIMIT 1",
                    account_id,
                    parent,
                    title,
                )
                .fetch_optional(&mut *tx)
                .await?;
                let id = match existing {
                    Some(id) => id,
                    None => {
                        if have + imported.folders as i64 >= tree::MAX_FOLDERS {
                            return Err(bad("too many folders"));
                        }
                        imported.folders += 1;
                        let position = tree::end_of(&mut tx, account_id, parent).await?;
                        sqlx::query_scalar!(
                            "INSERT INTO folders (account_id, parent_id, title, position)
                             VALUES ($1, $2, $3, $4) RETURNING id",
                            account_id,
                            parent,
                            title,
                            position,
                        )
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(tree::rule)?
                    }
                };
                ids.insert(outline.folders[at].clone(), id);
            }
            Step::Feed(at) => {
                let Some((path, url, title)) = outline.feeds.get(at) else {
                    continue;
                };
                let feed = match known(&mut tx, url.as_str(), None).await? {
                    Some(id) => id,
                    None => {
                        sqlx::query_scalar!(
                            "INSERT INTO feeds (feed_url, title) VALUES ($1, $2) RETURNING id",
                            url.as_str(),
                            title.as_deref().unwrap_or_default(),
                        )
                        .fetch_one(&mut *tx)
                        .await?
                    }
                };
                if add(&mut tx, account_id, feed, ids.get(path).copied(), false).await? {
                    imported.feeds += 1;
                } else {
                    imported.known += 1;
                }
            }
        }
    }
    tx.commit().await?;
    Ok(Json(imported))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    fn names(path: &[&str]) -> Vec<String> {
        path.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn an_address_is_of_the_web_and_without_a_password() {
        let read = |text: &str| address(text).ok().map(String::from);
        assert_eq!(
            read(" blog.example/feed "),
            Some("https://blog.example/feed".into())
        );
        assert_eq!(
            read("http://blog.example"),
            Some("http://blog.example/".into())
        );
        assert_eq!(
            read("HTTPS://Blog.Example/A b"),
            Some("https://blog.example/A%20b".into())
        );
        for wrong in [
            "",
            "   ",
            "ftp://blog.example/feed",
            "file:///etc/passwd",
            "javascript://blog.example/%0aalert(1)",
            "https://ada:secret@blog.example/feed",
            "https://ada@blog.example/feed",
            "https://",
            "https://blog.example/\nfeed",
        ] {
            assert_eq!(read(wrong), None, "{wrong}");
        }
        assert_eq!(
            read(&format!("https://blog.example/{}", "a".repeat(MAX_URL))),
            None
        );
    }

    #[test]
    fn an_opml_file_gives_folders_and_feeds() {
        let file = r#"<?xml version="1.0" encoding="UTF-8"?>
            <opml version="2.0"><head><title>Mine</title></head><body>
              <outline text="News" title="ignored">
                <outline type="rss" text="The &amp; Example" xmlUrl="https://news.example/feed" htmlUrl="https://news.example/"/>
                <outline text="Deeper">
                  <outline type="rss" xmlUrl="https://deep.example/atom.xml"></outline>
                </outline>
                <outline text="Empty"></outline>
              </outline>
              <outline TYPE="rss" TITLE="Top" XMLURL="blog.example/rss"/>
              <outline text="News"><outline text="Again" xmlUrl="https://news.example/feed"/></outline>
              <outline text="No address" type="rss"/>
              <outline text="Not the web" xmlUrl="ftp://files.example/feed"/>
            </body></opml>"#;
        let read = outline(file).unwrap();
        assert_eq!(
            read.folders,
            [
                names(&["News"]),
                names(&["News", "Deeper"]),
                names(&["News", "Empty"])
            ]
        );
        assert_eq!(
            read.feeds,
            [
                (
                    names(&["News"]),
                    url("https://news.example/feed"),
                    Some("The & Example".into())
                ),
                (
                    names(&["News", "Deeper"]),
                    url("https://deep.example/atom.xml"),
                    None
                ),
                (
                    names(&[]),
                    url("https://blog.example/rss"),
                    Some("Top".into())
                ),
            ]
        );
        assert_eq!(read.skipped, 1);
        // Both in the order of the file.
        use Step::{Feed, Folder};
        assert_eq!(
            read.order,
            [Folder(0), Feed(0), Folder(1), Feed(1), Folder(2), Feed(2)]
        );
    }

    #[test]
    fn folders_deeper_than_the_tree_goes_give_their_feeds_to_the_deepest() {
        let mut file = String::from("<opml><body>");
        for level in 1..=7 {
            file.push_str(&format!(
                "<outline text=\"L{level}\"><outline xmlUrl=\"https://f{level}.example/\"/>"
            ));
        }
        file.push_str(&"</outline>".repeat(7));
        file.push_str("<outline xmlUrl=\"https://top.example/\"/></body></opml>");
        let read = outline(&file).unwrap();
        assert_eq!(read.folders.len(), 5);
        let depths: Vec<_> = read.feeds.iter().map(|(path, ..)| path.len()).collect();
        assert_eq!(depths, [1, 2, 3, 4, 5, 5, 5, 0]);
    }

    #[test]
    fn an_opml_file_that_breaks_the_rules_is_refused_or_cut() {
        assert!(outline("<opml><body><outline text=\"a\"></body></opml>").is_err());
        let deep = "<outline>".repeat(MAX_OPML_DEPTH + 1);
        assert_eq!(
            outline(&format!("<opml><body>{deep}")),
            Err("the outlines of this file nest too deep")
        );
        // No entity is taken from a file but the five XML knows.
        let bomb = r#"<!DOCTYPE opml [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;">]>
            <opml><body><outline text="&b;&b;&b;"><outline text="&b;" xmlUrl="https://a.example/"/></outline></body></opml>"#;
        let read = outline(bomb).unwrap();
        assert_eq!(read.folders, Vec::<Vec<String>>::new());
        assert_eq!(read.feeds, [(names(&[]), url("https://a.example/"), None)]);

        let many: String = (0..=tree::MAX_FOLDERS)
            .map(|n| {
                format!(
                    "<outline text=\"f{n}\"><outline xmlUrl=\"https://a.example/{n}\"/></outline>"
                )
            })
            .collect();
        assert_eq!(
            outline(&format!("<opml><body>{many}</body></opml>")),
            Err("this file has too many folders")
        );
        let feeds: String = (0..MAX_OPML_FEEDS + 3)
            .map(|n| format!("<outline xmlUrl=\"https://a.example/{n}\"/>"))
            .collect();
        let read = outline(&format!("<opml><body>{feeds}</body></opml>")).unwrap();
        assert_eq!((read.feeds.len(), read.skipped), (MAX_OPML_FEEDS, 3));
        // Names are cut to what the tree takes.
        let long = format!(
            "<opml><body><outline text=\" {}\u{7} \"><outline xmlUrl=\"a.example\"/></outline></body></opml>",
            "ä".repeat(300)
        );
        assert_eq!(
            outline(&long).unwrap().folders[0][0],
            "ä".repeat(tree::MAX_TITLE)
        );
        assert_eq!(outline("").unwrap(), Outline::default());
        assert_eq!(outline("not xml at all").unwrap(), Outline::default());
    }

    #[test]
    fn what_goes_into_an_opml_file_is_escaped() {
        assert_eq!(
            xml("a <b> & \"c\"\u{0}\u{8}\n d\u{FFFF}"),
            "a &lt;b&gt; &amp; &quot;c&quot;  d"
        );
    }
}
