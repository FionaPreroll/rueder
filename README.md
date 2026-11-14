# rueder

[![CI](https://github.com/FionaPreroll/rueder/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/FionaPreroll/rueder/actions/workflows/ci.yml) [![codecov](https://codecov.io/gh/FionaPreroll/rueder/graph/badge.svg?branch=main)](https://codecov.io/gh/FionaPreroll/rueder)

rueder is a self-hosted feed reader for a handful of people, in the spirit of
Google Reader. It is one Rust server next to PostgreSQL, with a Svelte web page
inside it and an Android app around that page.

[Try the demo](https://fionapreroll.github.io/rueder/demo/) or read the
[documentation](https://fionapreroll.github.io/rueder/): installing it, the
Android app, the REST API, and the code of the server and the web page.

![Temporary project logo: A red cartoon crab holding a newspaper titled 'rueder'](docs/images/logo.png)

<a href="https://fionapreroll.github.io/rueder/demo/">
<picture>
<source media="(prefers-color-scheme: dark)" srcset="https://fionapreroll.github.io/rueder/screenshots/three-panes-dark.png">
<img alt="rueder with three panes: folders and feeds with their unread counts, the list of all items, and an opened article" src="https://fionapreroll.github.io/rueder/screenshots/three-panes-light.png">
</picture>
</a>

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
- **Other apps:** feed reader apps such as NetNewsWire and Read You read and
  change an account through the Google Reader API. Settings, Apps walks you
  through it: pick the app, make an app password for it, and enter the
  address, name and app password the page shows; the page says when the app
  has logged in. Each app has its own app password, which you can revoke.
- **Running it:** local accounts with passwords or login through OpenID
  Connect, open or invitation-only registration, a status page for admins, and
  moving an account between rueder installations as one file.

## Screenshots

The pictures show [the demo](https://fionapreroll.github.io/rueder/demo/)
with its made-up feeds, light or dark as GitHub is for you. CI takes them anew
for every change on `main` ([`web/showcase/`](web/showcase/)), so they show
rueder as it is.

<table>
<tr>
<td width="50%">

<picture>
<source media="(prefers-color-scheme: dark)" srcset="https://fionapreroll.github.io/rueder/screenshots/magazine-dark.png">
<img alt="The magazine view: the newest article large with its picture, the others as cards below it" src="https://fionapreroll.github.io/rueder/screenshots/magazine-light.png">
</picture>

The magazine view, one of four views of a list.

</td>
<td width="50%">

<picture>
<source media="(prefers-color-scheme: dark)" srcset="https://fionapreroll.github.io/rueder/screenshots/versions-dark.png">
<img alt="The versions of an article: the earlier title struck out, removed text in red and added text in green" src="https://fionapreroll.github.io/rueder/screenshots/versions-light.png">
</picture>

An article that changed at its source, compared with what it was.

</td>
</tr>
<tr>
<td width="50%">

<picture>
<source media="(prefers-color-scheme: dark)" srcset="https://fionapreroll.github.io/rueder/screenshots/filter-rules-dark.png">
<img alt="A new filter rule that hides articles whose title contains a word, saying how many articles it applies to" src="https://fionapreroll.github.io/rueder/screenshots/filter-rules-light.png">
</picture>

A filter rule says what it would catch before it is saved.

</td>
<td width="50%">

<picture>
<source media="(prefers-color-scheme: dark)" srcset="https://fionapreroll.github.io/rueder/screenshots/apps-dark.png">
<img alt="Setting up NetNewsWire: what to enter in the app, the server address and a new app password, each with a button to copy it" src="https://fionapreroll.github.io/rueder/screenshots/apps-light.png">
</picture>

Feed reader apps get an app password each, and the page says what to enter.

</td>
</tr>
</table>

<table>
<tr>
<td width="33%">

<picture>
<source media="(prefers-color-scheme: dark)" srcset="https://fionapreroll.github.io/rueder/screenshots/phone-home-dark.png">
<img alt="The start page on a phone: what is new, feed by feed" src="https://fionapreroll.github.io/rueder/screenshots/phone-home-light.png">
</picture>

The start page on a phone.

</td>
<td width="33%">

<picture>
<source media="(prefers-color-scheme: dark)" srcset="https://fionapreroll.github.io/rueder/screenshots/phone-sidebar-dark.png">
<img alt="The sidebar on a phone: the lists, the labels and the folders with their unread counts" src="https://fionapreroll.github.io/rueder/screenshots/phone-sidebar-light.png">
</picture>

Folders and labels.

</td>
<td width="33%">

<picture>
<source media="(prefers-color-scheme: dark)" srcset="https://fionapreroll.github.io/rueder/screenshots/phone-article-dark.png">
<img alt="An article opened on a phone, with its picture" src="https://fionapreroll.github.io/rueder/screenshots/phone-article-light.png">
</picture>

An article.

</td>
</tr>
</table>

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
| [`pages/`](pages/README.md) | The site on GitHub Pages: an overview, the demo and the documentation, put together from what the other parts build. |
| [`docs/`](docs/implementation.md) | Plan, architecture and state of the work, in German. |
| `.github/` | CI, the site on GitHub Pages, CodeQL, gitleaks and Dependabot. |

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
- `pages/`: Node 22 and pnpm; `pnpm lint` and `pnpm test`, and
  `node build.js` once the other parts are built.

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
