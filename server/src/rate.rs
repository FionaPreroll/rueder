//! How much one sender and one account may ask of the server.
//!
//! These limits, all counted in memory:
//!
//! - every request, per address it comes from ([`REQUESTS`]), so that nobody
//!   keeps the server busy for everybody else, logged in or not;
//! - per account, what makes the server reach out to an address the reader
//!   names ([`LOOKUPS`]): looking for the feeds of a page and subscribing. It
//!   keeps an account from using rueder to knock on other servers' doors;
//! - per account, the feeds nobody read before that it brings in
//!   ([`NEW_FEEDS`]), alone or many at once with an OPML file or a move: each
//!   is fetched from then on, for as long as somebody reads it;
//! - per account, the reports of errors from its browsers ([`REPORTS`]) and
//!   the moves from another rueder ([`IMPORTS`]), which add to what is stored;
//! - what costs the server more than a moment, such as looking at all of an
//!   account's articles for its rules, runs only a few at a time
//!   ([`HEAVY_AT_ONCE`], [`HEAVY_PER_ACCOUNT`], see [`Rates::heavy`]), so that
//!   no account takes the database's connections and the processors from
//!   everybody else.
//!
//! Wrong passwords have their own, much smaller limit in [`crate::login`].
//! Who is over a limit gets 429 with `Retry-After`.

use std::{
    collections::HashMap,
    hash::Hash,
    net::IpAddr,
    sync::{Mutex, PoisonError},
    time::{Duration, Instant},
};

use tokio::sync::{Semaphore, SemaphorePermit};

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    AppError, AppState,
    login::{self, Sender},
};

/// What an address may send: 1200 requests at once, and 50 more every second.
/// Opening a list asks for a few dozen images at once, and several readers
/// may share an address, so this is far above what reading needs.
pub const REQUESTS: Rate = Rate::new(1200, Duration::from_millis(20));
/// How many addresses an account may have the server look at: 30 at once, and
/// one more every two seconds.
pub const LOOKUPS: Rate = Rate::new(30, Duration::from_secs(2));
/// How many feeds nobody read before an account may bring in: as many as one
/// OPML file can hold at once, and one more every 30 seconds, close to 3000 a
/// day.
pub const NEW_FEEDS: Rate = Rate::new(
    crate::subscribe::MAX_OPML_FEEDS as u32,
    Duration::from_secs(30),
);
/// How many reports of errors in its browsers an account may send: 100 at
/// once, and about 100 more a day.
pub const REPORTS: Rate = Rate::new(100, Duration::from_secs(864));
/// How many feeds and articles an account may report: 20 at once, and about
/// 20 more a day, as a reader reports now and then.
pub const CONTENT_REPORTS: Rate = Rate::new(20, Duration::from_secs(4320));
/// How many changes apps of other makers may send for an account: 60 at
/// once, and one more every second. A sync sends a handful; NetNewsWire sends
/// read states in pieces of 1000, which are one request each.
pub const APP_CHANGES: Rate = Rate::new(60, Duration::from_secs(1));
/// How many moves from another rueder an account may bring in: 3 at once, and
/// one more every two hours. Each can bring a lot (see `transfer`), and a
/// move is done once.
pub const IMPORTS: Rate = Rate::new(3, Duration::from_secs(2 * 60 * 60));
/// How many addresses and accounts are remembered at most.
const MAX_KEYS: usize = 10_000;
/// How many costly requests run at once on the whole server: fewer than the
/// database has connections, so that reading goes on meanwhile. More wait.
pub const HEAVY_AT_ONCE: usize = 3;
/// How many costly requests one account may have under way, waiting or
/// running. More are refused.
pub const HEAVY_PER_ACCOUNT: usize = 2;

/// A limit: `burst` at once, and one more after every `every`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rate {
    pub burst: u32,
    pub every: Duration,
}

impl Rate {
    pub const fn new(burst: u32, every: Duration) -> Self {
        Self { burst, every }
    }

    /// How long it takes until all of `burst` is there again.
    fn full(&self) -> Duration {
        self.every.saturating_mul(self.burst)
    }
}

/// Counts what each key has used of a [`Rate`].
///
/// Per key it keeps one point in time: when everything the key has used will
/// have come back. A key may ask as long as that point is less than a whole
/// burst away.
pub struct Limiter<K> {
    rate: Rate,
    max_keys: usize,
    used: Mutex<HashMap<K, Instant>>,
}

impl<K: Hash + Eq + Clone> Limiter<K> {
    pub fn new(rate: Rate) -> Self {
        Self {
            rate,
            max_keys: MAX_KEYS,
            used: Mutex::default(),
        }
    }

