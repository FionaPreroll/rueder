# rueder web

The new frontend of rueder: SvelteKit (as a single-page app), Svelte 5, Tailwind CSS 4
and TypeScript. It replaced the old frontend.

## State: connected to the server

The page talks to the Rust server: it logs in, shows the tree of folders and feeds,
the lists, the articles and their earlier versions, and sends back what was read,
starred and sorted and the filter rules. It finds the feeds of a website and
subscribes to them, renames and ends subscriptions, and takes them in and out as an
OPML file; admins open the status of the server from the account menu. What changes
elsewhere, a fetch of the server or another tab, comes in as it happens over the
server's live updates. It installs as an app (PWA): its service worker keeps the
app's own files, never the reader's data, and offers to reload when the server has
a new version. The work packages, done and open, are in
[docs/implementation.md](../docs/implementation.md#arbeitspakete).

- `src/lib/reader.ts` is the seam: what the page needs from whatever holds the feeds
  and articles. Components get their data from a `Reader` and nowhere else.
- `src/lib/api/` fills it from the server: `client.ts` has a function per endpoint,
  `reader.svelte.ts` is the reader, `session.svelte.ts` knows who is logged in. A
  change shows at once and is sent in the background; one the server refuses is
  taken back.
- `src/lib/demo/` fills it from made-up feeds and articles that live in the browser,
  the same on every start: for the tests, for `pnpm dev:demo` and for the one-file
  build. Nothing of it is saved.
- Where the reader was (the list shown, the open article and how far the sidebar, the
  list and the article were scrolled) is kept in the browser's local storage
  (`src/lib/view.ts`), so a reload starts there again; logging out forgets it.
- The settings (theme, density, language, keys of one's own …) are kept with the
  account through the server's reader, and in the browser's local storage, so the
  page looks right before the server answers. The draft keeps them in the browser
  only.

The models the server follows:

- Unread counts (`src/lib/unread.ts`): a mark per feed ("everything up to article N
  is read") plus the exceptions, so counts are derived and cannot drift.
- Filter rules (`src/lib/filters.ts`): hide or mark as read by title, text, author
  or tag, for all feeds, a folder or one feed.
- The tree of folders (`src/lib/tree.ts`): sorted by drag and drop or with Alt and
  the arrow keys, nested at most five levels deep.
- Comparing versions of an article (`src/lib/diff.ts`).
- The types of the server's API (`src/lib/api/types.ts`), which `client.ts` takes
  its shapes from. The server writes this file out of its own types, see
  [server/README.md](../server/README.md): it is not edited by hand and not
  formatted by Prettier.

## Commands

Node 22 and the pnpm version pinned in `package.json` (`packageManager`).

```sh
pnpm install
pnpm dev             # the page at http://localhost:5173, with the server on 127.0.0.1:3000 behind it
pnpm dev:demo        # the same without a server, with made-up data
pnpm lint            # prettier --check
pnpm check           # svelte-check and TypeScript
pnpm test            # Vitest, tests sit next to the code as *.test.ts
pnpm test:coverage
pnpm e2e             # end-to-end tests in a browser against the real server
pnpm build           # the app for the server to serve, in build/
pnpm build:single    # the whole app as one build/index.html
pnpm build:docs      # the documentation of src/lib with TypeDoc, in build-docs/
```

`pnpm dev` passes `/api` and `/img` on to a rueder server on this machine (`cargo run`
in `../server`, an account from `cargo run -- account add <name>`), so the page and
the API share one address as in production; `RUEDER_SERVER` names another server.
Logging in needs `http://localhost:5173`, not the machine's name: browsers keep the
session cookie for HTTPS and for localhost only.

`pnpm build:single` packs the reader with made-up data into one HTML file that runs
from any address or straight from the file system, for handing the interface around.
CI attaches it to every run as the artifact `rueder-ui-draft`, and it is the demo on
GitHub Pages, at https://fionapreroll.github.io/rueder/demo/, next to what
`pnpm build:docs` writes ([pages/README.md](../pages/README.md)). Files from `static/`
are therefore named with `asset()` from `$app/paths`, never from the top of the site.

## End-to-end tests

`pnpm e2e` runs the built page in Chromium against the real server (`e2e/`, with
Playwright): logging in, subscribing and reading, folders, filter rules, live
updates, and that one account sees nothing of another. It needs

- the server's binary: `cargo build` in `../server`, or `RUEDER_BIN` names it,
- the built page: `pnpm build`,
- a PostgreSQL that `DATABASE_URL` names, with an account that may create
  databases, and `psql`,
- the browser, once: `pnpm exec playwright install chromium`.

Every run makes the database `rueder_e2e` anew, starts the server on
`localhost:3210` and a made-up website with a few feeds on `127.0.0.1:3211`
(`e2e/harness/`). Each spec logs in as an account of its own. `pnpm e2e reading`
runs one spec, `pnpm e2e --ui` shows the browser.

## Screenshots

`pnpm screenshots` takes a picture of every view, for looking at the design: the
login, the reader with its lists and views, menus, dialogs, the settings, the
account's settings and the status of the server, on a desktop and on a phone,
light and dark, in English and German (`SCREENSHOT_LANGUAGES=de` for one). It
needs what `pnpm e2e` needs and runs the same way, against the real server, but
with the made-up feeds of the draft (`src/lib/demo/data.ts`), which a made-up
website serves as RSS with their pictures (`screenshots/site.ts`). The pictures
land in `screenshots/out/<language>-<device>-<theme>/`, numbered in the order of
the walk through the views (`screenshots/views.spec.ts`); a long dialog gives one
picture per screen. On GitHub the workflow Screenshots does the same when started
by hand, and for a pull request that changes how the pictures are taken, and
attaches the pictures to its run as the artifact `screenshots`.

Next to each picture the walk notes what can be measured of the page: the
size of every target that can be pressed and the characters to a line of
article text (`measure.jsonl` in each folder). `pnpm screenshots:report` adds
it up in `screenshots/out/measure.md`, against the measure of the design review
(`../docs/design_review.md`): 44 by 44 pixels on a phone, 24 by 24 elsewhere
unless there is room around a target as WCAG 2.5.8 asks, at most 80 characters
to the line. The colors are checked by
`src/routes/colors.test.ts` with the other tests.

## The pictures in the README

`pnpm showcase`, after `pnpm build:single`, takes the pictures the README at
the top of the repository shows: what rueder does best, in the demo with its
English made-up feeds, on a desktop and on a phone, light and dark. Each
picture is a test of its own in `showcase/showcase.spec.ts`; the test serves the
single-file build to the browser itself, so nothing else has to run, and keeps
the demo's interval of new made-up articles from starting, so that none
arrives in the middle of a picture.
The pictures land in `showcase/out/` as `<picture>-light.png` and
`<picture>-dark.png`. The workflow Pages takes them for every pull request and
publishes them with the site under `/rueder/screenshots/`, which is where the
README finds them; they are never committed. A new picture gets its name in the
README too.

## Conventions

- Texts live in `src/lib/i18n/` (English and German); components read them as
  `m.settings.title`. A new language is a new catalog with the shape of `en.ts`.
- Colors are named for what they are for (`bg-paper`, `text-soft`, `border-line`)
  and defined in `src/routes/layout.css`; components use no fixed grays, so themes
  and accent colors only repaint variables.
- Imports from `src/lib` are written `#lib/…` with the file extension.
