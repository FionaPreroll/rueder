//! What apps read: subscriptions, folders and labels, unread counts, lists of
//! item IDs and the items themselves.
//!
//! A list is the page's list of a feed, folder, label or of everything, read
//! with the same queries (`articles::page`, `articles::marked`): nothing
//! hidden by a rule, nothing of a blocked feed. What only apps ask for (only
//! what is read, what arrived after a time) is looked at a page at a time,
//! within [`SCAN`] articles per request; the continuation says where the next
//! request goes on.

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use time::OffsetDateTime;

use super::described;
use super::{
    App, Fields,
    parse::{self, FeedRef, Params, State as Mark, Stream},
};
use crate::{
    AppError, AppState, api,
    articles::{self, ListItem, Marked, Page},
    images::Size,
    sessions::Session,
};

/// How many item IDs an answer has at most: as many as NetNewsWire and Read
/// You ask for.
pub const MAX_IDS: usize = 1000;
/// How many items with their texts an answer has at most. A text can have
/// tens of thousands of bytes; NetNewsWire asks for 150, Read You for 100.
pub const MAX_ITEMS: usize = 250;
/// How many items a list has when the app does not say.
const DEFAULT_ITEMS: usize = 20;
/// How many articles one request looks at for a filter only apps have.
pub const SCAN: usize = 5000;
/// How many articles are read from the database at a time.
const CHUNK: i64 = 200;

/// The account's folders by their paths, as the API names them.
pub(crate) struct Folders {
    paths: HashMap<i32, Vec<String>>,
}

impl Folders {
    pub(crate) async fn of(db: &PgPool, account_id: i32) -> anyhow::Result<Self> {
        let rows = sqlx::query!(
            "SELECT id, parent_id, title FROM folders WHERE account_id = $1",
            account_id
        )
        .fetch_all(db)
        .await?;
        let by_id: HashMap<i32, (Option<i32>, String)> = rows
            .into_iter()
            .map(|row| (row.id, (row.parent_id, row.title)))
            .collect();
        let mut paths = HashMap::new();
        for &id in by_id.keys() {
            let mut path = Vec::new();
            let mut at = Some(id);
            // The schema keeps folders at most five deep and without circles;
            // the bound is for a schema that would not.
            while let Some(folder) = at {
                let Some((parent, title)) = by_id.get(&folder) else {
                    break;
                };
                if path.len() > parse::MAX_DEPTH {
                    break;
                }
                path.push(title.clone());
                at = *parent;
            }
            path.reverse();
            paths.insert(id, path);
        }
        Ok(Self { paths })
    }

    /// The name of a folder as a label: its path.
    pub(crate) fn name(&self, id: i32) -> Option<String> {
        self.paths.get(&id).map(|path| parse::path_name(path))
    }

    /// The folder at a path, the names compared as they are written.
    pub(crate) fn find(&self, path: &[String]) -> Option<i32> {
        self.paths
            .iter()
            .find(|(_, own)| own.as_slice() == path)
            .map(|(id, _)| *id)
    }

    fn all(&self) -> impl Iterator<Item = (i32, String)> + '_ {
        self.paths
            .iter()
            .map(|(id, path)| (*id, parse::path_name(path)))
    }
}

/// The account's labels: number and name.
async fn labels(db: &PgPool, account_id: i32) -> anyhow::Result<Vec<(i32, String)>> {
    let rows = sqlx::query!(
        "SELECT id, title FROM labels WHERE account_id = $1 ORDER BY position, id",
        account_id
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|row| (row.id, row.title)).collect())
}

/// The feed an app names, if the account reads it.
pub(crate) async fn feed_of(
    state: &AppState,
    session: &Session,
    feed: &FeedRef,
) -> Result<i32, AppError> {
    match feed {
        FeedRef::Id(id) => Ok(*id),
        FeedRef::Url(url) => sqlx::query_scalar!(
            "SELECT s.feed_id FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
             WHERE s.account_id = $1 AND f.feed_url = $2",
            session.account_id,
            url.trim()
        )
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound),
    }
}

