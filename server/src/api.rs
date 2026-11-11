//! The API a logged-in reader's interface lives on: the tree of folders and
//! feeds, the lists of articles, an article and its versions, and what was
//! read.
//!
//! Every handler takes a [`Session`] and binds what it asks the database to
//! that account: a feed nobody of this account subscribes to, or a folder of
//! somebody else, is "not found", the same as one that does not exist.
//!
//! No image goes out with its own address: texts pass through
//! [`Signer::in_html`], thumbnails, images and icons through
//! [`Signer::address`]. What has no address at the proxy goes out without.

use std::{collections::HashMap, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;
use url::Url;

use crate::{
    AppError, AppState,
    articles::{self, ListItem, Marked, Opened, Page},
    cleaning::{self, Base, Rules},
    filters,
    images::{Signer, Size},
    reading,
    sessions::Session,
};

/// How many articles a page of a list has if nothing else is asked, and at most.
const PAGE: i64 = 50;
const MAX_PAGE: i64 = 200;

/// The types this module sends and reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types
        .add::<Tree>()
        .add::<ListQuery>()
        .add::<List>()
        .add::<Article>()
        .add::<Versions>()
        .add::<StateChange>()
        .add::<Upto>();
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/tree", get(tree))
        .route("/articles", get(all_articles))
        .route("/starred", get(starred_articles))
        .route("/hidden", get(hidden_articles))
        .route("/labels/{label_id}/articles", get(label_articles))
        .route("/read", post(read_all))
        .route("/feeds/{feed_id}/articles", get(feed_articles))
        .route("/feeds/{feed_id}/read", post(read_feed))
        .route("/feeds/{feed_id}/articles/{seq}", get(article))
        .route("/feeds/{feed_id}/articles/{seq}/versions", get(versions))
        .route("/feeds/{feed_id}/articles/{seq}/state", put(set_state))
        .route("/folders/{folder_id}/articles", get(folder_articles))
        .route("/folders/{folder_id}/read", post(read_folder))
}

impl AppState {
    fn signer(&self) -> Option<&Signer> {
        self.images.as_ref().map(|images| images.signer())
    }

    /// The address of a picture at the proxy, if it has one.
    fn picture(&self, size: Size, url: Option<String>) -> Option<String> {
        self.signer()?.address(size, &url?)
    }

    /// A cleaned text with its images at the proxy.
    fn text(&self, html: &str) -> String {
        match self.signer() {
            Some(signer) => signer.in_html(html),
            None => cleaning::with_images(html, |_| None),
        }
    }
}

/// A folder of the tree. `parent_id` is the folder it lies in, null at the top.
#[derive(Serialize, TS)]
struct Folder {
    id: i32,
    parent_id: Option<i32>,
    title: String,
    position: f64,
}

/// A subscription with what the tree shows of its feed.
///
/// `folder_id` is the folder it lies in, null at the top.
#[derive(Serialize, TS)]
struct Feed {
    id: i32,
    folder_id: Option<i32>,
    position: f64,
    /// The name the reader gave it, else the feed's own, else, for a feed
    /// that has none (yet), the host of its address.
    title: String,
    feed_title: String,
    feed_url: String,
    site_url: Option<String>,
    icon_url: Option<String>,
    #[ts(as = "crate::tree::FetchAgent")]
    fetch_agent: String,
    /// Why the last fetch failed, if it did.
    error: Option<String>,
    /// Whether the feed was fetched at all yet. One that came with an OPML
    /// file waits for the next round.
    fetched: bool,
    last_seq: i32,
    read_upto: i32,
    unread: i64,
}

/// What `GET /api/tree` answers: the account's folders and subscriptions, each
/// as a plain list. `parent_id` and `folder_id` say what lies where, `position`
/// in which order.
#[derive(Serialize, TS)]
struct Tree {
    folders: Vec<Folder>,
    feeds: Vec<Feed>,
    /// How many articles the account's rules hide, for the list of them.
    hidden: i64,
}

