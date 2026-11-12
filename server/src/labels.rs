//! Labels: a reader's own marks for articles, each with a name and a color.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch, post, put},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    AppError, AppState,
    sessions::Session,
    tree::{bad, title},
};

/// How many labels an account can have.
pub const MAX_LABELS: i64 = 100;

/// The types this module sends and reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types
        .add::<Label>()
        .add::<NewLabel>()
        .add::<Created>()
        .add::<Change>()
        .add::<Order>();
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/labels", get(labels).post(add))
        .route("/labels/order", post(order))
        .route("/labels/{id}", patch(change).delete(remove))
        .route(
            "/feeds/{feed_id}/articles/{seq}/labels/{id}",
            put(attach).delete(detach),
        )
}

/// A label. `GET /api/labels` answers with a list of them.
#[derive(Serialize, TS)]
struct Label {
    id: i32,
    title: String,
    color: String,
    position: f64,
}

/// A color as `#rrggbb`, which is all the interface makes.
pub(crate) fn color(color: &str) -> Result<&str, AppError> {
    let digits = color.strip_prefix('#').unwrap_or_default();
    if digits.len() == 6 && digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(color)
    } else {
        Err(bad("a color looks like #1a2b3c"))
    }
}

/// Two labels of one account cannot have the same name.
fn taken(err: sqlx::Error) -> AppError {
    match &err {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            bad("there is a label of this name")
        }
        _ => err.into(),
    }
}

async fn labels(
    State(state): State<AppState>,
    session: Session,
) -> Result<Json<Vec<Label>>, AppError> {
    let labels = sqlx::query_as!(
        Label,
        "SELECT id, title, color, position FROM labels
         WHERE account_id = $1 ORDER BY position, id",
        session.account_id
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(labels))
}

/// What `POST /api/labels` reads. It answers with `Created`.
#[derive(Deserialize, TS)]
struct NewLabel {
    title: String,
    color: String,
}

/// The answer to making something: the number it got.
#[derive(Serialize, TS)]
struct Created {
    id: i32,
}

/// A new label, after the ones that are there.
async fn add(
    State(state): State<AppState>,
    session: Session,
    Json(new): Json<NewLabel>,
) -> Result<(StatusCode, Json<Created>), AppError> {
    let (title, color) = (title(&new.title)?, color(&new.color)?);
    let id = sqlx::query_scalar!(
        "INSERT INTO labels (account_id, title, color, position)
         SELECT $1, $2, $3, coalesce(max(position), 0) + 1 FROM labels WHERE account_id = $1
         HAVING count(*) < $4
         RETURNING id",
        session.account_id,
        title,
        color,
        MAX_LABELS,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(taken)?
    .ok_or_else(|| bad("too many labels"))?;
    Ok((StatusCode::CREATED, Json(Created { id })))
}

/// What to change about a label; what is left out stays.
///
/// What `PATCH /api/labels/{id}` reads.
#[derive(Deserialize, TS)]
#[ts(rename = "LabelChange", optional_fields = nullable)]
struct Change {
    title: Option<String>,
    color: Option<String>,
}

async fn change(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i32>,
    Json(change): Json<Change>,
) -> Result<StatusCode, AppError> {
    let title = change.title.as_deref().map(title).transpose()?;
    let color = change.color.as_deref().map(color).transpose()?;
    if title.is_none() && color.is_none() {
        return Err(bad("nothing to change"));
    }
    let changed = sqlx::query!(
        "UPDATE labels SET title = coalesce($3, title), color = coalesce($4, color)
         WHERE account_id = $1 AND id = $2",
        session.account_id,
        id,
        title,
        color,
    )
    .execute(&state.db)
    .await
    .map_err(taken)?;
    if changed.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Takes a label away, from every article that had it too.
async fn remove(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let gone = sqlx::query!(
        "DELETE FROM labels WHERE account_id = $1 AND id = $2",
        session.account_id,
        id
    )
    .execute(&state.db)
    .await?;
    if gone.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// A new order: every number once, in the order they shall stand in.
#[derive(Deserialize, TS)]
struct Order {
    ids: Vec<i32>,
}

/// Puts the labels in this order. Labels that are not named keep their place
/// among themselves, after the named ones.
async fn order(
    State(state): State<AppState>,
    session: Session,
    Json(order): Json<Order>,
) -> Result<StatusCode, AppError> {
    if order.ids.len() > MAX_LABELS as usize {
        return Err(bad("too many labels"));
    }
    sqlx::query!(
        "UPDATE labels l SET position = n.position
         FROM (
             SELECT k.id, row_number() OVER (ORDER BY t.at, k.position, k.id) AS position
             FROM labels k
             LEFT JOIN unnest($2::int4[]) WITH ORDINALITY AS t(id, at) ON t.id = k.id
             WHERE k.account_id = $1
         ) n
         WHERE l.id = n.id",
        session.account_id,
        &order.ids,
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Puts a label on an article the reader can read.
async fn attach(
    State(state): State<AppState>,
    session: Session,
    Path((feed_id, seq, id)): Path<(i32, i32, i32)>,
) -> Result<StatusCode, AppError> {
    let found = sqlx::query!(
        r#"WITH new AS (
               SELECT l.id, a.feed_id, a.seq
               FROM labels l, subscriptions s
               JOIN articles a ON a.feed_id = s.feed_id
               WHERE l.account_id = $1 AND l.id = $2
                 AND s.account_id = $1 AND s.feed_id = $3 AND a.seq = $4
           ), put AS (
               INSERT INTO article_labels (label_id, feed_id, seq)
               SELECT id, feed_id, seq FROM new
               ON CONFLICT DO NOTHING
           )
           SELECT count(*) AS "n!" FROM new"#,
        session.account_id,
        id,
        feed_id,
        seq,
    )
    .fetch_one(&state.db)
    .await?;
    if found.n == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Takes a label off an article. Taking off what is not on is no error, as
/// long as the label is the reader's.
async fn detach(
    State(state): State<AppState>,
    session: Session,
    Path((feed_id, seq, id)): Path<(i32, i32, i32)>,
) -> Result<StatusCode, AppError> {
    let own = sqlx::query_scalar!(
        "SELECT id FROM labels WHERE account_id = $1 AND id = $2",
        session.account_id,
        id
    )
    .fetch_optional(&state.db)
    .await?;
    if own.is_none() {
        return Err(AppError::NotFound);
    }
    sqlx::query!(
        "DELETE FROM article_labels WHERE label_id = $1 AND feed_id = $2 AND seq = $3",
        id,
        feed_id,
        seq,
    )
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
