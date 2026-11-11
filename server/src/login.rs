//! Logging in and out over the API, and the limit on wrong passwords.
//!
//! `POST /api/login` takes a name and a password and answers with the session
//! cookie; `POST /api/logout` ends the session; `GET /api/me` says who is
//! logged in. Wrong tries are counted per name and per address, so neither one
//! account nor one sender can be tried without end.

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::{Mutex, PoisonError},
    time::{Duration, Instant},
};

use axum::{
    Json,
    extract::{ConnectInfo, FromRequestParts, State},
    http::{HeaderMap, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    AppError, AppState, accounts,
    sessions::{self, Session},
};

/// How long wrong tries count.
pub const WINDOW: Duration = Duration::from_secs(15 * 60);
/// How many wrong tries a name and an address have within [`WINDOW`]. An
/// address has more: several readers may share one.
pub const PER_NAME: u32 = 10;
pub const PER_ADDRESS: u32 = 30;
/// How many names and addresses are remembered at most.
const MAX_KEYS: usize = 10_000;

/// What wrong tries are counted for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Name(String),
    Address(IpAddr),
}

impl Key {
    /// Names differ neither by case nor by blanks around them, as at login.
    pub fn name(name: &str) -> Self {
        let name = name.trim().to_lowercase();
        Self::Name(name.chars().take(accounts::MAX_NAME).collect())
    }

    /// An IPv6 sender has a whole /64 to pick addresses from, so that is what
    /// counts as one.
    pub fn address(ip: IpAddr) -> Self {
        Self::Address(match ip.to_canonical() {
            IpAddr::V6(v6) => {
                let net = u128::from(v6) & !u128::from(u64::MAX);
                IpAddr::V6(Ipv6Addr::from(net))
            }
            v4 => v4,
        })
    }

    fn limit(&self) -> u32 {
        match self {
            Self::Name(_) => PER_NAME,
            Self::Address(_) => PER_ADDRESS,
        }
    }
}

struct Tries {
    since: Instant,
    wrong: u32,
}

/// Counts wrong tries. It lives in memory: a restart forgets them, which costs
/// an attacker more than it gains.
#[derive(Default)]
pub struct Limiter {
    tries: Mutex<HashMap<Key, Tries>>,
}

impl Limiter {
    /// How long the key has to wait, if it has used up its tries.
    pub fn blocked(&self, key: &Key, now: Instant) -> Option<Duration> {
        let tries = self.tries.lock().unwrap_or_else(PoisonError::into_inner);
        let tries = tries.get(key)?;
        let left = WINDOW.checked_sub(now.saturating_duration_since(tries.since))?;
        (tries.wrong >= key.limit() && !left.is_zero()).then_some(left)
    }

    /// Counts a wrong try.
    pub fn failed(&self, key: Key, now: Instant) {
        let mut tries = self.tries.lock().unwrap_or_else(PoisonError::into_inner);
        if tries.len() >= MAX_KEYS && !tries.contains_key(&key) {
            tries.retain(|_, tries| now.saturating_duration_since(tries.since) < WINDOW);
            // Still full: the one that has waited longest makes room.
            if tries.len() >= MAX_KEYS
                && let Some(oldest) = tries
                    .iter()
                    .min_by_key(|(_, tries)| tries.since)
                    .map(|(key, _)| key.clone())
            {
                tries.remove(&oldest);
            }
        }
        let entry = tries.entry(key).or_insert(Tries {
            since: now,
            wrong: 0,
        });
        if now.saturating_duration_since(entry.since) >= WINDOW {
            *entry = Tries {
                since: now,
                wrong: 0,
            };
        }
        entry.wrong += 1;
    }

    /// Forgets the wrong tries of a key, after a login that worked.
    pub fn clear(&self, key: &Key) {
        self.tries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(key);
    }
}

/// The address a request comes from: the one the connection has, or, behind a
/// reverse proxy, the one the proxy names in the header the operator set.
pub struct Sender(pub IpAddr);

impl FromRequestParts<AppState> for Sender {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        // A proxy appends what it saw, so the last entry is the one it vouches
        // for; anything before it is what the sender claimed.
        let named = state.client_ip_header.as_ref().and_then(|name| {
            let value = parts.headers.get_all(name).iter().next_back()?;
            value.to_str().ok()?.rsplit(',').next()?.trim().parse().ok()
        });
        let connected = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(addr)| addr.ip());
        // Without either, all senders count as one.
        Ok(Self(
            named
                .or(connected)
                .unwrap_or(IpAddr::V6(Ipv6Addr::UNSPECIFIED)),
        ))
    }
}

/// The types this module sends and reads, for the frontend.
pub(crate) fn types(types: &mut crate::typescript::Types) {
    types.add::<Credentials>().add::<Me>().add::<Ways>();
}

/// What `POST /api/login` reads. It answers with `Me` and sets the session's
/// cookie.
#[derive(Deserialize, TS)]
pub struct Credentials {
    username: String,
    password: String,
}

/// Who is logged in, as the API says it.
///
/// `GET /api/me` and `POST /api/login` answer with it.
#[derive(Serialize, TS)]
pub struct Me {
    id: i32,
    username: String,
    is_admin: bool,
}

/// The ways to log in this server has besides name and password.
///
/// `GET /api/login/ways` answers with it, to anybody.
#[derive(Serialize, TS)]
#[ts(rename = "LoginWays")]
pub struct Ways {
    /// Through the OpenID Connect provider, at `/api/oidc/start`.
    oidc: bool,
}

