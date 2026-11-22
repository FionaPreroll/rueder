//! `rssifier health`, the health check of the container: it asks a running
//! RSSifier and says whether it is well.

use std::{net::SocketAddr, time::Duration};

use hardened_http::{Config, HttpClient};
use rssifier::{AppState, Sites, app, health, limits::Pace, recipes::Recipes, tokens::Keeper};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const WAIT: Duration = Duration::from_secs(5);

/// A server on a free local port that answers every connection with `answer`.
async fn answering(answer: &'static str) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            tokio::spawn(async move {
                let mut request = [0; 1024];
                let _ = stream.read(&mut request).await;
                let _ = stream.write_all(answer.as_bytes()).await;
            });
        }
    });
    address
}

#[tokio::test]
async fn a_running_rssifier_is_well() {
    let client = HttpClient::new(Config::default()).unwrap();
    let keeper = Keeper::new("no-such-tokens.json".into()).unwrap();
    let sites = Sites::new(Recipes::built_in().unwrap(), "");
    let state = AppState::new(client, keeper, Pace::default(), sites);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app(state)).await });
    health::check(address, WAIT).await.unwrap();
}

#[tokio::test]
async fn only_200_counts_and_nobody_listening_is_not_well() {
    for answer in ["HTTP/1.1 503 Service Unavailable\r\n\r\n", "200\r\n", ""] {
        let address = answering(answer).await;
        assert!(health::check(address, WAIT).await.is_err(), "{answer:?}");
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    assert!(health::check(address, WAIT).await.is_err());
}

#[tokio::test]
async fn a_server_on_every_address_is_asked_locally() {
    let address = health::address("0.0.0.0:3100").await.unwrap();
    assert_eq!(address, "127.0.0.1:3100".parse::<SocketAddr>().unwrap());
    assert!(health::address("3100").await.is_err());
}
