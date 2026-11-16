//! `rueder bench fetch`: the source on this machine and the measurement.

mod common;

use std::time::Duration;

use rueder::{bench::fetching, fetch::Pace};
use sqlx::PgPool;

/// A port free on the first name of the source, for the others too.
fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.1.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

fn options(port: u16) -> fetching::Options {
    fetching::Options {
        feeds: 6,
        hosts: 3,
        slow: 0,
        seconds: 2,
        port,
        pace: Pace {
            round: 6,
            at_once: 3,
            every: Duration::from_secs(1),
            per_host: 1,
        },
    }
}

#[sqlx::test]
async fn fetches_the_made_up_feeds_round_after_round(db: PgPool) {
    let report = fetching::run(&db, options(free_port())).await.unwrap();
    assert!(report.rounds >= 2, "{report}");
    // The first round brings every feed; after that most are unchanged.
    assert!(report.fetched >= 6, "{report}");
    assert!(report.new_articles >= 6 * 20, "{report}");
    assert_eq!((report.failed, report.broken), (0, 0), "{report}");
    assert_eq!(report.fetches(), report.fetched + report.unchanged);
    assert!(report.per_hour() > 0.0);
    let text = report.to_string();
    assert!(text.contains("6 feeds under 3 names"), "{text}");

    // A second run finds the feeds of the first.
    let port = report.options.port;
    fetching::run(&db, options(port)).await.unwrap();
    let feeds: i64 = sqlx::query_scalar("SELECT count(*) FROM feeds")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(feeds, 6);
}

/// The slow name holds up only itself: the others are fetched round after
/// round while it takes half a second for each of its feeds.
#[sqlx::test]
async fn a_slow_name_is_fetched_beside_the_others(db: PgPool) {
    let mut options = fetching::Options {
        slow: 3,
        seconds: 3,
        ..options(free_port())
    };
    options.pace.round = 20;
    let report = fetching::run(&db, options).await.unwrap();
    assert!(report.slow >= 2, "{report}");
    // In three seconds the six quick ones came at least twice, whatever the
    // slow ones took.
    assert!(report.fetches() - report.slow >= 6 * 2, "{report}");
    assert!(report.to_string().contains("and 3 slow ones"), "{report}");
}

#[sqlx::test]
async fn wants_a_database_where_nobody_reads_anything_else(db: PgPool) {
    common::reader(&db, "ada", 0).await;
    let error = fetching::run(&db, options(free_port())).await.unwrap_err();
    assert!(error.to_string().contains("database of its own"), "{error}");
    for wrong in [
        fetching::Options {
            hosts: 0,
            ..options(1)
        },
        fetching::Options {
            hosts: fetching::MAX_HOSTS + 1,
            ..options(1)
        },
        fetching::Options {
            feeds: 0,
            ..options(1)
        },
    ] {
        assert!(fetching::run(&db, wrong).await.is_err());
    }
}
