//! The one way out to the internet: feeds, web pages for the feed search and
//! images are all fetched through this client, so the rules against fetching
//! from inside the own network (SSRF) are in one place.
//!
//! - Only `http` and `https`.
//! - Names are looked up here, every address a name has is checked against
//!   [`address::internal`], and the connection goes to exactly the addresses that
//!   were checked. A name cannot pass the check with one address and be
//!   connected to at another.
//! - Redirects are followed here, one at a time, and each target is checked like
//!   the first address.
//! - Limits for the time of the whole fetch, the number of redirects and the size
//!   of the body, counted after decompression.
//! - No proxy from the environment (a proxy would do the lookup in our place), no
//!   cookies, no referrer.

pub mod address;

use std::{
    error::Error as StdError,
    fmt,
    future::Future,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use reqwest::{
    StatusCode, Url,
    dns::{Addrs, Name, Resolve, Resolving},
    header::{self, HeaderMap},
};

pub use address::Net;

/// What rueder calls itself when it fetches.
const AGENT_RUEDER: &str = concat!(
    "rueder/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/FionaPreroll/rueder)"
);
/// For sites that turn away clients they do not know.
const AGENT_BROWSER: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0";

/// The name rueder fetches under.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Agent {
    /// As itself: "rueder", with version and the project's address.
    #[default]
    Rueder,
    /// As a common browser.
    Browser,
}

/// The limits of the client and the exceptions from its rules.
#[derive(Debug, Clone)]
pub struct Config {
    /// Networks that are fetched from although they are internal, e.g. the one a
    /// feed generator of one's own runs in. Empty unless the operator says so.
    pub allow: Vec<Net>,
    /// The largest body, in bytes after decompression.
    pub max_body: usize,
    /// How many redirects one fetch follows.
    pub max_redirects: usize,
    /// For the whole fetch: lookups, redirects and reading the body.
    pub timeout: Duration,
    /// For setting up one connection.
    pub connect_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            allow: Vec::new(),
            max_body: 20 * 1024 * 1024,
            max_redirects: 5,
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
        }
    }
}

/// Why a fetch did not happen or did not finish. The texts are fit to be shown
/// at a feed; they name hosts, never passwords from an address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Not an address, or not one with `http` or `https` and a host.
    Address(String),
    /// The host is, or leads to, an address rueder does not fetch from.
    Internal {
        host: String,
        addr: IpAddr,
        why: &'static str,
    },
    /// More redirects than [`Config::max_redirects`].
    TooManyRedirects(usize),
    /// A redirect that says nowhere to go.
    BadRedirect,
    /// The body is larger than allowed, in bytes.
    TooLarge(usize),
    /// The whole fetch took longer than [`Config::timeout`].
    Timeout,
    /// The name has no address, nobody answers, the connection broke.
    Network(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Address(why) => write!(f, "not an address rueder can fetch: {why}"),
            // Not the address itself: a reader who typed the name learns
            // nothing about the network the server is in.
            Self::Internal { host, why, .. } => {
                write!(f, "{host} is not on the public internet ({why})")
            }
            Self::TooManyRedirects(limit) => write!(f, "more than {limit} redirects"),
            Self::BadRedirect => write!(f, "a redirect without a usable target"),
            Self::TooLarge(limit) => write!(f, "larger than {limit} bytes"),
            Self::Timeout => write!(f, "took too long"),
            Self::Network(why) => write!(f, "{why}"),
        }
    }
}

impl StdError for Error {}

/// Looks up the addresses of a name. The system's resolver in the server, a
/// table in tests.
pub trait Lookup: Send + Sync {
    fn lookup<'a>(&'a self, host: &'a str) -> Looking<'a>;
}

/// The answer of a [`Lookup`], once it is there.
pub type Looking<'a> = Pin<Box<dyn Future<Output = std::io::Result<Vec<IpAddr>>> + Send + 'a>>;

/// The resolver of the operating system.
pub struct SystemLookup;

impl Lookup for SystemLookup {
    fn lookup<'a>(&'a self, host: &'a str) -> Looking<'a> {
        Box::pin(async move {
            let addrs = tokio::net::lookup_host((host, 0)).await?;
            Ok(addrs.map(|addr| addr.ip()).collect())
        })
    }
}

/// The rules for addresses, shared by the check before a request and the lookup
/// inside the connection.
struct Rules {
    allow: Vec<Net>,
}

impl Rules {
    fn check_addr(&self, host: &str, addr: IpAddr) -> Result<(), Error> {
        // ::ffff:10.0.0.1 is 10.0.0.1, also for the operator's exceptions.
        let addr = addr.to_canonical();
        if self.allow.iter().any(|net| net.contains(addr)) {
            return Ok(());
        }
        match address::internal(addr) {
            None => Ok(()),
            Some(why) => Err(Error::Internal {
                host: host.to_owned(),
                addr,
                why,
            }),
        }
    }

