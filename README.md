# rueder

[![CI](https://github.com/FionaPreroll/rueder/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/FionaPreroll/rueder/actions/workflows/ci.yml) [![codecov](https://codecov.io/gh/FionaPreroll/rueder/graph/badge.svg?branch=main)](https://codecov.io/gh/FionaPreroll/rueder)

rueder is a self-hosted feed reader for a handful of people, in the spirit of
Google Reader. It is one Rust server next to PostgreSQL, with a Svelte web page
inside it and an Android app around that page.

![Temporary project logo: A red cartoon crab holding a newspaper titled 'rueder'](docs/images/logo.png)

- **Reading:** a dense list, an expanded view, three panes (tree, list,
  article) and a magazine view; keyboard shortcuts, dark mode, a layout for
  phones, and settings for theme, accent color, density, font and sorting.
- **Feeds and folders:** RSS, Atom and JSON Feed; folders up to five levels
  deep, rearranged by drag and drop or keyboard; finding the feed on a web page;
  OPML import and export.
- **Unread counts that stay right:** what each account has read lives on the
  server, and every count is worked out from it, never kept as a counter.
- **Live updates:** new articles, reading on another device and changes to the
  folders reach every open tab at once.
- **Versions:** when a feed changes an article, rueder keeps up to 20 versions
  of it and marks the article as updated without making it unread again.
- **Filters, stars and labels:** rules per account on keywords, authors and
  other fields that hide or mark articles as they arrive; stars, colored labels
  and a search over your own subscriptions.
- **Privacy:** the server removes ads, trackers and unsafe markup from every
  article before storing it, and images come through its own image proxy, so
  the browser talks to nothing but rueder.
- **Running it:** local accounts with passwords or login through OpenID
  Connect, open or invitation-only registration, a status page for admins, and
  moving an account between rueder installations as one file.

## Installing

rueder runs as one container next to PostgreSQL, behind a reverse proxy that
holds the TLS certificate. CI builds and tests the image for every change on
`main` and publishes it as `ghcr.io/fionapreroll/rueder:latest`.

```sh
git clone https://github.com/FionaPreroll/rueder.git
cd rueder/deploy
cp .env.example .env    # set POSTGRES_PASSWORD
docker compose up --detach --build
docker compose exec rueder rueder account add <name> --admin
```

[deploy/README.md](deploy/README.md) has the whole of it: the reverse proxy,
settings, registration, updates, backups, checking an installation and running
without Docker. The Android app comes from the release
[`android-latest`](https://github.com/FionaPreroll/rueder/releases/tag/android-latest);
it asks for the address of your rueder at its first start.

## Repository

| Path | What |
|---|---|
| [`server/`](server/README.md) | The server in Rust (axum, tokio, sqlx): API, fetching, cleaning, image proxy, accounts and the command line, in one program. |
| [`web/`](web/README.md) | The web page: SvelteKit as a single-page app, Svelte 5, Tailwind CSS and TypeScript. The server serves it. |
| [`app/`](app/README.md) | The Android app, Capacitor around a start page that opens your rueder. |
| [`deploy/`](deploy/README.md), `Dockerfile` | The container image, the compose file with PostgreSQL, and how to install it. |
| [`docs/`](docs/implementation.md) | Plan, architecture and state of the work, in German. |
| `.github/` | CI, CodeQL, gitleaks and Dependabot. |

## Developing

Each part has its own README with its commands and the checks to run before a
push:

- `server/`: Rust (stable) and PostgreSQL 16. `cargo run` serves on
  `http://127.0.0.1:3000`; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
  and `cargo test`.
- `web/`: Node 22 and pnpm. `pnpm dev` talks to the server above,
  `pnpm dev:demo` runs with made-up data and no server; `pnpm lint`,
  `pnpm check`, `pnpm test`, `pnpm build`, and `pnpm e2e` for the tests in a
  browser against the real server.
- `app/`: Node 22, pnpm, Java 21 and the Android SDK.

How the system is built and why, what is done and what is still open is written
down in German:

- [Plan](docs/plan.md): goals, requirements, decisions and the four steps
- [Architecture](docs/architecture.md): how the system is built, data model,
  security
- [Implementation](docs/implementation.md): where the work stands, conventions,
  measurements and the open work packages, for anyone taking over

[CLAUDE.md](CLAUDE.md) sums up the rules of the project in English.

## Security

Please report a vulnerability privately, as [SECURITY.md](SECURITY.md)
describes, and not in a public issue.

## The old rueder

rueder used to be five Go services with a separate Svelte frontend, loginsrv
and imgproxy. The rewrite replaced it in November 2026, and its code was removed
from this repository. To read it, check out commit `0b36a96`, the last one
where it still built and ran. Its data comes over once with
`rueder import-legacy`, as
[deploy/README.md](deploy/README.md#bringing-over-the-old-rueder) describes.
