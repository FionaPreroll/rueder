//! What apps change: read and starred, labels on articles, everything up to
//! a time as read, subscriptions, and the names of folders and labels.
//!
//! Each goes through what the page's API does, with its checks and limits;
//! changes count per account against `rate::APP_CHANGES`. Item IDs that are
//! not the account's are left out without an error, as apps send what they
//! have kept, and a feed may be gone since.

use std::{collections::BTreeMap, time::Instant};

use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use serde_json::json;

use super::{
    App, Form, ok,
    parse::{self, Params, State as Mark, Stream},
    read::{self, Folders, Source, feed_of, source},
};
use crate::{
    AppError, AppState, api, filters, labels,
    reading::{self, Many},
    sessions::Session,
    subscribe, tree,
};

/// How many item IDs one change may name: as many as NetNewsWire and Read
/// You send at once.
pub const MAX_IDS: usize = 1000;
/// The colors a label made by an app gets, in turn, as the page gives them.
const COLORS: [&str; 8] = [
    "#d93025", "#e8710a", "#f9ab00", "#1e8e3e", "#12b5cb", "#1a73e8", "#9334e6", "#5f6368",
];

/// Counts a change against the account's limit.
fn counted(state: &AppState, session: &Session) -> Result<(), AppError> {
    state.rates.app_change(session.account_id, Instant::now())
}

/// What one tag of `edit-tag` does to the articles.
enum Tag {
    Many(Many),
    Label(String),
    Nothing,
}

fn tag(stream: &str, add: bool) -> Result<Tag, AppError> {
    let stream = parse::stream(stream).ok_or_else(|| AppError::BadRequest("unknown tag".into()))?;
    Ok(match stream {
        Stream::State(Mark::Read) => Tag::Many(Many::Read(add)),
        Stream::State(Mark::KeptUnread) => Tag::Many(Many::Read(!add)),
        Stream::State(Mark::Starred) => Tag::Many(Many::Starred(add)),
        Stream::Label(name) => Tag::Label(name),
        Stream::State(_) | Stream::Feed(_) => Tag::Nothing,
    })
}

/// `POST /reader/api/0/edit-tag`: adds (`a`) and removes (`r`) tags on the
/// articles of the IDs `i`: read, kept unread, starred, labels.
pub(super) async fn edit_tag(
    State(state): State<AppState>,
    App(login): App,
    Form(params): Form,
) -> Result<Response, AppError> {
    let session = &login.session;
    counted(&state, session)?;
    let ids: Vec<&str> = params.all("i").collect();
    if ids.len() > MAX_IDS {
        let message = format!("at most {MAX_IDS} items at a time");
        return Err(AppError::BadRequest(message));
    }
    let mut changes = Vec::new();
    for added in params.all("a") {
        changes.push((tag(added, true)?, true));
    }
    for removed in params.all("r") {
        changes.push((tag(removed, false)?, false));
    }
    let many: Vec<Many> = changes
        .iter()
        .filter_map(|(change, _)| match change {
            Tag::Many(many) => Some(*many),
            _ => None,
        })
        .collect();
    let read: Vec<bool> = many
        .iter()
        .filter_map(|many| match many {
            Many::Read(read) => Some(*read),
            Many::Starred(_) => None,
        })
        .collect();
    let starred: Vec<bool> = many
        .iter()
        .filter_map(|many| match many {
            Many::Starred(on) => Some(*on),
            Many::Read(_) => None,
        })
        .collect();
    if read.windows(2).any(|pair| pair[0] != pair[1])
        || starred.windows(2).any(|pair| pair[0] != pair[1])
    {
        return Err(AppError::BadRequest(
            "a tag is added and removed at once".into(),
        ));
    }
    // By feed, as the read state is kept per feed.
    let mut by_feed: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
    for (feed, seq) in ids.into_iter().filter_map(parse::item) {
        by_feed.entry(feed).or_default().push(seq);
    }
    let mut tx = state.db.begin().await?;
    for (feed, seqs) in &by_feed {
        for change in &many {
            reading::set_many_on(&mut tx, session.account_id, *feed, seqs, *change).await?;
        }
    }
    tx.commit().await?;
    for (change, add) in &changes {
        if let Tag::Label(name) = change {
            label_articles(&state, session, name, *add, &by_feed).await?;
        }
    }
    Ok(ok())
}