/// The name a subscription goes by: the reader's own, the feed's, or, where the
/// feed has none, the host of its address, so that no row is without a name.
fn shown_title(own: Option<String>, feed: &str, url: &str) -> String {
    let named = |title: &str| !title.trim().is_empty();
    match own {
        Some(own) if named(&own) => own,
        _ if named(feed) => feed.to_owned(),
        _ => Url::parse(url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .unwrap_or_else(|| url.to_owned()),
    }
}

async fn tree(State(state): State<AppState>, session: Session) -> Result<Json<Tree>, AppError> {
    let folders = sqlx::query_as!(
        Folder,
        "SELECT id, parent_id, title, position FROM folders
         WHERE account_id = $1 ORDER BY position, id",
        session.account_id
    )
    .fetch_all(&state.db)
    .await?;
    let counts: HashMap<i32, _> = reading::unread_counts(&state.db, session.account_id)
        .await?
        .into_iter()
        .map(|count| (count.feed_id, count))
        .collect();
    let subscriptions = sqlx::query!(
        r#"SELECT s.feed_id, s.folder_id, s.position, s.title AS own_title, f.title AS feed_title,
                  f.feed_url, f.site_url, f.icon_url, f.fetch_agent, f.error_message,
                  f.fetched_at IS NOT NULL AS "fetched!"
           FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
           WHERE s.account_id = $1 ORDER BY s.position, s.feed_id"#,
        session.account_id
    )
    .fetch_all(&state.db)
    .await?;
    let feeds = subscriptions
        .into_iter()
        .map(|row| {
            let count = counts.get(&row.feed_id);
            Feed {
                id: row.feed_id,
                folder_id: row.folder_id,
                position: row.position,
                title: shown_title(row.own_title, &row.feed_title, &row.feed_url),
                feed_title: row.feed_title,
                feed_url: row.feed_url,
                site_url: row.site_url,
                icon_url: state.picture(Size::Icon, row.icon_url),
                fetch_agent: row.fetch_agent,
                error: row.error_message,
                fetched: row.fetched,
                last_seq: count.map_or(0, |count| count.last_seq),
                read_upto: count.map_or(0, |count| count.read_upto),
                unread: count.map_or(0, |count| count.unread),
            }
        })
        .collect();
    let hidden = sqlx::query_scalar!(
        r#"SELECT count(*) AS "hidden!" FROM article_states
           WHERE account_id = $1 AND hidden"#,
        session.account_id
    )
    .fetch_one(&state.db)
    .await?;
    Ok(Json(Tree {
        folders,
        feeds,
        hidden,
    }))
}

/// Which part of a list is asked for.
///
/// The query of every list: `GET /api/articles`, `/api/starred`, `/api/hidden`,
/// `/api/feeds/{id}/articles`, `/api/folders/{id}/articles` and
/// `/api/labels/{id}/articles`. Each answers with `List`.
#[derive(Deserialize, TS)]
#[ts(optional_fields)]
struct ListQuery {
    /// Only what arrived before the article of this `id`.
    before: Option<i64>,
    /// Only what arrived after the article of this `id`, the oldest first; 0
    /// starts at the oldest article there is. Not together with `before`.
    after: Option<i64>,
    limit: Option<i64>,
    /// Leave out what was read.
    #[serde(default)]
    #[ts(as = "Option<bool>")]
    unread: bool,
}

/// A line of a list, with the labels the reader gave the article.
#[derive(Serialize, TS)]
#[ts(rename = "ListItem")]
struct Item {
    #[serde(flatten)]
    article: ListItem,
    labels: Vec<i32>,
}

/// A page of a list, the newest article first. For the next page, ask again
/// with `before` set to the `id` of the last article of this one. A list asked
/// for with `after` has the oldest article first and goes on the same way,
/// with `after`.
#[derive(Serialize, TS)]
struct List {
    articles: Vec<Item>,
}

impl ListQuery {
    fn page(&self) -> Result<Page, AppError> {
        if self.before.is_some() && self.after.is_some() {
            return Err(AppError::BadRequest("give before or after".into()));
        }
        Ok(Page {
            before: self.before,
            after: self.after,
            limit: self.limit.unwrap_or(PAGE).clamp(1, MAX_PAGE),
            unread_only: self.unread,
        })
    }
}

/// The account's labels on these articles, by feed and number.
async fn labels_of(
    state: &AppState,
    session: &Session,
    articles: &[(i32, i32)],
) -> Result<HashMap<(i32, i32), Vec<i32>>, AppError> {
    let (feeds, seqs): (Vec<i32>, Vec<i32>) = articles.iter().copied().unzip();
    let rows = sqlx::query!(
        r#"SELECT al.feed_id, al.seq, al.label_id
           FROM labels l JOIN article_labels al ON al.label_id = l.id
           WHERE l.account_id = $1
             AND (al.feed_id, al.seq) IN (SELECT * FROM unnest($2::int4[], $3::int4[]))
           ORDER BY l.position, l.id"#,
        session.account_id,
        &feeds,
        &seqs,
    )
    .fetch_all(&state.db)
    .await?;
    let mut labels: HashMap<_, Vec<i32>> = HashMap::new();
    for row in rows {
        let of_article = labels.entry((row.feed_id, row.seq)).or_default();
        of_article.push(row.label_id);
    }
    Ok(labels)
}

/// What a list sends out: pictures at the proxy, labels on each line.
async fn listed(
    state: &AppState,
    session: &Session,
    articles: Vec<ListItem>,
) -> Result<Json<List>, AppError> {
    let keys: Vec<_> = articles.iter().map(|a| (a.feed_id, a.seq)).collect();
    let mut labels = labels_of(state, session, &keys).await?;
    let articles = articles
        .into_iter()
        .map(|mut article| {
            article.thumbnail_url = state.picture(Size::Thumbnail, article.thumbnail_url.take());
            let labels = labels.remove(&(article.feed_id, article.seq));
            Item {
                article,
                labels: labels.unwrap_or_default(),
            }
        })
        .collect();
    Ok(Json(List { articles }))
}

async fn list(
    state: &AppState,
    session: &Session,
    feed_ids: &[i32],
    query: ListQuery,
) -> Result<Json<List>, AppError> {
    let articles = articles::page(&state.db, session.account_id, feed_ids, query.page()?).await?;
    listed(state, session, articles).await
}

async fn marked(
    state: &AppState,
    session: &Session,
    which: Marked,
    query: ListQuery,
) -> Result<Json<List>, AppError> {
    let articles = articles::marked(&state.db, session.account_id, which, query.page()?).await?;
    listed(state, session, articles).await
}

async fn starred_articles(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<ListQuery>,
) -> Result<Json<List>, AppError> {
    marked(&state, &session, Marked::Starred, query).await
}

async fn hidden_articles(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<ListQuery>,
) -> Result<Json<List>, AppError> {
    marked(&state, &session, Marked::Hidden, query).await
}

async fn label_articles(
    State(state): State<AppState>,
    session: Session,
    Path(label_id): Path<i32>,
    Query(query): Query<ListQuery>,
) -> Result<Json<List>, AppError> {
    let own = sqlx::query_scalar!(
        "SELECT id FROM labels WHERE account_id = $1 AND id = $2",
        session.account_id,
        label_id
    )
    .fetch_optional(&state.db)
    .await?;
    if own.is_none() {
        return Err(AppError::NotFound);
    }
    marked(&state, &session, Marked::Label(label_id), query).await
}

/// The feeds the account subscribes to: all of them, or the ones in a folder
/// and the folders inside it. `None` if the folder is not the account's.
async fn feeds_of(
    state: &AppState,
    session: &Session,
    folder_id: Option<i32>,
) -> Result<Option<Vec<i32>>, AppError> {
    let Some(folder_id) = folder_id else {
        let all = sqlx::query_scalar!(
            "SELECT feed_id FROM subscriptions WHERE account_id = $1",
            session.account_id
        )
        .fetch_all(&state.db)
        .await?;
        return Ok(Some(all));
    };
    let own = sqlx::query_scalar!(
        "SELECT id FROM folders WHERE account_id = $1 AND id = $2",
        session.account_id,
        folder_id
    )
    .fetch_optional(&state.db)
    .await?;
    if own.is_none() {
        return Ok(None);
    }
    let inside = sqlx::query_scalar!(
        "WITH RECURSIVE inside AS (
             SELECT id FROM folders WHERE account_id = $1 AND id = $2
             UNION ALL
             SELECT f.id FROM folders f JOIN inside ON f.parent_id = inside.id
             WHERE f.account_id = $1
         )
         SELECT s.feed_id FROM subscriptions s
         WHERE s.account_id = $1 AND s.folder_id IN (SELECT id FROM inside)",
        session.account_id,
        folder_id
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Some(inside))
}

