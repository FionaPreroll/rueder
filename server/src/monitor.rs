//! What the server notices about itself while it runs: how long requests take,
//! how fetching, the feed search and the image proxy fare, and the warnings and
//! errors it logs. All of it lives in memory, for the last hour, and is gone
//! with a restart; the status page of the admins shows it ([`crate::status`]).
//!
//! Nothing here keeps an address, a name or a text of anybody's: requests are
//! counted by the route they took, not by the address they asked for, and the
//! problems are what the server itself wrote to its log.

use std::{
    collections::{HashMap, VecDeque},
    fmt::{self, Write as _},
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{MatchedPath, Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use time::OffsetDateTime;
use tower_http::classify::{
    ClassifiedResponse, ClassifyResponse, NeverClassifyEos, ServerErrorsAsFailures,
    ServerErrorsFailureClass,
};
use tracing::{
    Event, Level, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::layer::{Context, Layer};

use crate::{AppState, fetch::Pace, images::SourceFailed};

/// How long one part of the hour is.
const SLOT: Duration = Duration::from_secs(10 * 60);
/// How many parts the window has: an hour.
const SLOTS: usize = 6;
/// How many routes are counted apart. Routes are what the code defines, so
/// there are far fewer; the rest would count as one.
const MAX_ROUTES: usize = 200;
/// How many different problems are kept.
const MAX_PROBLEMS: usize = 100;
/// The longest text of a problem, in characters.
const MAX_PROBLEM_TEXT: usize = 1000;

/// When the parts of the hour are counted from: the first time anybody asked.
static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

/// The number of the part of the hour `now` falls in.
fn slot(now: Instant) -> u64 {
    now.saturating_duration_since(*EPOCH).as_secs() / SLOT.as_secs()
}

/// What can be counted in a window: two counts make one.
pub trait Tally: Default + Clone {
    fn add(&mut self, other: &Self);
}

/// Counts for the last hour, in parts of ten minutes. A part that is older
/// than an hour is used again for the part of now. Next to them, all that was
/// counted since the process started, for Prometheus ([`crate::metrics`]).
#[derive(Clone)]
pub struct Window<T> {
    slots: [(u64, T); SLOTS],
    /// All that was counted since the start: it begins at 0 with every
    /// process, as Prometheus expects of a counter.
    total: T,
}

impl<T: Tally> Default for Window<T> {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| (0, T::default())),
            total: T::default(),
        }
    }
}

impl<T: Tally> Window<T> {
    /// Counts in the part `now` falls in and in the total since the start.
    pub fn count(&mut self, now: Instant, count: impl Fn(&mut T)) {
        count(self.at(now));
        count(&mut self.total);
    }

    /// All that was counted since the start.
    pub fn total(&self) -> T {
        self.total.clone()
    }

    /// The count of the part `now` falls in.
    fn at(&mut self, now: Instant) -> &mut T {
        let current = slot(now);
        let (number, tally) = &mut self.slots[(current % SLOTS as u64) as usize];
        if *number != current {
            *number = current;
            *tally = T::default();
        }
        tally
    }

    /// All that was counted in the hour before `now`.
    pub fn last_hour(&self, now: Instant) -> T {
        let current = slot(now);
        let oldest = current.saturating_sub(SLOTS as u64 - 1);
        let mut sum = T::default();
        for (number, tally) in &self.slots {
            if (oldest..=current).contains(number) {
                sum.add(tally);
            }
        }
        sum
    }
}

/// How many buckets a [`Timing`] has. They begin at 0.1 ms and grow by a
/// third of an octave, so the last one starts at about 80 seconds.
const BUCKETS: usize = 60;

/// The upper end of a bucket, in microseconds.
fn bound(bucket: usize) -> f64 {
    100.0 * 2_f64.powf(bucket as f64 / 3.0)
}

/// How long things took: how many, the sum, the longest, and how many fell in
/// each bucket, which is enough to tell the median and the 95th percentile to
/// within a quarter.
#[derive(Clone, Debug, PartialEq)]
pub struct Timing {
    pub count: u64,
    total_us: u64,
    max_us: u64,
    buckets: [u64; BUCKETS],
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            count: 0,
            total_us: 0,
            max_us: 0,
            buckets: [0; BUCKETS],
        }
    }
}