/// Where the articles of a stream come from.
pub(crate) enum Source {
    Feeds(Vec<i32>),
    Marked(Marked),
}

/// The articles a stream stands for, as the page's lists have them.
pub(crate) async fn source(
    state: &AppState,
    session: &Session,
    stream: &Stream,
) -> Result<Source, AppError> {
    Ok(match stream {
        Stream::State(Mark::Starred) => Source::Marked(Marked::Starred),
        Stream::State(Mark::Other) => Source::Feeds(Vec::new()),
        Stream::State(_) => {
            let all = api::feeds_of(state, session, None).await?;
            Source::Feeds(all.unwrap_or_default())
        }
        Stream::Feed(feed) => {
            let id = feed_of(state, session, feed).await?;
            Source::Feeds(api::subscribed(state, session, id).await?)
        }
        Stream::Label(name) => {
            let folders = Folders::of(&state.db, session.account_id).await?;
            if let Some(folder) = parse::path(name).and_then(|path| folders.find(&path)) {
                let feeds = api::feeds_of(state, session, Some(folder)).await?;
                Source::Feeds(feeds.ok_or(AppError::NotFound)?)
            } else {
                let labels = labels(&state.db, session.account_id).await?;
                let label = labels.iter().find(|(_, title)| title == name.trim());
                let (id, _) = label.ok_or(AppError::NotFound)?;
                Source::Marked(Marked::Label(*id))
            }
        }
    })
}

/// What only some articles of a list pass: read or not, starred or not.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Filter {
    read: Option<bool>,
    starred: Option<bool>,
}

impl Filter {
    /// The filter of a request: the stream (`read` is what is read) and
    /// what it includes (`it`) and leaves out (`xt`).
    fn of(stream: &Stream, params: &Params) -> Self {
        let mut filter = Self::default();
        match stream {
            Stream::State(Mark::Read) => filter.read = Some(true),
            Stream::State(Mark::KeptUnread) => filter.read = Some(false),
            _ => {}
        }
        for included in params.all("it").filter_map(parse::stream) {
            match included {
                Stream::State(Mark::Read) => filter.read = Some(true),
                Stream::State(Mark::KeptUnread) => filter.read = Some(false),
                Stream::State(Mark::Starred) => filter.starred = Some(true),
                _ => {}
            }
        }
        for left_out in params.all("xt").filter_map(parse::stream) {
            match left_out {
                Stream::State(Mark::Read) => filter.read = Some(false),
                Stream::State(Mark::Starred) => filter.starred = Some(false),
                _ => {}
            }
        }
        filter
    }

    fn passes(self, item: &ListItem) -> bool {
        self.read.is_none_or(|read| item.read == read)
            && self.starred.is_none_or(|starred| item.starred == starred)
    }
}

/// The order of arrival from which on articles arrived at or after `at`, by
/// the marks of arrival (see migration `0013_arrivals.sql`). Before the
/// first mark nothing is known, and everything counts.
async fn arrived_since(db: &PgPool, at: OffsetDateTime) -> anyhow::Result<i64> {
    let row = sqlx::query!(
        r#"SELECT EXISTS (SELECT 1 FROM arrivals WHERE at <= $1) AS "known!",
                  (SELECT min(first_id) FROM arrivals WHERE at >= date_trunc('minute', $1::timestamptz))
                      AS first"#,
        at
    )
    .fetch_one(db)
    .await?;
    Ok(match (row.known, row.first) {
        (false, _) => 0,
        (true, Some(first)) => first,
        (true, None) => i64::MAX,
    })
}

/// The order of arrival up to which articles arrived before `at`, by the
/// same marks: everything that came before the first one after `at`.
pub(crate) async fn arrived_before(db: &PgPool, at: OffsetDateTime) -> anyhow::Result<i64> {
    let first = sqlx::query_scalar!(
        "SELECT min(first_id) FROM arrivals WHERE at > date_trunc('minute', $1::timestamptz)",
        at
    )
    .fetch_one(db)
    .await?;
    Ok(first.unwrap_or(i64::MAX))
}