async fn subscribed(state: &AppState, session: &Session, feed_id: i32) -> Result<(), AppError> {
    let subscribed = sqlx::query_scalar!(
        "SELECT feed_id FROM subscriptions WHERE account_id = $1 AND feed_id = $2",
        session.account_id,
        feed_id
    )
    .fetch_optional(&state.db)
    .await?;
    subscribed.map(|_| ()).ok_or(AppError::NotFound)
}

async fn all_articles(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<ListQuery>,
) -> Result<Json<List>, AppError> {
    let feeds = feeds_of(&state, &session, None).await?.unwrap_or_default();
    list(&state, &session, &feeds, query).await
}

async fn folder_articles(
    State(state): State<AppState>,
    session: Session,
    Path(folder_id): Path<i32>,
    Query(query): Query<ListQuery>,
) -> Result<Json<List>, AppError> {
    let feeds = feeds_of(&state, &session, Some(folder_id))
        .await?
        .ok_or(AppError::NotFound)?;
    list(&state, &session, &feeds, query).await
}

async fn feed_articles(
    State(state): State<AppState>,
    session: Session,
    Path(feed_id): Path<i32>,
    Query(query): Query<ListQuery>,
) -> Result<Json<List>, AppError> {
    subscribed(&state, &session, feed_id).await?;
    list(&state, &session, &[feed_id], query).await
}

