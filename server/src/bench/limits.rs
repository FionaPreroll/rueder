//! What a limit in memory costs one request, with few keys and with a full
//! table: `rueder bench limits`. Every key has asked a moment ago, so that
//! none of them could simply be forgotten.

use std::{
    fmt::Write,
    net::{IpAddr, Ipv4Addr},
    time::{Duration, Instant},
};

use crate::{
    login::{self, Key},
    rate::{self, Rates},
};

/// How many calls are timed for each line.
const CALLS: usize = 5_000;

/// Times a known and a new key against every request's limit and against the
/// wrong passwords, with 100 keys and with as many as are kept.
pub fn run() -> String {
    let mut report = format!(
        "{:<16} {:>7}   {:>22}   {:>22}\n",
        "limit", "keys", "known: mean / 99%", "new: mean / 99%"
    );
    for keys in [100, rate::MAX_KEYS] {
        let (rates, now) = (Rates::default(), Instant::now());
        for n in 0..keys {
            let _ = rates.request(sender(n), now);
        }
        let known = timed(|n| {
            let _ = rates.request(sender(n % keys), now);
        });
        let new = timed(|n| {
            let _ = rates.request(sender(keys + n), now);
        });
        line(&mut report, "requests", keys, known, new);
    }
    for keys in [100, login::MAX_KEYS] {
        let (limiter, now) = (login::Limiter::default(), Instant::now());
        for n in 0..keys {
            limiter.failed(name(n), now);
        }
        let known = timed(|n| {
            limiter.blocked(&name(n % keys), now);
        });
        let new = timed(|n| limiter.failed(name(keys + n), now));
        line(&mut report, "wrong passwords", keys, known, new);
    }
    report
}

fn sender(n: usize) -> IpAddr {
    IpAddr::V4(Ipv4Addr::from(0x0a00_0000_u32.wrapping_add(n as u32)))
}

fn name(n: usize) -> Key {
    Key::name(&format!("reader-{n}"))
}

/// The mean time of a call and the time 99 of 100 calls stay within.
fn timed(mut call: impl FnMut(usize)) -> [Duration; 2] {
    let mut took: Vec<Duration> = (0..CALLS)
        .map(|n| {
            let start = Instant::now();
            call(n);
            start.elapsed()
        })
        .collect();
    took.sort();
    let mean = took.iter().sum::<Duration>() / CALLS as u32;
    [mean, took[CALLS * 99 / 100]]
}

fn line(report: &mut String, limit: &str, keys: usize, known: [Duration; 2], new: [Duration; 2]) {
    let pair = |[mean, most]: [Duration; 2]| format!("{mean:.1?} / {most:.1?}");
    let _ = writeln!(
        report,
        "{limit:<16} {keys:>7}   {:>22}   {:>22}",
        pair(known),
        pair(new)
    );
}
