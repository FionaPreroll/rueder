//! Labels, and the lists of what a reader starred, labelled or has hidden.

mod common;

use axum::http::{Method, StatusCode};
use common::call;
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

async fn get(state: &AppState, path: &str, cookie: &str) -> (StatusCode, Value) {
    call(state, Method::GET, path, Some(cookie), None).await
}

async fn add(state: &AppState, cookie: &str, title: &str, color: &str) -> (StatusCode, i32) {
    let body = json!({ "title": title, "color": color });
    let (status, created) =
        call(state, Method::POST, "/api/labels", Some(cookie), Some(body)).await;
    (status, created["id"].as_i64().unwrap_or(0) as i32)
}

async fn titles(state: &AppState, cookie: &str) -> Vec<String> {
    let (status, labels) = get(state, "/api/labels", cookie).await;
    assert_eq!(status, StatusCode::OK);
    let labels = labels.as_array().unwrap().iter();
    labels
        .map(|label| label["title"].as_str().unwrap().to_owned())
        .collect()
}

fn seqs(list: &Value) -> Vec<i64> {
    let articles = list["articles"].as_array().unwrap().iter();
    articles
        .map(|article| article["seq"].as_i64().unwrap())
        .collect()
}

#[sqlx::test]
async fn labels_have_a_name_a_color_and_an_order(db: PgPool) {
    let ada = common::account(&db, "ada").await;
    let grace = common::account(&db, "grace").await;
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );

    let (status, later) = add(&state, &cookie, " Later ", "#1a2B3c").await;
    assert_eq!(status, StatusCode::CREATED);
    let (_, work) = add(&state, &cookie, "Work", "#000000").await;
    let (_, fun) = add(&state, &cookie, "Fun", "#ffffff").await;
    let (_, labels) = get(&state, "/api/labels", &cookie).await;
    assert_eq!(
        labels[0],
        json!({ "id": later, "title": "Later", "color": "#1a2B3c", "position": 1.0 })
    );
    assert_eq!(titles(&state, &cookie).await, ["Later", "Work", "Fun"]);

    // Names are the reader's own: grace can have a "Work" too, ada no second.
    let (status, hers) = add(&state, &grace_cookie, "Work", "#00ff00").await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        add(&state, &cookie, "Work", "#00ff00").await.0,
        StatusCode::BAD_REQUEST
    );
    for (title, color) in [
        ("", "#000000"),
        ("Red", "red"),
        ("Red", "#12345"),
        ("Red", "#1234567"),
        ("Red", "#12345g"),
        ("Red", "123456"),
        (&"x".repeat(101), "#000000"),
    ] {
        assert_eq!(
            add(&state, &cookie, title, color).await.0,
            StatusCode::BAD_REQUEST,
            "{title} {color}"
        );
    }

    let path = format!("/api/labels/{work}");
    assert_eq!(
        send(
            &state,
            Method::PATCH,
            &path,
            &cookie,
            json!({ "title": "Job" })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send(
            &state,
            Method::PATCH,
            &path,
            &cookie,
            json!({ "color": "#abcdef" })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    let (_, labels) = get(&state, "/api/labels", &cookie).await;
    assert_eq!(
        (&labels[1]["title"], &labels[1]["color"]),
        (&json!("Job"), &json!("#abcdef"))
    );
    for bad in [
        json!({}),
        json!({ "title": "Fun" }),
        json!({ "color": "blue" }),
        json!({ "title": " " }),
    ] {
        assert_eq!(
            send(&state, Method::PATCH, &path, &cookie, bad.clone()).await,
            StatusCode::BAD_REQUEST,
            "{bad}"
        );
    }

    // A new order; a label that is not named comes after the named ones, and
    // ids that are not ada's change nothing.
    let order = json!({ "ids": [fun, hers, later, 999_999] });
    assert_eq!(
        send(&state, Method::POST, "/api/labels/order", &cookie, order).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(titles(&state, &cookie).await, ["Fun", "Later", "Job"]);
    assert_eq!(titles(&state, &grace_cookie).await, ["Work"]);

    // Grace cannot touch ada's labels.
    assert_eq!(
        send(
            &state,
            Method::PATCH,
            &path,
            &grace_cookie,
            json!({ "title": "Hers" })
        )
        .await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&state, Method::DELETE, &path, &grace_cookie, Value::Null).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        get(&state, &format!("{path}/articles"), &grace_cookie)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&state, Method::DELETE, &path, &cookie, Value::Null).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send(&state, Method::DELETE, &path, &cookie, Value::Null).await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(titles(&state, &cookie).await, ["Fun", "Later"]);

    // There is a limit to them.
    sqlx::query(
        "INSERT INTO labels (account_id, title, color, position)
         SELECT $1, 'label ' || n, '#000000', n + 10 FROM generate_series(1, 98) n",
    )
    .bind(ada)
    .execute(&db)
    .await
    .unwrap();
    assert_eq!(
        add(&state, &cookie, "One more", "#000000").await.0,
        StatusCode::BAD_REQUEST
    );

    for (method, path) in [
        (Method::GET, "/api/labels".to_owned()),
        (Method::POST, "/api/labels".to_owned()),
        (Method::POST, "/api/labels/order".to_owned()),
        (Method::PATCH, path.clone()),
        (Method::DELETE, path.clone()),
        (Method::GET, format!("{path}/articles")),
        (Method::PUT, format!("/api/feeds/1/articles/1/labels/{fun}")),
        (
            Method::DELETE,
            format!("/api/feeds/1/articles/1/labels/{fun}"),
        ),
        (Method::GET, "/api/starred".to_owned()),
        (Method::GET, "/api/hidden".to_owned()),
    ] {
        let (status, _) = call(
            &state,
            method.clone(),
            &path,
            None,
            Some(json!({ "ids": [] })),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {path}");
    }
}

#[sqlx::test]
async fn articles_carry_labels_and_are_listed_by_them(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 4).await;
    let blog = common::feed(&db, "blog").await;
    common::subscribe(&db, ada, blog).await;
    common::add(&db, blog, 2).await;
    let grace = common::account(&db, "grace").await;
    common::subscribe(&db, grace, news).await;
    let (_, others) = common::reader(&db, "eve", 2).await;
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let (_, later) = add(&state, &cookie, "Later", "#111111").await;
    let (_, work) = add(&state, &cookie, "Work", "#222222").await;
    let (_, hers) = add(&state, &grace_cookie, "Hers", "#333333").await;
    let on = |feed: i32, seq: i32, label: i32| {
        format!("/api/feeds/{feed}/articles/{seq}/labels/{label}")
    };

    for (feed, seq, label) in [
        (news, 1, later),
        (news, 3, later),
        (blog, 2, later),
        (news, 3, work),
    ] {
        assert_eq!(
            send(
                &state,
                Method::PUT,
                &on(feed, seq, label),
                &cookie,
                Value::Null
            )
            .await,
            StatusCode::NO_CONTENT
        );
    }
    // Twice is once.
    assert_eq!(
        send(
            &state,
            Method::PUT,
            &on(news, 1, later),
            &cookie,
            Value::Null
        )
        .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send(
            &state,
            Method::PUT,
            &on(news, 3, hers),
            &grace_cookie,
            Value::Null
        )
        .await,
        StatusCode::NO_CONTENT
    );

    // The lists and the article show ada's labels, in the labels' order.
    let (_, list) = get(&state, &format!("/api/feeds/{news}/articles"), &cookie).await;
    let labels: Vec<_> = list["articles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["labels"].clone())
        .collect();
    assert_eq!(
        labels,
        [json!([]), json!([later, work]), json!([]), json!([later])]
    );
    let (_, article) = get(&state, &format!("/api/feeds/{news}/articles/3"), &cookie).await;
    assert_eq!(article["labels"], json!([later, work]));
    assert_eq!(article["title"], "Article 3");
    // Grace sees hers on the same article, and none of ada's.
    let (_, article) = get(
        &state,
        &format!("/api/feeds/{news}/articles/3"),
        &grace_cookie,
    )
    .await;
    assert_eq!(article["labels"], json!([hers]));

    // The list of a label, newest first, across feeds, page by page.
    let path = format!("/api/labels/{later}/articles");
    let (status, list) = get(&state, &path, &cookie).await;
    assert_eq!(status, StatusCode::OK);
    let pairs: Vec<_> = list["articles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            (
                a["feed_id"].as_i64().unwrap() as i32,
                a["seq"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(pairs, [(blog, 2), (news, 3), (news, 1)]);
    assert_eq!(list["articles"][1]["labels"], json!([later, work]));
    let before = list["articles"][0]["id"].as_i64().unwrap();
    let (_, page) = get(&state, &format!("{path}?limit=1&before={before}"), &cookie).await;
    assert_eq!(seqs(&page), [3]);
    let read = format!("/api/feeds/{news}/articles/3/state");
    send(&state, Method::PUT, &read, &cookie, json!({ "read": true })).await;
    let (_, page) = get(&state, &format!("{path}?unread=true"), &cookie).await;
    assert_eq!(seqs(&page), [2, 1]);

    // Off again; what was not on, and what is not there.
    assert_eq!(
        send(
            &state,
            Method::DELETE,
            &on(news, 1, later),
            &cookie,
            Value::Null
        )
        .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send(
            &state,
            Method::DELETE,
            &on(news, 1, later),
            &cookie,
            Value::Null
        )
        .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(seqs(&get(&state, &path, &cookie).await.1), [2, 3]);
    for (feed, seq, label) in [
        (news, 9, later),
        (news, 0, later),
        (others, 1, later),
        (news, 1, 999_999),
    ] {
        assert_eq!(
            send(
                &state,
                Method::PUT,
                &on(feed, seq, label),
                &cookie,
                Value::Null
            )
            .await,
            StatusCode::NOT_FOUND
        );
    }
    // Nobody labels with another's label, or takes another's label off.
    assert_eq!(
        send(
            &state,
            Method::PUT,
            &on(news, 2, later),
            &grace_cookie,
            Value::Null
        )
        .await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(
            &state,
            Method::DELETE,
            &on(news, 3, later),
            &grace_cookie,
            Value::Null
        )
        .await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(
            &state,
            Method::PUT,
            &on(news, 2, hers),
            &cookie,
            Value::Null
        )
        .await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(seqs(&get(&state, &path, &cookie).await.1), [2, 3]);

    // Ending a subscription takes the labels of its articles along.
    assert_eq!(
        send(
            &state,
            Method::DELETE,
            &format!("/api/feeds/{news}"),
            &cookie,
            Value::Null
        )
        .await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(seqs(&get(&state, &path, &cookie).await.1), [2]);
    let left: i64 =
        sqlx::query_scalar("SELECT count(*) FROM article_labels WHERE label_id = ANY($1)")
            .bind(vec![later, work])
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(left, 1);
    // Grace's label on that feed is still there.
    let (_, list) = get(
        &state,
        &format!("/api/labels/{hers}/articles"),
        &grace_cookie,
    )
    .await;
    assert_eq!(seqs(&list), [3]);
    // Removing a label takes it off its articles.
    assert_eq!(
        send(
            &state,
            Method::DELETE,
            &format!("/api/labels/{later}"),
            &cookie,
            Value::Null
        )
        .await,
        StatusCode::NO_CONTENT
    );
    let (_, article) = get(&state, &format!("/api/feeds/{blog}/articles/2"), &cookie).await;
    assert_eq!(article["labels"], json!([]));
}

#[sqlx::test]
async fn starred_and_hidden_articles_have_their_lists(db: PgPool) {
    let (ada, news) = common::reader(&db, "ada", 5).await;
    let blog = common::feed(&db, "blog").await;
    common::subscribe(&db, ada, blog).await;
    common::add(&db, blog, 2).await;
    let grace = common::account(&db, "grace").await;
    common::subscribe(&db, grace, news).await;
    let state = AppState::new(db.clone());
    let (cookie, grace_cookie) = (
        common::session(&db, ada).await,
        common::session(&db, grace).await,
    );
    let change = async |cookie: &str, feed: i32, seq: i32, body: Value| {
        let path = format!("/api/feeds/{feed}/articles/{seq}/state");
        send(&state, Method::PUT, &path, cookie, body).await
    };
    let unread = async |cookie: &str| {
        let (_, tree) = get(&state, "/api/tree", cookie).await;
        tree["feeds"][0]["unread"].as_i64().unwrap()
    };

    for (feed, seq) in [(news, 2), (news, 4), (blog, 1)] {
        assert_eq!(
            change(&cookie, feed, seq, json!({ "starred": true })).await,
            StatusCode::NO_CONTENT
        );
    }
    assert_eq!(
        change(&grace_cookie, news, 5, json!({ "starred": true })).await,
        StatusCode::NO_CONTENT
    );
    let (status, list) = get(&state, "/api/starred", &cookie).await;
    assert_eq!(status, StatusCode::OK);
    let pairs: Vec<_> = list["articles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            (
                a["feed_id"].as_i64().unwrap() as i32,
                a["seq"].as_i64().unwrap(),
                a["starred"] == true,
            )
        })
        .collect();
    assert_eq!(pairs, [(blog, 1, true), (news, 4, true), (news, 2, true)]);
    assert_eq!(
        seqs(&get(&state, "/api/starred?limit=1", &cookie).await.1),
        [1]
    );
    assert_eq!(
        seqs(&get(&state, "/api/starred", &grace_cookie).await.1),
        [5]
    );

    // Hiding takes an article out of every list but the one of the hidden,
    // and out of the unread count.
    assert_eq!(unread(&cookie).await, 5);
    assert_eq!(
        change(&cookie, news, 4, json!({ "hidden": true })).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        change(&cookie, news, 5, json!({ "hidden": true })).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(unread(&cookie).await, 3);
    assert_eq!(
        seqs(
            &get(&state, &format!("/api/feeds/{news}/articles"), &cookie)
                .await
                .1
        ),
        [3, 2, 1]
    );
    assert_eq!(seqs(&get(&state, "/api/starred", &cookie).await.1), [1, 2]);
    assert_eq!(seqs(&get(&state, "/api/hidden", &cookie).await.1), [5, 4]);
    // Grace has hidden nothing, and sees all of the feed.
    assert_eq!(
        seqs(&get(&state, "/api/hidden", &grace_cookie).await.1),
        Vec::<i64>::new()
    );
    assert_eq!(
        seqs(
            &get(
                &state,
                &format!("/api/feeds/{news}/articles"),
                &grace_cookie
            )
            .await
            .1
        ),
        [5, 4, 3, 2, 1]
    );
    assert_eq!(unread(&grace_cookie).await, 5);

    // And back.
    assert_eq!(
        change(&cookie, news, 4, json!({ "hidden": false })).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(seqs(&get(&state, "/api/hidden", &cookie).await.1), [5]);
    assert_eq!(
        seqs(&get(&state, "/api/starred", &cookie).await.1),
        [1, 4, 2]
    );
    assert_eq!(unread(&cookie).await, 4);
    assert_eq!(
        change(&cookie, news, 9, json!({ "hidden": true })).await,
        StatusCode::NOT_FOUND
    );
}
