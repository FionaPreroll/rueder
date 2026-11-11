//! Changing the tree of folders and feeds over the API, and the settings.

mod common;

use axum::http::{Method, StatusCode};
use common::{Random, call};
use rueder::AppState;
use serde_json::{Value, json};
use sqlx::PgPool;

async fn send(
    state: &AppState,
    method: Method,
    path: &str,
    cookie: &str,
    body: Value,
) -> StatusCode {
    let body = (body != Value::Null).then_some(body);
    call(state, method, path, Some(cookie), body).await.0
}

/// The tree as a model: what is on each level, in order.
#[derive(Debug, Clone, PartialEq)]
enum Node {
    Folder(i32, Vec<Node>),
    Feed(i32),
}

/// A node's name in a request: its kind and its id.
type Key = (&'static str, i32);

impl Node {
    fn key(&self) -> Key {
        match self {
            Node::Folder(id, _) => ("folder", *id),
            Node::Feed(id) => ("feed", *id),
        }
    }

    /// How many levels of folders the node is, itself included.
    fn levels(&self) -> usize {
        match self {
            Node::Folder(_, inside) => 1 + inside.iter().map(Node::levels).max().unwrap_or(0),
            Node::Feed(_) => 0,
        }
    }
}

fn request(key: Key) -> Value {
    json!({ "kind": key.0, "id": key.1 })
}

/// Where a node is: the index on each level, from the top down.
fn path(level: &[Node], key: Key) -> Option<Vec<usize>> {
    for (index, node) in level.iter().enumerate() {
        if node.key() == key {
            return Some(vec![index]);
        }
        if let Node::Folder(_, inside) = node
            && let Some(mut below) = path(inside, key)
        {
            below.insert(0, index);
            return Some(below);
        }
    }
    None
}

/// The level a path's last step is on.
fn level_mut<'a>(mut level: &'a mut Vec<Node>, above: &[usize]) -> &'a mut Vec<Node> {
    for index in above {
        let Node::Folder(_, inside) = &mut level[*index] else {
            panic!("a feed has no inside");
        };
        level = inside;
    }
    level
}

fn keys(level: &[Node], all: &mut Vec<Key>) {
    for node in level {
        all.push(node.key());
        if let Node::Folder(_, inside) = node {
            keys(inside, all);
        }
    }
}

/// The model's move, as `moveNode` in `web/src/lib/tree.ts`: `false` if it is
/// not allowed, and then nothing changes.
fn move_node(tree: &mut Vec<Node>, key: Key, target: Key, position: &str) -> bool {
    if key == target {
        return false;
    }
    let (Some(from), Some(to)) = (path(tree, key), path(tree, target)) else {
        return false;
    };
    if position == "inside" && target.0 != "folder" {
        return false;
    }
    // The folders the node would be in afterwards.
    let around = if position == "inside" {
        to.len()
    } else {
        to.len() - 1
    };
    // Into itself: the target is the node or below it.
    let below = to.len() >= from.len() && to[..from.len()] == from[..];
    if key.0 == "folder" && below && (position == "inside" || to.len() > from.len()) {
        return false;
    }
    let (last, above) = from.split_last().unwrap();
    let levels = level_mut(tree, above)[*last].levels();
    if around + levels > 5 {
        return false;
    }
    let moved = level_mut(tree, above).remove(*last);
    // Looked up again, as taking the node out may have moved the target.
    let to = path(tree, target).unwrap();
    if position == "inside" {
        level_mut(tree, &to).push(moved);
    } else {
        let (last, above) = to.split_last().unwrap();
        let at = last + usize::from(position == "after");
        level_mut(tree, above).insert(at, moved);
    }
    true
}