    /// Takes one from what the key has left, or says how long it has to wait
    /// for the next.
    pub fn take(&self, key: &K, now: Instant) -> Result<(), Duration> {
        self.take_many(key, 1, now)
    }

    /// Takes `n` at once from what the key has left, or none of them and says
    /// how long it has to wait for all of them. More than a burst never fits.
    pub fn take_many(&self, key: &K, n: u32, now: Instant) -> Result<(), Duration> {
        if n == 0 {
            return Ok(());
        }
        let mut used = self.used.lock().unwrap_or_else(PoisonError::into_inner);
        let back = used.get(key).map_or(now, |back| (*back).max(now));
        let ahead = back.saturating_duration_since(now) + self.rate.every.saturating_mul(n);
        if let Some(wait) = ahead.checked_sub(self.rate.full())
            && !wait.is_zero()
        {
            return Err(wait);
        }
        if used.len() >= self.max_keys && !used.contains_key(key) {
            // Whoever has everything back needs no entry.
            used.retain(|_, back| *back > now);
            // Still full: the one that is closest to having everything back
            // makes room.
            if used.len() >= self.max_keys
                && let Some(least) = used
                    .iter()
                    .min_by_key(|(_, back)| **back)
                    .map(|(key, _)| key.clone())
            {
                used.remove(&least);
            }
        }
        used.insert(key.clone(), now + ahead);
        Ok(())
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.used.lock().unwrap().len()
    }
}

/// Costly requests: how many run, and how many each account has under way.
struct Heavy {
    running: Semaphore,
    accounts: Mutex<HashMap<i32, usize>>,
}

/// A costly request's turn: while it is kept, the request counts.
pub struct Turn<'a> {
    heavy: &'a Heavy,
    account_id: i32,
    _running: Option<SemaphorePermit<'a>>,
}

impl Drop for Turn<'_> {
    fn drop(&mut self) {
        let mut accounts = self
            .heavy
            .accounts
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(count) = accounts.get_mut(&self.account_id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                accounts.remove(&self.account_id);
            }
        }
    }
}

/// The limits of a server.
pub struct Rates {
    requests: Limiter<IpAddr>,
    lookups: Limiter<i32>,
    new_feeds: Limiter<i32>,
    reports: Limiter<i32>,
    content_reports: Limiter<i32>,
    imports: Limiter<i32>,
    app_changes: Limiter<i32>,
    heavy: Heavy,
}

impl Rates {
    pub fn new(requests: Rate, lookups: Rate) -> Self {
        Self {
            requests: Limiter::new(requests),
            lookups: Limiter::new(lookups),
            new_feeds: Limiter::new(NEW_FEEDS),
            reports: Limiter::new(REPORTS),
            content_reports: Limiter::new(CONTENT_REPORTS),
            imports: Limiter::new(IMPORTS),
            app_changes: Limiter::new(APP_CHANGES),
            heavy: Heavy {
                running: Semaphore::new(HEAVY_AT_ONCE),
                accounts: Mutex::default(),
            },
        }
    }