impl Tally for Timing {
    fn add(&mut self, other: &Self) {
        self.count += other.count;
        self.total_us = self.total_us.saturating_add(other.total_us);
        self.max_us = self.max_us.max(other.max_us);
        for (mine, theirs) in self.buckets.iter_mut().zip(other.buckets) {
            *mine += theirs;
        }
    }
}

impl Timing {
    pub fn note(&mut self, took: Duration) {
        let us = u64::try_from(took.as_micros()).unwrap_or(u64::MAX);
        self.count += 1;
        self.total_us = self.total_us.saturating_add(us);
        self.max_us = self.max_us.max(us);
        let bucket = (0..BUCKETS)
            .find(|&bucket| us as f64 <= bound(bucket))
            .unwrap_or(BUCKETS - 1);
        self.buckets[bucket] += 1;
    }

    /// The time in milliseconds that the share `q` of all took at most, as the
    /// upper end of its bucket, but never more than the longest. None if
    /// nothing was timed.
    pub fn quantile(&self, q: f64) -> Option<f64> {
        if self.count == 0 {
            return None;
        }
        let wanted = ((self.count as f64) * q).ceil().max(1.0) as u64;
        let mut seen = 0;
        for (bucket, count) in self.buckets.iter().enumerate() {
            seen += count;
            if seen >= wanted {
                return Some(bound(bucket).min(self.max_us as f64) / 1000.0);
            }
        }
        Some(self.max_us as f64 / 1000.0)
    }

    pub fn mean(&self) -> Option<f64> {
        (self.count > 0).then(|| self.total_us as f64 / self.count as f64 / 1000.0)
    }

    pub fn max(&self) -> Option<f64> {
        (self.count > 0).then(|| self.max_us as f64 / 1000.0)
    }

    /// The sum of all times, in seconds.
    pub fn sum_seconds(&self) -> f64 {
        self.total_us as f64 / 1_000_000.0
    }

    /// The buckets of a histogram for Prometheus: for each upper end in
    /// [`HISTOGRAM`], in seconds, how many took at most that long. Each upper
    /// end is that of a bucket here, so the counts are exact.
    pub fn histogram(&self) -> [(f64, u64); HISTOGRAM.len()] {
        let mut seen = 0;
        let mut counted = 0;
        HISTOGRAM.map(|last| {
            for count in &self.buckets[counted..=last] {
                seen += count;
            }
            counted = last + 1;
            (bound(last) / 1_000_000.0, seen)
        })
    }
}

/// The buckets of a [`Timing`] whose upper ends are those of a histogram for
/// Prometheus: every sixth, four times as long each, from 0.4 ms to 6.6 s.
/// Sixty buckets for each route would be more than Prometheus wants to keep.
pub const HISTOGRAM: [usize; 8] = [6, 12, 18, 24, 30, 36, 42, 48];

/// The requests to one route.
#[derive(Clone, Default, Debug)]
pub struct Requests {
    pub timing: Timing,
    /// Answers by their first digit: 1xx to 5xx. An image whose source did
    /// not deliver it counts as refused (see `counted`).
    pub answers: [u64; 5],
    /// Answers that said "too many requests".
    pub limited: u64,
}

impl Tally for Requests {
    fn add(&mut self, other: &Self) {
        self.timing.add(&other.timing);
        for (mine, theirs) in self.answers.iter_mut().zip(other.answers) {
            *mine += theirs;
        }
        self.limited += other.limited;
    }
}

/// How fetching went.
#[derive(Clone, Default, Debug)]
pub struct Fetches {
    /// How long a fetch took, from asking the site to having stored it.
    pub timing: Timing,
    /// Fetches that brought the feed.
    pub fetched: u64,
    /// Fetches the site answered with "not modified".
    pub unchanged: u64,
    /// Fetches that failed at the site or with what it sent.
    pub failed: u64,
    /// Fetches that failed at the database: a problem of the server.
    pub broken: u64,
    /// The articles the fetches brought that were new.
    pub new_articles: u64,
}

impl Tally for Fetches {
    fn add(&mut self, other: &Self) {
        self.timing.add(&other.timing);
        self.fetched += other.fetched;
        self.unchanged += other.unchanged;
        self.failed += other.failed;
        self.broken += other.broken;
        self.new_articles += other.new_articles;
    }
}

