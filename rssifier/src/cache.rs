//! The answers for pages, kept for [`crate::limits::Pace::cache_for`], so that
//! many readers of a page, and many clients, cost its site one fetch. Problems
//! are kept as well: a page that has nothing to offer is not asked again and
//! again either.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

use crate::feed::{Problem, Written};

/// How many pages are kept at most.
pub const MAX_PAGES: usize = 10_000;

pub type Answer = Arc<Result<Written, Problem>>;

pub struct Cache {
    keep_for: Duration,
    pages: Mutex<HashMap<String, (Instant, Answer)>>,
}

impl Cache {
    pub fn new(keep_for: Duration) -> Self {
        Self {
            keep_for,
            pages: Mutex::default(),
        }
    }

    /// The answer for the page, if it is fresh at `now`.
    pub fn get(&self, address: &str, now: Instant) -> Option<Answer> {
        let pages = self.pages.lock().unwrap_or_else(PoisonError::into_inner);
        let (at, answer) = pages.get(address)?;
        (now.saturating_duration_since(*at) < self.keep_for).then(|| answer.clone())
    }

    /// Keeps the answer. When the cache is full, the pages that ran out go,
    /// and if that is not enough, the oldest.
    pub fn put(&self, address: &str, answer: Answer, now: Instant) {
        let mut pages = self.pages.lock().unwrap_or_else(PoisonError::into_inner);
        if pages.len() >= MAX_PAGES && !pages.contains_key(address) {
            let keep_for = self.keep_for;
            pages.retain(|_, (at, _)| now.saturating_duration_since(*at) < keep_for);
            if pages.len() >= MAX_PAGES
                && let Some(oldest) = pages
                    .iter()
                    .min_by_key(|(_, (at, _))| *at)
                    .map(|(address, _)| address.clone())
            {
                pages.remove(&oldest);
            }
        }
        pages.insert(address.to_owned(), (now, answer));
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;

    #[test]
    fn an_answer_is_kept_for_its_time_and_the_oldest_makes_room() {
        let cache = Cache::new(Duration::from_secs(60));
        let now = Instant::now();
        let problem = |n: usize| {
            Arc::new(Err(Problem::new(
                StatusCode::NOT_FOUND,
                "no-items",
                n.to_string(),
            )))
        };
        cache.put("a", problem(0), now);
        assert!(cache.get("a", now + Duration::from_secs(59)).is_some());
        assert!(cache.get("a", now + Duration::from_secs(60)).is_none());
        for n in 1..=MAX_PAGES {
            cache.put(
                &n.to_string(),
                problem(n),
                now + Duration::from_millis(n as u64),
            );
        }
        assert!(cache.get("a", now).is_none());
        assert!(cache.get(&MAX_PAGES.to_string(), now).is_some());
    }
}