/// What the login page asks before anybody is logged in, to know whether to
/// offer the way through the provider. It says nothing about any account.
pub async fn ways(State(state): State<AppState>) -> Json<Ways> {
    Json(Ways {
        oidc: state.oidc.is_some(),
    })
}

pub async fn login(
    State(state): State<AppState>,
    Sender(ip): Sender,
    headers: HeaderMap,
    Json(credentials): Json<Credentials>,
) -> Result<Response, AppError> {
    let now = Instant::now();
    let keys = [Key::name(&credentials.username), Key::address(ip)];
    let wait = keys
        .iter()
        .filter_map(|key| state.logins.blocked(key, now))
        .max();
    if let Some(wait) = wait {
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, wait.as_secs().max(1).to_string())],
            "too many wrong tries, try again later",
        )
            .into_response());
    }

    let account = accounts::login(&state.db, &credentials.username, &credentials.password).await?;
    let Some(account) = account else {
        for key in keys {
            state.logins.failed(key, now);
        }
        // The same answer whether the name or the password was wrong.
        return Err(AppError::Status(
            StatusCode::UNAUTHORIZED,
            "wrong name or password".into(),
        ));
    };
    state.logins.clear(&keys[0]);

    // A session the browser still had ends here, so a login always starts a
    // fresh one.
    if let Some(old) = sessions::token(&headers) {
        sessions::end(&state.db, old).await?;
    }
    let token = sessions::start(&state.db, account.id).await?;
    let me = Me {
        id: account.id,
        username: account.username,
        is_admin: account.is_admin,
    };
    Ok(([(header::SET_COOKIE, sessions::cookie(&token))], Json(me)).into_response())
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if let Some(token) = sessions::token(&headers) {
        sessions::end(&state.db, token).await?;
    }
    Ok((
        StatusCode::NO_CONTENT,
        [(header::SET_COOKIE, sessions::no_cookie())],
    )
        .into_response())
}

pub async fn me(session: Session) -> Json<Me> {
    Json(Me {
        id: session.account_id,
        username: session.username,
        is_admin: session.is_admin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(n: usize) -> Key {
        Key::name(&format!("name-{n}"))
    }

    #[test]
    fn tries_are_used_up_and_come_back() {
        let limiter = Limiter::default();
        let start = Instant::now();
        let ada = Key::name("Ada ");
        assert_eq!(ada, Key::name("ada"));
        for n in 0..PER_NAME {
            assert_eq!(limiter.blocked(&ada, start), None, "{n}");
            limiter.failed(ada.clone(), start + Duration::from_secs(u64::from(n)));
        }
        assert_eq!(limiter.blocked(&ada, start), Some(WINDOW));
        let later = start + WINDOW - Duration::from_secs(1);
        assert_eq!(limiter.blocked(&ada, later), Some(Duration::from_secs(1)));
        assert_eq!(limiter.blocked(&name(1), later), None);
        // When the time is over, counting starts again.
        assert_eq!(limiter.blocked(&ada, start + WINDOW), None);
        limiter.failed(ada.clone(), start + WINDOW);
        assert_eq!(limiter.blocked(&ada, start + WINDOW), None);

        for _ in 0..PER_NAME {
            limiter.failed(name(2), start);
        }
        assert!(limiter.blocked(&name(2), start).is_some());
        limiter.clear(&name(2));
        assert_eq!(limiter.blocked(&name(2), start), None);
    }

    #[test]
    fn an_address_is_its_network() {
        let key = |ip: &str| Key::address(ip.parse().unwrap());
        assert_eq!(
            key("2001:db8:1:2::1"),
            key("2001:db8:1:2:ffff:ffff:ffff:ffff")
        );
        assert_ne!(key("2001:db8:1:2::1"), key("2001:db8:1:3::1"));
        assert_eq!(key("::ffff:192.0.2.7"), key("192.0.2.7"));
        assert_ne!(key("192.0.2.7"), key("192.0.2.8"));

        let limiter = Limiter::default();
        let now = Instant::now();
        for n in 0..PER_ADDRESS {
            limiter.failed(key(&format!("2001:db8::{n:x}")), now);
        }
        assert!(limiter.blocked(&key("2001:db8::abcd"), now).is_some());
        assert_eq!(limiter.blocked(&key("2001:db8:0:1::"), now), None);
    }

    #[test]
    fn what_is_remembered_has_a_limit() {
        let limiter = Limiter::default();
        let start = Instant::now();
        for _ in 0..PER_NAME {
            limiter.failed(name(0), start);
        }
        for n in 1..MAX_KEYS {
            limiter.failed(name(n), start + Duration::from_secs(1));
        }
        let count = || limiter.tries.lock().unwrap().len();
        assert_eq!(count(), MAX_KEYS);
        assert!(limiter.blocked(&name(0), start).is_some());
        // One more: the oldest goes.
        limiter.failed(name(MAX_KEYS), start + Duration::from_secs(2));
        assert_eq!(count(), MAX_KEYS);
        assert_eq!(limiter.blocked(&name(0), start), None);
        // Once their time is over, all of them go.
        limiter.failed(name(MAX_KEYS + 1), start + WINDOW + Duration::from_secs(2));
        assert_eq!(count(), 1);
    }
}