/// How a fetch ended, for the count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fetched {
    Fetched { new: u64 },
    Unchanged,
    Failed,
    Broken,
}

/// A round of fetching: when, how many feeds and how long.
#[derive(Clone, Copy, Debug)]
pub struct Round {
    pub at: OffsetDateTime,
    pub feeds: usize,
    pub took: Duration,
}

/// How the feed search went: what a reader typed was looked at for feeds.
#[derive(Clone, Default, Debug)]
pub struct Lookups {
    pub timing: Timing,
    /// Lookups that found at least one feed.
    pub found: u64,
    /// Lookups of a page without feeds.
    pub none: u64,
    /// Lookups the site did not answer, or not with a page.
    pub failed: u64,
    /// Lookups that were over the limit of their account.
    pub limited: u64,
}

impl Tally for Lookups {
    fn add(&mut self, other: &Self) {
        self.timing.add(&other.timing);
        self.found += other.found;
        self.none += other.none;
        self.failed += other.failed;
        self.limited += other.limited;
    }
}

/// How a lookup ended, for the count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Looked {
    Found,
    None,
    Failed,
    Limited,
}

/// What the image proxy served.
#[derive(Clone, Default, Debug)]
pub struct Images {
    /// How long images took that were not in the cache.
    pub timing: Timing,
    /// Images from the cache.
    pub cached: u64,
    /// Images fetched from their source.
    pub fetched: u64,
    /// Addresses the server did not make.
    pub not_found: u64,
    /// Sources that did not answer with an image, or failed a moment ago.
    pub source_failed: u64,
    /// What was no image, or none the proxy lets through.
    pub not_image: u64,
    /// Images with more bytes or pixels than the proxy takes.
    pub too_large: u64,
}

impl Tally for Images {
    fn add(&mut self, other: &Self) {
        self.timing.add(&other.timing);
        self.cached += other.cached;
        self.fetched += other.fetched;
        self.not_found += other.not_found;
        self.source_failed += other.source_failed;
        self.not_image += other.not_image;
        self.too_large += other.too_large;
    }
}

/// What the server notices about itself. One for each server; cheap to share.
pub struct Monitor {
    pub started: Instant,
    pub started_at: OffsetDateTime,
    requests: Mutex<HashMap<String, Window<Requests>>>,
    fetches: Mutex<Window<Fetches>>,
    last_round: Mutex<Option<Round>>,
    /// The pace of the loop that fetches feeds, while it runs.
    pace: Mutex<Option<Pace>>,
    lookups: Mutex<Window<Lookups>>,
    pub problems: Arc<Problems>,
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new(Arc::default())
    }
}

impl Monitor {
    /// A monitor that shows the problems the log gathers in `problems`.
    pub fn new(problems: Arc<Problems>) -> Self {
        Self {
            started: Instant::now(),
            started_at: OffsetDateTime::now_utc(),
            requests: Mutex::default(),
            fetches: Mutex::default(),
            last_round: Mutex::default(),
            pace: Mutex::default(),
            lookups: Mutex::default(),
            problems,
        }
    }

    /// Counts a request to a route, such as "GET /api/tree".
    pub fn request(&self, route: &str, status: StatusCode, took: Duration, now: Instant) {
        let Ok(mut routes) = self.requests.lock() else {
            return;
        };
        let route = if routes.contains_key(route) || routes.len() < MAX_ROUTES {
            route
        } else {
            "other"
        };
        let class = usize::from(status.as_u16() / 100).clamp(1, 5) - 1;
        let limited = status == StatusCode::TOO_MANY_REQUESTS;
        let window = routes.entry(route.to_owned()).or_default();
        window.count(now, |requests| {
            requests.timing.note(took);
            requests.answers[class] += 1;
            if limited {
                requests.limited += 1;
            }
        });
    }

    /// The requests of the last hour, by route.
    pub fn requests(&self, now: Instant) -> Vec<(String, Requests)> {
        let Ok(routes) = self.requests.lock() else {
            return Vec::new();
        };
        routes
            .iter()
            .map(|(route, window)| (route.clone(), window.last_hour(now)))
            .filter(|(_, requests)| requests.timing.count > 0)
            .collect()
    }

