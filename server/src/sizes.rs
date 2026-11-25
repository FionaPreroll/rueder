//! How large the server is set up (work package 5.33): how many connections
//! to the database it keeps, how many costly requests run at once and how
//! many mails it sends a day. As rueder comes they suit an instance of a few
//! thousand accounts; a large one sets them with `DATABASE_CONNECTIONS`,
//! `HEAVY_AT_ONCE` and `MAILS_PER_DAY`, which the server checks against
//! [`CONNECTIONS`], [`HEAVY`] and [`MAILS`] when it starts.

use std::{fmt::Display, ops::RangeInclusive, str::FromStr, time::Duration};

use crate::rate::{self, Limits, Rate};

/// The sizes of a server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sizes {
    /// The connections to the database the server keeps at most.
    pub connections: u32,
    /// The costly requests that run at once on the whole server
    /// ([`rate::Rates::heavy`]); always fewer than the connections, so that
    /// reading goes on meanwhile.
    pub heavy_at_once: usize,
    /// The mails the whole server sends in a day, of which links to set a new
    /// password may be three in ten.
    pub mails_per_day: u32,
}

/// How many connections to the database a server may be given. PostgreSQL
/// takes 100 as it comes, a few of which it keeps for itself.
pub const CONNECTIONS: RangeInclusive<u32> = 2..=90;
/// How many costly requests at once a server may be given.
pub const HEAVY: RangeInclusive<usize> = 1..=64;
/// How many mails a day a server may be given.
pub const MAILS: RangeInclusive<u32> = 1..=1_000_000;

impl Default for Sizes {
    fn default() -> Self {
        Self {
            connections: 10,
            heavy_at_once: rate::HEAVY_AT_ONCE,
            mails_per_day: rate::MAILS_PER_DAY.burst,
        }
    }
}

impl Sizes {
    /// The sizes the environment sets, with the default for what it leaves out.
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_vars(|name| std::env::var(name).ok())
    }

    /// The sizes `var` gives, with the default for what it leaves out. A value
    /// that is no number or out of bounds is an error, not quietly the default.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let sizes = Self::default();
        let sizes = Self {
            connections: within(
                "DATABASE_CONNECTIONS",
                var("DATABASE_CONNECTIONS"),
                &CONNECTIONS,
            )?
            .unwrap_or(sizes.connections),
            heavy_at_once: within("HEAVY_AT_ONCE", var("HEAVY_AT_ONCE"), &HEAVY)?
                .unwrap_or(sizes.heavy_at_once),
            mails_per_day: within("MAILS_PER_DAY", var("MAILS_PER_DAY"), &MAILS)?
                .unwrap_or(sizes.mails_per_day),
        };
        if sizes.heavy_at_once >= sizes.connections as usize {
            anyhow::bail!(
                "HEAVY_AT_ONCE ({}) must be fewer than DATABASE_CONNECTIONS ({}), so that reading goes on meanwhile",
                sizes.heavy_at_once,
                sizes.connections
            );
        }
        Ok(sizes)
    }

    /// What the server allows with these sizes: the limits rueder comes with,
    /// but as many costly requests at once and mails a day as these say.
    pub fn limits(&self) -> Limits {
        let day = Duration::from_secs(24 * 60 * 60);
        let per_day = |count: u32| Rate::new(count, day / count);
        Limits {
            heavy_at_once: self.heavy_at_once,
            mails_per_day: per_day(self.mails_per_day),
            reset_mails_per_day: per_day((self.mails_per_day * 3 / 10).max(1)),
            inactive_mails_per_day: per_day((self.mails_per_day * 3 / 10).max(1)),
            ..Limits::default()
        }
    }
}

/// The number a setting gives, if it is set: one that is no number or out of
/// `bounds` is an error.
pub fn within<T: FromStr + PartialOrd + Display>(
    name: &str,
    value: Option<String>,
    bounds: &RangeInclusive<T>,
) -> anyhow::Result<Option<T>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let number: T = value
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("{name} is no number: {value:?}"))?;
    in_bounds(name, number, bounds)
        .map(Some)
        .map_err(anyhow::Error::msg)
}

