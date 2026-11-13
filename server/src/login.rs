//! Logging in and out over the API, and the limit on wrong passwords.
//!
//! `POST /api/login` takes a name and a password and answers with the session
//! cookie, or, for an account the operator disabled, with `403 disabled`;
//! `POST /api/logout` ends the session; `GET /api/me` says who is logged in. Wrong tries are counted per name, per address and per network
//! around it, so neither one account nor one sender can be tried without end.

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
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
    legal::Legal,
    registration, second_factor,
    sessions::{self, Session},
};

/// How long wrong tries count.
pub const WINDOW: Duration = Duration::from_secs(15 * 60);
/// How many wrong tries a name and an address have within [`WINDOW`]. An
/// address has more: several readers may share one.
pub const PER_NAME: u32 = 10;
pub const PER_ADDRESS: u32 = 30;
/// How many wrong tries a whole network has within [`WINDOW`], whichever of its
/// addresses they come from: an IPv4 /24, or an IPv6 /48, which is what one
/// site gets and holds 65,536 networks of a /64 each.
pub const PER_NETWORK: u32 = 100;
/// How many names and addresses are remembered at most.
const MAX_KEYS: usize = 10_000;

/// What wrong tries are counted for.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Name(String),
    Address(IpAddr),
    Network(IpAddr),
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
        Self::Address(network(ip))
    }

    /// The wider network around a sender: who has a whole IPv6 /48 or many
    /// addresses of one IPv4 /24 counts there once more.
    pub fn network(ip: IpAddr) -> Self {
        Self::Network(match ip.to_canonical() {
            IpAddr::V6(v6) => IpAddr::V6(Ipv6Addr::from(u128::from(v6) & !(u128::MAX >> 48))),
            IpAddr::V4(v4) => IpAddr::V4(Ipv4Addr::from(u32::from(v4) & 0xffff_ff00)),
        })
    }

    /// What a try from this sender for this name counts for.
    pub fn of(name: &str, ip: IpAddr) -> [Self; 3] {
        [Self::name(name), Self::address(ip), Self::network(ip)]
    }

    fn limit(&self) -> u32 {
        match self {
            Self::Name(_) => PER_NAME,
            Self::Address(_) => PER_ADDRESS,
            Self::Network(_) => PER_NETWORK,
        }
    }
}

/// What counts as one sender: an IPv4 address, or the /64 of an IPv6 address.
pub fn network(ip: IpAddr) -> IpAddr {
    match ip.to_canonical() {
        IpAddr::V6(v6) => {
            let net = u128::from(v6) & !u128::from(u64::MAX);
            IpAddr::V6(Ipv6Addr::from(net))
        }
        v4 => v4,
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
        wait(&tries, key, now)
    }

    /// Counts a wrong try.
    pub fn failed(&self, key: Key, now: Instant) {
        let mut tries = self.tries.lock().unwrap_or_else(PoisonError::into_inner);
        count(&mut tries, key, now);
    }

    /// Counts a try before its password is checked, for all its keys at once,
    /// or says how long the longest-waiting of them has to wait. Counted
    /// beforehand, tries sent all at once cannot pass the check together while
    /// none of them is counted yet; [`Limiter::succeeded`] takes the count back
    /// for a try that was right.
    pub fn attempt(&self, keys: &[Key], now: Instant) -> Result<(), Duration> {
        let mut tries = self.tries.lock().unwrap_or_else(PoisonError::into_inner);
        let wait = keys.iter().filter_map(|key| wait(&tries, key, now)).max();
        if let Some(wait) = wait {
            return Err(wait);
        }
        for key in keys {
            count(&mut tries, key.clone(), now);
        }
        Ok(())
    }

    /// After a try of [`Limiter::attempt`] that was right: the name's wrong
    /// tries are forgotten, and the address and its network get back the one
    /// they were charged.
    pub fn succeeded(&self, keys: &[Key]) {
        let mut tries = self.tries.lock().unwrap_or_else(PoisonError::into_inner);
        for key in keys {
            match key {
                Key::Name(_) => {
                    tries.remove(key);
                }
                Key::Address(_) | Key::Network(_) => {
                    if let Some(entry) = tries.get_mut(key) {
                        entry.wrong = entry.wrong.saturating_sub(1);
                    }
                }
            }
        }
    }

    /// Forgets the wrong tries of a key, after a login that worked.
    pub fn clear(&self, key: &Key) {
        self.tries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(key);
    }
}