/// What a request asks of a list.
struct Ask {
    filter: Filter,
    n: usize,
    oldest_first: bool,
    /// Where the request goes on: the order of arrival of the last article
    /// of the page before.
    after: Option<i64>,
    /// Only articles that arrived from this order on, and before that one.
    from: i64,
    upto: i64,
}

impl Ask {
    async fn of(
        state: &AppState,
        stream: &Stream,
        params: &Params,
        max: usize,
    ) -> Result<Self, AppError> {
        let default = if max == MAX_IDS {
            MAX_IDS
        } else {
            DEFAULT_ITEMS
        };
        let from = match params.one("ot").and_then(parse::time) {
            Some(at) => arrived_since(&state.db, at).await?,
            None => 0,
        };
        let upto = match params.one("nt").and_then(parse::time) {
            Some(at) => arrived_before(&state.db, at).await?,
            None => i64::MAX,
        };
        Ok(Self {
            filter: Filter::of(stream, params),
            n: parse::count(params.one("n"), default, max),
            oldest_first: params.one("r") == Some("o"),
            after: params.one("c").and_then(parse::continuation),
            from,
            upto,
        })
    }
}

/// A page of a list: its articles, and where the next one starts, if there
/// can be one.
struct Scanned {
    items: Vec<ListItem>,
    next: Option<i64>,
}

/// Reads a list a chunk at a time until it has what was asked, has looked at
/// [`SCAN`] articles, or has none left.
async fn scan(
    state: &AppState,
    session: &Session,
    source: &Source,
    ask: &Ask,
) -> Result<Scanned, AppError> {
    let mut items = Vec::new();
    let mut anchor = ask.after;
    let mut looked = 0;
    loop {
        let page = if ask.oldest_first {
            Page {
                before: None,
                after: Some(anchor.unwrap_or(0).max(ask.from.saturating_sub(1))),
                limit: CHUNK,
                unread_only: ask.filter.read == Some(false),
            }
        } else {
            Page {
                before: Some(anchor.unwrap_or(i64::MAX).min(ask.upto)),
                after: None,
                limit: CHUNK,
                unread_only: ask.filter.read == Some(false),
            }
        };
        let chunk = match source {
            Source::Feeds(feeds) if feeds.is_empty() => Vec::new(),
            Source::Feeds(feeds) => {
                articles::page(&state.db, session.account_id, feeds, page).await?
            }
            Source::Marked(marked) => {
                articles::marked(&state.db, session.account_id, *marked, page).await?
            }
        };
        let whole = chunk.len() == usize::try_from(CHUNK).unwrap_or(usize::MAX);
        for item in chunk {
            let outside = if ask.oldest_first {
                item.id >= ask.upto
            } else {
                item.id < ask.from
            };
            if outside {
                return Ok(Scanned { items, next: None });
            }
            anchor = Some(item.id);
            looked += 1;
            if ask.filter.passes(&item) {
                items.push(item);
                if items.len() == ask.n {
                    return Ok(Scanned {
                        items,
                        next: anchor,
                    });
                }
            }
        }
        if !whole {
            return Ok(Scanned { items, next: None });
        }
        if looked >= SCAN {
            return Ok(Scanned {
                items,
                next: anchor,
            });
        }
    }
}

/// The stream of a request: `s`, or the reading list.
fn stream_of(params: &Params) -> Result<Stream, AppError> {
    match params.one("s") {
        None => Ok(Stream::State(Mark::ReadingList)),
        Some(s) => parse::stream(s).ok_or_else(|| AppError::BadRequest("unknown stream".into())),
    }
}

fn usec(at: OffsetDateTime) -> String {
    (at.unix_timestamp_nanos() / 1_000).to_string()
}