/// Puts a label on these articles, or takes it off. An app may name a label
/// that is not there yet; putting it on makes it, with the limits of the
/// page.
async fn label_articles(
    state: &AppState,
    session: &Session,
    name: &str,
    add: bool,
    articles: &BTreeMap<i32, Vec<i32>>,
) -> Result<(), AppError> {
    let name = name.trim();
    let found = sqlx::query_scalar!(
        "SELECT id FROM labels WHERE account_id = $1 AND title = $2",
        session.account_id,
        name
    )
    .fetch_optional(&state.db)
    .await?;
    let label = match (found, add) {
        (Some(id), _) => id,
        (None, false) => return Ok(()),
        (None, true) => {
            let have = sqlx::query_scalar!(
                r#"SELECT count(*) AS "n!" FROM labels WHERE account_id = $1"#,
                session.account_id
            )
            .fetch_one(&state.db)
            .await?;
            let color = COLORS[usize::try_from(have).unwrap_or(0) % COLORS.len()];
            let new = labels::NewLabel {
                title: name.to_owned(),
                color: color.to_owned(),
            };
            let (_, Json(created)) =
                labels::add(State(state.clone()), session.clone(), Json(new)).await?;
            created.id
        }
    };
    let (feeds, seqs): (Vec<i32>, Vec<i32>) = articles
        .iter()
        .flat_map(|(feed, seqs)| seqs.iter().map(move |seq| (*feed, *seq)))
        .unzip();
    if add {
        sqlx::query!(
            "INSERT INTO article_labels (label_id, feed_id, seq)
             SELECT $2, a.feed_id, a.seq
             FROM unnest($3::int4[], $4::int4[]) AS n (feed_id, seq)
             JOIN subscriptions s ON s.account_id = $1 AND s.feed_id = n.feed_id
             JOIN articles a ON a.feed_id = n.feed_id AND a.seq = n.seq
             ON CONFLICT DO NOTHING",
            session.account_id,
            label,
            &feeds,
            &seqs,
        )
        .execute(&state.db)
        .await?;
    } else {
        sqlx::query!(
            "DELETE FROM article_labels
             WHERE label_id = $1
               AND (feed_id, seq) IN (SELECT * FROM unnest($2::int4[], $3::int4[]))",
            label,
            &feeds,
            &seqs,
        )
        .execute(&state.db)
        .await?;
    }
    Ok(())
}

/// `POST /reader/api/0/mark-all-as-read`: marks a stream as read, up to what
/// had arrived at `ts`, if the app says; what came after stays unread.
pub(super) async fn mark_all_read(
    State(state): State<AppState>,
    App(login): App,
    Form(params): Form,
) -> Result<Response, AppError> {
    let session = &login.session;
    counted(&state, session)?;
    let stream = params
        .one("s")
        .and_then(parse::stream)
        .ok_or_else(|| AppError::BadRequest("unknown stream".into()))?;
    let feeds = match source(&state, session, &stream).await? {
        Source::Feeds(feeds) => feeds,
        // Starred articles and labels have no read mark of their own.
        Source::Marked(_) => return Ok(ok()),
    };
    let upto = match params.one("ts").and_then(parse::time) {
        // The last article that had arrived before `at`.
        Some(at) => read::arrived_before(&state.db, at).await?.saturating_sub(1),
        None => i64::MAX,
    };
    api::read_upto_id(&state, session, &feeds, upto, None).await?;
    Ok(ok())
}

/// The folder at this path, made where it is not there yet, with the limits
/// of the page.
async fn folder_at(state: &AppState, session: &Session, path: &[String]) -> Result<i32, AppError> {
    let mut parent = None;
    for depth in 1..=path.len() {
        let folders = Folders::of(&state.db, session.account_id).await?;
        parent = match folders.find(&path[..depth]) {
            Some(id) => Some(id),
            None => {
                let new = tree::NewFolder {
                    title: path[depth - 1].clone(),
                    parent_id: parent,
                };
                let (_, Json(created)) =
                    tree::add_folder(State(state.clone()), session.clone(), Json(new)).await?;
                Some(created.id)
            }
        };
    }
    parent.ok_or_else(|| AppError::BadRequest("a folder needs a name".into()))
}