    /// The requests since the start, by route.
    pub fn requests_total(&self) -> Vec<(String, Requests)> {
        let Ok(routes) = self.requests.lock() else {
            return Vec::new();
        };
        let mut total: Vec<(String, Requests)> = routes
            .iter()
            .map(|(route, window)| (route.clone(), window.total()))
            .filter(|(_, requests)| requests.timing.count > 0)
            .collect();
        total.sort_by(|(a, _), (b, _)| a.cmp(b));
        total
    }

    /// Notes that the loop that fetches feeds runs, at this pace.
    pub fn fetching_runs(&self, pace: Pace) {
        if let Ok(mut running) = self.pace.lock() {
            *running = Some(pace);
        }
    }

    /// The pace feeds are fetched at; none where the server does not fetch.
    pub fn fetching(&self) -> Option<Pace> {
        self.pace.lock().ok().and_then(|pace| *pace)
    }

    /// Counts a fetch of a feed.
    pub fn fetch(&self, fetched: Fetched, took: Duration, now: Instant) {
        let Ok(mut window) = self.fetches.lock() else {
            return;
        };
        window.count(now, |fetches| {
            fetches.timing.note(took);
            match fetched {
                Fetched::Fetched { new } => {
                    fetches.fetched += 1;
                    fetches.new_articles += new;
                }
                Fetched::Unchanged => fetches.unchanged += 1,
                Fetched::Failed => fetches.failed += 1,
                Fetched::Broken => fetches.broken += 1,
            }
        });
    }

    pub fn fetches(&self, now: Instant) -> Fetches {
        self.fetches
            .lock()
            .map(|window| window.last_hour(now))
            .unwrap_or_default()
    }

    /// The fetches since the start.
    pub fn fetches_total(&self) -> Fetches {
        self.fetches
            .lock()
            .map(|window| window.total())
            .unwrap_or_default()
    }

    /// Notes a round of fetching that is over.
    pub fn round(&self, round: Round) {
        if let Ok(mut last) = self.last_round.lock() {
            *last = Some(round);
        }
    }

    pub fn last_round(&self) -> Option<Round> {
        self.last_round.lock().ok().and_then(|last| *last)
    }

    /// Counts a lookup of the feed search.
    pub fn lookup(&self, looked: Looked, took: Duration, now: Instant) {
        let Ok(mut window) = self.lookups.lock() else {
            return;
        };
        window.count(now, |lookups| match looked {
            Looked::Found => {
                lookups.found += 1;
                lookups.timing.note(took);
            }
            Looked::None => {
                lookups.none += 1;
                lookups.timing.note(took);
            }
            Looked::Failed => {
                lookups.failed += 1;
                lookups.timing.note(took);
            }
            // Refused before anything was fetched: no time to count.
            Looked::Limited => lookups.limited += 1,
        });
    }

    pub fn lookups(&self, now: Instant) -> Lookups {
        self.lookups
            .lock()
            .map(|window| window.last_hour(now))
            .unwrap_or_default()
    }

    /// The lookups since the start.
    pub fn lookups_total(&self) -> Lookups {
        self.lookups
            .lock()
            .map(|window| window.total())
            .unwrap_or_default()
    }
}

/// Around everything the server answers: counts the request for its route,
/// with the time until the answer begins. A stream of events counts until it
/// is open, not for as long as it stays so.
pub async fn time(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let route = match request.extensions().get::<MatchedPath>() {
        Some(path) => format!("{} {}", request.method(), path.as_str()),
        // What no route of the API took: the files of the frontend, or
        // addresses that are not there.
        None if request.uri().path().starts_with("/api/") => "other".to_owned(),
        None => "frontend".to_owned(),
    };
    let started = Instant::now();
    let response = next.run(request).await;
    let now = Instant::now();
    state
        .monitor
        .request(&route, counted(&response), now - started, now);
    response
}

/// What an answer counts as: its status, but an image whose source did not
/// deliver it, answered 502, counts as refused, not as an error of the
/// server. The fault is the source's, and the image proxy counts it as
/// "source failed".
fn counted(response: &Response) -> StatusCode {
    if response.extensions().get::<SourceFailed>().is_some() {
        StatusCode::NOT_FOUND
    } else {
        response.status()
    }
}

