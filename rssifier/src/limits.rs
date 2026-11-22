//! The limits of each token, and those of the whole server that no token
//! lifts: how many pages are fetched and read at once, and how often one host
//! is asked.
//!
//! Counters live in memory and start anew with the server. That is enough for
//! limits meant to keep a client from taking more than its share; nobody is
//! billed by them.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

use serde::Serialize;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard, OwnedSemaphorePermit, Semaphore};
use utoipa::ToSchema;

use crate::tokens::{Limits, Token};

/// A pot that fills by one for each use and empties at `per` uses in `over`.
/// A use that would make it overflow waits until there is room.
#[derive(Debug, Clone, Copy)]
struct Pot {
    level: f64,
    at: Instant,
}

impl Pot {
    fn new(now: Instant) -> Self {
        Self {
            level: 0.0,
            at: now,
        }
    }

    /// The level at `now`, after what ran out since.
    fn drain(&mut self, per: u32, over: Duration, now: Instant) {
        let rate = f64::from(per) / over.as_secs_f64();
        let passed = now.saturating_duration_since(self.at).as_secs_f64();
        self.level = (self.level - passed * rate).max(0.0);
        self.at = now;
    }

    /// Takes one use, or says how long until there is room for it.
    fn take(&mut self, per: u32, over: Duration, now: Instant) -> Result<(), Duration> {
        self.drain(per, over, now);
        if self.level + 1.0 <= f64::from(per) {
            self.level += 1.0;
            return Ok(());
        }
        let rate = f64::from(per) / over.as_secs_f64();
        let wait = (self.level + 1.0 - f64::from(per)) / rate;
        Err(Duration::from_secs_f64(wait.max(0.0)))
    }

    /// How many uses the pot holds at `now`.
    fn used(&mut self, per: u32, over: Duration, now: Instant) -> u32 {
        self.drain(per, over, now);
        // The level never passes `per`, which is a u32.
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let used = self.level.ceil() as u32;
        used
    }
}

const MINUTE: Duration = Duration::from_secs(60);
const HOUR: Duration = Duration::from_secs(60 * 60);

#[derive(Debug)]
struct Counters {
    requests: Pot,
    fetches: Pot,
    under_way: u32,
}

/// What a token has used of its limits, for `GET /v1/usage`.
#[derive(Debug, Serialize, ToSchema)]
pub struct Usage {
    pub name: String,
    pub limits: Limits,
    /// Requests in the last minute, as far as the limit counts them; without
    /// a limit per minute always 0.
    pub requests: u32,
    /// Pages fetched in the last hour, as far as the limit counts them.
    pub fetches: u32,
    /// Requests under way now.
    pub under_way: u32,
}

/// Why a token may not go on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Over {
    /// Too many requests or fetches: come back after this long.
    Rate(Duration),
    /// Too many requests at once.
    AtOnce,
}

/// The counters of all tokens, by name.
#[derive(Default)]
pub struct Tokens {
    counters: Mutex<HashMap<String, Counters>>,
}

/// A request under way; it stops counting when dropped.
pub struct Turn {
    tokens: Arc<Tokens>,
    name: String,
}

impl Drop for Turn {
    fn drop(&mut self) {
        let mut counters = self.tokens.lock();
        if let Some(counters) = counters.get_mut(&self.name) {
            counters.under_way = counters.under_way.saturating_sub(1);
        }
    }
}

impl Tokens {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Counters>> {
        self.counters.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn with<T>(&self, name: &str, now: Instant, f: impl FnOnce(&mut Counters) -> T) -> T {
        let mut counters = self.lock();
        let counters = counters.entry(name.to_owned()).or_insert(Counters {
            requests: Pot::new(now),
            fetches: Pot::new(now),
            under_way: 0,
        });
        f(counters)
    }

    /// Counts a request of the token, before anything is done for it.
    pub fn request(self: &Arc<Self>, token: &Token, now: Instant) -> Result<Turn, Over> {
        let limits = token.limits;
        self.with(&token.name, now, |counters| {
            if limits
                .at_once
                .is_some_and(|most| counters.under_way >= most)
            {
                return Err(Over::AtOnce);
            }
            if let Some(per) = limits.per_minute {
                counters
                    .requests
                    .take(per, MINUTE, now)
                    .map_err(Over::Rate)?;
            }
            counters.under_way += 1;
            Ok(())
        })?;
        Ok(Turn {
            tokens: self.clone(),
            name: token.name.clone(),
        })
    }

    /// Counts a page the cache could not answer, before it is fetched.
    pub fn fetch(&self, token: &Token, now: Instant) -> Result<(), Over> {
        let Some(per) = token.limits.fetches_per_hour else {
            return Ok(());
        };
        self.with(&token.name, now, |counters| {
            counters.fetches.take(per, HOUR, now).map_err(Over::Rate)
        })
    }

