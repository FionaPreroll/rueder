//! The hardened HTTP client against servers on this machine: what it sends, how
//! it follows redirects, what it refuses and where its limits are.
//!
//! The machine's own addresses are internal, so the client under test gets an
//! exception for 127.0.0.1, where the server for the tests listens. A second
//! server on 127.0.0.2 plays what must never be reached: no exception covers it,
//! and it counts every request that gets through.

use std::{
    collections::HashMap,
    io::Write,
    net::{IpAddr, SocketAddr},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    extract::{Path, Query, Request as ServerRequest},
    http::{HeaderMap, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response as ServerResponse},
    routing::get,
};
use rueder::http_client::{Agent, Config, Error, HttpClient, Looking, Lookup, Request};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

/// Names and the answers they get, one after the other; the last one stays.
#[derive(Default)]
struct Names {
    answers: Mutex<HashMap<String, Vec<Vec<IpAddr>>>>,
    asked: Mutex<Vec<String>>,
}

impl Names {
    fn set(&self, name: &str, answers: &[&[&str]]) {
        let answers = answers
            .iter()
            .map(|addrs| addrs.iter().map(|addr| addr.parse().unwrap()).collect())
            .collect();
        self.answers
            .lock()
            .unwrap()
            .insert(name.to_owned(), answers);
    }

    fn asked(&self, name: &str) -> usize {
        let asked = self.asked.lock().unwrap();
        asked.iter().filter(|asked| *asked == name).count()
    }
}

impl Lookup for Names {
    fn lookup<'a>(&'a self, host: &'a str) -> Looking<'a> {
        Box::pin(async move {
            self.asked.lock().unwrap().push(host.to_owned());
            let mut answers = self.answers.lock().unwrap();
            match answers.get_mut(host) {
                None => Err(std::io::Error::other("no such name")),
                Some(answers) if answers.len() > 1 => Ok(answers.remove(0)),
                Some(answers) => Ok(answers[0].clone()),
            }
        })
    }
}

/// Everything a test needs: the client, the server it may reach and the one it
/// may not.
struct World {
    client: HttpClient,
    names: Arc<Names>,
    /// Where the server for the tests listens, on 127.0.0.1.
    outside: SocketAddr,
    outside_hits: Arc<AtomicUsize>,
    /// Where the server listens that must never be reached, on 127.0.0.2.
    inside: SocketAddr,
    inside_hits: Arc<AtomicUsize>,
}

impl World {
    async fn new() -> Self {
        Self::with(Config::default()).await
    }

    async fn with(config: Config) -> Self {
        let (outside, outside_hits) = serve("127.0.0.1", site()).await;
        let (inside, inside_hits) =
            serve("127.0.0.2", Router::new().fallback(|| async { "secret" })).await;
        let names = Arc::new(Names::default());
        let config = Config {
            allow: vec!["127.0.0.1".parse().unwrap()],
            ..config
        };
        let client = HttpClient::with_lookup(config, names.clone()).unwrap();
        Self {
            client,
            names,
            outside,
            outside_hits,
            inside,
            inside_hits,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.outside)
    }

    /// The address of a page that redirects to `target`.
    fn redirect_to(&self, target: &str) -> String {
        url::Url::parse_with_params(&self.url("/to"), [("target", target)])
            .unwrap()
            .into()
    }

    async fn get(&self, url: &str) -> Result<rueder::http_client::Response, Error> {
        self.client.get(Request::new(url)).await
    }
}

/// Starts a server and counts the requests that reach it.
async fn serve(ip: &str, router: Router) -> (SocketAddr, Arc<AtomicUsize>) {
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    let router = router.layer(middleware::from_fn(
        move |request: ServerRequest, next: Next| {
            counter.fetch_add(1, Ordering::SeqCst);
            next.run(request)
        },
    ));
    let listener = TcpListener::bind((ip, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (addr, hits)
}

/// A server that sends `head` and then whatever `body` writes, to do what a
/// well-behaved server would not.
async fn serve_raw<F, Fut>(head: &'static str, body: F) -> SocketAddr
where
    F: Fn(tokio::net::TcpStream) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            // The request fits one read; what it says does not matter.
            let _ = stream.read(&mut [0; 4096]).await;
            if stream.write_all(head.as_bytes()).await.is_ok() {
                body(stream).await;
            }
        }
    });
    addr
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