/// `GET /reader/api/0/stream/items/ids`: the IDs of a list, newest first or
/// with `r=o` oldest first, at most [`MAX_IDS`] at a time.
///
/// The parameters may come in the query or as a form.
#[utoipa::path(
    method(get, post),
    path = "/reader/api/0/stream/items/ids",
    tag = "Google Reader API",
    params(described::StreamPage),
    security(("app_password" = [])),
    responses((status = OK, description = "`itemRefs` (each `id` as a number, with `directStreamIds` and `timestampUsec`) and, if there is more, `continuation`.", body = serde_json::Value), (status = BAD_REQUEST, description = "A parameter the server cannot use, such as an unknown stream."), crate::openapi::NeedsAppPassword),
)]
pub(super) async fn item_ids(
    State(state): State<AppState>,
    App(login): App,
    Fields(params): Fields,
) -> Result<Json<Value>, AppError> {
    let session = &login.session;
    let stream = stream_of(&params)?;
    let source = source(&state, session, &stream).await?;
    let ask = Ask::of(&state, &stream, &params, MAX_IDS).await?;
    let scanned = scan(&state, session, &source, &ask).await?;
    let refs: Vec<Value> = scanned
        .items
        .iter()
        .filter_map(|item| {
            let id = parse::item_number(item.feed_id, item.seq)?;
            Some(json!({
                "id": id.to_string(),
                "directStreamIds": [],
                "timestampUsec": usec(item.published_at),
            }))
        })
        .collect();
    let mut answer = json!({ "itemRefs": refs });
    if let Some(next) = scanned.next {
        answer["continuation"] = json!(next.to_string());
    }
    Ok(Json(answer))
}

/// An article with what an app shows of it.
struct Opened {
    feed_id: i32,
    seq: i32,
    published_at: OffsetDateTime,
    updated_at: Option<OffsetDateTime>,
    title: String,
    author: String,
    url: Option<String>,
    html: String,
    enclosures: Value,
    read: bool,
    starred: bool,
    kept_unread: bool,
}

/// These articles with their texts, in this order, as far as the account
/// reads their feeds and nothing hides them.
async fn open(db: &PgPool, account_id: i32, keys: &[(i32, i32)]) -> anyhow::Result<Vec<Opened>> {
    let (feeds, seqs): (Vec<i32>, Vec<i32>) = keys.iter().copied().unzip();
    let rows = sqlx::query!(
        r#"SELECT a.feed_id, a.seq, a.published_at, a.updated_at, a.title, a.author, a.url,
                  c.html, c.enclosures,
                  CASE WHEN a.seq <= s.read_upto THEN NOT coalesce(st.kept_unread, false)
                       ELSE coalesce(st.read, false) END AS "read!",
                  coalesce(st.starred, false) AS "starred!",
                  a.seq <= s.read_upto AND coalesce(st.kept_unread, false) AS "kept_unread!"
           FROM unnest($2::int4[], $3::int4[]) WITH ORDINALITY AS k (feed_id, seq, ord)
           JOIN subscriptions s ON s.account_id = $1 AND s.feed_id = k.feed_id
           JOIN feeds f ON f.id = k.feed_id AND f.blocked_at IS NULL
           JOIN articles a ON a.feed_id = k.feed_id AND a.seq = k.seq
           JOIN article_content c ON c.feed_id = a.feed_id AND c.seq = a.seq
           LEFT JOIN article_states st
               ON st.account_id = $1 AND st.feed_id = a.feed_id AND st.seq = a.seq
           WHERE NOT coalesce(st.hidden, false)
           ORDER BY k.ord"#,
        account_id,
        &feeds,
        &seqs,
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| Opened {
            feed_id: row.feed_id,
            seq: row.seq,
            published_at: row.published_at,
            updated_at: row.updated_at,
            title: row.title,
            author: row.author,
            url: row.url,
            html: row.html,
            enclosures: row.enclosures,
            read: row.read,
            starred: row.starred,
            kept_unread: row.kept_unread,
        })
        .collect())
}

/// The feeds of the account as items name them: stream ID, title, site and
/// folder.
struct Origins {
    feeds: HashMap<i32, (String, Option<String>, Option<i32>)>,
}