/// The tree as the server gives it out, in the model's form.
async fn tree_of(state: &AppState, cookie: &str) -> Vec<Node> {
    let (status, tree) = call(state, Method::GET, "/api/tree", Some(cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    fn build(tree: &Value, parent: &Value) -> Vec<Node> {
        let mut level: Vec<(f64, Node)> = Vec::new();
        for folder in tree["folders"].as_array().unwrap() {
            if folder["parent_id"] == *parent {
                let id = folder["id"].as_i64().unwrap() as i32;
                let inside = build(tree, &folder["id"]);
                level.push((
                    folder["position"].as_f64().unwrap(),
                    Node::Folder(id, inside),
                ));
            }
        }
        for feed in tree["feeds"].as_array().unwrap() {
            if feed["folder_id"] == *parent {
                let id = feed["id"].as_i64().unwrap() as i32;
                level.push((feed["position"].as_f64().unwrap(), Node::Feed(id)));
            }
        }
        level.sort_by(|a, b| a.0.total_cmp(&b.0));
        // A level is numbered 1, 2, 3 after every change to it.
        let positions: Vec<f64> = level.iter().map(|(position, _)| *position).collect();
        let mut distinct = positions.clone();
        distinct.dedup();
        assert_eq!(positions, distinct, "two nodes in one place");
        level.into_iter().map(|(_, node)| node).collect()
    }
    build(&tree, &Value::Null)
}

async fn add_folder(
    state: &AppState,
    cookie: &str,
    title: &str,
    parent: Option<i32>,
) -> (StatusCode, i32) {
    let body = json!({ "title": title, "parent_id": parent });
    let (status, created) = call(
        state,
        Method::POST,
        "/api/folders",
        Some(cookie),
        Some(body),
    )
    .await;
    (status, created["id"].as_i64().unwrap_or(0) as i32)
}

async fn move_over_api(
    state: &AppState,
    cookie: &str,
    key: Key,
    target: Key,
    position: &str,
) -> StatusCode {
    let body = json!({ "node": request(key), "target": request(target), "position": position });
    send(state, Method::POST, "/api/tree/move", cookie, body).await
}

#[sqlx::test]
async fn folders_and_feeds_move_as_in_the_interface(db: PgPool) {
    let (ada, one) = common::reader(&db, "ada", 0).await;
    let two = common::feed(&db, "two").await;
    common::subscribe(&db, ada, two).await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    let (f, d) = (Node::Feed, Node::Folder);

    let (status, tech) = add_folder(&state, &cookie, "  Tech ", None).await;
    assert_eq!(status, StatusCode::CREATED);
    let (_, rust) = add_folder(&state, &cookie, "Rust", Some(tech)).await;
    // New folders go to the end of their level.
    assert_eq!(
        tree_of(&state, &cookie).await,
        [f(one), f(two), d(tech, vec![d(rust, vec![])])]
    );
    let title: String = sqlx::query_scalar("SELECT title FROM folders WHERE id = $1")
        .bind(tech)
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(title, "Tech");

    let step = async |key: Key, target: Key, position: &str| {
        move_over_api(&state, &cookie, key, target, position).await
    };
    assert_eq!(
        step(("feed", two), ("folder", rust), "inside").await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        step(("feed", one), ("feed", two), "before").await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        tree_of(&state, &cookie).await,
        [d(tech, vec![d(rust, vec![f(one), f(two)])])]
    );
    assert_eq!(
        step(("folder", rust), ("folder", tech), "before").await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        step(("feed", one), ("folder", tech), "after").await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        tree_of(&state, &cookie).await,
        [d(rust, vec![f(two)]), d(tech, vec![]), f(one)]
    );

    // What the tree does not allow.
    assert_eq!(
        step(("folder", tech), ("folder", tech), "inside").await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        step(("feed", one), ("feed", two), "inside").await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        step(("folder", rust), ("feed", two), "before").await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        step(("feed", one), ("feed", 999_999), "before").await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        step(("folder", 999_999), ("feed", one), "before").await,
        StatusCode::NOT_FOUND
    );
    let body = json!({ "node": request(("feed", one)), "target": request(("feed", two)), "position": "under" });
    assert_eq!(
        send(&state, Method::POST, "/api/tree/move", &cookie, body).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        tree_of(&state, &cookie).await,
        [d(rust, vec![f(two)]), d(tech, vec![]), f(one)]
    );

    // Renaming, and taking a folder out: what was in it stays where it was.
    let path = format!("/api/folders/{rust}");
    assert_eq!(
        send(
            &state,
            Method::PATCH,
            &path,
            &cookie,
            json!({ "title": "Rost" })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    for bad in [
        json!({ "title": " " }),
        json!({}),
        json!({ "title": "x".repeat(101) }),
        json!({ "title": "a\u{7}b" }),
    ] {
        assert_eq!(
            send(&state, Method::PATCH, &path, &cookie, bad.clone()).await,
            StatusCode::BAD_REQUEST,
            "{bad}"
        );
    }
    assert_eq!(
        add_folder(&state, &cookie, "", None).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        add_folder(&state, &cookie, "Lost", Some(999_999)).await.0,
        StatusCode::NOT_FOUND
    );
    let (_, inner) = add_folder(&state, &cookie, "Inner", Some(rust)).await;
    assert_eq!(
        send(&state, Method::DELETE, &path, &cookie, Value::Null).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        tree_of(&state, &cookie).await,
        [f(two), d(inner, vec![]), d(tech, vec![]), f(one)]
    );
    assert_eq!(
        send(&state, Method::DELETE, &path, &cookie, Value::Null).await,
        StatusCode::NOT_FOUND
    );

    // Five levels and not one more, by adding and by moving.
    let mut parent = tech;
    for level in 2..=5 {
        let (status, id) =
            add_folder(&state, &cookie, &format!("Level {level}"), Some(parent)).await;
        assert_eq!(status, StatusCode::CREATED);
        parent = id;
    }
    assert_eq!(
        add_folder(&state, &cookie, "Level 6", Some(parent)).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        step(("folder", inner), ("folder", parent), "inside").await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        step(("folder", tech), ("folder", inner), "inside").await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        step(("folder", tech), ("folder", parent), "inside").await,
        StatusCode::BAD_REQUEST
    );
    // A feed is no level.
    assert_eq!(
        step(("feed", one), ("folder", parent), "inside").await,
        StatusCode::NO_CONTENT
    );

    // An account's folders have a limit.
    sqlx::query(
        "INSERT INTO folders (account_id, title, position)
         SELECT $1, 'many', n FROM generate_series(1, 500) n",
    )
    .bind(ada)
    .execute(&db)
    .await
    .unwrap();
    assert_eq!(
        add_folder(&state, &cookie, "One more", None).await.0,
        StatusCode::BAD_REQUEST
    );
}

/// Random changes to a tree, compared after each with the model.
#[sqlx::test]
async fn the_tree_matches_its_model(db: PgPool) {
    let state = AppState::new(db.clone());
    for seed in 1..=12 {
        let mut random = Random::new(seed);
        let account = common::account(&db, &format!("reader-{seed}")).await;
        let cookie = common::session(&db, account).await;
        let mut model = Vec::new();
        for n in 0..4 {
            let feed = common::feed(&db, &format!("feed-{seed}-{n}")).await;
            common::subscribe(&db, account, feed).await;
            model.push(Node::Feed(feed));
        }
        // Subscriptions made by hand all sit at their feed's id; number them.
        let first = model[0].key();
        let last = model[3].key();
        assert_eq!(
            move_over_api(&state, &cookie, last, first, "before").await,
            StatusCode::NO_CONTENT
        );
        assert!(move_node(&mut model, last, first, "before"));
        let mut log = Vec::new();

        for _ in 0..40 {
            let mut all = Vec::new();
            keys(&model, &mut all);
            let pick = |random: &mut Random| all[random.below(all.len())];
            let folders: Vec<Key> = all
                .iter()
                .copied()
                .filter(|key| key.0 == "folder")
                .collect();
            let step = match random.below(10) {
                0 | 1 => {
                    let parent = (!folders.is_empty() && random.below(3) > 0)
                        .then(|| folders[random.below(folders.len())]);
                    let depth = parent.map_or(0, |parent| path(&model, parent).unwrap().len());
                    let (status, id) =
                        add_folder(&state, &cookie, "Folder", parent.map(|parent| parent.1)).await;
                    if depth < 5 {
                        assert_eq!(status, StatusCode::CREATED, "{log:?}");
                        let above = parent.map(|parent| path(&model, parent).unwrap());
                        level_mut(&mut model, above.as_deref().unwrap_or(&[]))
                            .push(Node::Folder(id, vec![]));
                    } else {
                        assert_eq!(status, StatusCode::BAD_REQUEST, "{log:?}");
                    }
                    format!("add a folder in {parent:?}")
                }
                2 if !folders.is_empty() => {
                    let folder = folders[random.below(folders.len())];
                    let path_of = format!("/api/folders/{}", folder.1);
                    let status = send(&state, Method::DELETE, &path_of, &cookie, Value::Null).await;
                    assert_eq!(status, StatusCode::NO_CONTENT, "{log:?}");
                    let at = path(&model, folder).unwrap();
                    let (last, above) = at.split_last().unwrap();
                    let level = level_mut(&mut model, above);
                    let Node::Folder(_, inside) = level.remove(*last) else {
                        unreachable!()
                    };
                    level.splice(*last..*last, inside);
                    format!("remove {folder:?}")
                }
                _ => {
                    let (key, target) = (pick(&mut random), pick(&mut random));
                    let position = ["before", "after", "inside"][random.below(3)];
                    let status = move_over_api(&state, &cookie, key, target, position).await;
                    let allowed = move_node(&mut model, key, target, position);
                    let expected = if allowed {
                        StatusCode::NO_CONTENT
                    } else {
                        StatusCode::BAD_REQUEST
                    };
                    log.push(format!("move {key:?} {position} {target:?}"));
                    assert_eq!(status, expected, "{log:?}");
                    continue_with(&state, &cookie, &model, &log).await;
                    continue;
                }
            };
            log.push(step);
            continue_with(&state, &cookie, &model, &log).await;
        }
    }
}

async fn continue_with(state: &AppState, cookie: &str, model: &[Node], log: &[String]) {
    assert_eq!(tree_of(state, cookie).await, model, "{log:?}");
}

#[sqlx::test]
async fn nobody_changes_the_tree_of_another(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 2).await;
    let (grace, hers) = common::reader(&db, "grace", 2).await;
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let (_, tech) = add_folder(&state, &cookie, "Tech", None).await;
    let (_, her_folder) = add_folder(&state, &grace_cookie, "Hers", None).await;
    let before = (
        tree_of(&state, &cookie).await,
        tree_of(&state, &grace_cookie).await,
    );

    // Grace names ada's nodes: they are not there for her, in any place.
    for (key, target, position) in [
        (("feed", feed), ("folder", her_folder), "inside"),
        (("folder", tech), ("folder", her_folder), "inside"),
        (("feed", hers), ("folder", tech), "inside"),
        (("feed", hers), ("feed", feed), "before"),
        (("folder", her_folder), ("folder", tech), "after"),
    ] {
        let status = move_over_api(&state, &grace_cookie, key, target, position).await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "{key:?} {position} {target:?}"
        );
    }
    assert_eq!(
        add_folder(&state, &grace_cookie, "In hers", Some(tech))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let folder = format!("/api/folders/{tech}");
    let subscription = format!("/api/feeds/{feed}");
    for (method, path, body) in [
        (Method::PATCH, &folder, json!({ "title": "Mine now" })),
        (Method::DELETE, &folder, Value::Null),
        (Method::PATCH, &subscription, json!({ "title": "Mine now" })),
        (Method::DELETE, &subscription, Value::Null),
    ] {
        let status = send(&state, method.clone(), path, &grace_cookie, body).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {path}");
    }
    assert_eq!(
        (
            tree_of(&state, &cookie).await,
            tree_of(&state, &grace_cookie).await
        ),
        before
    );

    // And nobody at all without a session.
    for (method, path) in [
        (Method::POST, "/api/tree/move".to_owned()),
        (Method::POST, "/api/folders".to_owned()),
        (Method::PATCH, folder.clone()),
        (Method::DELETE, folder),
        (Method::PATCH, subscription.clone()),
        (Method::DELETE, subscription),
        (Method::GET, "/api/settings".to_owned()),
        (Method::PUT, "/api/settings".to_owned()),
    ] {
        let (status, _) = call(
            &state,
            method.clone(),
            &path,
            None,
            Some(json!({ "title": "x" })),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {path}");
    }
}

#[sqlx::test]
async fn a_subscription_has_a_name_and_an_end(db: PgPool) {
    let (ada, feed) = common::reader(&db, "ada", 3).await;
    let grace = common::account(&db, "grace").await;
    common::subscribe(&db, grace, feed).await;
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let path = format!("/api/feeds/{feed}");
    let titles = async |cookie: &str| {
        let (_, tree) = call(&state, Method::GET, "/api/tree", Some(cookie), None).await;
        tree["feeds"][0]["title"].clone()
    };

    assert_eq!(
        send(
            &state,
            Method::PATCH,
            &path,
            &cookie,
            json!({ "title": " My news " })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(titles(&cookie).await, "My news");
    // The name is ada's alone.
    assert_eq!(titles(&grace_cookie).await, "ada");
    assert_eq!(
        send(
            &state,
            Method::PATCH,
            &path,
            &cookie,
            json!({ "title": "x".repeat(101) })
        )
        .await,
        StatusCode::BAD_REQUEST
    );
    // Without a name the feed's own is back.
    for reset in [json!({ "title": "" }), json!({ "title": null }), json!({})] {
        send(
            &state,
            Method::PATCH,
            &path,
            &cookie,
            json!({ "title": "Again" }),
        )
        .await;
        assert_eq!(
            send(&state, Method::PATCH, &path, &cookie, reset).await,
            StatusCode::NO_CONTENT
        );
        assert_eq!(titles(&cookie).await, "ada");
    }

    // Unsubscribing takes what ada read there, and nothing of grace's.
    let read = format!("/api/feeds/{feed}/articles/2/state");
    for cookie in [&cookie, &grace_cookie] {
        assert_eq!(
            send(
                &state,
                Method::PUT,
                &read,
                cookie,
                json!({ "read": true, "starred": true })
            )
            .await,
            StatusCode::NO_CONTENT
        );
    }
    assert_eq!(
        send(&state, Method::DELETE, &path, &cookie, Value::Null).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send(&state, Method::DELETE, &path, &cookie, Value::Null).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(tree_of(&state, &cookie).await, []);
    let (status, _) = call(
        &state,
        Method::GET,
        &format!("/api/feeds/{feed}/articles"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let left: Vec<i32> = sqlx::query_scalar("SELECT account_id FROM article_states")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(left, [grace]);
    let (status, list) = call(
        &state,
        Method::GET,
        &format!("/api/feeds/{feed}/articles"),
        Some(&grace_cookie),
        None,
    )
    .await;
    assert_eq!(
        (status, list["articles"].as_array().unwrap().len()),
        (StatusCode::OK, 3)
    );
}

#[sqlx::test]
async fn settings_are_kept_per_account(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let grace = common::account(&db, "grace").await;
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let settings =
        async |cookie: &str| call(&state, Method::GET, "/api/settings", Some(cookie), None).await;

    assert_eq!(settings(&cookie).await, (StatusCode::OK, json!({})));
    let mine = json!({ "theme": "dark", "keys": { "next": "j" }, "density": 2 });
    assert_eq!(
        send(&state, Method::PUT, "/api/settings", &cookie, mine.clone()).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(settings(&cookie).await.1, mine);
    assert_eq!(settings(&grace_cookie).await.1, json!({}));
    // They are replaced as a whole.
    assert_eq!(
        send(
            &state,
            Method::PUT,
            "/api/settings",
            &cookie,
            json!({ "theme": "light" })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(settings(&cookie).await.1, json!({ "theme": "light" }));

    for bad in [
        json!([1, 2]),
        json!("dark"),
        json!(7),
        json!({ "big": "x".repeat(17_000) }),
    ] {
        assert_eq!(
            send(&state, Method::PUT, "/api/settings", &cookie, bad).await,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(settings(&cookie).await.1, json!({ "theme": "light" }));
}

#[sqlx::test]
async fn the_tree_has_room_for_so_many_folders_and_feeds(db: PgPool) {
    use rueder::tree::{MAX_FEEDS_IN_FOLDER, MAX_FOLDERS};
    let (ada, news) = common::reader(&db, "ada", 0).await;
    let full = common::folder(&db, ada, None, "Full").await;
    let state = AppState::new(db.clone());
    let cookie = common::session(&db, ada).await;
    sqlx::query(
        "WITH made AS (
             INSERT INTO feeds (feed_url)
             SELECT 'https://many.example/' || n FROM generate_series(1, $3) AS n RETURNING id
         )
         INSERT INTO subscriptions (account_id, feed_id, folder_id, position)
         SELECT $1, id, $2, id FROM made",
    )
    .bind(ada)
    .bind(full)
    .bind(MAX_FEEDS_IN_FOLDER)
    .execute(&db)
    .await
    .unwrap();
    let news_url: String = sqlx::query_scalar("SELECT feed_url FROM feeds WHERE id = $1")
        .bind(news)
        .fetch_one(&db)
        .await
        .unwrap();

    // The folder is full: nothing moves in, and nobody subscribes into it.
    let into = json!({
        "node": { "kind": "feed", "id": news },
        "target": { "kind": "folder", "id": full },
        "position": "inside",
    });
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/tree/move",
        Some(&cookie),
        Some(into),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let grace = common::account(&db, "grace").await;
    let other = common::feed(&db, "other").await;
    common::subscribe(&db, grace, other).await;
    let other_url: String = sqlx::query_scalar("SELECT feed_url FROM feeds WHERE id = $1")
        .bind(other)
        .fetch_one(&db)
        .await
        .unwrap();
    let new = json!({ "url": other_url, "folder_id": full });
    let (status, _) = call(&state, Method::POST, "/api/feeds", Some(&cookie), Some(new)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // Beside it there is room, and what is in it can still be put in order.
    let new = json!({ "url": other_url });
    let (status, _) = call(&state, Method::POST, "/api/feeds", Some(&cookie), Some(new)).await;
    assert_eq!(status, StatusCode::CREATED);
    let again = json!({ "url": news_url, "folder_id": full });
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/feeds",
        Some(&cookie),
        Some(again),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    sqlx::query(
        "INSERT INTO folders (account_id, title, position)
         SELECT $1, 'Folder ' || n, n FROM generate_series(2, $2) AS n",
    )
    .bind(ada)
    .bind(MAX_FOLDERS)
    .execute(&db)
    .await
    .unwrap();
    let one_more = json!({ "title": "One more" });
    let (status, _) = call(
        &state,
        Method::POST,
        "/api/folders",
        Some(&cookie),
        Some(one_more),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn whoever_reads_a_feed_says_as_what_it_is_fetched(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 1).await;
    let (grace, blog) = common::reader(&db, "grace", 1).await;
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let set = async |cookie: Option<&str>, feed: i32, body: Value| {
        let path = format!("/api/feeds/{feed}/agent");
        call(&state, Method::PUT, &path, cookie, Some(body)).await.0
    };
    let agent = async |cookie: &str| {
        let (_, tree) = call(&state, Method::GET, "/api/tree", Some(cookie), None).await;
        tree["feeds"][0]["fetch_agent"].as_str().unwrap().to_owned()
    };
    sqlx::query("UPDATE feeds SET next_fetch_at = now() + interval '1 hour'")
        .execute(&db)
        .await
        .unwrap();

    assert_eq!(agent(&cookie).await, "rueder");
    let browser = json!({ "agent": "browser" });
    assert_eq!(
        set(Some(&cookie), news, browser.clone()).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(agent(&cookie).await, "browser");
    // The feed is tried again at once, and no other feed is.
    let due: Vec<i32> = sqlx::query_scalar("SELECT id FROM feeds WHERE next_fetch_at <= now()")
        .fetch_all(&db)
        .await
        .unwrap();
    assert_eq!(due, [news]);

    // Not for a feed one does not read, not without a session, and not as
    // anything else.
    assert_eq!(
        set(Some(&cookie), blog, browser.clone()).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        set(None, news, browser.clone()).await,
        StatusCode::UNAUTHORIZED
    );
    let other = json!({ "agent": "curl/8" });
    assert_eq!(
        set(Some(&cookie), news, other).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(agent(&grace_cookie).await, "rueder");
    let back = json!({ "agent": "rueder" });
    assert_eq!(set(Some(&cookie), news, back).await, StatusCode::NO_CONTENT);
    assert_eq!(agent(&cookie).await, "rueder");
}