#[allow(
    clippy::disallowed_methods,
    reason = "a made-up site of the test, not a route of rueder"
)]
fn site() -> Router {
    Router::new()
        .route("/feed", get(feed))
        .route(
            "/redirect/{left}",
            get(|Path(left): Path<u32>| async move {
                // A relative target, as many servers send it.
                let target = if left > 1 {
                    format!("{}", left - 1)
                } else {
                    "../feed".to_owned()
                };
                (StatusCode::FOUND, [(header::LOCATION, target)])
            }),
        )
        .route(
            "/loop",
            get(|| async { (StatusCode::MOVED_PERMANENTLY, [(header::LOCATION, "/loop")]) }),
        )
        .route(
            "/to",
            get(|Query(query): Query<HashMap<String, String>>| async move {
                (
                    StatusCode::TEMPORARY_REDIRECT,
                    [(header::LOCATION, query["target"].clone())],
                )
            }),
        )
        .route("/nowhere", get(|| async { StatusCode::FOUND }))
        .route(
            "/missing",
            get(|| async { (StatusCode::NOT_FOUND, "nothing here") }),
        )
        .route("/big", get(|| async { vec![b'x'; 2_000_000] }))
        .route(
            "/packed",
            get(|| async {
                (
                    [(header::CONTENT_ENCODING, "gzip")],
                    gzip(&b"hello ".repeat(1000)),
                )
            }),
        )
        .route(
            "/bomb",
            get(|| async {
                (
                    [(header::CONTENT_ENCODING, "gzip")],
                    gzip(&vec![0; 50_000_000]),
                )
            }),
        )
        .route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(30)).await;
                "late"
            }),
        )
}

/// Answers with what the request said, or with "not modified" if the caller
/// has the current copy.
async fn feed(headers: HeaderMap) -> ServerResponse {
    let said = |name: header::HeaderName| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    };
    if said(header::IF_NONE_MATCH).as_deref() == Some("\"v1\"") {
        return StatusCode::NOT_MODIFIED.into_response();
    }
    let seen = serde_json::json!({
        "agent": said(header::USER_AGENT),
        "accept": said(header::ACCEPT),
        "modified": said(header::IF_MODIFIED_SINCE),
        "authorization": said(header::AUTHORIZATION),
        "cookie": said(header::COOKIE),
        "referer": said(header::REFERER),
        "host": said(header::HOST),
    });
    ([(header::ETAG, "\"v1\"")], Json(seen)).into_response()
}

fn said(response: &rueder::http_client::Response) -> serde_json::Value {
    serde_json::from_slice(&response.body).unwrap()
}

#[tokio::test]
async fn fetches_a_page_and_says_who_is_asking() {
    let world = World::new().await;
    let response = world.get(&world.url("/feed#top")).await.unwrap();
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.url.as_str(), world.url("/feed"));
    assert_eq!(response.header("etag"), Some("\"v1\""));
    let seen = said(&response);
    let agent = seen["agent"].as_str().unwrap();
    assert!(
        agent.starts_with("rueder/") && agent.contains("github.com"),
        "{agent}"
    );
    // Whoever says nothing about what they read accepts anything.
    assert_eq!(seen["accept"], "*/*");
    assert_eq!(seen["cookie"], serde_json::Value::Null);
    assert_eq!(seen["referer"], serde_json::Value::Null);

    let feed = world.url("/feed");
    let request = Request {
        agent: Agent::Browser,
        accept: Some("application/atom+xml"),
        last_modified: Some("Tue, 10 Nov 2026 10:00:00 GMT"),
        ..Request::new(&feed)
    };
    let seen = said(&world.client.get(request).await.unwrap());
    assert!(seen["agent"].as_str().unwrap().starts_with("Mozilla/5.0"));
    assert_eq!(seen["accept"], "application/atom+xml");
    assert_eq!(seen["modified"], "Tue, 10 Nov 2026 10:00:00 GMT");
}

#[tokio::test]
async fn not_modified_and_not_found_are_answers() {
    let world = World::new().await;
    let feed = world.url("/feed");
    let request = Request {
        etag: Some("\"v1\""),
        ..Request::new(&feed)
    };
    let response = world.client.get(request).await.unwrap();
    assert_eq!(response.status, StatusCode::NOT_MODIFIED);
    assert!(response.body.is_empty());

    let response = world.get(&world.url("/missing")).await.unwrap();
    assert_eq!(response.status, StatusCode::NOT_FOUND);
    assert_eq!(response.body, b"nothing here");
}

