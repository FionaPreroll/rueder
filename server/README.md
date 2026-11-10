# rueder server

The new server of rueder: one Rust program (axum, tokio, sqlx) next to PostgreSQL.
It replaces `backend/` once it can do what the plan asks of it. Plan, architecture
and work packages are in [docs/](../docs/implementation.md) (German).

## State: foundation, data, fetching and cleaning

What is here:

- The schema (`migrations/0001_initial.sql`, comments in German).
- The article store (`src/articles.rs`): three tables, so that lists never read
  text and old versions never touch what new articles are read from. One query
  serves the lists of a feed, a folder and all feeds.
- What an account has read (`src/reading.rs`): a mark per subscription plus the
  exceptions; unread counts are worked out from them, never stored.
- The import of the old rueder's database (`src/import.rs`): feeds, articles,
  accounts, folders and subscriptions. It only adds what is missing, so it can
  run more than once.
- The hardened HTTP client (`src/http_client.rs`), the one way out to the
  internet: only http and https, no address inside the own network however it is
  written or reached, limits for time, redirects and size.
- Fetching (`src/feeds.rs`, `src/fetch.rs`): `serve` fetches the feeds that are
  due and have a reader, with conditional requests, and stores new articles and
  new versions of changed ones.
- Cleaning (`src/cleaning.rs`): what an article's text keeps for reading. Links
  and images get whole addresses without tracking parameters, share buttons and
  counting pixels go, audio and video become enclosures, and a fixed list says
  which markup may stay. `reclean` does it again for what is stored.
- Made-up data in the size of a real installation and timings of these queries
  (`src/bench.rs`).
- `GET /api/health`, and the built frontend from `../web/build`.

This is step 1 of the plan and the first three work packages of step 2. The
image proxy, login and the API follow.

## Commands

Rust (stable) and PostgreSQL 16.

```sh
cp .env.example .env                 # DATABASE_URL
cargo run                            # applies the migrations, serves on http://127.0.0.1:3000
cargo test                           # creates a database per test on the server of DATABASE_URL
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo sqlx prepare -- --all-targets  # after changing a query: renew the cache in .sqlx/
```

Queries are checked against the schema when compiling. Without a database the
build uses the cache in `.sqlx/` (`SQLX_OFFLINE=true`, as in CI); `cargo sqlx` comes
with `cargo install sqlx-cli`.

Settings come from the environment or `.env`: `DATABASE_URL`, `LISTEN_ADDR`
(default `127.0.0.1:3000`) and `STATIC_DIR` (default `../web/build`).

## Cleaning

The lists the cleaning works with are in `rules/`: `blocked-links.txt` (domains
whose links and images are left out) and `tracking-parameters.txt` (parameters
that are taken out of addresses), one rule per line. They are built into the
program. To use lists of your own, put both files into a directory and name it
in `CLEANING_RULES_DIR`.

```sh
cargo run --release -- reclean            # clean all stored articles again
cargo run --release -- reclean --feed 12  # only the articles of one feed
```

Every version of an article is kept as it was fetched, so `reclean` brings
stored articles up to the rules and lists of today. It changes neither versions
nor what anybody has read.

## Importing the old database

```sh
cargo run --release -- import-legacy --from postgres://user:password@host/rueder
```

The old database is only read. The address can also come from
`LEGACY_DATABASE_URL`. Stop the new server's fetching while the import runs. At
the end it prints what it added and what it left out. What it brings over and
what a second run does is in
[docs/implementation.md](../docs/implementation.md#import-der-alten-daten).

## Measuring

```sh
cargo run --release -- bench generate   # 10 accounts, 300 feeds, 10 million articles
cargo run --release -- bench measure    # times the queries, prints a table
```

`bench generate` wants an empty database and takes `--accounts`, `--feeds` and
`--articles`; 10 million articles take about 17 minutes and 18 GB. `bench measure`
puts back what its writing queries changed, so it can run again. Results and how
they were measured are in
[docs/implementation.md](../docs/implementation.md#messung).