    /// What can be said about an address before any lookup: the scheme, and the
    /// host where it is written as a number.
    fn check_url(&self, url: &Url) -> Result<(), Error> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(Error::Address(format!(
                "{}: is neither http nor https",
                url.scheme()
            )));
        }
        let host = match url.host() {
            None => return Err(Error::Address("no host".to_owned())),
            Some(url::Host::Ipv4(addr)) => return self.check_addr(&addr.to_string(), addr.into()),
            Some(url::Host::Ipv6(addr)) => return self.check_addr(&addr.to_string(), addr.into()),
            Some(url::Host::Domain(host)) => host,
        };
        // The connection treats a host as a number if it reads as one, whatever
        // the parser of the address called it. So ask the same question here.
        if let Ok(addr) = host.trim_end_matches('.').parse::<IpAddr>() {
            return self.check_addr(host, addr);
        }
        Ok(())
    }
}

/// The lookup the connections use: the addresses of a name, all of them
/// checked. What it returns is what gets connected to.
struct CheckedLookup {
    lookup: Arc<dyn Lookup>,
    rules: Arc<Rules>,
}

impl Resolve for CheckedLookup {
    fn resolve(&self, name: Name) -> Resolving {
        let lookup = self.lookup.clone();
        let rules = self.rules.clone();
        Box::pin(async move {
            let host = name.as_str();
            let found = lookup.lookup(host).await?;
            if found.is_empty() {
                return Err(Error::Network(format!("{host} has no address")).into());
            }
            // One internal address among them refuses the name: a name that
            // points both outside and inside is up to something.
            for &addr in &found {
                rules.check_addr(host, addr)?;
            }
            let addrs: Addrs = Box::new(found.into_iter().map(|addr| SocketAddr::new(addr, 0)));
            Ok(addrs)
        })
    }
}

/// What to fetch.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    pub url: &'a str,
    pub agent: Agent,
    /// What the caller can read, as an `Accept` header.
    pub accept: Option<&'a str>,
    /// The `ETag` of the copy the caller has, to get "not modified" (304)
    /// instead of the same body again.
    pub etag: Option<&'a str>,
    /// The `Last-Modified` of the copy the caller has, for the same purpose.
    pub last_modified: Option<&'a str>,
    /// A smaller limit for the body than [`Config::max_body`]; a larger one
    /// counts as that.
    pub max_body: Option<usize>,
    /// Fields to send as a form. With them the request is a POST, and one
    /// that is never sent on to where a redirect points.
    pub form: Option<&'a [(&'a str, &'a str)]>,
    /// An `Authorization` header. It goes to the host of `url` only.
    pub authorization: Option<&'a str>,
}

impl<'a> Request<'a> {
    pub fn new(url: &'a str) -> Self {
        Self {
            url,
            agent: Agent::default(),
            accept: None,
            etag: None,
            last_modified: None,
            max_body: None,
            form: None,
            authorization: None,
        }
    }
}

/// What came back, after all redirects.
#[derive(Debug)]
pub struct Response {
    /// Where the body came from in the end.
    pub url: Url,
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}

impl Response {
    /// A header's value, if it is there and is text.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)?.to_str().ok()
    }
}

/// The hardened HTTP client. One for the whole server; cloning shares its
/// connections.
#[derive(Clone)]
pub struct HttpClient {
    inner: reqwest::Client,
    rules: Arc<Rules>,
    config: Arc<Config>,
}

impl HttpClient {
    /// A client that looks names up with the operating system's resolver.
    pub fn new(config: Config) -> anyhow::Result<Self> {
        Self::with_lookup(config, Arc::new(SystemLookup))
    }

    /// A client that looks names up with `lookup`.
    pub fn with_lookup(config: Config, lookup: Arc<dyn Lookup>) -> anyhow::Result<Self> {
        let rules = Arc::new(Rules {
            allow: config.allow.clone(),
        });
        // The one place that builds a client; `clippy.toml` forbids it elsewhere.
        #[expect(clippy::disallowed_methods)]
        let builder = reqwest::Client::builder();
        let inner = builder
            .tls_backend_preconfigured(tls()?)
            .dns_resolver(CheckedLookup {
                lookup,
                rules: rules.clone(),
            })
            // Redirects are followed in `follow`, where each target is checked.
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .referer(false)
            .connect_timeout(config.connect_timeout)
            .build()
            .context("building the HTTP client")?;
        Ok(Self {
            inner,
            rules,
            config: Arc::new(config),
        })
    }