/// An article for reading, with the labels the reader gave it.
///
/// `GET /api/feeds/{id}/articles/{seq}` answers with it.
#[derive(Serialize, TS)]
struct Article {
    #[serde(flatten)]
    article: Opened,
    enclosures: Vec<Enclosure>,
    labels: Vec<i32>,
    /// The rule that hides the article, if one does: the first of the
    /// account's rules that would, in their order.
    hidden_by: Option<i32>,
}

/// A file that comes with an article, like the episode of a podcast.
#[derive(Debug, PartialEq, Eq, Serialize, TS)]
struct Enclosure {
    url: String,
    /// Its media type, such as `audio/mpeg`, if the feed says.
    #[serde(rename = "type")]
    kind: Option<String>,
    /// Its size in bytes, if the feed says.
    length: Option<u64>,
    /// It stood in the article's text and not in the feed's own list.
    inline: bool,
}

/// How many files an article is sent out with at most.
const MAX_ENCLOSURES: usize = 100;

/// The files of an article in the one shape the API has for them. They are
/// stored as they came: the feed's own with a size as a number, those of the
/// old rueder with a size as text, and nothing has looked at their addresses.
/// Whatever is no web address stays behind.
fn enclosures(stored: &serde_json::Value) -> Vec<Enclosure> {
    let stored = stored.as_array().map(Vec::as_slice).unwrap_or_default();
    let enclosure = |stored: &serde_json::Value| {
        let url = stored.get("url")?.as_str()?.trim();
        let address = Url::parse(url).ok()?;
        let web = matches!(address.scheme(), "http" | "https") && address.host_str().is_some();
        let kind = stored.get("type").and_then(|kind| kind.as_str());
        let length = stored.get("length").and_then(|length| match length {
            serde_json::Value::String(text) => text.trim().parse().ok(),
            number => number.as_u64(),
        });
        let inline = stored.get("inline").and_then(|inline| inline.as_bool());
        web.then(|| Enclosure {
            url: url.to_owned(),
            kind: kind
                .map(str::trim)
                .filter(|kind| !kind.is_empty())
                .map(str::to_owned),
            length,
            inline: inline.unwrap_or(false),
        })
    };
    stored
        .iter()
        .filter_map(enclosure)
        .take(MAX_ENCLOSURES)
        .collect()
}