    /// Waits for a costly request's turn, or refuses it if the account has
    /// [`HEAVY_PER_ACCOUNT`] under way already. The request keeps the turn
    /// until it is done, and takes no connection of the database before.
    pub async fn heavy(&self, account_id: i32) -> Result<Turn<'_>, AppError> {
        {
            let mut accounts = self
                .heavy
                .accounts
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let count = accounts.entry(account_id).or_default();
            if *count >= HEAVY_PER_ACCOUNT {
                let message = "too much at once for this account, try again in a moment";
                return Err(AppError::TooMany(Duration::from_secs(1), message));
            }
            *count += 1;
        }
        let mut turn = Turn {
            heavy: &self.heavy,
            account_id,
            _running: None,
        };
        // Never closed, so the permit always comes.
        turn._running = self.heavy.running.acquire().await.ok();
        Ok(turn)
    }

    /// Other limits for the moves from another rueder than [`IMPORTS`].
    pub fn with_imports(self, imports: Rate) -> Self {
        Self {
            imports: Limiter::new(imports),
            ..self
        }
    }

    /// Counts a move from another rueder into the account.
    pub fn import(&self, account_id: i32, now: Instant) -> Result<(), AppError> {
        self.imports
            .take(&account_id, now)
            .map_err(|wait| AppError::TooMany(wait, "too many imports, try again later"))
    }

    /// Other limits for the feeds nobody read before than [`NEW_FEEDS`].
    pub fn with_new_feeds(self, new_feeds: Rate) -> Self {
        Self {
            new_feeds: Limiter::new(new_feeds),
            ..self
        }
    }

    /// Counts `n` feeds nobody read before that the account brings in, all of
    /// them or none.
    pub fn new_feeds(&self, account_id: i32, n: usize, now: Instant) -> Result<(), AppError> {
        let n = u32::try_from(n).unwrap_or(u32::MAX);
        self.new_feeds
            .take_many(&account_id, n, now)
            .map_err(|wait| AppError::TooMany(wait, "too many new feeds at once, try again later"))
    }

    /// Other limits for the reports of errors than [`REPORTS`].
    pub fn with_reports(self, reports: Rate) -> Self {
        Self {
            reports: Limiter::new(reports),
            ..self
        }
    }

    /// Other limits for reports of feeds and articles than [`CONTENT_REPORTS`].
    pub fn with_content_reports(self, reports: Rate) -> Self {
        Self {
            content_reports: Limiter::new(reports),
            ..self
        }
    }

    /// Counts a report of a feed or an article the account sends.
    pub fn content_report(&self, account_id: i32, now: Instant) -> Result<(), AppError> {
        self.content_reports
            .take(&account_id, now)
            .map_err(|wait| AppError::TooMany(wait, "too many reports, try again later"))
    }

    /// Other limits for the changes of apps than [`APP_CHANGES`].
    pub fn with_app_changes(self, changes: Rate) -> Self {
        Self {
            app_changes: Limiter::new(changes),
            ..self
        }
    }

    /// Counts a change an app of another maker sends for the account.
    pub fn app_change(&self, account_id: i32, now: Instant) -> Result<(), AppError> {
        self.app_changes
            .take(&account_id, now)
            .map_err(|wait| AppError::TooMany(wait, "too many changes, try again later"))
    }

    /// Counts a report of an error the account's browser sends.
    pub fn report(&self, account_id: i32, now: Instant) -> Result<(), AppError> {
        self.reports
            .take(&account_id, now)
            .map_err(|wait| AppError::TooMany(wait, "too many reports, try again later"))
    }

    /// Counts a request from this address. An IPv6 sender counts as its /64,
    /// as for wrong passwords.
    pub fn request(&self, ip: IpAddr, now: Instant) -> Result<(), Duration> {
        self.requests.take(&login::network(ip), now)
    }

    /// Counts an address the account has the server look at.
    pub fn lookup(&self, account_id: i32, now: Instant) -> Result<(), AppError> {
        self.lookups
            .take(&account_id, now)
            .map_err(|wait| AppError::TooMany(wait, "too many addresses at once, try again later"))
    }
}

impl Default for Rates {
    fn default() -> Self {
        Self::new(REQUESTS, LOOKUPS)
    }
}

/// The answer to somebody over a limit: when to come back, in whole seconds.
pub fn too_many(wait: Duration, message: &'static str) -> Response {
    let seconds = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(header::RETRY_AFTER, seconds.max(1).to_string())],
        message,
    )
        .into_response()
}

