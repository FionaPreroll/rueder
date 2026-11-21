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
//!   everybody else;
//! - per account, what reads much at once but is reading all the same, such as
//!   a search or texts for an app: only a few of an account at a time
//!   ([`LONG_READS_PER_ACCOUNT`], see [`Rates::long_read`]), so that no
//!   account holds all connections, while those of different accounts never
//!   wait for each other.
//!
//! - mail the server sends ([`MAILS_PER_ACCOUNT`], [`MAILS_PER_ADDRESS`],
//!   [`MAILS_PER_DAY`]): per account, per address it goes to and for the whole
//!   server, so that nobody uses rueder to fill somebody's inbox, nor the
//!   relay's allowance. Links to set a new password, which anybody can ask
//!   for, have a share of the day of their own ([`RESET_MAILS_PER_DAY`]), so
//!   that asking for many does not use up the mail of everybody else, and so
//!   do the mails to accounts nobody uses ([`INACTIVE_MAILS_PER_DAY`]), which
//!   may be many at once when the operator turns that on.
//!
//! - registrations ([`REGISTRATIONS_PER_NETWORK`], [`OPEN_REGISTRATIONS`]):
//!   per network, before anything else is looked at, and those without an
//!   invitation for the whole server, so that a few networks cannot keep the
//!   server hashing passwords nor make many accounts in a short while.
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
    table::Table,
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
/// How many mails one account may have the server send: 5 at once, and one
/// more every hour.
pub const MAILS_PER_ACCOUNT: Rate = Rate::new(5, Duration::from_secs(60 * 60));
/// How many mails may go to one address: 3 at once, and one more every two
/// hours, whichever accounts ask for them.
pub const MAILS_PER_ADDRESS: Rate = Rate::new(3, Duration::from_secs(2 * 60 * 60));
/// How many mails the whole server sends: 1000 at once, and 1000 more a day.
/// `MAILS_PER_DAY` sets another number (see `sizes`).
pub const MAILS_PER_DAY: Rate = Rate::new(1000, Duration::from_millis(86_400));
/// How many of them may be links to set a new password: 300 at once, and 300
/// more a day. Anybody can ask for one for any name, so these have a share of
/// their own, and the rest of the day's mail stays for confirmations. With
/// another `MAILS_PER_DAY` the share is three in ten of it (see `sizes`).
pub const RESET_MAILS_PER_DAY: Rate = Rate::new(300, Duration::from_millis(288_000));
/// How many of them may be mails to accounts nobody uses (see `inactive`), in
/// the same way: three in ten of the day's.
pub const INACTIVE_MAILS_PER_DAY: Rate = Rate::new(300, Duration::from_millis(288_000));
/// How many registrations one network may try: 5 at once, and 5 more an hour.
/// A network is an IPv4 /24 or an IPv6 /48, as for wrong passwords, so that
/// many addresses of one network count once; a few tries leave room for a
/// name that is taken and for readers behind one address. It counts before
/// anything is looked at, the captcha included.
pub const REGISTRATIONS_PER_NETWORK: Rate = Rate::new(5, Duration::from_secs(12 * 60));
/// How many registrations without an invitation the whole server takes: 20
/// at once, and one more a minute. Only those with a captcha that counts are
/// counted, or all where the operator turned it off; and since each network
/// counts first, it takes many networks to use this up for everybody.
pub const OPEN_REGISTRATIONS: Rate = Rate::new(20, Duration::from_secs(60));
/// How many addresses and accounts each limit remembers at most: as many as
/// a server of 100,000 accounts has, all asking at once. Past that, one that
/// has (nearly) everything back makes room (see [`Table`]).
pub const MAX_KEYS: usize = 100_000;
/// How many costly requests run at once on the whole server, as rueder
/// comes: fewer than the database has connections, so that reading goes on
/// meanwhile. More wait. `HEAVY_AT_ONCE` sets another (see `sizes`).
pub const HEAVY_AT_ONCE: usize = 4;
/// How many costly requests one account may have under way, waiting or
/// running. More are refused.
pub const HEAVY_PER_ACCOUNT: usize = 2;
/// How many long reads one account may have under way. More are refused.
pub const LONG_READS_PER_ACCOUNT: usize = 2;

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
    used: Mutex<Table<K, Instant>>,
}