impl Origins {
    async fn of(db: &PgPool, account_id: i32) -> anyhow::Result<Self> {
        let rows = sqlx::query!(
            "SELECT s.feed_id, s.folder_id, s.title AS own_title, f.title, f.feed_url,
                    f.site_url
             FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
             WHERE s.account_id = $1",
            account_id
        )
        .fetch_all(db)
        .await?;
        let feeds = rows
            .into_iter()
            .map(|row| {
                let title = api::shown_title(row.own_title, &row.title, &row.feed_url);
                (row.feed_id, (title, row.site_url, row.folder_id))
            })
            .collect();
        Ok(Self { feeds })
    }
}

/// The items of these articles, as `stream/items/contents` and
/// `stream/contents` answer.
async fn items(
    state: &AppState,
    login: &super::AppLogin,
    origin: Option<&str>,
    keys: &[(i32, i32)],
) -> Result<Vec<Value>, AppError> {
    let account_id = login.session.account_id;
    let opened = open(&state.db, account_id, keys).await?;
    let origins = Origins::of(&state.db, account_id).await?;
    let folders = Folders::of(&state.db, account_id).await?;
    let label_names: HashMap<i32, String> =
        labels(&state.db, account_id).await?.into_iter().collect();
    let keys: Vec<(i32, i32)> = opened.iter().map(|o| (o.feed_id, o.seq)).collect();
    let mut labelled = api::labels_of(state, &login.session, &keys).await?;
    let signer = state.images.as_ref().map(|images| images.signer());
    Ok(opened
        .into_iter()
        .filter_map(|article| {
            let number = parse::item_number(article.feed_id, article.seq)?;
            let (title, site, folder) = origins.feeds.get(&article.feed_id)?.clone();
            let mut categories = vec![json!(Mark::ReadingList.id())];
            if article.read {
                categories.push(json!(Mark::Read.id()));
            }
            if article.starred {
                categories.push(json!(Mark::Starred.id()));
            }
            if article.kept_unread {
                categories.push(json!(Mark::KeptUnread.id()));
            }
            if let Some(name) = folder.and_then(|folder| folders.name(folder)) {
                categories.push(json!(parse::label_id(&name)));
            }
            let labels = labelled.remove(&(article.feed_id, article.seq));
            for label in labels.unwrap_or_default() {
                if let Some(name) = label_names.get(&label) {
                    categories.push(json!(parse::label_id(name)));
                }
            }
            let html = match (signer, origin) {
                (Some(signer), Some(origin)) => {
                    signer.in_html_for_app(login.id, origin, &article.html)
                }
                _ => crate::cleaning::with_images(&article.html, |_| None),
            };
            let link: Vec<Value> = article
                .url
                .iter()
                .map(|url| json!({ "href": url, "type": "text/html" }))
                .collect();
            let enclosures: Vec<Value> = api::enclosures(&article.enclosures)
                .into_iter()
                .map(|e| json!({ "href": e.url, "type": e.kind, "length": e.length }))
                .collect();
            let updated = article.updated_at.unwrap_or(article.published_at);
            Some(json!({
                "id": parse::long_item_id(number),
                "crawlTimeMsec": (article.published_at.unix_timestamp_nanos() / 1_000_000)
                    .to_string(),
                "timestampUsec": usec(article.published_at),
                "published": article.published_at.unix_timestamp(),
                "updated": updated.unix_timestamp(),
                "title": article.title,
                "author": article.author,
                "canonical": link,
                "alternate": link,
                "categories": categories,
                "origin": {
                    "streamId": parse::feed_id(article.feed_id),
                    "title": title,
                    "htmlUrl": site,
                },
                "summary": { "direction": "ltr", "content": html },
                "enclosure": enclosures,
            }))
        })
        .collect())
}

