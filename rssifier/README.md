# RSSifier

Makes feeds of web pages that have none, for rueder and other feed readers. A
program of its own next to rueder, without a database: rueder asks it for a
page that names no feed and reads the JSON Feed it answers like any feed from
outside. The design, in German, is under “RSSifier” in
`../docs/architecture.md`.

## What it reads

The first way that finds at least three entries wins:

1. JSON-LD of schema.org: an `ItemList`, or articles in a list or a `@graph`.
2. Microformats: `h-entry` elements, as an `h-feed` has them.

An entry gets a title, its link (which is also its id), a short text without
the page's markup, and an image and a date where the page names them. Pages
made in the browser by scripts are recognized and refused as
`needs-javascript`; there is no headless browser.

## API

Every request but `/health` and `/openapi.json` names a token in
`Authorization: Bearer rsf_…`. The description is `openapi.json`, served at
`GET /openapi.json`.

| Route | Answer |
|---|---|
| `GET /v1/feed?url=<page>` | The page's feed as JSON Feed 1.1. `home_page_url` is the page as RSSifier writes it; with `If-None-Match` an unchanged feed is 304. |
| `GET /v1/usage` | The token's limits and what it used. |
| `GET /health` | `ok`. |

Problems come as `application/problem+json` with a fixed `type`: `token`
(401), `address` (400), `no-items`, `needs-javascript` and `too-large` (422),
`quota` (429, with `Retry-After`), `upstream` (502), `busy` (503, with
`Retry-After`) and `timeout` (504).

## Limits

- **Per token**, each optional: requests per minute, pages fetched per hour
  (requests the cache could not answer) and requests at once. Counted in
  memory; they start anew with the server.
- **For the whole server** (`limits::Pace`): an answer for a page is kept for
  15 minutes for every token, one host is asked one request at a time and at
  most every 2 seconds, 16 pages are fetched and 4 read at once.
- **For each page**: 4 MB, the limits of the HTML parser (`src/html.rs`, the
  same as rueder's cleaning), at most 100 entries with short texts.

Everything goes out through the hardened client of `../hardened-http`.

## Running it

```sh
cargo run -- token add rueder --per-minute 120 --fetches-per-hour 600 --at-once 8
cargo run -- serve                       # on 127.0.0.1:3100
cargo run -- token list
cargo run -- token limit rueder --per-minute 60   # limits not named no longer hold
cargo run -- token revoke rueder         # the server stops taking it at once
```

| Setting | Default | |
|---|---|---|
| `RSSIFIER_TOKENS` | `tokens.json` | The tokens, as SHA-256 with name and limits. `rssifier token` writes it, only readable by its owner; the server reads it again when it changes. |
| `LISTEN_ADDR` | `127.0.0.1:3100` | Where the server listens. |
| `FETCH_ALLOW_NETS` | none | Internal networks that may be fetched from, comma separated; for tests. |
| `RUST_LOG` | `info` | |

As a container, built from the root of the repository:

```sh
docker build -f rssifier/Dockerfile -t rssifier .
docker run --rm -v rssifier:/etc/rssifier rssifier token add rueder
docker run -d -v rssifier:/etc/rssifier -p 127.0.0.1:3100:3100 rssifier
```

## Checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                      # UPDATE_OPENAPI=1 writes openapi.json anew
cargo audit && cargo deny check
```