/// Which answers the log takes for failures: those with an error of the
/// server, as `tower_http` has it, but not an image whose source did not
/// deliver it, as with [`counted`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Failures;

impl ClassifyResponse for Failures {
    type FailureClass = ServerErrorsFailureClass;
    type ClassifyEos = NeverClassifyEos<ServerErrorsFailureClass>;

    fn classify_response<B>(
        self,
        response: &axum::http::Response<B>,
    ) -> ClassifiedResponse<Self::FailureClass, Self::ClassifyEos> {
        if response.extensions().get::<SourceFailed>().is_some() {
            return ClassifiedResponse::Ready(Ok(()));
        }
        ServerErrorsAsFailures::new().classify_response(response)
    }

    fn classify_error<E>(self, error: &E) -> Self::FailureClass
    where
        E: fmt::Display + 'static,
    {
        ServerErrorsAsFailures::new().classify_error(error)
    }
}

/// How serious a problem is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

/// A warning or an error the server logged, with how often it did.
#[derive(Clone, Debug)]
pub struct Problem {
    pub severity: Severity,
    /// Where in the server it comes from, such as "rueder::fetch".
    pub target: String,
    pub text: String,
    pub count: u64,
    pub first: OffsetDateTime,
    pub last: OffsetDateTime,
}

#[derive(Clone, Default, Debug)]
pub struct ProblemCount {
    pub warnings: u64,
    pub errors: u64,
}

impl Tally for ProblemCount {
    fn add(&mut self, other: &Self) {
        self.warnings += other.warnings;
        self.errors += other.errors;
    }
}

/// The warnings and errors of the log, the latest first. The same text from
/// the same place is one problem that happened more than once.
#[derive(Default)]
pub struct Problems {
    kept: Mutex<Kept>,
}

#[derive(Default)]
struct Kept {
    problems: VecDeque<Problem>,
    counts: Window<ProblemCount>,
}

impl Problems {
    pub fn note(&self, severity: Severity, target: &str, text: &str, now: Instant) {
        let Ok(mut kept) = self.kept.lock() else {
            return;
        };
        kept.counts.count(now, |counts| match severity {
            Severity::Warning => counts.warnings += 1,
            Severity::Error => counts.errors += 1,
        });
        let text = shortened(text);
        let at = OffsetDateTime::now_utc();
        let known = kept.problems.iter().position(|problem| {
            problem.severity == severity && problem.target == target && problem.text == text
        });
        let problem = match known.and_then(|known| kept.problems.remove(known)) {
            Some(problem) => Problem {
                count: problem.count + 1,
                last: at,
                ..problem
            },
            None => Problem {
                severity,
                target: target.to_owned(),
                text,
                count: 1,
                first: at,
                last: at,
            },
        };
        kept.problems.push_front(problem);
        kept.problems.truncate(MAX_PROBLEMS);
    }

    /// The problems, the latest first.
    pub fn list(&self) -> Vec<Problem> {
        self.kept
            .lock()
            .map(|kept| kept.problems.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// How many warnings and errors there were in the last hour.
    pub fn last_hour(&self, now: Instant) -> ProblemCount {
        self.kept
            .lock()
            .map(|kept| kept.counts.last_hour(now))
            .unwrap_or_default()
    }

    /// How many warnings and errors there were since the start.
    pub fn total(&self) -> ProblemCount {
        self.kept
            .lock()
            .map(|kept| kept.counts.total())
            .unwrap_or_default()
    }

    /// What gathers the warnings and errors of the log here.
    pub fn layer(self: &Arc<Self>) -> ProblemLayer {
        ProblemLayer {
            problems: self.clone(),
        }
    }
}

fn shortened(text: &str) -> String {
    match text.char_indices().nth(MAX_PROBLEM_TEXT) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_owned(),
    }
}

/// A layer of the log that hands its warnings and errors to [`Problems`].
pub struct ProblemLayer {
    problems: Arc<Problems>,
}

impl<S: Subscriber> Layer<S> for ProblemLayer {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let severity = match *event.metadata().level() {
            Level::ERROR => Severity::Error,
            Level::WARN => Severity::Warning,
            _ => return,
        };
        let mut text = Text::default();
        event.record(&mut text);
        let text = match (text.message.is_empty(), text.fields.is_empty()) {
            (_, true) => text.message,
            (true, false) => text.fields,
            (false, false) => format!("{} ({})", text.message, text.fields),
        };
        let target = event.metadata().target();
        self.problems.note(severity, target, &text, Instant::now());
    }
}

