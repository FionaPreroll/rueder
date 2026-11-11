# rueder web

The new frontend of rueder: SvelteKit (as a single-page app), Svelte 5, Tailwind CSS 4
and TypeScript. It replaces `frontend/` once the Rust server is in place.

## State: connected to the server

The page talks to the Rust server: it logs in, shows the tree of folders and feeds,
the lists, the articles and their earlier versions, and sends back what was read,
starred and sorted and the filter rules. It finds the feeds of a website and
subscribes to them, renames and ends subscriptions, and takes them in and out as an
OPML file. What changes elsewhere, a fetch of the server or another tab, comes in as
it happens over the server's live updates. What comes with later work packages is in
[docs/implementation.md](../docs/implementation.md#schritt-3-frontend).

- `src/lib/reader.ts` is the seam: what the page needs from whatever holds the feeds
  and articles. Components get their data from a `Reader` and nowhere else.
- `src/lib/api/` fills it from the server: `client.ts` has a function per endpoint,
  `reader.svelte.ts` is the reader, `session.svelte.ts` knows who is logged in. A
  change shows at once and is sent in the background; one the server refuses is
  taken back.
- `src/lib/demo/` fills it from made-up feeds and articles that live in the browser,
  the same on every start: for the tests, for `pnpm dev:demo` and for the one-file
  build. Nothing of it is saved.
- The settings (theme, density, language …) still stay in the browser's local
  storage.

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
pnpm build           # the app for the server to serve, in build/
pnpm build:single    # the whole app as one build/index.html
```

`pnpm dev` passes `/api` and `/img` on to a rueder server on this machine (`cargo run`
in `../server`, an account from `cargo run -- account add <name>`), so the page and
the API share one address as in production; `RUEDER_SERVER` names another server.
Logging in needs `http://localhost:5173`, not the machine's name: browsers keep the
session cookie for HTTPS and for localhost only.

`pnpm build:single` packs the reader with made-up data into one HTML file that runs
from any address or straight from the file system, for handing the interface around.
CI attaches it to every run as the artifact `rueder-ui-draft`.

## Conventions

- Texts live in `src/lib/i18n/` (English and German); components read them as
  `m.settings.title`. A new language is a new catalog with the shape of `en.ts`.
- Colors are named for what they are for (`bg-paper`, `text-soft`, `border-line`)
  and defined in `src/routes/layout.css`; components use no fixed grays, so themes
  and accent colors only repaint variables.
- Imports from `src/lib` are written `#lib/…` with the file extension.
