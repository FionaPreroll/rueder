# rueder server

The new server of rueder: one Rust program (axum, tokio, sqlx) next to PostgreSQL.
It replaced the old Go backend. Plan, architecture and work packages are in
[docs/](../docs/implementation.md) (German).

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
  first. A GIN index over `article_content` holds the words. A search first
  looks at the newest few hundred articles, and from how common the words are
  among them chooses whether to go on looking at articles one by one or to ask
  the index, in growing windows of the order of arrival. After half a second
  it says where to go on.
- Moving an account (`src/transfer.rs`): `GET /api/export` gives out everything
  a reader made of their account as one JSON file (settings, folders, feeds,
  what was read, stars, labels, rules), and `POST /api/import` takes such a
  file in, from this rueder or another one. Feeds are found again by their
  address and articles by their id in the feed, as the numbers differ from one
  server to the next. `rueder account export <name>` and
  `rueder account import <name> <file>` do the same on the command line.
- Deleting and disabling an account (`src/accounts.rs`, `src/profile.rs`):
  `DELETE /api/account` deletes the reader's own account after their password,
  and `rueder account remove <name>` any account; everything that is the
  account's own goes with it, the feeds stay for their other readers.
  `rueder account disable <name>` keeps a misused account from logging in and
  ends its sessions; `enable` undoes it.
- Apps of other makers (`src/greader/`, `src/app_passwords.rs`): the Google
  Reader API under `/greader`, for apps such as NetNewsWire and Read You,
  logged in with app passwords that readers make under Settings, Apps
  (`/api/app-passwords`), never with the account's password. `APP_API=off`
  switches it off.
- Passkeys (`src/passkeys.rs`): with `PUBLIC_URL` set to the address browsers
  reach rueder at, readers add passkeys under Account settings, Security
  (`/api/passkeys`) and log in with them without name or password
  (`/api/login/passkey`). A passkey counts as a second factor: once an account
  has one, its password alone opens nothing, and wherever a change asks for the
  password and a code, a passkey stands in for both. The checking is
  `webauthn-rs`'s; `.cargo/audit.toml` says why `rsa`'s advisory does not
  touch rueder.
- Reports and blocked feeds (`src/moderation.rs`): with `CONTENT_REPORTS=on`
  readers report a feed or an article with a reason and a note, at most 20 a
  day; the status page lists the reported feeds with their address, never who
  reported. `rueder feed block <address>` stops fetching the feed, hides its
  articles from everybody, refuses new subscriptions and removes its images
  from the cache; `rueder feed unblock <address>` lifts it.
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

- Fuzz targets for the cleaning, the reading of feeds, the image proxy and the
  requests of the Google Reader API
  (`fuzz/`), whose corpus runs as an ordinary test, and a soak test of the
  whole program under the load of readers and fetching (`tests/soak.rs`).

This is step 1 of the plan and all of step 2, the security review (2.12)
included; its findings are in `docs/architecture.md`.

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

Accounts are made on the command line, or by an admin with `POST /api/accounts`.
Nobody can register or invite until the operator says so (`registration`,
below):

```sh
cargo run -- account add ada            # asks for the password; --admin for an admin
cargo run -- account password ada       # a new password, which ends ada's sessions
cargo run -- account list
cargo run -- account disable ada        # ada cannot log in, her sessions end; `enable` lets her again
cargo run -- account remove ada         # deletes ada and all that is hers, for good; feeds stay
cargo run -- registration show          # who may make an account, the limits, how many there are
cargo run -- registration set --open true --max-accounts 50 --per-day 5
cargo run -- registration set --invites 3 --invite-days 14 --admin-invite-accounts 20
cargo run -- account invites ada 10     # ada's own quota of invitations; `default` for everybody's
cargo run -- admin add ada              # ada sees the status of the server; remove, list
cargo run -- retention show             # how long the counts of the status page are kept
cargo run -- retention set --status-days 30
cargo run -- account second-factor-off ada   # ada lost the app and the recovery codes
cargo run -- feed block https://spam.example/feed.xml     # for everybody; `unblock` lifts it
```

Admins see the status of the server in the account menu of the page
(`GET /api/status`, `src/monitor.rs`, `src/status.rs`): how long requests take
by route, the database, fetching, the feed search, the image proxy and the
warnings and errors of the log, for the last hour or, if asked, longer. It
counts in memory and writes the counts to the database every minute and when
it stops (`src/history.rs`), so they survive a restart; they are kept for 7
days unless `retention set --status-days` says otherwise. It counts failing
feeds by why and names none.