/// How long the key has to wait, if it has used up its tries.
fn wait(tries: &HashMap<Key, Tries>, key: &Key, now: Instant) -> Option<Duration> {
    let tries = tries.get(key)?;
    let left = WINDOW.checked_sub(now.saturating_duration_since(tries.since))?;
    (tries.wrong >= key.limit() && !left.is_zero()).then_some(left)
}

/// Counts a try for the key.
fn count(tries: &mut HashMap<Key, Tries>, key: Key, now: Instant) {
    if tries.len() >= MAX_KEYS && !tries.contains_key(&key) {
        tries.retain(|_, tries| now.saturating_duration_since(tries.since) < WINDOW);
        // Still full: the one with the fewest tries makes room, of those the
        // one that has waited longest. A key that is held back stays held
        // back, so that a flood of other names cannot set it free.
        if tries.len() >= MAX_KEYS
            && let Some(least) = tries
                .iter()
                .min_by_key(|(key, tries)| (tries.wrong >= key.limit(), tries.wrong, tries.since))
                .map(|(key, _)| key.clone())
        {
            tries.remove(&least);
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
/// cookie. An account with a second factor needs `code` too, from the app or
/// a recovery code; without it, the answer is 401 with `second factor needed`.
#[derive(Deserialize, TS)]
pub struct Credentials {
    username: String,
    password: String,
    #[serde(default)]
    #[ts(optional)]
    code: Option<String>,
}

/// What the answer to a login says when the account needs a code.
pub const CODE_NEEDED: &str = "second factor needed";

/// Who is logged in, as the API says it.
///
/// `GET /api/me` and `POST /api/login` answer with it.
#[derive(Serialize, TS)]
pub struct Me {
    id: i32,
    username: String,
    is_admin: bool,
    /// Whether the account has a password, which deleting it asks for. One
    /// that logs in only through OpenID Connect has none.
    has_password: bool,
    /// An admin whose right waits for a login with a second factor, where the
    /// server wants one of admins.
    admin_needs_second_factor: bool,
    /// The version of the terms of use the account has yet to agree to, where
    /// the server names one; the page asks before anything else.
    terms_to_accept: Option<String>,
    /// Whether the account may report feeds and articles on this server.
    content_reports: bool,
    /// Whether apps of other makers may log in to this server with app
    /// passwords.
    app_api: bool,
    /// Whether this server has passkeys (see `PUBLIC_URL`).
    passkeys: bool,
}

impl Me {
    pub(crate) fn new(id: i32, username: String, is_admin: bool, has_password: bool) -> Self {
        Self {
            id,
            username,
            is_admin,
            has_password,
            admin_needs_second_factor: false,
            terms_to_accept: None,
            content_reports: false,
            app_api: false,
            passkeys: false,
        }
    }

    /// Who the session is, whether its account has a password, and the terms
    /// it has yet to agree to.
    pub(crate) fn of(session: Session, has_password: bool, legal: &Legal) -> Self {
        Self {
            id: session.account_id,
            username: session.username,
            is_admin: session.is_admin,
            has_password,
            admin_needs_second_factor: session.admin_needs_second_factor,
            terms_to_accept: legal.to_accept(session.terms_version.as_deref()),
            content_reports: false,
            app_api: false,
            passkeys: false,
        }
    }

    /// With whether the server takes reports of feeds and articles.
    pub(crate) fn reporting(self, content_reports: bool) -> Self {
        Self {
            content_reports,
            ..self
        }
    }

    /// Says what this server lets in besides the page: reports of feeds and
    /// articles, apps of other makers, and passkeys.
    pub(crate) fn offering(self, state: &crate::AppState) -> Self {
        Self {
            app_api: state.app_api,
            passkeys: state.passkeys.is_some(),
            ..self.reporting(state.content_reports)
        }
    }
}

/// The ways to log in this server has besides name and password.
///
/// `GET /api/login/ways` answers with it, to anybody.
#[derive(Serialize, TS)]
#[ts(rename = "LoginWays")]
pub struct Ways {
    /// Through the OpenID Connect provider, at `/api/oidc/start`.
    oidc: bool,
    /// Anybody can make an account for themselves, at `/api/register`.
    registration: bool,
    /// With a passkey, at `/api/login/passkey`.
    passkeys: bool,
    /// The operator's legal texts, for the login page to link to.
    legal: Legal,
}

/// What the login page asks before anybody is logged in, to know whether to
/// offer the way through the provider. It says nothing about any account.
pub async fn ways(State(state): State<AppState>) -> Result<Json<Ways>, AppError> {
    Ok(Json(Ways {
        oidc: state.oidc.is_some(),
        registration: registration::is_open(&state.db).await?,
        passkeys: state.passkeys.is_some(),
        legal: (*state.legal).clone(),
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Sender(ip): Sender,
    headers: HeaderMap,
    Json(credentials): Json<Credentials>,
) -> Result<Response, AppError> {
    let keys = Key::of(&credentials.username, ip);
    // Every try counts as wrong until it turns out right.
    if let Err(wait) = state.logins.attempt(&keys, Instant::now()) {
        let message = "too many wrong tries, try again later";
        return Err(AppError::TooMany(wait, message));
    }

    let account = accounts::login(&state.db, &credentials.username, &credentials.password).await?;
    let Some(account) = account else {
        // The same answer whether the name or the password was wrong.
        return Err(AppError::Status(
            StatusCode::UNAUTHORIZED,
            "wrong name or password".into(),
        ));
    };
    // Said only to whoever knows the password, so it tells nobody else which
    // names exist. No code is asked for: the account cannot log in anyway.
    if account.disabled {
        state.logins.succeeded(&keys);
        return Err(AppError::Status(StatusCode::FORBIDDEN, "disabled".into()));
    }
    let second_factor = second_factor::is_on(&state.db, account.id).await?;
    if second_factor {
        let Some(code) = credentials
            .code
            .as_deref()
            .filter(|code| !code.trim().is_empty())
        else {
            // The password was right; the code is what is missing.
            state.logins.succeeded(&keys);
            return Err(AppError::Status(
                StatusCode::UNAUTHORIZED,
                CODE_NEEDED.into(),
            ));
        };
        // A wrong code stays counted, like a wrong password.
        if !second_factor::take(&state.db, account.id, code).await? {
            let message = "wrong code";
            return Err(AppError::Status(StatusCode::UNAUTHORIZED, message.into()));
        }
    }
    state.logins.succeeded(&keys);
    start_session(&state, &headers, account.id, second_factor).await
}

/// Starts a session for an account that just logged in, and answers with who
/// it is and the session's cookie. A session the browser still had ends here,
/// so a login always starts a fresh one.
pub(crate) async fn start_session(
    state: &AppState,
    headers: &HeaderMap,
    account_id: i32,
    second_factor: bool,
) -> Result<Response, AppError> {
    if let Some(old) = sessions::token(headers) {
        sessions::end(&state.db, old).await?;
    }
    let token = sessions::start(&state.db, account_id, second_factor).await?;
    let account = sqlx::query!(
        r#"SELECT username, is_admin, terms_version, password_hash IS NOT NULL AS "has_password!"
           FROM accounts WHERE id = $1"#,
        account_id
    )
    .fetch_one(&state.db)
    .await?;
    let session = Session {
        account_id,
        username: account.username,
        is_admin: account.is_admin,
        second_factor,
        admin_needs_second_factor: false,
        terms_version: account.terms_version,
    };
    let me = Me::of(
        session.for_admins(state.admin_second_factor),
        account.has_password,
        &state.legal,
    )
    .offering(state);
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

pub async fn me(State(state): State<AppState>, session: Session) -> Result<Json<Me>, AppError> {
    let has_password = sqlx::query_scalar!(
        r#"SELECT password_hash IS NOT NULL AS "has!" FROM accounts WHERE id = $1"#,
        session.account_id
    )
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(false);
    Ok(Json(
        Me::of(session, has_password, &state.legal).offering(&state),
    ))
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
    fn tries_count_before_they_are_checked() {
        let limiter = Limiter::default();
        let now = Instant::now();
        let ip = Key::address("192.0.2.7".parse().unwrap());
        let keys = [Key::name("ada"), ip.clone()];
        // Tries that are all under way at once: only as many pass as the
        // name has, however many are sent.
        let passed = (0..PER_NAME * 3)
            .filter(|_| limiter.attempt(&keys, now).is_ok())
            .count();
        assert_eq!(passed, PER_NAME as usize);
        assert_eq!(limiter.attempt(&keys, now), Err(WINDOW));
        // Another name from the same address still has tries.
        let grace = [Key::name("grace"), ip.clone()];
        assert_eq!(limiter.attempt(&grace, now), Ok(()));
        // One that was right forgets the name's and gives the address its
        // own back, and no more.
        limiter.succeeded(&grace);
        assert_eq!(limiter.blocked(&Key::name("grace"), now), None);
        assert_eq!(
            limiter.tries.lock().unwrap()[&ip].wrong,
            PER_NAME,
            "the address keeps the wrong ones"
        );
        limiter.succeeded(&keys);
        assert_eq!(limiter.attempt(&keys, now), Ok(()));
        // The address runs out as well, whichever names it tries.
        let tried = (0..PER_ADDRESS * 2)
            .filter(|n| {
                let keys = [Key::name(&format!("n{n}")), ip.clone()];
                limiter.attempt(&keys, now).is_ok()
            })
            .count();
        assert_eq!(tried, (PER_ADDRESS - PER_NAME) as usize);
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
    fn a_network_counts_all_its_addresses() {
        let network = |ip: &str| Key::network(ip.parse().unwrap());
        assert_eq!(network("2001:db8:1:2::1"), network("2001:db8:1:ffff::1"));
        assert_ne!(network("2001:db8:1::1"), network("2001:db8:2::1"));
        assert_eq!(network("192.0.2.7"), network("192.0.2.200"));
        assert_eq!(network("::ffff:192.0.2.7"), network("192.0.2.1"));
        assert_ne!(network("192.0.2.7"), network("192.0.3.7"));

        // Every try from another /64 of the same /48, for another name: no
        // name and no address is used up, the network is.
        let limiter = Limiter::default();
        let now = Instant::now();
        let from = |n: u32| -> IpAddr { format!("2001:db8:7:{n:x}::1").parse().unwrap() };
        for n in 0..PER_NETWORK {
            let keys = Key::of(&format!("n{n}"), from(n));
            assert_eq!(limiter.attempt(&keys, now), Ok(()), "try {n}");
        }
        let keys = Key::of("ada", from(PER_NETWORK));
        assert!(limiter.attempt(&keys, now).is_err());
        // Another network is not held back.
        let elsewhere: IpAddr = "2001:db8:8::1".parse().unwrap();
        assert_eq!(limiter.attempt(&Key::of("ada", elsewhere), now), Ok(()));
        // A right password gives the network its try back.
        limiter.succeeded(&Key::of("ada", elsewhere));
        assert_eq!(limiter.blocked(&Key::network(elsewhere), now), None);
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
        let has = |key: &Key| limiter.tries.lock().unwrap().contains_key(key);
        assert_eq!(count(), MAX_KEYS);
        assert!(limiter.blocked(&name(0), start).is_some());
        // One more: of those with the fewest tries the oldest goes, and the
        // name that is held back stays held back.
        limiter.failed(name(MAX_KEYS), start + Duration::from_secs(2));
        assert_eq!(count(), MAX_KEYS);
        assert!(limiter.blocked(&name(0), start).is_some());
        assert!(has(&name(MAX_KEYS)));
        assert_eq!((1..MAX_KEYS).filter(|n| !has(&name(*n))).count(), 1);
        // Once their time is over, all of them go.
        limiter.failed(name(MAX_KEYS + 1), start + WINDOW + Duration::from_secs(2));
        assert_eq!(count(), 1);
    }
}
