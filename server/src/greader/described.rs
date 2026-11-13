//! What the requests of the Google Reader API carry, for its description
//! only. The handlers read their parameters themselves ([`super::parse`]),
//! from the query of a GET or the form of a POST alike, as apps send either;
//! these types say which there are, for `#[utoipa::path]` and nothing else.

// Never made: they only describe what `parse::Params` reads.
#![allow(dead_code)]

use utoipa::{IntoParams, ToSchema};

/// What `ClientLogin` reads, as a form.
#[derive(ToSchema)]
#[schema(as = GreaderClientLogin)]
#[allow(non_snake_case)]
pub struct ClientLogin {
    /// The name of the account (apps call it the e-mail).
    Email: String,
    /// An app password of the account, as Settings, Apps made it.
    Passwd: String,
    /// `json` for an answer in JSON instead of text.
    output: Option<String>,
}

/// Which part of a stream is asked for: `stream/items/ids` and
/// `stream/contents`, in the query or as a form.
#[derive(IntoParams)]
#[into_params(parameter_in = Query)]
pub struct StreamPage {
    /// The stream: `feed/<id>`, `user/-/label/<folder or label>`,
    /// `user/-/state/com.google/reading-list` (all), `.../starred` or
    /// `.../read`. Not for `stream/contents/<stream>`, which names it in the
    /// path.
    s: Option<String>,
    /// How many items at most.
    n: Option<u32>,
    /// `o` for the oldest first; the newest come first otherwise.
    r: Option<String>,
    /// Where the page goes on: the `continuation` of the page before.
    c: Option<String>,
    /// Leave out what has this state, such as
    /// `user/-/state/com.google/read`.
    xt: Option<String>,
    /// Only what has this state.
    it: Option<String>,
    /// Only what arrived after this time, since 1970 in seconds,
    /// milliseconds or microseconds (told apart by the number of digits).
    ot: Option<i64>,
    /// Only what arrived before this time, as `ot`.
    nt: Option<i64>,
}

/// What `edit-tag` reads, as a form.
#[derive(ToSchema)]
#[schema(as = GreaderEditTag)]
pub struct EditTag {
    /// The ID of an item; once per item.
    i: Vec<String>,
    /// A state or label to give them: `user/-/state/com.google/read`,
    /// `.../starred` or `user/-/label/<label>`.
    a: Option<String>,
    /// A state or label to take from them.
    r: Option<String>,
}

/// What `mark-all-as-read` reads, as a form.
#[derive(ToSchema)]
#[schema(as = GreaderMarkAllRead)]
pub struct MarkAllRead {
    /// The stream to mark read, as for `stream/contents`.
    s: String,
    /// Only what arrived before this time, as `ot` of `stream/contents`, so
    /// that nothing that arrived since the app looked is marked.
    ts: Option<i64>,
}

/// What `subscription/edit` reads, as a form.
#[derive(ToSchema)]
#[schema(as = GreaderEditSubscription)]
pub struct EditSubscription {
    /// `subscribe`, `unsubscribe` or `edit`.
    ac: String,
    /// The feed: `feed/<address>` to subscribe, `feed/<id>` otherwise.
    s: String,
    /// The folder to put it in: `user/-/label/<folder>`.
    a: Option<String>,
    /// The folder to take it out of.
    r: Option<String>,
    /// A name of the reader's own for it.
    t: Option<String>,
}

/// What `subscription/quickadd` reads, as a form.
#[derive(ToSchema)]
#[schema(as = GreaderQuickAdd)]
pub struct QuickAdd {
    /// The address of a feed, or of a web page that names one.
    quickadd: String,
}

/// What `rename-tag` reads, as a form.
#[derive(ToSchema)]
#[schema(as = GreaderRenameTag)]
pub struct RenameTag {
    /// The folder or label: `user/-/label/<name>`.
    s: String,
    /// What it is called from now on: `user/-/label/<name>`.
    dest: String,
}

/// What `disable-tag` reads, as a form.
#[derive(ToSchema)]
#[schema(as = GreaderDisableTag)]
pub struct DisableTag {
    /// The folder or label to take away: `user/-/label/<name>`. A folder's
    /// feeds move to the top; a label leaves its articles.
    s: String,
}