/// Around everything the server answers: counts the request for its sender.
pub async fn limit(
    State(state): State<AppState>,
    Sender(ip): Sender,
    request: Request,
    next: Next,
) -> Response {
    match state.rates.request(ip, Instant::now()) {
        Ok(()) => next.run(request).await,
        Err(wait) => too_many(wait, "too many requests, try again later"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn a_burst_is_used_up_and_comes_back() {
        let limiter = Limiter::new(Rate::new(3, SECOND));
        let start = Instant::now();
        for _ in 0..3 {
            assert_eq!(limiter.take(&"ada", start), Ok(()));
        }
        assert_eq!(limiter.take(&"ada", start), Err(SECOND));
        // Being refused costs nothing.
        assert_eq!(limiter.take(&"ada", start), Err(SECOND));
        assert_eq!(limiter.take(&"grace", start), Ok(()));

        let half = Duration::from_millis(500);
        assert_eq!(limiter.take(&"ada", start + half), Err(half));
        // One comes back every second, and no more than one.
        assert_eq!(limiter.take(&"ada", start + SECOND), Ok(()));
        assert_eq!(limiter.take(&"ada", start + SECOND), Err(SECOND));
        // After a long rest there is a burst again, not more.
        let later = start + Duration::from_secs(3600);
        for _ in 0..3 {
            assert_eq!(limiter.take(&"ada", later), Ok(()));
        }
        assert_eq!(limiter.take(&"ada", later), Err(SECOND));
    }

    #[test]
    fn many_are_taken_at_once_or_not_at_all() {
        let limiter = Limiter::new(Rate::new(10, SECOND));
        let start = Instant::now();
        assert_eq!(limiter.take_many(&"ada", 0, start), Ok(()));
        assert_eq!(limiter.len(), 0, "nothing taken, nothing remembered");
        assert_eq!(limiter.take_many(&"ada", 7, start), Ok(()));
        // Four do not fit into the three that are left, and cost nothing.
        assert_eq!(limiter.take_many(&"ada", 4, start), Err(SECOND));
        assert_eq!(limiter.take_many(&"ada", 3, start), Ok(()));
        assert_eq!(limiter.take(&"ada", start), Err(SECOND));
        assert_eq!(
            limiter.take_many(&"ada", 5, start + 2 * SECOND),
            Err(3 * SECOND)
        );
        assert_eq!(limiter.take_many(&"ada", 5, start + 5 * SECOND), Ok(()));
        // More than a burst never fits, however long one waits.
        let later = start + Duration::from_secs(3600);
        assert!(limiter.take_many(&"grace", 11, later).is_err());
        assert_eq!(limiter.take_many(&"grace", 10, later), Ok(()));
        assert_eq!(
            limiter.take_many(&"ada", u32::MAX, later).map_err(|_| ()),
            Err(())
        );
    }

    #[test]
    fn a_steady_sender_within_the_rate_is_never_refused() {
        let limiter = Limiter::new(Rate::new(2, SECOND));
        let start = Instant::now();
        for n in 0..100 {
            assert_eq!(limiter.take(&1, start + SECOND * n), Ok(()), "{n}");
        }
    }

    #[test]
    fn what_is_remembered_has_a_limit() {
        let mut limiter = Limiter::new(Rate::new(2, SECOND));
        limiter.max_keys = 100;
        let start = Instant::now();
        // The first has used up everything, the others one each.
        assert_eq!(limiter.take(&0, start), Ok(()));
        assert_eq!(limiter.take(&0, start), Ok(()));
        for key in 1..100 {
            assert_eq!(limiter.take(&key, start), Ok(()));
        }
        assert_eq!(limiter.len(), 100);
        // One more: one of those with the most left goes, not the first.
        assert_eq!(limiter.take(&100, start), Ok(()));
        assert_eq!(limiter.len(), 100);
        assert_eq!(limiter.take(&0, start), Err(SECOND));
        // Once everything is back, all of them go.
        assert_eq!(limiter.take(&101, start + SECOND * 5), Ok(()));
        assert_eq!(limiter.len(), 1);
    }

    #[test]
    fn an_ipv6_sender_is_its_network() {
        let rates = Rates::new(Rate::new(1, SECOND), LOOKUPS);
        let now = Instant::now();
        let ip = |text: &str| text.parse::<IpAddr>().unwrap();
        assert_eq!(rates.request(ip("2001:db8::1"), now), Ok(()));
        assert_eq!(rates.request(ip("2001:db8::2"), now), Err(SECOND));
        assert_eq!(rates.request(ip("2001:db8:0:1::1"), now), Ok(()));
        assert_eq!(rates.request(ip("192.0.2.1"), now), Ok(()));
        assert_eq!(rates.request(ip("192.0.2.2"), now), Ok(()));
    }

    #[tokio::test]
    async fn costly_requests_take_turns() {
        let rates = Rates::default();
        // An account has so many under way, and no more.
        let mut ada = Vec::new();
        for _ in 0..HEAVY_PER_ACCOUNT {
            ada.push(rates.heavy(1).await.ok().unwrap());
        }
        assert!(matches!(rates.heavy(1).await, Err(AppError::TooMany(..))));
        // Others still have theirs, until the server runs as many as it may.
        let mut others = Vec::new();
        for account in 2..=(HEAVY_AT_ONCE - HEAVY_PER_ACCOUNT + 1) as i32 {
            others.push(rates.heavy(account).await.ok().unwrap());
        }
        assert_eq!(rates.heavy.running.available_permits(), 0);
        // The next one waits for a turn to end, and then runs.
        let waiting = rates.heavy(99);
        tokio::pin!(waiting);
        let early = tokio::time::timeout(Duration::from_millis(50), &mut waiting).await;
        assert!(early.is_err());
        ada.pop();
        let turn = tokio::time::timeout(SECOND, &mut waiting).await;
        assert!(matches!(turn, Ok(Ok(_))));
        drop(turn);
        // A turn that ends gives the account its place back.
        assert!(rates.heavy(1).await.is_ok());
        drop(ada);
        drop(others);
        assert!(rates.heavy.accounts.lock().unwrap().len() <= 1);
    }

    #[test]
    fn the_wait_is_in_whole_seconds() {
        let seconds = |wait| {
            let answer = too_many(wait, "no");
            assert_eq!(answer.status(), StatusCode::TOO_MANY_REQUESTS);
            answer.headers()[header::RETRY_AFTER]
                .to_str()
                .unwrap()
                .to_owned()
        };
        assert_eq!(seconds(Duration::from_millis(1)), "1");
        assert_eq!(seconds(SECOND), "1");
        assert_eq!(seconds(Duration::from_millis(1001)), "2");
    }
}