    /// What the token has used.
    pub fn usage(&self, token: &Token, now: Instant) -> Usage {
        let limits = token.limits;
        self.with(&token.name, now, |counters| Usage {
            name: token.name.clone(),
            limits,
            requests: limits
                .per_minute
                .map_or(0, |per| counters.requests.used(per, MINUTE, now)),
            fetches: limits
                .fetches_per_hour
                .map_or(0, |per| counters.fetches.used(per, HOUR, now)),
            under_way: counters.under_way,
        })
    }
}

/// How the server goes easy on the sites and on itself.
#[derive(Debug, Clone, Copy)]
pub struct Pace {
    /// How long an answer for a page is given again without asking the site.
    pub cache_for: Duration,
    /// The time between two fetches from one host.
    pub host_spacing: Duration,
    /// Pages fetched at the same time, over all tokens.
    pub fetches_at_once: usize,
    /// Pages read at the same time; reading takes a processor.
    pub reads_at_once: usize,
    /// How long a request waits for its host before it gives up.
    pub host_wait: Duration,
}

impl Default for Pace {
    fn default() -> Self {
        Self {
            // As often as rueder fetches a feed at the most.
            cache_for: Duration::from_secs(15 * 60),
            host_spacing: Duration::from_secs(2),
            fetches_at_once: 16,
            reads_at_once: 4,
            host_wait: Duration::from_secs(20),
        }
    }
}

/// How many hosts are remembered before those nobody waits for go.
const MAX_HOSTS: usize = 10_000;

/// The limits of the whole server.
pub struct Gates {
    pub pace: Pace,
    fetches: Arc<Semaphore>,
    reads: Arc<Semaphore>,
    /// When each host was last asked; held while it is asked.
    hosts: Mutex<HashMap<String, Arc<AsyncMutex<Option<Instant>>>>>,
}

/// The right to fetch from a host; the next one waits until it is dropped
/// and the spacing has passed.
pub struct HostTurn {
    last: OwnedMutexGuard<Option<Instant>>,
}

impl Drop for HostTurn {
    fn drop(&mut self) {
        *self.last = Some(Instant::now());
    }
}

impl Gates {
    pub fn new(pace: Pace) -> Self {
        Self {
            pace,
            fetches: Arc::new(Semaphore::new(pace.fetches_at_once)),
            reads: Arc::new(Semaphore::new(pace.reads_at_once)),
            hosts: Mutex::default(),
        }
    }

    /// Waits for the host's turn, at most [`Pace::host_wait`]; `None` if it
    /// does not come.
    pub async fn host(&self, host: &str) -> Option<HostTurn> {
        let lock = {
            let mut hosts = self.hosts.lock().unwrap_or_else(PoisonError::into_inner);
            if hosts.len() >= MAX_HOSTS {
                hosts.retain(|_, lock| Arc::strong_count(lock) > 1);
            }
            hosts.entry(host.to_owned()).or_default().clone()
        };
        let spacing = self.pace.host_spacing;
        let wait = async move {
            let last = lock.lock_owned().await;
            if let Some(at) = *last {
                tokio::time::sleep_until((at + spacing).into()).await;
            }
            HostTurn { last }
        };
        tokio::time::timeout(self.pace.host_wait, wait).await.ok()
    }

    /// A place among the fetches under way.
    pub async fn fetch(&self) -> Option<OwnedSemaphorePermit> {
        self.fetches.clone().acquire_owned().await.ok()
    }

    /// A place among the pages being read.
    pub async fn read(&self) -> Option<OwnedSemaphorePermit> {
        self.reads.clone().acquire_owned().await.ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(limits: Limits) -> Token {
        Token {
            name: "test".into(),
            hash: String::new(),
            limits,
        }
    }

    #[test]
    fn a_limit_per_minute_lets_through_a_minute_at_once_and_then_one_at_its_pace() {
        let tokens = Arc::new(Tokens::default());
        let token = token(Limits {
            per_minute: Some(60),
            ..Limits::default()
        });
        let now = Instant::now();
        for _ in 0..60 {
            drop(tokens.request(&token, now).unwrap());
        }
        let Err(Over::Rate(wait)) = tokens.request(&token, now) else {
            panic!("the 61st request went through");
        };
        assert!(wait <= Duration::from_secs(1), "{wait:?}");
        assert!(tokens.request(&token, now + Duration::from_secs(1)).is_ok());
        assert_eq!(
            tokens.usage(&token, now + Duration::from_secs(1)).requests,
            60
        );
    }

    #[test]
    fn requests_at_once_end_with_their_turn() {
        let tokens = Arc::new(Tokens::default());
        let token = token(Limits {
            at_once: Some(1),
            ..Limits::default()
        });
        let now = Instant::now();
        let turn = tokens.request(&token, now).unwrap();
        assert_eq!(tokens.request(&token, now).err(), Some(Over::AtOnce));
        assert_eq!(tokens.usage(&token, now).under_way, 1);
        drop(turn);
        assert!(tokens.request(&token, now).is_ok());
    }

    #[test]
    fn fetches_per_hour_count_only_what_is_fetched() {
        let tokens = Arc::new(Tokens::default());
        let token = token(Limits {
            fetches_per_hour: Some(2),
            ..Limits::default()
        });
        let now = Instant::now();
        assert!(tokens.fetch(&token, now).is_ok());
        assert!(tokens.fetch(&token, now).is_ok());
        let Err(Over::Rate(wait)) = tokens.fetch(&token, now) else {
            panic!("the third fetch went through");
        };
        assert_eq!(wait, Duration::from_secs(30 * 60));
        // Requests are not limited by it.
        assert!(tokens.request(&token, now).is_ok());
    }

    #[tokio::test]
    async fn one_host_is_asked_one_at_a_time_and_not_too_often() {
        let gates = Gates::new(Pace {
            host_spacing: Duration::from_millis(200),
            host_wait: Duration::from_millis(100),
            ..Pace::default()
        });
        let first = gates.host("example.org").await.unwrap();
        // Another host does not wait.
        assert!(gates.host("example.com").await.is_some());
        assert!(gates.host("example.org").await.is_none());
        drop(first);
        // Within the spacing the host still is not asked.
        assert!(gates.host("example.org").await.is_none());
    }
}