/// `value` if it is within `bounds`, else what to say about `what`.
pub fn in_bounds<T: PartialOrd + Display>(
    what: &str,
    value: T,
    bounds: &RangeInclusive<T>,
) -> Result<T, String> {
    if bounds.contains(&value) {
        Ok(value)
    } else {
        Err(format!(
            "{what} must be from {} to {}, not {value}",
            bounds.start(),
            bounds.end()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sizes(pairs: &[(&str, &str)]) -> anyhow::Result<Sizes> {
        Sizes::from_vars(|name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        })
    }

    #[test]
    fn the_sizes_come_from_the_environment_within_bounds() {
        // As rueder comes: 10 connections, 4 costly requests at once, 1,000
        // mails a day, and the limits it has without any setting.
        let default = sizes(&[]).unwrap();
        assert_eq!(default, Sizes::default());
        assert_eq!(
            (
                default.connections,
                default.heavy_at_once,
                default.mails_per_day
            ),
            (10, 4, 1000)
        );
        assert_eq!(default.limits(), Limits::default());
        let large = sizes(&[
            ("DATABASE_CONNECTIONS", " 20 "),
            ("HEAVY_AT_ONCE", "6"),
            ("MAILS_PER_DAY", "10000"),
        ])
        .unwrap();
        assert_eq!(
            (large.connections, large.heavy_at_once, large.mails_per_day),
            (20, 6, 10_000)
        );
        // Only what is set changes.
        assert_eq!(sizes(&[("MAILS_PER_DAY", "50")]).unwrap().connections, 10);
        // At the bounds, and beyond them.
        for fine in [
            &[("DATABASE_CONNECTIONS", "90")][..],
            &[("DATABASE_CONNECTIONS", "2"), ("HEAVY_AT_ONCE", "1")],
            &[("DATABASE_CONNECTIONS", "65"), ("HEAVY_AT_ONCE", "64")],
            &[("MAILS_PER_DAY", "1")],
            &[("MAILS_PER_DAY", "1000000")],
        ] {
            assert!(sizes(fine).is_ok(), "{fine:?}");
        }
        for wrong in [
            &[("DATABASE_CONNECTIONS", "1")][..],
            &[("DATABASE_CONNECTIONS", "91")],
            &[("DATABASE_CONNECTIONS", "ten")],
            &[("HEAVY_AT_ONCE", "0")],
            &[("HEAVY_AT_ONCE", "-3")],
            &[("MAILS_PER_DAY", "0")],
            &[("MAILS_PER_DAY", "1000001")],
            &[("MAILS_PER_DAY", "")],
        ] {
            assert!(sizes(wrong).is_err(), "{wrong:?}");
        }
        // Costly requests always leave a connection for reading.
        for wrong in [
            &[("HEAVY_AT_ONCE", "10")][..],
            &[("DATABASE_CONNECTIONS", "3")],
            &[("DATABASE_CONNECTIONS", "64"), ("HEAVY_AT_ONCE", "64")],
        ] {
            let err = sizes(wrong).unwrap_err().to_string();
            assert!(
                err.contains("fewer than DATABASE_CONNECTIONS"),
                "{wrong:?}: {err}"
            );
        }
    }

    #[test]
    fn mail_a_day_keeps_its_share_for_new_passwords() {
        let limits = sizes(&[("MAILS_PER_DAY", "10000")]).unwrap().limits();
        assert_eq!(
            limits.mails_per_day,
            Rate::new(10_000, Duration::from_millis(8640))
        );
        assert_eq!(
            limits.reset_mails_per_day,
            Rate::new(3000, Duration::from_millis(28_800))
        );
        // However few mails a day, a link to set a new password can go.
        let few = sizes(&[("MAILS_PER_DAY", "3")]).unwrap().limits();
        assert_eq!(few.reset_mails_per_day.burst, 1);
        // The rest stays as rueder comes.
        assert_eq!(limits.requests, Limits::default().requests);
        assert_eq!(
            limits.mails_per_account,
            Limits::default().mails_per_account
        );
    }
}