impl<K: Hash + Eq + Clone> Limiter<K> {
    pub fn new(rate: Rate) -> Self {
        Self::with_room(rate, MAX_KEYS)
    }

    fn with_room(rate: Rate, keys: usize) -> Self {
        Self {
            rate,
            used: Mutex::new(Table::new(keys)),
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
        // In a full table one that has everything back makes room, or else
        // the one closest to it of those looked at: one that is held back
        // stays as long as there are others beside it.
        *used.entry(key.clone(), || now, |_, back| *back) = now + ahead;
        Ok(())
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.used.lock().unwrap().len()
    }
}

/// How many requests of a kind each account has under way.
#[derive(Default)]
struct UnderWay(Mutex<HashMap<i32, usize>>);

impl UnderWay {
    fn counts(&self) -> std::sync::MutexGuard<'_, HashMap<i32, usize>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Counts one more for the account, or refuses it at `most`.
    fn take(&self, account_id: i32, most: usize) -> Result<Turn<'_>, AppError> {
        let mut counts = self.counts();
        let count = counts.entry(account_id).or_default();
        if *count >= most {
            let message = "too much at once for this account, try again in a moment";
            return Err(AppError::TooMany(Duration::from_secs(1), message));
        }
        *count += 1;
        Ok(Turn {
            under_way: self,
            account_id,
            _running: None,
        })
    }
}

/// Costly requests: how many run, and how many each account has under way.
struct Heavy {
    running: Semaphore,
    accounts: UnderWay,
}

/// A counted request's turn: while it is kept, the request counts.
pub struct Turn<'a> {
    under_way: &'a UnderWay,
    account_id: i32,
    _running: Option<SemaphorePermit<'a>>,
}

impl Drop for Turn<'_> {
    fn drop(&mut self) {
        let mut counts = self.under_way.counts();
        if let Some(count) = counts.get_mut(&self.account_id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                counts.remove(&self.account_id);
            }
        }
    }
}

/// What a server allows. The default is what rueder comes with; a test sets
/// another for what it looks at, as in `Limits { imports: …, ..Limits::default() }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub requests: Rate,
    pub lookups: Rate,
    pub new_feeds: Rate,
    pub reports: Rate,
    pub content_reports: Rate,
    pub imports: Rate,
    pub app_changes: Rate,
    pub mails_per_account: Rate,
    pub mails_per_address: Rate,
    pub mails_per_day: Rate,
    pub reset_mails_per_day: Rate,
    pub inactive_mails_per_day: Rate,
    pub registrations_per_network: Rate,
    pub open_registrations: Rate,
    /// How many costly requests run at once ([`Rates::heavy`]).
    pub heavy_at_once: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            requests: REQUESTS,
            lookups: LOOKUPS,
            new_feeds: NEW_FEEDS,
            reports: REPORTS,
            content_reports: CONTENT_REPORTS,
            imports: IMPORTS,
            app_changes: APP_CHANGES,
            mails_per_account: MAILS_PER_ACCOUNT,
            mails_per_address: MAILS_PER_ADDRESS,
            mails_per_day: MAILS_PER_DAY,
            reset_mails_per_day: RESET_MAILS_PER_DAY,
            inactive_mails_per_day: INACTIVE_MAILS_PER_DAY,
            registrations_per_network: REGISTRATIONS_PER_NETWORK,
            open_registrations: OPEN_REGISTRATIONS,
            heavy_at_once: HEAVY_AT_ONCE,
        }
    }
}

/// What each sender and each account has used of the [`Limits`] of a server.
pub struct Rates {
    requests: Limiter<IpAddr>,
    lookups: Limiter<i32>,
    new_feeds: Limiter<i32>,
    reports: Limiter<i32>,
    content_reports: Limiter<i32>,
    imports: Limiter<i32>,
    app_changes: Limiter<i32>,
    mails_per_account: Limiter<i32>,
    mails_per_address: Limiter<String>,
    mails_per_day: Limiter<()>,
    reset_mails_per_day: Limiter<()>,
    inactive_mails_per_day: Limiter<()>,
    registrations_per_network: Limiter<IpAddr>,
    open_registrations: Limiter<()>,
    heavy: Heavy,
    long_reads: UnderWay,
}

