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

use crate::{
    AppError, AppState,
    articles::{self, ListItem, Opened, Page},
    cleaning::{self, Base, Rules},
    images::{Signer, Size},
    reading,
    sessions::Session,
};

/// How many articles a page of a list has if nothing else is asked, and at most.
const PAGE: i64 = 50;
const MAX_PAGE: i64 = 200;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/tree", get(tree))
        .route("/articles", get(all_articles))
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

#[derive(Serialize)]
struct Folder {
    id: i32,
    parent_id: Option<i32>,
    title: String,
    position: f64,
}

/// A subscription with what the tree shows of its feed.
#[derive(Serialize)]
struct Feed {
    id: i32,
    folder_id: Option<i32>,
    position: f64,
    /// The name the reader gave it, else the feed's own.
    title: String,
    feed_title: String,
    feed_url: String,
    site_url: Option<String>,
    icon_url: Option<String>,
    fetch_agent: String,
    /// Why the last fetch failed, if it did.
    error: Option<String>,
    last_seq: i32,
    read_upto: i32,
    unread: i64,
}

#[derive(Serialize)]
struct Tree {
    folders: Vec<Folder>,
    feeds: Vec<Feed>,
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
        "SELECT s.feed_id, s.folder_id, s.position, s.title AS own_title, f.title AS feed_title,
                f.feed_url, f.site_url, f.icon_url, f.fetch_agent, f.error_message
         FROM subscriptions s JOIN feeds f ON f.id = s.feed_id
         WHERE s.account_id = $1 ORDER BY s.position, s.feed_id",
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
                title: row.own_title.unwrap_or_else(|| row.feed_title.clone()),
                feed_title: row.feed_title,
                feed_url: row.feed_url,
                site_url: row.site_url,
                icon_url: state.picture(Size::Icon, row.icon_url),
                fetch_agent: row.fetch_agent,
                error: row.error_message,
                last_seq: count.map_or(0, |count| count.last_seq),
                read_upto: count.map_or(0, |count| count.read_upto),
                unread: count.map_or(0, |count| count.unread),
            }
        })
        .collect();
    Ok(Json(Tree { folders, feeds }))
}

/// Which part of a list is asked for.
#[derive(Deserialize)]
struct ListQuery {
    /// Only what arrived before the article of this `id`.
    before: Option<i64>,
    limit: Option<i64>,
    /// Leave out what was read.
    #[serde(default)]
    unread: bool,
}

#[derive(Serialize)]
struct List {
    articles: Vec<ListItem>,
}

async fn list(
    state: &AppState,
    session: &Session,
    feed_ids: &[i32],
    query: ListQuery,
) -> Result<Json<List>, AppError> {
    let page = Page {
        before: query.before,
        limit: query.limit.unwrap_or(PAGE).clamp(1, MAX_PAGE),
        unread_only: query.unread,
    };
    let mut articles = articles::page(&state.db, session.account_id, feed_ids, page).await?;
    for article in &mut articles {
        article.thumbnail_url = state.picture(Size::Thumbnail, article.thumbnail_url.take());
    }
    Ok(Json(List { articles }))
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

async fn article(
    State(state): State<AppState>,
    session: Session,
    Path((feed_id, seq)): Path<(i32, i32)>,
) -> Result<Json<Opened>, AppError> {
    let mut opened = articles::open(&state.db, session.account_id, feed_id, seq)
        .await?
        .ok_or(AppError::NotFound)?;
    opened.html = state.text(&opened.html);
    opened.image_url = state.picture(Size::Image, opened.image_url.take());
    Ok(Json(opened))
}

/// One version of an article, cleaned for reading like the current one.
#[derive(Serialize)]
struct Version {
    version: i16,
    #[serde(with = "time::serde::rfc3339")]
    fetched_at: OffsetDateTime,
    title: String,
    html: String,
    authors: Vec<String>,
    tags: Vec<String>,
    url: Option<String>,
}

#[derive(Serialize)]
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
#[derive(Deserialize)]
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
#[derive(Deserialize)]
struct Upto {
    upto: Option<i32>,
    upto_id: Option<i64>,
}

/// Marks as read, in each of these feeds, what arrived up to the article of
/// this id. Articles that came in after the reader's list was loaded stay
/// unread.
async fn read_upto_id(
    state: &AppState,
    session: &Session,
    feed_ids: &[i32],
    upto_id: i64,
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
            reading::mark_all_read(&state.db, session.account_id, mark.feed_id, upto).await?;
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
    match (upto.upto, upto.upto_id) {
        (Some(seq), None) if seq >= 0 => {
            reading::mark_all_read(&state.db, session.account_id, feed_id, seq).await?;
        }
        (None, Some(id)) => read_upto_id(&state, &session, &[feed_id], id).await?,
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
    let feeds = feeds_of(state, session, folder_id)
        .await?
        .ok_or(AppError::NotFound)?;
    read_upto_id(state, session, &feeds, upto_id).await?;
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