async fn article(
    State(state): State<AppState>,
    session: Session,
    Path((feed_id, seq)): Path<(i32, i32)>,
) -> Result<Json<Article>, AppError> {
    let mut opened = articles::open(&state.db, session.account_id, feed_id, seq)
        .await?
        .ok_or(AppError::NotFound)?;
    let hidden_by = filters::hiding_rule(&state.db, session.account_id, &opened).await?;
    opened.html = state.text(&opened.html);
    opened.image_url = state.picture(Size::Image, opened.image_url.take());
    let mut labels = labels_of(&state, &session, &[(feed_id, seq)]).await?;
    Ok(Json(Article {
        enclosures: enclosures(&opened.enclosures),
        article: opened,
        labels: labels.remove(&(feed_id, seq)).unwrap_or_default(),
        hidden_by,
    }))
}

/// One version of an article, cleaned for reading like the current one.
#[derive(Serialize, TS)]
#[ts(rename = "ArticleVersion")]
struct Version {
    version: i16,
    #[serde(with = "time::serde::rfc3339")]
    #[ts(type = "string")]
    fetched_at: OffsetDateTime,
    title: String,
    html: String,
    authors: Vec<String>,
    tags: Vec<String>,
    url: Option<String>,
}

/// What `GET /api/feeds/{id}/articles/{seq}/versions` answers: the versions of
/// the article that are kept, oldest first.
#[derive(Serialize, TS)]
struct Versions {
    versions: Vec<Version>,
}

async fn versions(
    State(state): State<AppState>,
    session: Session,
    Path((feed_id, seq)): Path<(i32, i32)>,
) -> Result<Json<Versions>, AppError> {
    let kept = articles::versions(&state.db, session.account_id, feed_id, seq).await?;
    if kept.is_empty() {
        return Err(AppError::NotFound);
    }
    let feed_url = sqlx::query_scalar!("SELECT feed_url FROM feeds WHERE id = $1", feed_id)
        .fetch_one(&state.db)
        .await?;
    let rules = match &state.rules {
        Some(rules) => rules.clone(),
        None => Arc::new(Rules::built_in()?),
    };
    // Versions are kept as they were fetched. Nothing of that goes out
    // without the cleaning the current text went through, which is work for
    // a thread of its own.
    let cleaned = tokio::task::spawn_blocking(move || {
        kept.into_iter()
            .map(|version| {
                let original = version.original;
                let base = Base {
                    article: original.url.as_deref(),
                    feed: Some(&feed_url),
                };
                let html = cleaning::clean(&original.content, base, &rules).html;
                (version.version, version.fetched_at, original, html)
            })
            .collect::<Vec<_>>()
    })
    .await?;
    let versions = cleaned
        .into_iter()
        .map(|(version, fetched_at, original, html)| Version {
            version,
            fetched_at,
            title: original.title,
            html: state.text(&html),
            authors: original.authors,
            tags: original.tags,
            url: original.url,
        })
        .collect();
    Ok(Json(Versions { versions }))
}

/// What to change about one article; what is left out stays.
///
/// What `PUT /api/feeds/{id}/articles/{seq}/state` reads.
#[derive(Deserialize, TS)]
#[ts(optional_fields = nullable)]
struct StateChange {
    read: Option<bool>,
    starred: Option<bool>,
}