/// The frame around items: the stream, and when it was answered.
fn contents(id: &str, items: Vec<Value>, next: Option<i64>) -> Json<Value> {
    let mut answer = json!({
        "direction": "ltr",
        "id": id,
        "updated": OffsetDateTime::now_utc().unix_timestamp(),
        "items": items,
    });
    if let Some(next) = next {
        answer["continuation"] = json!(next.to_string());
    }
    Json(answer)
}

/// `POST /reader/api/0/stream/items/contents`: the items of these IDs (`i`),
/// at most [`MAX_ITEMS`]. What the account cannot see is left out.
///
/// The IDs may come in the query or as a form (`i`, once per item).
#[utoipa::path(
    method(get, post),
    path = "/reader/api/0/stream/items/contents",
    tag = "Google Reader API",
    params(("i" = Vec<String>, Query, description = "The ID of an item, long or short; once per item.")),
    security(("app_password" = [])),
    responses((status = OK, description = "`items`: each with `id`, `title`, `published`, `author`, `canonical`, `alternate`, `summary.content` (the cleaned text), `categories` (read, starred, labels) and `origin` (its feed).", body = serde_json::Value), (status = BAD_REQUEST, description = "A parameter the server cannot use, such as an unknown stream."), crate::openapi::NeedsAppPassword),
)]
pub(super) async fn item_contents(
    State(state): State<AppState>,
    headers: HeaderMap,
    App(login): App,
    Fields(params): Fields,
) -> Result<Json<Value>, AppError> {
    let keys: Vec<(i32, i32)> = params.all("i").filter_map(parse::item).collect();
    if keys.len() > MAX_ITEMS {
        let message = format!("at most {MAX_ITEMS} items at a time");
        return Err(AppError::BadRequest(message));
    }
    let origin = super::origin(&headers);
    let items = items(&state, &login, origin.as_deref(), &keys).await?;
    let id = Mark::ReadingList.id().unwrap_or_default();
    Ok(contents(id, items, None))
}

/// `GET /reader/api/0/stream/contents/<stream>`: a page of a list with the
/// items' texts, at most [`MAX_ITEMS`].
///
/// The stream is the rest of the path, slashes included, such as `feed/12` or
/// `user/-/state/com.google/reading-list`; the other parameters may come in the
/// query or as a form.
#[utoipa::path(
    method(get, post),
    path = "/reader/api/0/stream/contents/{*stream}",
    tag = "Google Reader API",
    params(("*stream" = String, Path, description = "The stream, slashes included."), described::StreamPage),
    security(("app_password" = [])),
    responses((status = OK, description = "As `stream/contents` answers.", body = serde_json::Value), (status = BAD_REQUEST, description = "A parameter the server cannot use, such as an unknown stream."), crate::openapi::NeedsAppPassword),
)]
pub(super) async fn stream_contents_of(
    State(state): State<AppState>,
    headers: HeaderMap,
    App(login): App,
    Path(stream): Path<String>,
    Fields(params): Fields,
) -> Result<Json<Value>, AppError> {
    let stream_id = stream.trim_start_matches('/').to_owned();
    contents_of(&state, &headers, &login, &stream_id, &params).await
}

/// `GET /reader/api/0/stream/contents?s=<stream>`, the same.
///
/// The parameters may come in the query or as a form.
#[utoipa::path(
    method(get, post),
    path = "/reader/api/0/stream/contents",
    tag = "Google Reader API",
    params(described::StreamPage),
    security(("app_password" = [])),
    responses((status = OK, description = "`items` as `stream/items/contents` has them, and, if there is more, `continuation`.", body = serde_json::Value), (status = BAD_REQUEST, description = "A parameter the server cannot use, such as an unknown stream."), crate::openapi::NeedsAppPassword),
)]
pub(super) async fn stream_contents(
    State(state): State<AppState>,
    headers: HeaderMap,
    App(login): App,
    Fields(params): Fields,
) -> Result<Json<Value>, AppError> {
    let stream_id = params
        .one("s")
        .unwrap_or(Mark::ReadingList.id().unwrap_or_default())
        .to_owned();
    contents_of(&state, &headers, &login, &stream_id, &params).await
}

