# rueder server

The new server of rueder: one Rust program (axum, tokio, sqlx) next to PostgreSQL.
It replaces `backend/` once it can do what the plan asks of it. Plan, architecture
and work packages are in [docs/](../docs/implementation.md) (German).

## State: foundation, data, fetching, cleaning, images, login, the API, its hardening and the search

What is here:

- The schema (`migrations/`, comments in German).
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
- The image proxy (`src/images.rs`): `/img/<signature>/<size>/<address>` serves
  the images of articles, so readers' browsers ask rueder and not the sites.
  Addresses are signed, only PNG, JPEG, GIF and WebP pass, large images are
  made to fit, and what was fetched is kept in a cache on disk.
- Accounts, sessions and the login (`src/accounts.rs`, `src/sessions.rs`,
  `src/login.rs`, `src/oidc.rs`): passwords as Argon2id hashes, a session cookie
  scripts cannot read, a limit on wrong passwords, and optionally the login
  through an OpenID Connect provider. The image proxy answers logged-in
  readers only.
- Made-up data in the size of a real installation and timings of these queries
  (`src/bench.rs`).
- The API under `/api` (`src/api.rs`, `src/tree.rs`, `src/labels.rs`,
  `src/filters.rs`, `src/subscribe.rs`, `src/profile.rs`): the tree with unread
  counts, lists, articles and their versions, what was read and starred,
  labels, filter rules with a preview, subscribing with a search for the feeds
  of a web page, OPML in and out, settings, the own sessions, and accounts for
  admins. Every query is bound to the account of the session; the list of
  endpoints is in [docs/architecture.md](../docs/architecture.md#api).
- The search (`src/search.rs`): `GET /api/search` finds the articles of the
  own feeds that have all the words, or words that begin with them, newest
  first. A GIN index over `article_content` holds the words; the search goes
  back in growing windows of the order of arrival, so that a common word costs
  no more than a rare one, and says where to go on after half a second.
- Live updates (`src/events.rs`): `GET /api/events` tells an account's open
  tabs what changed, as server-sent events.
- The API's types for the frontend (`src/typescript.rs`): every type that is
  sent or read as JSON is written to `../web/src/lib/api/types.ts`, and a test
  fails when that file is out of date.
- Hardening (`src/hardening.rs`, `src/rate.rs`): every answer carries a
  Content-Security-Policy and the other security headers, a request that
  changes something is refused when a page of another site sent it, each
  address may send only so many requests, and each account has limits for
  feeds, folders, rules, labels and sessions. See
  [docs/architecture.md](../docs/architecture.md#header-herkunft-und-limits).
- The built frontend from `../web/build`. The policy names the scripts inside
  its `index.html` by their hash, read when the server starts: restart the
  server after building the frontend anew.
- The container image (`../Dockerfile`) with the server and the frontend, and
  `rueder health`, which asks the running server whether it is well. How to
  install rueder with it is in [deploy/README.md](../deploy/README.md).

- Fuzz targets for the cleaning, the reading of feeds and the image proxy
  (`fuzz/`), whose corpus runs as an ordinary test, and a soak test of the
  whole program under the load of readers and fetching (`tests/soak.rs`).

This is step 1 of the plan and work packages 2.1 to 2.11 and 2.13 of step 2.
The security review (2.12) follows.

## Commands

Rust (stable) and PostgreSQL 16.

```sh
cp .env.example .env                 # DATABASE_URL
cargo run                            # applies the migrations, serves on http://127.0.0.1:3000
cargo test                           # creates a database per test on the server of DATABASE_URL
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo sqlx prepare -- --all-targets  # after changing a query: renew the cache in .sqlx/
cargo audit && cargo deny check      # after changing dependencies: advisories, licenses, sources
UPDATE_API_TYPES=1 cargo test --test api_types   # after changing a type of the API: write the frontend's types anew
```

Queries are checked against the schema when compiling. Without a database the
build uses the cache in `.sqlx/` (`SQLX_OFFLINE=true`, as in CI); `cargo sqlx` comes
with `cargo install sqlx-cli`. `cargo audit` and `cargo deny` come with
`cargo install cargo-audit cargo-deny`; what they allow is in `deny.toml`, and an
advisory that does not apply is listed with its reason in `.cargo/audit.toml`.

A type that goes over the API as JSON derives `TS` next to `Serialize` or
`Deserialize`, and its module lists it in its function `types`. In TypeScript
all types share one room: where the Rust name says little outside its module,
`#[ts(rename = "...")]` gives the type a name of its own there. The comment
on a type goes into the file with it and names the endpoint the type is for.

Settings come from the environment or `.env`: `DATABASE_URL`, `LISTEN_ADDR`
(default `127.0.0.1:3000`) and `STATIC_DIR` (default `../web/build`).

## Accounts and login

Nobody can register. Accounts are made on the command line, or by an admin
with `POST /api/accounts`:

```sh
cargo run -- account add ada            # asks for the password; --admin for an admin
cargo run -- account password ada       # a new password, which ends ada's sessions
cargo run -- account list
cargo run -- admin add ada              # ada sees the status of the server; remove, list
```

Admins see the status of the server in the account menu of the page
(`GET /api/status`, `src/monitor.rs`, `src/status.rs`): how long requests take
by route, the database, fetching, the feed search, the image proxy and the
warnings and errors of the log, for the last hour, kept in memory. It counts
failing feeds by why and names none.

Without a terminal the password is read as one line from the input, for
scripts. The session cookie is `Secure`, so logging in needs HTTPS in front of
the server; `http://localhost:3000` works for development.

Wrong passwords are counted per name and per address. Behind a reverse proxy
set `CLIENT_IP_HEADER` to the header the proxy puts the sender's address in
(e.g. `X-Forwarded-For`), or all readers count as one address. Do not set it if
no proxy sets that header: senders could then name any address.

For the login through an OpenID Connect provider set `OIDC_ISSUER`,
`OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET` and `OIDC_REDIRECT_URL` (the address of
`/api/oidc/callback` as browsers reach it), and send browsers to
`/api/oidc/start`. The provider must be reached over https. Logging in there
makes no account: tie an identity to an account with
`cargo run -- account link ada <subject>`, or log in with the password and open
`/api/oidc/start?link`. A provider inside your own network needs
`FETCH_ALLOW_NETS`.

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

## Live updates

`GET /api/events` is a stream of server-sent events for the logged-in reader:
new articles, changes made in another tab or on another device, feeds that
start or stop failing. What the events are and what a browser does with each
is in [docs/architecture.md](../docs/architecture.md#live-updates).

A reverse proxy in front of the server must pass the stream on as it comes.
The server says so in the header `X-Accel-Buffering: no`, which nginx heeds;
other proxies need buffering switched off for this path.

## Images

The proxy keeps what it fetched in `IMAGE_CACHE_DIR` (default `image-cache` in
the working directory), up to `IMAGE_CACHE_MB` megabytes (default 1024); beyond
that, the files unused longest go. This directory is the only place the server
writes files to.

The addresses of the proxy are signed with `IMAGE_PROXY_KEY`, a secret of 32
characters or more. If there is none, the server makes one on first start and
keeps it in the file `key` of the cache directory. Losing the key loses
nothing: the addresses are made anew whenever an article is delivered.

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

## Fuzzing

The fuzz targets in `fuzz/` throw any bytes at the cleaning, at the reading of
feeds and at the image proxy, and check what
[docs/architecture.md](../docs/architecture.md#fuzzing) says must hold for
every input: no panic, no script or foreign image address in a cleaned text,
the limits of a feed, only the formats and sizes the proxy lets through. They
need nightly Rust and `cargo install cargo-fuzz`:

```sh
cd fuzz
cargo +nightly fuzz run -O cleaning -- -max_total_time=1800 -timeout=10 -max_len=131072
cargo +nightly fuzz run -O feed -- -max_total_time=1800 -timeout=10 -max_len=131072
cargo +nightly fuzz run -O image -- -max_total_time=1800 -timeout=10
cargo +nightly fuzz cmin -O feed     # afterwards: keep only what adds coverage
```

What a target checks is in `fuzz/checks.rs`, which `tests/fuzz_corpus.rs` also
uses: `cargo test` plays the stored corpus in `fuzz/corpus/` with stable Rust.
An input that broke a property goes into the corpus, under a name that says
what it found, once it is fixed.

## Soak test

`tests/soak.rs` starts the server program on a database of its own, with a
small server of made-up feeds and images next to it, and lets six made-up
readers read for a while: lists, articles with their images, marking read,
rules, the event stream, logging in and out. Every request has to get the
answer it should, the unread counts have to agree with the lists, the log has
to stay free of errors, and the server has to stop cleanly when told to.
`cargo test` runs it for 20 seconds. A run of five minutes or more also checks
that memory, open files, threads and response times stay level:

```sh
SOAK_SECONDS=3600 cargo test --release --test soak -- --nocapture   # prints a sample every half minute
```