#[tokio::test]
async fn a_password_in_the_address_goes_to_its_host_only() {
    let world = World::new().await;
    let with_password = format!("http://user:secret@{}/feed", world.outside);
    let seen_there = said(&world.get(&with_password).await.unwrap());
    assert_eq!(seen_there["authorization"], "Basic dXNlcjpzZWNyZXQ=");

    // Redirected from an address without one, the host sees none.
    world.names.set("other.test", &[&["127.0.0.1"]]);
    let elsewhere = format!("http://other.test:{}/feed", world.outside.port());
    let moved = format!("http://user:secret@{}/to?target={elsewhere}", world.outside);
    let response = world.get(&moved).await.unwrap();
    assert_eq!(response.url.as_str(), elsewhere);
    assert_eq!(said(&response)["authorization"], serde_json::Value::Null);
}

#[tokio::test]
async fn follows_redirects_up_to_the_limit() {
    let world = World::new().await;
    let response = world.get(&world.url("/redirect/5")).await.unwrap();
    assert_eq!(response.url.as_str(), world.url("/feed"));
    assert_eq!(response.status, StatusCode::OK);

    assert_eq!(
        world.get(&world.url("/redirect/6")).await.unwrap_err(),
        Error::TooManyRedirects(5)
    );
    assert_eq!(
        world.get(&world.url("/loop")).await.unwrap_err(),
        Error::TooManyRedirects(5)
    );
    assert_eq!(
        world.get(&world.url("/nowhere")).await.unwrap_err(),
        Error::BadRedirect
    );
}

#[tokio::test]
async fn a_redirect_into_the_own_network_is_refused_and_not_followed() {
    let world = World::new().await;
    let inside = world.inside.port();
    world.names.set("inside.test", &[&["127.0.0.2"]]);
    world.names.set("mapped.test", &[&["::ffff:127.0.0.2"]]);
    // A name that points outside and inside at once.
    world
        .names
        .set("both.test", &[&["93.184.216.34", "127.0.0.2"]]);
    let targets = [
        format!("http://127.0.0.2:{inside}/"),
        // 127.0.0.2 in other notations.
        format!("http://2130706434:{inside}/"),
        format!("http://0x7f.0.0.2:{inside}/"),
        format!("http://[::ffff:127.0.0.2]:{inside}/"),
        format!("http://inside.test:{inside}/"),
        format!("http://mapped.test:{inside}/"),
        format!("http://both.test:{inside}/"),
        "http://10.0.0.1/".to_owned(),
        "http://169.254.169.254/latest/meta-data/".to_owned(),
        "http://[::1]/".to_owned(),
        "http://[fd00::1]/".to_owned(),
    ];
    for target in targets {
        let err = world.get(&world.redirect_to(&target)).await.unwrap_err();
        assert!(matches!(err, Error::Internal { .. }), "{target}: {err:?}");
        // And directly, without a redirect before it.
        let err = world.get(&target).await.unwrap_err();
        assert!(matches!(err, Error::Internal { .. }), "{target}: {err:?}");
    }
    assert_eq!(world.inside_hits.load(Ordering::SeqCst), 0);

    // The refusal says which host and why, for the person looking at the feed,
    // but not where in the network it is.
    let err = world
        .get(&format!("http://inside.test:{inside}/"))
        .await
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "inside.test is not on the public internet (loopback)"
    );
}

#[tokio::test]
async fn a_redirect_to_another_scheme_is_refused() {
    let world = World::new().await;
    for target in [
        "file:///etc/passwd",
        "ftp://127.0.0.1/",
        "gopher://127.0.0.1:70/",
        "data:text/plain,hello",
    ] {
        let err = world.get(&world.redirect_to(target)).await.unwrap_err();
        assert!(matches!(err, Error::Address(_)), "{target}: {err:?}");
    }
}

