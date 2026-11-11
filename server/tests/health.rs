//! `rueder health`, the health check of the container: it asks a running
//! server and says whether it is well.

use std::{net::SocketAddr, time::Duration};

use rueder::{AppState, health, router};
use sqlx::PgPool;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const WAIT: Duration = Duration::from_secs(5);

/// A server on a free local port that answers every connection with `answer`,
/// or, without one, never.
async fn answering(answer: Option<&'static str>) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let mut request = [0; 1024];
                let _ = stream.read(&mut request).await;
                match answer {
                    Some(answer) => {
                        let _ = stream.write_all(answer.as_bytes()).await;
                    }
                    None => std::future::pending().await,
                }
            });
        }
    });
    address
}

#[sqlx::test]
async fn a_running_server_is_well(db: PgPool) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = router(AppState::new(db), "missing".as_ref());
    tokio::spawn(async move { axum::serve(listener, app).await });

    health::check(address, WAIT).await.unwrap();
}

#[sqlx::test]
async fn a_server_without_its_database_is_not(db: PgPool) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = router(AppState::new(db.clone()), "missing".as_ref());
    tokio::spawn(async move { axum::serve(listener, app).await });
    db.close().await;

    let err = health::check(address, WAIT).await.unwrap_err();
    assert!(format!("{err:#}").contains("500"), "{err:#}");
}

#[tokio::test]
async fn nothing_listening_is_not_well() {
    // A port that was free a moment ago and has nobody behind it now.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);

    let err = health::check(address, WAIT).await.unwrap_err();
    assert!(format!("{err:#}").contains("connecting to"), "{err:#}");
}

#[tokio::test]
async fn only_200_counts() {
    for answer in [
        "HTTP/1.1 503 Service Unavailable\r\n\r\n",
        "HTTP/1.1 404 Not Found\r\n\r\n",
        // Not HTTP at all, whatever it says after.
        "SSH-2.0 200 OK\r\n",
        "200\r\n",
        "",
    ] {
        let address = answering(Some(answer)).await;
        assert!(health::check(address, WAIT).await.is_err(), "{answer:?}");
    }
    let address = answering(Some("HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")).await;
    health::check(address, WAIT).await.unwrap();
}

#[tokio::test]
async fn a_server_that_never_answers_is_not_waited_for() {
    let address = answering(None).await;
    let err = health::check(address, Duration::from_millis(200))
        .await
        .unwrap_err();
    assert!(format!("{err:#}").contains("in time"), "{err:#}");
}

#[tokio::test]
async fn a_server_on_every_address_is_asked_locally() {
    for (listen, asked) in [
        ("0.0.0.0:3000", "127.0.0.1:3000"),
        ("[::]:8080", "[::1]:8080"),
        ("127.0.0.1:3000", "127.0.0.1:3000"),
        ("192.0.2.7:80", "192.0.2.7:80"),
    ] {
        let address = health::address(listen).await.unwrap();
        assert_eq!(address, asked.parse::<SocketAddr>().unwrap(), "{listen}");
    }
    for listen in ["", "3000", "0.0.0.0", "no port.example"] {
        assert!(health::address(listen).await.is_err(), "{listen:?}");
    }
}