async fn contents_of(
    state: &AppState,
    headers: &HeaderMap,
    login: &super::AppLogin,
    stream_id: &str,
    params: &Params,
) -> Result<Json<Value>, AppError> {
    let stream =
        parse::stream(stream_id).ok_or_else(|| AppError::BadRequest("unknown stream".into()))?;
    let source = source(state, &login.session, &stream).await?;
    let ask = Ask::of(state, &stream, params, MAX_ITEMS).await?;
    let scanned = scan(state, &login.session, &source, &ask).await?;
    let keys: Vec<(i32, i32)> = scanned
        .items
        .iter()
        .map(|item| (item.feed_id, item.seq))
        .collect();
    let origin = super::origin(headers);
    let items = items(state, login, origin.as_deref(), &keys).await?;
    Ok(contents(stream_id, items, scanned.next))
}

/// The folder of a subscription as a category.
fn category(name: &str) -> Value {
    json!({ "id": parse::label_id(name), "label": name, "type": "folder" })
}

/// `GET /reader/api/0/subscription/list`: the account's feeds, each with its
/// folder.
#[utoipa::path(
    method(get, post),
    path = "/reader/api/0/subscription/list",
    tag = "Google Reader API",
    security(("app_password" = [])),
    responses((status = OK, description = "`subscriptions`: each feed with `id` (`feed/<id>`), `title`, `url`, `htmlUrl`, `iconUrl` and its folder in `categories`.", body = serde_json::Value), crate::openapi::NeedsAppPassword),
)]
pub(super) async fn subscriptions(
    State(state): State<AppState>,
    headers: HeaderMap,
    App(login): App,
) -> Result<Json<Value>, AppError> {
    let account_id = login.session.account_id;
    let folders = Folders::of(&state.db, account_id).await?;
    let rows = sqlx::query!(
        "SELECT s.feed_id, s.folder_id, s.title AS own_title, f.title, f.feed_url, f.site_url,
                f.icon_url
         FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
         WHERE s.account_id = $1 ORDER BY s.position, s.feed_id",
        account_id
    )
    .fetch_all(&state.db)
    .await?;
    let origin = super::origin(&headers);
    let signer = state.images.as_ref().map(|images| images.signer());
    let subscriptions: Vec<Value> = rows
        .into_iter()
        .enumerate()
        .map(|(n, row)| {
            let categories: Vec<Value> = row
                .folder_id
                .and_then(|folder| folders.name(folder))
                .map(|name| category(&name))
                .into_iter()
                .collect();
            let icon = match (signer, &origin, &row.icon_url) {
                (Some(signer), Some(origin), Some(icon)) => signer
                    .app_address(login.id, Size::Icon, icon)
                    .map(|address| format!("{origin}{address}")),
                _ => None,
            };
            json!({
                "id": parse::feed_id(row.feed_id),
                "title": api::shown_title(row.own_title, &row.title, &row.feed_url),
                "categories": categories,
                "sortid": format!("{n:08X}"),
                "firstitemmsec": "0",
                "url": row.feed_url,
                "htmlUrl": row.site_url,
                "iconUrl": icon.unwrap_or_default(),
            })
        })
        .collect();
    Ok(Json(json!({ "subscriptions": subscriptions })))
}

/// `GET /reader/api/0/tag/list`: the star, the folders by their paths and the
/// labels.
#[utoipa::path(
    method(get, post),
    path = "/reader/api/0/tag/list",
    tag = "Google Reader API",
    security(("app_password" = [])),
    responses((status = OK, description = "`tags`: the star, each folder and each label, by `id`.", body = serde_json::Value), crate::openapi::NeedsAppPassword),
)]
pub(super) async fn tags(
    State(state): State<AppState>,
    App(login): App,
) -> Result<Json<Value>, AppError> {
    let account_id = login.session.account_id;
    let folders = Folders::of(&state.db, account_id).await?;
    let mut named: Vec<(i32, String)> = folders.all().collect();
    named.sort_by(|a, b| a.1.cmp(&b.1));
    let mut tags = vec![json!({ "id": Mark::Starred.id() })];
    tags.extend(
        named.into_iter().map(
            |(_, name)| json!({ "id": parse::label_id(&name), "label": name, "type": "folder" }),
        ),
    );
    for (_, title) in labels(&state.db, account_id).await? {
        tags.push(json!({ "id": parse::label_id(&title), "label": title, "type": "tag" }));
    }
    Ok(Json(json!({ "tags": tags })))
}