async fn set_state(
    State(state): State<AppState>,
    session: Session,
    Path((feed_id, seq)): Path<(i32, i32)>,
    Json(change): Json<StateChange>,
) -> Result<StatusCode, AppError> {
    let account_id = session.account_id;
    if change.read.is_none() && change.starred.is_none() {
        return Err(AppError::BadRequest("nothing to change".into()));
    }
    if let Some(read) = change.read
        && !reading::set_read(&state.db, account_id, feed_id, seq, read).await?
    {
        return Err(AppError::NotFound);
    }
    if let Some(starred) = change.starred
        && !reading::set_starred(&state.db, account_id, feed_id, seq, starred).await?
    {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// How far to mark as read: up to a number in the feed, or up to the article
/// of this `id` in the order of arrival, which is what a mixed list knows.
/// With `older_than`, only what was published more than that many seconds
/// ago.
///
/// What `POST /api/feeds/{id}/read`, `/api/folders/{id}/read` and `/api/read`
/// read.
#[derive(Deserialize, TS)]
#[ts(optional_fields = nullable)]
struct Upto {
    upto: Option<i32>,
    upto_id: Option<i64>,
    older_than: Option<i64>,
}

/// The longest age that can be asked for, in seconds: a hundred years.
const MAX_AGE: i64 = 100 * 365 * 86_400;

impl Upto {
    /// The time before which an article is old enough, if an age is asked for.
    fn cutoff(&self) -> Result<Option<OffsetDateTime>, AppError> {
        match self.older_than {
            None => Ok(None),
            Some(age) if (1..=MAX_AGE).contains(&age) => Ok(Some(
                OffsetDateTime::now_utc() - time::Duration::seconds(age),
            )),
            Some(_) => Err(AppError::BadRequest("older_than is out of range".into())),
        }
    }
}

/// Marks as read up to a number in a feed: everything, or what is older than
/// `cutoff`.
async fn read_upto(
    state: &AppState,
    session: &Session,
    feed_id: i32,
    upto: i32,
    cutoff: Option<OffsetDateTime>,
) -> Result<(), AppError> {
    let account_id = session.account_id;
    match cutoff {
        Some(cutoff) => {
            reading::mark_older_read(&state.db, account_id, feed_id, upto, cutoff).await?
        }
        None => reading::mark_all_read(&state.db, account_id, feed_id, upto).await?,
    };
    Ok(())
}

/// Marks as read, in each of these feeds, what arrived up to the article of
/// this id. Articles that came in after the reader's list was loaded stay
/// unread.
async fn read_upto_id(
    state: &AppState,
    session: &Session,
    feed_ids: &[i32],
    upto_id: i64,
    cutoff: Option<OffsetDateTime>,
) -> Result<(), AppError> {
    let marks = sqlx::query!(
        r#"SELECT s.feed_id,
                  (SELECT a.seq FROM articles a
                   WHERE a.feed_id = s.feed_id AND a.id <= $3
                   ORDER BY a.id DESC LIMIT 1) AS upto
           FROM subscriptions s
           WHERE s.account_id = $1 AND s.feed_id = ANY($2)"#,
        session.account_id,
        feed_ids,
        upto_id,
    )
    .fetch_all(&state.db)
    .await?;
    for mark in marks {
        if let Some(upto) = mark.upto {
            read_upto(state, session, mark.feed_id, upto, cutoff).await?;
        }
    }
    Ok(())
}

async fn read_feed(
    State(state): State<AppState>,
    session: Session,
    Path(feed_id): Path<i32>,
    Json(upto): Json<Upto>,
) -> Result<StatusCode, AppError> {
    subscribed(&state, &session, feed_id).await?;
    let cutoff = upto.cutoff()?;
    match (upto.upto, upto.upto_id) {
        (Some(seq), None) if seq >= 0 => read_upto(&state, &session, feed_id, seq, cutoff).await?,
        (None, Some(id)) => read_upto_id(&state, &session, &[feed_id], id, cutoff).await?,
        _ => return Err(AppError::BadRequest("give upto or upto_id".into())),
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn read_many(
    state: &AppState,
    session: &Session,
    folder_id: Option<i32>,
    upto: Upto,
) -> Result<StatusCode, AppError> {
    let (None, Some(upto_id)) = (upto.upto, upto.upto_id) else {
        return Err(AppError::BadRequest("give upto_id".into()));
    };
    let cutoff = upto.cutoff()?;
    let feeds = feeds_of(state, session, folder_id)
        .await?
        .ok_or(AppError::NotFound)?;
    read_upto_id(state, session, &feeds, upto_id, cutoff).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn read_folder(
    State(state): State<AppState>,
    session: Session,
    Path(folder_id): Path<i32>,
    Json(upto): Json<Upto>,
) -> Result<StatusCode, AppError> {
    read_many(&state, &session, Some(folder_id), upto).await
}

async fn read_all(
    State(state): State<AppState>,
    session: Session,
    Json(upto): Json<Upto>,
) -> Result<StatusCode, AppError> {
    read_many(&state, &session, None, upto).await
}