#[tokio::test]
async fn a_name_is_connected_to_at_the_address_that_was_checked() {
    let world = World::new().await;
    // The name answers with an allowed address when it is checked and with an
    // internal one ever after. A client that looked it up twice, once to check
    // and once to connect, would land inside.
    world
        .names
        .set("turning.test", &[&["127.0.0.1"], &["127.0.0.2"]]);
    let url = format!("http://turning.test:{}/feed", world.outside.port());
    let response = world.get(&url).await.unwrap();
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(
        said(&response)["host"],
        format!("turning.test:{}", world.outside.port())
    );
    assert_eq!(world.names.asked("turning.test"), 1);

    // Later fetches reuse the connection or look the name up again and are
    // refused; none of them reaches the inside.
    for _ in 0..3 {
        if let Err(err) = world.get(&url).await {
            assert!(matches!(err, Error::Internal { .. }), "{err:?}");
        }
    }
    assert_eq!(world.inside_hits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn without_an_exception_the_own_machine_is_refused() {
    let world = World::new().await;
    // A client as the server builds it: no exceptions, the system's resolver.
    let client = HttpClient::new(Config::default()).unwrap();
    for host in ["127.0.0.1", "localhost"] {
        let url = format!("http://{host}:{}/feed", world.outside.port());
        let err = client.get(Request::new(&url)).await.unwrap_err();
        assert!(matches!(err, Error::Internal { .. }), "{host}: {err:?}");
    }
    assert_eq!(world.outside_hits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_body_over_the_limit_is_refused() {
    let world = World::with(Config {
        max_body: 1_000_000,
        ..Config::default()
    })
    .await;
    // The server says how much is coming.
    assert_eq!(
        world.get(&world.url("/big")).await.unwrap_err(),
        Error::TooLarge(1_000_000)
    );

    // The server just keeps sending.
    let endless = serve_raw(
        "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n",
        |mut stream| async move { while stream.write_all(&[b'x'; 65536]).await.is_ok() {} },
    )
    .await;
    let started = Instant::now();
    let err = world.get(&format!("http://{endless}/")).await.unwrap_err();
    assert_eq!(err, Error::TooLarge(1_000_000));
    assert!(started.elapsed() < Duration::from_secs(10));

    // A caller can ask for less than the client allows, not for more.
    let (missing, big) = (world.url("/missing"), world.url("/big"));
    let small = Request {
        max_body: Some(100),
        ..Request::new(&missing)
    };
    assert_eq!(world.client.get(small).await.unwrap().body, b"nothing here");
    let small = Request {
        max_body: Some(5),
        ..Request::new(&missing)
    };
    assert_eq!(
        world.client.get(small).await.unwrap_err(),
        Error::TooLarge(5)
    );
    let large = Request {
        max_body: Some(5_000_000),
        ..Request::new(&big)
    };
    assert_eq!(
        world.client.get(large).await.unwrap_err(),
        Error::TooLarge(1_000_000)
    );
}

#[tokio::test]
async fn a_small_body_that_unpacks_to_a_huge_one_is_refused() {
    let world = World::with(Config {
        max_body: 1_000_000,
        ..Config::default()
    })
    .await;
    let response = world.get(&world.url("/packed")).await.unwrap();
    assert_eq!(response.body, b"hello ".repeat(1000));

    // 50 MB of zeros travel as some 50 kB. The limit counts what comes out.
    let err = world.get(&world.url("/bomb")).await.unwrap_err();
    assert_eq!(err, Error::TooLarge(1_000_000));
}

#[tokio::test]
async fn a_fetch_that_takes_too_long_is_given_up() {
    let world = World::with(Config {
        timeout: Duration::from_millis(400),
        ..Config::default()
    })
    .await;
    // The server does not answer.
    let started = Instant::now();
    assert_eq!(
        world.get(&world.url("/slow")).await.unwrap_err(),
        Error::Timeout
    );
    assert!(started.elapsed() < Duration::from_secs(5));

    // The server answers and then sends a byte now and then, forever. The time
    // is for the whole fetch, not for the wait between two bytes.
    let dripping = serve_raw(
        "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n",
        |mut stream| async move {
            while stream.write_all(b"x").await.is_ok() {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        },
    )
    .await;
    let started = Instant::now();
    assert_eq!(
        world.get(&format!("http://{dripping}/")).await.unwrap_err(),
        Error::Timeout
    );
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn what_the_network_does_wrong_is_told_without_the_password() {
    let world = World::new().await;
    // Nobody listens there any more.
    let gone = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap();
    let err = world
        .get(&format!("http://user:secret@{gone}/"))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Network(_)), "{err:?}");
    assert!(err.to_string().starts_with("could not connect"), "{err}");
    assert!(!err.to_string().contains("secret"), "{err}");

    // The name does not exist, or exists without an address.
    let err = world
        .get("http://user:secret@unknown.test/")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Network(_)), "{err:?}");
    assert!(!err.to_string().contains("secret"), "{err}");
    world.names.set("empty.test", &[&[]]);
    let err = world.get("http://empty.test/").await.unwrap_err();
    assert_eq!(err, Error::Network("empty.test has no address".to_owned()));

    // The server hangs up in the middle of the answer.
    let cut = serve_raw(
        "HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\nonly this",
        |_| async {},
    )
    .await;
    let err = world.get(&format!("http://{cut}/")).await.unwrap_err();
    assert!(matches!(err, Error::Network(_)), "{err:?}");
}