impl Rates {
    pub fn new(limits: Limits) -> Self {
        Self {
            requests: Limiter::new(limits.requests),
            lookups: Limiter::new(limits.lookups),
            new_feeds: Limiter::new(limits.new_feeds),
            reports: Limiter::new(limits.reports),
            content_reports: Limiter::new(limits.content_reports),
            imports: Limiter::new(limits.imports),
            app_changes: Limiter::new(limits.app_changes),
            mails_per_account: Limiter::new(limits.mails_per_account),
            mails_per_address: Limiter::new(limits.mails_per_address),
            mails_per_day: Limiter::new(limits.mails_per_day),
            reset_mails_per_day: Limiter::new(limits.reset_mails_per_day),
            inactive_mails_per_day: Limiter::new(limits.inactive_mails_per_day),
            registrations_per_network: Limiter::new(limits.registrations_per_network),
            open_registrations: Limiter::new(limits.open_registrations),
            heavy: Heavy {
                running: Semaphore::new(limits.heavy_at_once),
                accounts: UnderWay::default(),
            },
            long_reads: UnderWay::default(),
        }
    }

    /// Waits for a costly request's turn, or refuses it if the account has
    /// [`HEAVY_PER_ACCOUNT`] under way already. The request keeps the turn
    /// until it is done, and takes no connection of the database before.
    pub async fn heavy(&self, account_id: i32) -> Result<Turn<'_>, AppError> {
        let mut turn = self.heavy.accounts.take(account_id, HEAVY_PER_ACCOUNT)?;
        // Never closed, so the permit always comes.
        turn._running = self.heavy.running.acquire().await.ok();
        Ok(turn)
    }

    /// A long read's turn, or a refusal if the account has
    /// [`LONG_READS_PER_ACCOUNT`] under way already. Nothing waits: a long
    /// read of one account never holds up another's.
    pub fn long_read(&self, account_id: i32) -> Result<Turn<'_>, AppError> {
        self.long_reads.take(account_id, LONG_READS_PER_ACCOUNT)
    }

    /// Counts a move from another rueder into the account.
    pub fn import(&self, account_id: i32, now: Instant) -> Result<(), AppError> {
        self.imports
            .take(&account_id, now)
            .map_err(|wait| AppError::TooMany(wait, "too many imports, try again later"))
    }

    /// Counts `n` feeds nobody read before that the account brings in, all of
    /// them or none.
    pub fn new_feeds(&self, account_id: i32, n: usize, now: Instant) -> Result<(), AppError> {
        let n = u32::try_from(n).unwrap_or(u32::MAX);
        self.new_feeds
            .take_many(&account_id, n, now)
            .map_err(|wait| AppError::TooMany(wait, "too many new feeds at once, try again later"))
    }

    /// Counts a report of a feed or an article the account sends.
    pub fn content_report(&self, account_id: i32, now: Instant) -> Result<(), AppError> {
        self.content_reports
            .take(&account_id, now)
            .map_err(|wait| AppError::TooMany(wait, "too many reports, try again later"))
    }

    /// Counts a change an app of another maker sends for the account.
    pub fn app_change(&self, account_id: i32, now: Instant) -> Result<(), AppError> {
        self.app_changes
            .take(&account_id, now)
            .map_err(|wait| AppError::TooMany(wait, "too many changes, try again later"))
    }

    /// Counts a mail the server is to send for the account to the address:
    /// against the account, the address and the day. Whichever is over first
    /// says how long to wait.
    pub fn mail(&self, account_id: i32, address: &str, now: Instant) -> Result<(), AppError> {
        self.mails(account_id, address, None, now)
    }

    /// Counts a link to set a new password like any mail, and against the
    /// share of the day such links have, before the day of all mail: whoever
    /// asks for many uses up that share, not everybody's confirmations.
    pub fn reset_mail(&self, account_id: i32, address: &str, now: Instant) -> Result<(), AppError> {
        self.mails(account_id, address, Some(&self.reset_mails_per_day), now)
    }

    /// Counts a mail to an account nobody uses like any mail, and against the
    /// share of the day such mails have.
    pub fn inactive_mail(
        &self,
        account_id: i32,
        address: &str,
        now: Instant,
    ) -> Result<(), AppError> {
        self.mails(account_id, address, Some(&self.inactive_mails_per_day), now)
    }

    /// Counts a mail against the account, the address, a share of the day if
    /// it has one, and the day. Whichever is over first says how long to wait.
    fn mails(
        &self,
        account_id: i32,
        address: &str,
        share: Option<&Limiter<()>>,
        now: Instant,
    ) -> Result<(), AppError> {
        let address = address.to_lowercase();
        let message = "too many mails, try again later";
        self.mails_per_account
            .take(&account_id, now)
            .and_then(|()| self.mails_per_address.take(&address, now))
            .and_then(|()| share.map_or(Ok(()), |share| share.take(&(), now)))
            .and_then(|()| self.mails_per_day.take(&(), now))
            .map_err(|wait| AppError::TooMany(wait, message))
    }

    /// Counts a try to register from the sender's network.
    pub fn registration(&self, ip: IpAddr, now: Instant) -> Result<(), AppError> {
        self.registrations_per_network
            .take(&login::wide(ip), now)
            .map_err(|wait| {
                AppError::TooMany(wait, "too many registrations from here, try again later")
            })
    }

    /// Counts a registration without an invitation against the whole server.
    pub fn open_registration(&self, now: Instant) -> Result<(), AppError> {
        self.open_registrations
            .take(&(), now)
            .map_err(|wait| AppError::TooMany(wait, "too many registrations, try again later"))
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
        Self::new(Limits::default())
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
    fn what_is_remembered_has_a_limit_that_holds() {
        let limiter = Limiter::with_room(Rate::new(2, SECOND), 100);
        let start = Instant::now();
        // The first has used up everything, the others one each.
        assert_eq!(limiter.take(&0, start), Ok(()));
        assert_eq!(limiter.take(&0, start), Ok(()));
        for key in 1..100 {
            assert_eq!(limiter.take(&key, start), Ok(()));
        }
        // A hundred times as many new ones as there is room for: one with
        // more left makes room each time, and the first stays held back.
        for key in 100..10_000 {
            assert_eq!(limiter.take(&key, start), Ok(()));
            assert_eq!(limiter.len(), 100);
        }
        assert_eq!(limiter.take(&0, start), Err(SECOND));
        // Once everything is back, new ones take the places of the old.
        let later = start + SECOND * 5;
        for key in 10_000..10_200 {
            assert_eq!(limiter.take(&key, later), Ok(()));
        }
        assert_eq!(limiter.len(), 100);
        assert_eq!(limiter.take(&0, later), Ok(()));
    }

    #[test]
    fn an_ipv6_sender_is_its_network() {
        let rates = Rates::new(Limits {
            requests: Rate::new(1, SECOND),
            ..Limits::default()
        });
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
        assert!(rates.heavy.accounts.counts().len() <= 1);
    }

    #[test]
    fn an_account_has_only_a_few_long_reads_at_once() {
        let rates = Rates::default();
        let ada: Vec<_> = (0..LONG_READS_PER_ACCOUNT)
            .map(|_| rates.long_read(1).ok().unwrap())
            .collect();
        assert!(matches!(rates.long_read(1), Err(AppError::TooMany(..))));
        // Others are not held up, and a costly request is counted apart.
        assert!(rates.long_read(2).is_ok());
        assert_eq!(rates.heavy.accounts.counts().get(&1), None);
        drop(ada);
        assert!(rates.long_read(1).is_ok());
        assert!(rates.long_reads.counts().is_empty());
    }

    #[tokio::test]
    async fn the_server_runs_as_many_costly_requests_as_it_is_set_to() {
        let rates = Rates::new(Limits {
            heavy_at_once: 5,
            ..Limits::default()
        });
        let mut turns = Vec::new();
        for account in 1..=5 {
            turns.push(rates.heavy(account).await.ok().unwrap());
        }
        let sixth = tokio::time::timeout(Duration::from_millis(50), rates.heavy(6)).await;
        assert!(sixth.is_err());
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