/// Fields that differ each time, such as how long something took: with them,
/// the same problem would never be the same.
const CHANGING: &[&str] = &[
    "latency",
    "elapsed",
    "elapsed_secs",
    "rows_affected",
    "rows_returned",
];

/// The message of an event and its other fields.
#[derive(Default)]
struct Text {
    message: String,
    fields: String,
}

impl Visit for Text {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
            return;
        }
        if CHANGING.contains(&field.name()) {
            return;
        }
        if !self.fields.is_empty() {
            self.fields.push_str(", ");
        }
        let _ = write!(self.fields, "{}={value:?}", field.name());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            self.record_debug(field, &value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn a_window_forgets_what_is_older_than_an_hour() {
        // The start of a part, so that the test does not depend on the clock.
        let start = *EPOCH;
        let mut window = Window::<Lookups>::default();
        window.at(start).found += 1;
        window.at(start + Duration::from_secs(30 * 60)).found += 2;
        assert_eq!(
            window.last_hour(start + Duration::from_secs(40 * 60)).found,
            3
        );
        // An hour after the first, only the second is left.
        let later = start + SLOT * SLOTS as u32;
        assert_eq!(window.last_hour(later).found, 2);
        // Its part was used again, and starts from nothing.
        window.at(later).found += 5;
        assert_eq!(window.last_hour(later).found, 7);
        assert_eq!(window.last_hour(later + SLOT * SLOTS as u32).found, 0);
    }

    #[test]
    fn the_quantiles_of_a_timing_are_near_the_true_ones() {
        // A simple pseudo-random sequence: the same every run.
        let mut seed = 0x2545_f491_4f6c_dd1d_u64;
        let mut random = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for round in 0..50 {
            let mut timing = Timing::default();
            let mut times: Vec<u64> = (0..1 + random() % 500)
                .map(|_| {
                    // From microseconds to a minute, more of them short.
                    let scale = 10_u64.pow((random() % 8) as u32);
                    random() % scale.max(2)
                })
                .collect();
            for &us in &times {
                timing.note(Duration::from_micros(us));
            }
            times.sort_unstable();
            for q in [0.5, 0.95, 1.0] {
                let index = ((times.len() as f64 * q).ceil() as usize).max(1) - 1;
                let truth = times[index] as f64 / 1000.0;
                let guess = timing.quantile(q).unwrap();
                assert!(
                    guess >= truth.min(0.1),
                    "round {round}, q {q}: {guess} < {truth}"
                );
                // A bucket is a third of an octave wide.
                assert!(
                    guess <= (truth * 2_f64.powf(1.0 / 3.0)).max(0.1) + 1e-9,
                    "round {round}, q {q}: {guess} far above {truth}"
                );
            }
            assert_eq!(timing.max(), Some(*times.last().unwrap() as f64 / 1000.0));
        }
        assert_eq!(Timing::default().quantile(0.5), None);
    }

    #[test]
    fn the_same_problem_is_counted_and_comes_first() {
        let problems = Problems::default();
        let now = Instant::now();
        problems.note(Severity::Error, "rueder", "the database is gone", now);
        problems.note(
            Severity::Warning,
            "rueder::oidc",
            "OpenID Connect: no answer",
            now,
        );
        problems.note(Severity::Error, "rueder", "the database is gone", now);
        let list = problems.list();
        assert_eq!(list.len(), 2);
        assert_eq!(
            (list[0].text.as_str(), list[0].count),
            ("the database is gone", 2)
        );
        assert_eq!(list[1].severity, Severity::Warning);
        let counted = problems.last_hour(now);
        assert_eq!((counted.errors, counted.warnings), (2, 1));
    }

    #[test]
    fn problems_are_kept_within_limits() {
        let problems = Problems::default();
        let now = Instant::now();
        for n in 0..MAX_PROBLEMS + 20 {
            problems.note(Severity::Warning, "rueder", &format!("problem {n}"), now);
        }
        let list = problems.list();
        assert_eq!(list.len(), MAX_PROBLEMS);
        assert_eq!(list[0].text, format!("problem {}", MAX_PROBLEMS + 19));
        problems.note(Severity::Error, "rueder", &"x".repeat(5000), now);
        assert_eq!(
            problems.list()[0].text.chars().count(),
            MAX_PROBLEM_TEXT + 1
        );
    }

    #[test]
    fn the_layer_gathers_warnings_and_errors_of_the_log() {
        use tracing_subscriber::layer::SubscriberExt;

        let problems = Arc::new(Problems::default());
        let subscriber = tracing_subscriber::registry().with(problems.layer());
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("all is well");
            tracing::warn!(feed_id = 7, "fetching: the site is slow");
            tracing::error!(latency = 12, "response failed");
            tracing::error!(latency = 13, "response failed");
        });
        let list = problems.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].text, "response failed");
        assert_eq!(list[0].count, 2);
        assert_eq!(list[1].text, "fetching: the site is slow (feed_id=7)");
        assert_eq!(list[1].severity, Severity::Warning);
    }

    #[test]
    fn an_image_whose_source_failed_is_no_error_of_the_server() {
        let failed = (
            StatusCode::BAD_GATEWAY,
            axum::Extension(crate::images::SourceFailed),
        )
            .into_response();
        let other = StatusCode::BAD_GATEWAY.into_response();
        assert_eq!(counted(&failed), StatusCode::NOT_FOUND);
        assert_eq!(counted(&other), StatusCode::BAD_GATEWAY);
        let classified = |response: &Response| {
            matches!(
                Failures.classify_response(response),
                ClassifiedResponse::Ready(Err(_))
            )
        };
        assert!(!classified(&failed));
        assert!(classified(&other));
        assert!(!classified(&StatusCode::OK.into_response()));
    }

    #[test]
    fn routes_are_counted_within_a_limit() {
        let monitor = Monitor::default();
        let now = Instant::now();
        for n in 0..MAX_ROUTES + 5 {
            let route = format!("GET /api/{n}");
            monitor.request(&route, StatusCode::OK, Duration::from_millis(3), now);
        }
        monitor.request(
            "GET /api/0",
            StatusCode::TOO_MANY_REQUESTS,
            Duration::ZERO,
            now,
        );
        let requests = monitor.requests(now);
        assert_eq!(requests.len(), MAX_ROUTES + 1);
        let other = requests.iter().find(|(route, _)| route == "other").unwrap();
        assert_eq!(other.1.timing.count, 5);
        let first = requests
            .iter()
            .find(|(route, _)| route == "GET /api/0")
            .unwrap();
        assert_eq!((first.1.answers, first.1.limited), ([0, 1, 0, 1, 0], 1));
    }

    proptest::proptest! {
        /// The total since the start has all that was counted, whichever part
        /// of the hour it fell in, and the histogram for Prometheus counts
        /// exactly what took at most each of its upper ends.
        #[test]
        fn the_total_and_its_histogram_are_exact(
            times in proptest::collection::vec((0u64..20_000_000, 0u32..20), 0..200),
        ) {
            let start = Instant::now();
            let mut window = Window::<Lookups>::default();
            for (us, part) in &times {
                window.count(start + SLOT * *part, |lookups| {
                    lookups.found += 1;
                    lookups.timing.note(Duration::from_micros(*us));
                });
            }
            let total = window.total();
            proptest::prop_assert_eq!(total.found, times.len() as u64);
            proptest::prop_assert_eq!(total.timing.count, times.len() as u64);
            let sum: u64 = times.iter().map(|(us, _)| us).sum();
            proptest::prop_assert!((total.timing.sum_seconds() - sum as f64 / 1e6).abs() < 1e-6);
            let mut last = 0;
            for (&bucket, (seconds, count)) in HISTOGRAM.iter().zip(total.timing.histogram()) {
                let within = times.iter().filter(|(us, _)| *us as f64 <= bound(bucket)).count();
                proptest::prop_assert_eq!(count, within as u64);
                proptest::prop_assert!(count >= last);
                proptest::prop_assert!((seconds * 1e6 - bound(bucket)).abs() < 1e-6);
                last = count;
            }
        }
    }
}