    /// Reads an address somebody entered and says whether rueder would fetch
    /// it, as far as that shows without a lookup.
    pub fn parse(&self, address: &str) -> Result<Url, Error> {
        let mut url = Url::parse(address.trim()).map_err(|err| Error::Address(err.to_string()))?;
        url.set_fragment(None);
        self.rules.check_url(&url)?;
        Ok(url)
    }

    /// Fetches the address with GET and follows redirects. Any status is an
    /// answer, also 304 and 404; an error means there was none worth having.
    pub async fn get(&self, request: Request<'_>) -> Result<Response, Error> {
        match tokio::time::timeout(self.config.timeout, self.follow(request)).await {
            Ok(result) => result,
            Err(_) => Err(Error::Timeout),
        }
    }

    async fn follow(&self, request: Request<'_>) -> Result<Response, Error> {
        let mut url = self.parse(request.url)?;
        for _ in 0..=self.config.max_redirects {
            let mut response = self.send(&url, request).await?;
            if let Some(target) = redirect(&response) {
                // What was posted, and a password, stay with whom they were for.
                if request.form.is_some() || request.authorization.is_some() {
                    return Err(Error::BadRedirect);
                }
                url = url.join(target?).map_err(|_| Error::BadRedirect)?;
                url.set_fragment(None);
                self.rules.check_url(&url)?;
                continue;
            }

            let limit = self.config.max_body;
            let limit = request.max_body.map_or(limit, |asked| asked.min(limit));
            let too_large = |len: u64| usize::try_from(len).map_or(true, |len| len > limit);
            if response.content_length().is_some_and(too_large) {
                return Err(Error::TooLarge(limit));
            }
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(explain)? {
                if body.len() + chunk.len() > limit {
                    return Err(Error::TooLarge(limit));
                }
                body.extend_from_slice(&chunk);
            }
            return Ok(Response {
                url,
                status: response.status(),
                headers: std::mem::take(response.headers_mut()),
                body,
            });
        }
        Err(Error::TooManyRedirects(self.config.max_redirects))
    }

    async fn send(&self, url: &Url, request: Request<'_>) -> Result<reqwest::Response, Error> {
        let agent = match request.agent {
            Agent::Rueder => AGENT_RUEDER,
            Agent::Browser => AGENT_BROWSER,
        };
        let mut builder = match request.form {
            None => self.inner.get(url.clone()),
            Some(fields) => {
                let mut form = url::form_urlencoded::Serializer::new(String::new());
                form.extend_pairs(fields);
                self.inner
                    .post(url.clone())
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(form.finish())
            }
        };
        builder = builder.header(header::USER_AGENT, agent);
        let optional = [
            (header::AUTHORIZATION, request.authorization),
            (header::ACCEPT, request.accept),
            (header::IF_NONE_MATCH, request.etag),
            (header::IF_MODIFIED_SINCE, request.last_modified),
        ];
        for (name, value) in optional {
            if let Some(value) = value {
                builder = builder.header(name, value);
            }
        }
        builder.send().await.map_err(explain)
    }
}

/// The target of a redirect, if the response is one: `None` if it is not,
/// an error if it is but names no target.
fn redirect(response: &reqwest::Response) -> Option<Result<&str, Error>> {
    let moved = matches!(
        response.status(),
        StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT
    );
    if !moved {
        return None;
    }
    let target = response.headers().get(header::LOCATION);
    Some(
        target
            .and_then(|target| target.to_str().ok())
            .ok_or(Error::BadRedirect),
    )
}

/// Turns the HTTP library's error into ours. A refusal of our own lookup sits
/// somewhere down its chain of causes.
fn explain(err: reqwest::Error) -> Error {
    let mut cause: Option<&(dyn StdError + 'static)> = Some(&err);
    let mut last = String::new();
    while let Some(err) = cause {
        if let Some(ours) = err.downcast_ref::<Error>() {
            return ours.clone();
        }
        last = err.to_string();
        cause = err.source();
    }
    if err.is_timeout() {
        return Error::Timeout;
    }
    // The innermost cause says what happened ("connection refused"); the outer
    // ones repeat the address, which may hold a password.
    let what = if err.is_connect() {
        "could not connect"
    } else if err.is_body() || err.is_decode() {
        "the answer broke off"
    } else {
        "the request failed"
    };
    Error::Network(format!("{what}: {last}"))
}

/// TLS with the certificate authorities Mozilla trusts, compiled in, so it
/// works the same in a container without certificates of its own.
fn tls() -> anyhow::Result<rustls::ClientConfig> {
    let roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .context("setting up TLS")?
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(config)
}
