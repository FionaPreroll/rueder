//! How much one sender and one account may ask of the server.
//!
//! Two limits, both counted in memory:
//!
//! - every request, per address it comes from ([`REQUESTS`]), so that nobody
//!   keeps the server busy for everybody else, logged in or not;
//! - per account, what makes the server reach out to an address the reader
//!   names ([`LOOKUPS`]): looking for the feeds of a page and subscribing. It
//!   keeps an account from using rueder to knock on other servers' doors.
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
/// How many addresses and accounts are remembered at most.
const MAX_KEYS: usize = 10_000;

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
        let mut used = self.used.lock().unwrap_or_else(PoisonError::into_inner);
        let back = used.get(key).map_or(now, |back| (*back).max(now));
        let ahead = back.saturating_duration_since(now) + self.rate.every;
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

/// The limits of a server.
pub struct Rates {
    requests: Limiter<IpAddr>,
    lookups: Limiter<i32>,
}

impl Rates {
    pub fn new(requests: Rate, lookups: Rate) -> Self {
        Self {
            requests: Limiter::new(requests),
            lookups: Limiter::new(lookups),
        }
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
