//! Changing the tree of folders and feeds, and a reader's settings.
//!
//! A level of the tree (the top, or the inside of a folder) holds folders and
//! feeds in one order. Whenever a level changes, its nodes are numbered anew
//! from 1, so positions never run out of room between two neighbors. Changes
//! to one account's tree happen one after the other: each locks the account.
//!
//! The rules of the tree itself (no folder inside itself, at most five levels)
//! are the database's; see the trigger in the first migration.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch, post, put},
};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;

use crate::{AppError, AppState, filters, sessions::Session};

/// The longest name of a folder or a subscription, in characters.
pub const MAX_TITLE: usize = 100;
/// How many folders an account can have.
pub const MAX_FOLDERS: i64 = 100;
/// How many feeds an account can subscribe to. Every one of them is fetched
/// for as long as somebody reads it.
pub const MAX_FEEDS: i64 = 2000;
/// How many feeds one folder, or the top level, can hold.
pub const MAX_FEEDS_IN_FOLDER: i64 = 1000;
/// The largest settings of an account, in bytes of JSON.
pub const MAX_SETTINGS: usize = 16 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/tree/move", post(move_node))
        .route("/folders", post(add_folder))
        .route("/folders/{id}", patch(rename_folder).delete(remove_folder))
        .route("/feeds/{id}", patch(rename_feed).delete(unsubscribe))
        .route("/feeds/{id}/agent", put(set_agent))
        .route("/settings", get(settings).put(set_settings))
}

/// A folder or a subscription, as the tree's changes name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "lowercase")]
pub enum Node {
    Folder(i32),
    Feed(i32),
}

pub(crate) fn bad(message: &str) -> AppError {
    AppError::BadRequest(message.into())
}

/// A name as it is stored, or why it cannot be one.
pub(crate) fn title(title: &str) -> Result<&str, AppError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(bad("the name is empty"));
    }
    if title.chars().count() > MAX_TITLE {
        return Err(bad("the name is too long"));
    }
    if title.chars().any(char::is_control) {
        return Err(bad("the name has characters that cannot be typed"));
    }
    Ok(title)
}

/// What the database says when a change breaks a rule of the tree becomes an
/// answer the reader can do something with.
pub(crate) fn rule(err: sqlx::Error) -> AppError {
    let broken = match &err {
        sqlx::Error::Database(db) => db.constraint().map(str::to_owned),
        _ => None,
    };
    match broken.as_deref() {
        Some("folders_max_depth") => bad("folders nest at most 5 levels deep"),
        Some("folders_no_circle") => bad("a folder cannot move into itself"),
        _ => err.into(),
    }
}

/// One change to an account's tree at a time.
pub(crate) async fn lock(conn: &mut PgConnection, account_id: i32) -> Result<(), AppError> {
    sqlx::query!(
        "SELECT id FROM accounts WHERE id = $1 FOR UPDATE",
        account_id
    )
    .fetch_one(conn)
    .await?;
    Ok(())
}

/// Whether a folder, or the top level, can take one more feed.
pub(crate) async fn room_for_feed(
    conn: &mut PgConnection,
    account_id: i32,
    parent: Option<i32>,
) -> Result<(), AppError> {
    let have = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM subscriptions
           WHERE account_id = $1 AND folder_id IS NOT DISTINCT FROM $2"#,
        account_id,
        parent,
    )
    .fetch_one(conn)
    .await?;
    if have >= MAX_FEEDS_IN_FOLDER {
        return Err(bad("too many feeds in this folder"));
    }
    Ok(())
}

/// Whether the account can subscribe to one more feed.
pub(crate) async fn room_for_subscription(
    conn: &mut PgConnection,
    account_id: i32,
) -> Result<(), AppError> {
    let have = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM subscriptions WHERE account_id = $1"#,
        account_id,
    )
    .fetch_one(conn)
    .await?;
    if have >= MAX_FEEDS {
        return Err(bad("too many feeds"));
    }
    Ok(())
}

