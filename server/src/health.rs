//! Asking a running rueder whether it is well: `rueder health`, the health
//! check of the container. The image holds nothing but the program, so there
//! is no curl to ask with.
//!
//! This speaks to the server on its own machine and nowhere else, which is why
//! it is a few lines of HTTP by hand and not the hardened client: that one
//! refuses local addresses, as it should.

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    time::Duration,
};

use anyhow::{Context, bail};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, lookup_host},
    time::timeout,
};

/// How long the server gets to answer if nothing else is asked.
pub const WAIT: Duration = Duration::from_secs(5);

/// How much of the answer is read. The status is in its first line.
const MAX_ANSWER: u64 = 4 * 1024;

/// Where a server that listens on `listen` (as in `LISTEN_ADDR`) is reached
/// from its own machine. One that listens on every address is asked on the
/// local one.
pub async fn address(listen: &str) -> anyhow::Result<SocketAddr> {
    let mut address = lookup_host(listen)
        .await
        .with_context(|| format!("{listen} is not an address to listen on"))?
        .next()
        .with_context(|| format!("{listen} has no address"))?;
    if address.ip().is_unspecified() {
        address.set_ip(match address.ip() {
            IpAddr::V4(_) => Ipv4Addr::LOCALHOST.into(),
            IpAddr::V6(_) => Ipv6Addr::LOCALHOST.into(),
        });
    }
    Ok(address)
}

/// Asks the server at `address` for `/api/health`. It is well if it answers
/// 200 within `wait`: it then serves requests and reaches its database.
pub async fn check(address: SocketAddr, wait: Duration) -> anyhow::Result<()> {
    timeout(wait, ask(address))
        .await
        .with_context(|| format!("{address} did not answer in time"))?
}

async fn ask(address: SocketAddr) -> anyhow::Result<()> {
    let mut stream = TcpStream::connect(address)
        .await
        .with_context(|| format!("connecting to {address}"))?;
    stream
        .write_all(b"GET /api/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await?;
    let mut answer = Vec::new();
    stream.take(MAX_ANSWER).read_to_end(&mut answer).await?;
    // "HTTP/1.1 200 OK"
    let line = answer.split(|byte| *byte == b'\r' || *byte == b'\n').next();
    let line = String::from_utf8_lossy(line.unwrap_or_default());
    let mut parts = line.split(' ');
    match (parts.next(), parts.next()) {
        (Some(protocol), Some("200")) if protocol.starts_with("HTTP/1.") => Ok(()),
        _ if line.is_empty() => bail!("{address} closed the connection without an answer"),
        _ => bail!("{address} answered {line:?}"),
    }
}