/// `GET /reader/api/0/unread-count`: the unread count of each feed, folder
/// and of all, worked out like the page's (`reading::unread_counts`).
#[utoipa::path(
    method(get, post),
    path = "/reader/api/0/unread-count",
    tag = "Google Reader API",
    security(("app_password" = [])),
    responses((status = OK, description = "`max` and `unreadcounts`: per feed, folder and for all, with `id`, `count` and `newestItemTimestampUsec`.", body = serde_json::Value), crate::openapi::NeedsAppPassword),
)]
pub(super) async fn unread_counts(
    State(state): State<AppState>,
    App(login): App,
) -> Result<Json<Value>, AppError> {
    let account_id = login.session.account_id;
    let rows = sqlx::query!(
        r#"SELECT s.feed_id, s.folder_id, f.blocked_at IS NOT NULL AS "blocked!",
                  (SELECT a.published_at FROM articles a
                   WHERE a.feed_id = s.feed_id ORDER BY a.seq DESC LIMIT 1) AS newest
           FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
           WHERE s.account_id = $1"#,
        account_id
    )
    .fetch_all(&state.db)
    .await?;
    let counts: HashMap<i32, i64> = crate::reading::unread_counts(&state.db, account_id)
        .await?
        .into_iter()
        .map(|count| (count.feed_id, count.unread))
        .collect();
    let folders = Folders::of(&state.db, account_id).await?;
    let parents = sqlx::query!(
        "SELECT id, parent_id FROM folders WHERE account_id = $1",
        account_id
    )
    .fetch_all(&state.db)
    .await?;
    let parent: HashMap<i32, Option<i32>> = parents
        .into_iter()
        .map(|row| (row.id, row.parent_id))
        .collect();
    let mut folder_counts: HashMap<i32, (i64, Option<OffsetDateTime>)> = HashMap::new();
    let mut total = (0i64, None::<OffsetDateTime>);
    let mut entries = Vec::new();
    for row in rows {
        let count = if row.blocked {
            0
        } else {
            counts.get(&row.feed_id).copied().unwrap_or(0)
        };
        let newest = row.newest.filter(|_| !row.blocked);
        let add = |sum: &mut (i64, Option<OffsetDateTime>)| {
            sum.0 += count;
            sum.1 = sum.1.max(newest);
        };
        add(&mut total);
        let mut at = row.folder_id;
        let mut depth = 0;
        while let Some(folder) = at {
            add(folder_counts.entry(folder).or_default());
            at = parent.get(&folder).copied().flatten();
            depth += 1;
            if depth > parse::MAX_DEPTH {
                break;
            }
        }
        entries.push(json!({
            "id": parse::feed_id(row.feed_id),
            "count": count,
            "newestItemTimestampUsec": newest.map(usec).unwrap_or_else(|| "0".into()),
        }));
    }
    for (folder, (count, newest)) in folder_counts {
        if let Some(name) = folders.name(folder) {
            entries.push(json!({
                "id": parse::label_id(&name),
                "count": count,
                "newestItemTimestampUsec": newest.map(usec).unwrap_or_else(|| "0".into()),
            }));
        }
    }
    entries.push(json!({
        "id": Mark::ReadingList.id(),
        "count": total.0,
        "newestItemTimestampUsec": total.1.map(usec).unwrap_or_else(|| "0".into()),
    }));
    Ok(Json(json!({ "max": MAX_IDS, "unreadcounts": entries })))
}
