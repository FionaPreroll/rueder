# rueder web

The new frontend of rueder: SvelteKit (as a single-page app), Svelte 5, Tailwind CSS 4
and TypeScript. It replaces `frontend/` once the Rust server is in place.

## State: a draft to click through

For now this is a draft of the interface, to settle how the reader looks and feels
before the server exists. There is no backend behind it:

- Feeds, articles and their earlier versions are made up (`src/lib/demo/data.ts`),
  the same on every start.
- `src/lib/demo/reader.svelte.ts` stands in for the server. It holds what was read,
  the filter rules and the tree of folders in the browser, and lets new articles
  "arrive" on a timer.
- Nothing is saved except the settings (theme, density, language …), which stay in
  the browser's local storage.

What already is the real thing, and what the server will follow:

- Unread counts (`src/lib/unread.ts`): a mark per feed ("everything up to article N
  is read") plus the exceptions, so counts are derived and cannot drift.
- Filter rules (`src/lib/filters.ts`): hide or mark as read by title, text, author
  or tag, for all feeds, a folder or one feed.
- The tree of folders (`src/lib/tree.ts`): sorted by drag and drop or with Alt and
  the arrow keys, nested at most five levels deep.
- Comparing versions of an article (`src/lib/diff.ts`).

## Commands

Node 22 and the pnpm version pinned in `package.json` (`packageManager`).

```sh
pnpm install
pnpm dev             # the draft at http://localhost:5173
pnpm lint            # prettier --check
pnpm check           # svelte-check and TypeScript
pnpm test            # Vitest, tests sit next to the code as *.test.ts
pnpm test:coverage
pnpm build           # the app for the server to serve, in build/
pnpm build:single    # the whole app as one build/index.html
```

`pnpm build:single` packs everything into one HTML file that runs from any address
or straight from the file system, for handing the draft around. CI attaches it to
every run as the artifact `rueder-ui-draft`.

## Conventions

- Texts live in `src/lib/i18n/` (English and German); components read them as
  `m.settings.title`. A new language is a new catalog with the shape of `en.ts`.
- Colors are named for what they are for (`bg-paper`, `text-soft`, `border-line`)
  and defined in `src/routes/layout.css`; components use no fixed grays, so themes
  and accent colors only repaint variables.
- Imports from `src/lib` are written `#lib/…` with the file extension.