/// The position after the last node of a level.
pub(crate) async fn end_of(
    conn: &mut PgConnection,
    account_id: i32,
    parent: Option<i32>,
) -> Result<f64, AppError> {
    let last = sqlx::query_scalar!(
        r#"SELECT greatest(
               (SELECT max(position) FROM folders
                WHERE account_id = $1 AND parent_id IS NOT DISTINCT FROM $2),
               (SELECT max(position) FROM subscriptions
                WHERE account_id = $1 AND folder_id IS NOT DISTINCT FROM $2))"#,
        account_id,
        parent,
    )
    .fetch_one(conn)
    .await?;
    Ok(last.unwrap_or(0.0) + 1.0)
}

/// Where a node is: `None` if the account has no such node, else its folder.
async fn parent_of(
    conn: &mut PgConnection,
    account_id: i32,
    node: Node,
) -> Result<Option<Option<i32>>, AppError> {
    let parent = match node {
        Node::Folder(id) => {
            sqlx::query_scalar!(
                "SELECT parent_id FROM folders WHERE account_id = $1 AND id = $2",
                account_id,
                id
            )
            .fetch_optional(conn)
            .await?
        }
        Node::Feed(id) => {
            sqlx::query_scalar!(
                "SELECT folder_id FROM subscriptions WHERE account_id = $1 AND feed_id = $2",
                account_id,
                id
            )
            .fetch_optional(conn)
            .await?
        }
    };
    Ok(parent)
}

/// The nodes of one level in their order. Where positions are equal, folders
/// come first, then the older node.
async fn level(
    conn: &mut PgConnection,
    account_id: i32,
    parent: Option<i32>,
) -> Result<Vec<Node>, AppError> {
    let rows = sqlx::query!(
        r#"SELECT true AS "folder!", id AS "id!", position AS "position!" FROM folders
           WHERE account_id = $1 AND parent_id IS NOT DISTINCT FROM $2
           UNION ALL
           SELECT false, feed_id, position FROM subscriptions
           WHERE account_id = $1 AND folder_id IS NOT DISTINCT FROM $2
           ORDER BY 3, 1 DESC, 2"#,
        account_id,
        parent,
    )
    .fetch_all(conn)
    .await?;
    let node = |folder, id| {
        if folder {
            Node::Folder(id)
        } else {
            Node::Feed(id)
        }
    };
    Ok(rows
        .into_iter()
        .map(|row| node(row.folder, row.id))
        .collect())
}