Readers who agree in the settings (Privacy) send reports of errors in their
browser to `POST /api/reports` (`src/reports.rs`): scrubbed of addresses and
numbers, counted per error, kept 30 days, and shown on the status page with how
many accounts they hit, never which. `CLIENT_REPORTS=off` takes none.

Without a terminal the password is read as one line from the input, for
scripts. The session cookie is `Secure`, so logging in needs HTTPS in front of
the server; `http://localhost:3000` works for development.

Wrong passwords are counted per name, per address and per network (an IPv6
/48 or an IPv4 /24). Behind a reverse proxy set `CLIENT_IP_HEADER` to the
header the proxy puts the sender's address in (e.g. `X-Forwarded-For`), or all
readers count as one address. Do not set it if no proxy sets that header:
senders could then name any address.

For the login through an OpenID Connect provider set `OIDC_ISSUER`,
`OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET` and `OIDC_REDIRECT_URL` (the address of
`/api/oidc/callback` as browsers reach it), and send browsers to
`/api/oidc/start`. The provider must be reached over https. Logging in there
makes no account: tie an identity to an account with
`cargo run -- account link ada <subject>`, or log in with the password and open
`/api/oidc/start?link` from a page of rueder or by typing it (a link on another
site is refused). A provider inside your own network needs
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
cargo run --release -- import-legacy --from postgres://user:password@host/rueder --dry-run
cargo run --release -- import-legacy --from postgres://user:password@host/rueder
```

The old database is only read. The address can also come from
`LEGACY_DATABASE_URL`. The import merges the old data with what the new rueder
already has: an old feed goes into the feed with the same address, its old
articles before the ones fetched since, and an old user into the account with
the same name. `--account <old name>=<new name>` sends an old user to an account
of another name; it may be given more than once. Old users without an account
get one without a password, which `account password` sets.

Stop the server and back up its database first: the import moves the numbers of
articles in merged feeds, which must not happen while it runs. `--dry-run` says
which feeds and accounts would be merged and which added, and writes nothing.
At the end the import prints what it merged, added and left out. If it stops
halfway, run it again: it carries on with what is missing. The details are in
[docs/implementation.md](../docs/implementation.md#einmaliger-import-in-den-laufenden-bestand).

## Measuring

```sh
cargo run --release -- bench generate   # 10 accounts, 300 feeds, 10 million articles
cargo run --release -- bench measure    # times the queries, prints a table
```

`bench generate` wants an empty database and takes `--accounts`, `--feeds` and
`--articles`; 10 million articles take about 50 minutes, 30 of them for the
search index, and 21 GB. `bench measure`
puts back what its writing queries changed, so it can run again.

```sh
FETCH_ROUND=1000 FETCH_AT_ONCE=64 FETCH_EVERY=10 \
  cargo run --release -- bench fetch --feeds 5000 --hosts 250 --seconds 300
```

`bench fetch` measures how many feeds the server fetches in an hour at the pace
the three settings give. It serves made-up feeds itself, under `--hosts` names
out of 127.0.0.0/8 (one fetch runs per name), on `--port` (39871), and fetches
them round after round, every feed due again before each round. It wants a
database of its own, where nobody reads other feeds.

Results and how they were measured are in
[docs/implementation.md](../docs/implementation.md#messung).

## Fuzzing

The fuzz targets in `fuzz/` throw any bytes at the cleaning, at the reading of
feeds, at the image proxy and at the reading of requests of the Google Reader
API, and check what
[docs/architecture.md](../docs/architecture.md#fuzzing) says must hold for
every input: no panic, no script or foreign image address in a cleaned text,
the limits of a feed, only the formats and sizes the proxy lets through, IDs
of the Google Reader API that read back as themselves. They run on GitHub
(`.github/workflows/fuzz.yml`): three minutes each on a pull request that
touches them, half an hour each on Mondays, and as long as asked when started
by hand; a finding fails the run and comes as an artifact. By hand they need
nightly Rust and `cargo install cargo-fuzz`:

```sh
cd fuzz
cargo +nightly fuzz run -O cleaning -- -max_total_time=1800 -timeout=10 -max_len=131072
cargo +nightly fuzz run -O feed -- -max_total_time=1800 -timeout=10 -max_len=131072
cargo +nightly fuzz run -O image -- -max_total_time=1800 -timeout=10
cargo +nightly fuzz run -O greader -- -max_total_time=1800 -timeout=10
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
