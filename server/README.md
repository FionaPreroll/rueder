# rueder server

The new server of rueder: one Rust program (axum, tokio, sqlx) next to PostgreSQL.
It replaces `backend/` once it can do what the plan asks of it. Plan, architecture
and work packages are in [docs/](../docs/implementation.md) (German).

## State: foundation, data, fetching, cleaning, images and login

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
- `GET /api/health`, `POST /api/login`, `GET /api/login/ways`, `POST /api/logout`,
  `GET /api/me`, and
  the built frontend from `../web/build`.
- The container image (`../Dockerfile`) with the server and the frontend, and
  `rueder health`, which asks the running server whether it is well. How to
  install rueder with it is in [deploy/README.md](../deploy/README.md).

This is step 1 of the plan, the first five work packages of step 2 and its
work package 2.10. The API
for feeds, articles and what was read follows.

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

## Accounts and login

Nobody can register. Accounts are made on the command line:

```sh
cargo run -- account add ada            # asks for the password; --admin for an admin
cargo run -- account password ada       # a new password, which ends ada's sessions
cargo run -- account list
```

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