/// Numbers the nodes of a level from 1, in this order.
async fn renumber(
    conn: &mut PgConnection,
    account_id: i32,
    nodes: &[Node],
) -> Result<(), AppError> {
    let (mut folder_ids, mut folder_positions) = (Vec::new(), Vec::new());
    let (mut feed_ids, mut feed_positions) = (Vec::new(), Vec::new());
    for (index, node) in nodes.iter().enumerate() {
        let position = (index + 1) as f64;
        match node {
            Node::Folder(id) => {
                folder_ids.push(*id);
                folder_positions.push(position);
            }
            Node::Feed(id) => {
                feed_ids.push(*id);
                feed_positions.push(position);
            }
        }
    }
    sqlx::query!(
        "UPDATE folders f SET position = t.position
         FROM unnest($2::int4[], $3::float8[]) AS t(id, position)
         WHERE f.account_id = $1 AND f.id = t.id",
        account_id,
        &folder_ids,
        &folder_positions,
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "UPDATE subscriptions s SET position = t.position
         FROM unnest($2::int4[], $3::float8[]) AS t(id, position)
         WHERE s.account_id = $1 AND s.feed_id = t.id",
        account_id,
        &feed_ids,
        &feed_positions,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Drop {
    Before,
    After,
    Inside,
}

/// Where a node goes, seen from the node it is dropped on, as in the
/// interface's own model of the tree.
#[derive(Deserialize)]
struct Move {
    node: Node,
    target: Node,
    position: Drop,
}

async fn move_node(
    State(state): State<AppState>,
    session: Session,
    Json(change): Json<Move>,
) -> Result<StatusCode, AppError> {
    let account_id = session.account_id;
    if change.node == change.target {
        return Err(bad("a node cannot move onto itself"));
    }
    let mut tx = state.db.begin().await?;
    lock(&mut tx, account_id).await?;
    let Some(from) = parent_of(&mut tx, account_id, change.node).await? else {
        return Err(AppError::NotFound);
    };
    let Some(beside) = parent_of(&mut tx, account_id, change.target).await? else {
        return Err(AppError::NotFound);
    };
    let parent = match (&change.position, change.target) {
        (Drop::Inside, Node::Folder(id)) => Some(id),
        (Drop::Inside, Node::Feed(_)) => return Err(bad("only a folder has an inside")),
        _ => beside,
    };
    if matches!(change.node, Node::Feed(_)) && from != parent {
        room_for_feed(&mut tx, account_id, parent).await?;
    }

    match change.node {
        Node::Folder(id) => sqlx::query!(
            "UPDATE folders SET parent_id = $3 WHERE account_id = $1 AND id = $2",
            account_id,
            id,
            parent,
        )
        .execute(&mut *tx)
        .await
        .map_err(rule)?,
        Node::Feed(id) => {
            sqlx::query!(
                "UPDATE subscriptions SET folder_id = $3 WHERE account_id = $1 AND feed_id = $2",
                account_id,
                id,
                parent,
            )
            .execute(&mut *tx)
            .await?
        }
    };

    let mut nodes = level(&mut tx, account_id, parent).await?;
    nodes.retain(|node| *node != change.node);
    let at = nodes.iter().position(|node| *node == change.target);
    let index = match (change.position, at) {
        (Drop::Before, Some(at)) => at,
        (Drop::After, Some(at)) => at + 1,
        _ => nodes.len(),
    };
    nodes.insert(index, change.node);
    renumber(&mut tx, account_id, &nodes).await?;
    tx.commit().await?;
    filters::tree_changed(&state.db, account_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct NewFolder {
    title: String,
    parent_id: Option<i32>,
}

#[derive(Serialize)]
struct Created {
    id: i32,
}

/// A new, empty folder at the end of its level.
async fn add_folder(
    State(state): State<AppState>,
    session: Session,
    Json(new): Json<NewFolder>,
) -> Result<(StatusCode, Json<Created>), AppError> {
    let account_id = session.account_id;
    let title = title(&new.title)?;
    let mut tx = state.db.begin().await?;
    lock(&mut tx, account_id).await?;
    if let Some(parent) = new.parent_id
        && parent_of(&mut tx, account_id, Node::Folder(parent))
            .await?
            .is_none()
    {
        return Err(AppError::NotFound);
    }
    let have = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM folders WHERE account_id = $1"#,
        account_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if have >= MAX_FOLDERS {
        return Err(bad("too many folders"));
    }
    let id = sqlx::query_scalar!(
        "INSERT INTO folders (account_id, parent_id, title, position) VALUES ($1, $2, $3, 0)
         RETURNING id",
        account_id,
        new.parent_id,
        title,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(rule)?;
    let mut nodes = level(&mut tx, account_id, new.parent_id).await?;
    nodes.retain(|node| *node != Node::Folder(id));
    nodes.push(Node::Folder(id));
    renumber(&mut tx, account_id, &nodes).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(Created { id })))
}

#[derive(Deserialize)]
struct Rename {
    title: Option<String>,
}

async fn rename_folder(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i32>,
    Json(rename): Json<Rename>,
) -> Result<StatusCode, AppError> {
    let title = title(rename.title.as_deref().unwrap_or_default())?;
    let renamed = sqlx::query!(
        "UPDATE folders SET title = $3 WHERE account_id = $1 AND id = $2",
        session.account_id,
        id,
        title,
    )
    .execute(&state.db)
    .await?;
    if renamed.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Takes a folder out. What was in it moves to where the folder was.
async fn remove_folder(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let account_id = session.account_id;
    let mut tx = state.db.begin().await?;
    lock(&mut tx, account_id).await?;
    let Some(parent) = parent_of(&mut tx, account_id, Node::Folder(id)).await? else {
        return Err(AppError::NotFound);
    };
    let inside = level(&mut tx, account_id, Some(id)).await?;
    let mut nodes = level(&mut tx, account_id, parent).await?;
    if let Some(at) = nodes.iter().position(|node| *node == Node::Folder(id)) {
        nodes.splice(at..=at, inside);
    }
    sqlx::query!(
        "UPDATE folders SET parent_id = $3 WHERE account_id = $1 AND parent_id = $2",
        account_id,
        id,
        parent,
    )
    .execute(&mut *tx)
    .await
    .map_err(rule)?;
    sqlx::query!(
        "UPDATE subscriptions SET folder_id = $3 WHERE account_id = $1 AND folder_id = $2",
        account_id,
        id,
        parent,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM folders WHERE account_id = $1 AND id = $2",
        account_id,
        id
    )
    .execute(&mut *tx)
    .await?;
    renumber(&mut tx, account_id, &nodes).await?;
    tx.commit().await?;
    filters::tree_changed(&state.db, account_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Gives a subscription a name of the reader's own, or, without one, the
/// feed's name back.
async fn rename_feed(
    State(state): State<AppState>,
    session: Session,
    Path(feed_id): Path<i32>,
    Json(rename): Json<Rename>,
) -> Result<StatusCode, AppError> {
    let own = match rename.title.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(own) => Some(title(own)?),
    };
    let renamed = sqlx::query!(
        "UPDATE subscriptions SET title = $3 WHERE account_id = $1 AND feed_id = $2",
        session.account_id,
        feed_id,
        own,
    )
    .execute(&state.db)
    .await?;
    if renamed.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum FetchAgent {
    Rueder,
    Browser,
}

#[derive(Deserialize)]
struct NewAgent {
    agent: FetchAgent,
}

/// Says as what a feed is fetched: as rueder, or as a browser for sites that
/// turn rueder away. The feed is one for all who read it, so this holds for
/// all of them, and any of them can say it. The next fetch comes at once.
async fn set_agent(
    State(state): State<AppState>,
    session: Session,
    Path(feed_id): Path<i32>,
    Json(new): Json<NewAgent>,
) -> Result<StatusCode, AppError> {
    let agent = match new.agent {
        FetchAgent::Rueder => "rueder",
        FetchAgent::Browser => "browser",
    };
    let changed = sqlx::query!(
        "UPDATE feeds f SET fetch_agent = $3, next_fetch_at = now()
         FROM subscriptions s
         WHERE s.account_id = $1 AND s.feed_id = $2 AND f.id = s.feed_id",
        session.account_id,
        feed_id,
        agent,
    )
    .execute(&state.db)
    .await?;
    if changed.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Ends a subscription, and with it what the reader read, starred and
/// labelled there.
/// The feed and its articles stay for whoever else reads it.
async fn unsubscribe(
    State(state): State<AppState>,
    session: Session,
    Path(feed_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let mut tx = state.db.begin().await?;
    let gone = sqlx::query!(
        "DELETE FROM subscriptions WHERE account_id = $1 AND feed_id = $2",
        session.account_id,
        feed_id,
    )
    .execute(&mut *tx)
    .await?;
    if gone.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    // Labels hang on the article, not on the subscription.
    sqlx::query!(
        "DELETE FROM article_labels al USING labels l
         WHERE l.id = al.label_id AND l.account_id = $1 AND al.feed_id = $2",
        session.account_id,
        feed_id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn settings(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<serde_json::Value>, AppError> {
    let settings = sqlx::query_scalar!(
        "SELECT settings FROM accounts WHERE id = $1",
        session.account_id
    )
    .fetch_one(&state.db)
    .await?;
    Ok(Json(settings))
}

/// Replaces the settings. What is in them is the interface's business; the
/// server only keeps them, within a size.
async fn set_settings(
    State(state): State<AppState>,
    session: Session,
    Json(settings): Json<serde_json::Value>,
) -> Result<StatusCode, AppError> {
    if !settings.is_object() {
        return Err(bad("settings are an object"));
    }
    if settings.to_string().len() > MAX_SETTINGS {
        return Err(bad("the settings are too large"));
    }
    sqlx::query!(
        "UPDATE accounts SET settings = $2 WHERE id = $1",
        session.account_id,
        settings,
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