/// The folder a label stream names, made if it is not there.
async fn folder_of(state: &AppState, session: &Session, stream: &str) -> Result<i32, AppError> {
    let Some(Stream::Label(name)) = parse::stream(stream) else {
        return Err(AppError::BadRequest("a folder is a label".into()));
    };
    let path = parse::path(&name).ok_or_else(|| AppError::BadRequest("no such folder".into()))?;
    folder_at(state, session, &path).await
}

/// Moves a subscription to the end of a folder, or of the top level.
async fn place(
    state: &AppState,
    session: &Session,
    feed_id: i32,
    folder: Option<i32>,
) -> Result<(), AppError> {
    let account_id = session.account_id;
    let mut tx = state.db.begin().await?;
    tree::lock(&mut tx, account_id).await?;
    let now = sqlx::query_scalar!(
        "SELECT folder_id FROM subscriptions WHERE account_id = $1 AND feed_id = $2",
        account_id,
        feed_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;
    if now == folder {
        return Ok(());
    }
    tree::room_for_feed(&mut tx, account_id, folder).await?;
    let position = tree::end_of(&mut tx, account_id, folder).await?;
    sqlx::query!(
        "UPDATE subscriptions SET folder_id = $3, position = $4
         WHERE account_id = $1 AND feed_id = $2",
        account_id,
        feed_id,
        folder,
        position,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    filters::tree_changed(&state.db, account_id).await?;
    Ok(())
}

/// `POST /reader/api/0/subscription/edit`: subscribes (`ac=subscribe`, `s`
/// the feed's address), unsubscribes (`ac=unsubscribe`, every `s`) or
/// changes a subscription (`ac=edit`): a name (`t`), a folder to go into
/// (`a`) or to leave for the top level (`r`).
pub(super) async fn edit_subscription(
    State(state): State<AppState>,
    App(login): App,
    Form(params): Form,
) -> Result<Response, AppError> {
    let session = &login.session;
    counted(&state, session)?;
    match params.one("ac") {
        Some("subscribe") => {
            let Some(Stream::Feed(parse::FeedRef::Url(url))) =
                params.one("s").and_then(parse::stream)
            else {
                return Err(AppError::BadRequest("subscribe to feed/<address>".into()));
            };
            let folder = match params.one("a") {
                Some(label) => Some(folder_of(&state, session, label).await?),
                None => None,
            };
            let new = subscribe::New {
                url,
                folder_id: folder,
            };
            let (_, Json(subscribed)) =
                subscribe::subscribe(State(state.clone()), session.clone(), Json(new)).await?;
            rename(&state, session, subscribed.id, &params).await?;
        }
        Some("unsubscribe") => {
            for stream in params.all("s") {
                let Some(Stream::Feed(feed)) = parse::stream(stream) else {
                    return Err(AppError::BadRequest("unsubscribe from feed/<id>".into()));
                };
                let id = feed_of(&state, session, &feed).await?;
                tree::unsubscribe(State(state.clone()), session.clone(), Path(id)).await?;
            }
        }
        Some("edit") => {
            let Some(Stream::Feed(feed)) = params.one("s").and_then(parse::stream) else {
                return Err(AppError::BadRequest("edit feed/<id>".into()));
            };
            let id = feed_of(&state, session, &feed).await?;
            rename(&state, session, id, &params).await?;
            if let Some(label) = params.one("a") {
                let folder = folder_of(&state, session, label).await?;
                place(&state, session, id, Some(folder)).await?;
            } else if params.one("r").is_some() {
                place(&state, session, id, None).await?;
            }
        }
        _ => {
            return Err(AppError::BadRequest(
                "ac is subscribe, unsubscribe or edit".into(),
            ));
        }
    }
    Ok(ok())
}

/// Gives a subscription the name `t`, if the app sends one.
async fn rename(
    state: &AppState,
    session: &Session,
    feed_id: i32,
    params: &Params,
) -> Result<(), AppError> {
    let Some(title) = params.one("t").filter(|title| !title.trim().is_empty()) else {
        return Ok(());
    };
    let rename = tree::Rename {
        title: Some(title.to_owned()),
    };
    tree::rename_feed(
        State(state.clone()),
        session.clone(),
        Path(feed_id),
        Json(rename),
    )
    .await?;
    Ok(())
}

/// `POST /reader/api/0/subscription/quickadd`: subscribes to the feed at an
/// address, or to the first one a web page names.
pub(super) async fn quick_add(
    State(state): State<AppState>,
    App(login): App,
    Form(params): Form,
) -> Result<Response, AppError> {
    let session = &login.session;
    counted(&state, session)?;
    let query = params.one("quickadd").unwrap_or_default().trim().to_owned();
    let find = subscribe::Find { url: query.clone() };
    let Json(found) = subscribe::find(State(state.clone()), session.clone(), Json(find)).await?;
    let Some(first) = found.feeds.into_iter().next() else {
        return Ok(Json(json!({ "numResults": 0 })).into_response());
    };
    let new = subscribe::New {
        url: first.url,
        folder_id: None,
    };
    let (_, Json(subscribed)) =
        subscribe::subscribe(State(state.clone()), session.clone(), Json(new)).await?;
    Ok(Json(json!({
        "numResults": 1,
        "query": query,
        "streamId": parse::feed_id(subscribed.id),
        "streamName": first.title,
    }))
    .into_response())
}

/// `POST /reader/api/0/rename-tag`: renames a folder (`s`, `dest`: paths
/// that differ in their last part) or a label.
pub(super) async fn rename_tag(
    State(state): State<AppState>,
    App(login): App,
    Form(params): Form,
) -> Result<Response, AppError> {
    let session = &login.session;
    counted(&state, session)?;
    let (Some(Stream::Label(from)), Some(Stream::Label(to))) = (
        params.one("s").and_then(parse::stream),
        params.one("dest").and_then(parse::stream),
    ) else {
        return Err(AppError::BadRequest("s and dest are labels".into()));
    };
    let folders = Folders::of(&state.db, session.account_id).await?;
    let paths = (parse::path(&from), parse::path(&to));
    if let (Some(old), Some(new)) = paths
        && let Some(folder) = folders.find(&old)
    {
        let (Some((last, parent)), Some((title, parent_to))) = (old.split_last(), new.split_last())
        else {
            return Err(AppError::BadRequest("a folder needs a name".into()));
        };
        if parent != parent_to {
            return Err(AppError::BadRequest(
                "only the name of a folder changes here".into(),
            ));
        }
        if last != title {
            let rename = tree::Rename {
                title: Some(title.clone()),
            };
            tree::rename_folder(
                State(state.clone()),
                session.clone(),
                Path(folder),
                Json(rename),
            )
            .await?;
        }
        return Ok(ok());
    }
    let label = sqlx::query_scalar!(
        "SELECT id FROM labels WHERE account_id = $1 AND title = $2",
        session.account_id,
        from.trim()
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let change = labels::Change {
        title: Some(to.trim().to_owned()),
        color: None,
    };
    labels::change(
        State(state.clone()),
        session.clone(),
        Path(label),
        Json(change),
    )
    .await?;
    Ok(ok())
}

/// `POST /reader/api/0/disable-tag`: takes folders out, whose feeds move up
/// to where they were, and deletes labels.
pub(super) async fn disable_tag(
    State(state): State<AppState>,
    App(login): App,
    Form(params): Form,
) -> Result<Response, AppError> {
    let session = &login.session;
    counted(&state, session)?;
    for stream in params.all("s") {
        let Some(Stream::Label(name)) = parse::stream(stream) else {
            return Err(AppError::BadRequest("s is a label".into()));
        };
        let folders = Folders::of(&state.db, session.account_id).await?;
        if let Some(folder) = parse::path(&name).and_then(|path| folders.find(&path)) {
            tree::remove_folder(State(state.clone()), session.clone(), Path(folder)).await?;
            continue;
        }
        let label = sqlx::query_scalar!(
            "SELECT id FROM labels WHERE account_id = $1 AND title = $2",
            session.account_id,
            name.trim()
        )
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
        labels::remove(State(state.clone()), session.clone(), Path(label)).await?;
    }
    Ok(ok())
}
