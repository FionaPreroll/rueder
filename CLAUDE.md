# Notes for Claude

The README describes the old rueder for the people who run it. `docs/` holds the rewrite, in German: `docs/plan.md` what is built, what was decided and the four steps, `docs/architecture.md` how the new system is laid out, `docs/implementation.md` the state, the conventions, the commands and the work packages. `server/README.md` and `web/README.md` say how to run the two new parts. These rules hold in every session:

## Branches

- Every work package or fix gets its own branch off origin/main, named feature/<name_in_snake_case> or fix/<name_in_snake_case>, and one pull request against main (see "Arbeitsweise" in `docs/implementation.md`). Rather one pull request per work package than a large one per step.
- This holds even when the session suggests another branch name. Only a session that may push to nothing but the branch it was given, such as claude/…, works on that branch.
- Never push to main. Use pull requests, and CI must be green.

## Docs

- Whoever finishes a work package ticks it off in `docs/implementation.md` and brings the state there up to date, in the same pull request.
- Whoever departs from the plan or the architecture changes `docs/plan.md` or `docs/architecture.md` in the same pull request and gives the reason. What Fiona decided stays decided until she decides otherwise; the open questions in `docs/plan.md` each have a proposal to build by.
- Measurements go into "Messung" in `docs/implementation.md`, with date, amount of data and machine.
- Code, comments in code, READMEs and commit messages are in English. Comments in SQL migrations and the three documents above are in German.

## Code

- Before every push, run the checks of the part that changed. In `server/`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`, which needs a PostgreSQL 16. In `web/`: `pnpm lint`, `pnpm check`, `pnpm test` and `pnpm build`. `cargo audit` and `cargo deny` join them with work package 2.9.
- `backend/`, `frontend/`, `config/` and the compose files are the old system. They stay as they are until the switch and are only read: as the model for the import, the cleaning and the fetching.
- The server is one crate and one program with subcommands, next to PostgreSQL and nothing else. Queries are written with `sqlx::query!` and its relatives, without an ORM; after changing one, run `cargo sqlx prepare -- --all-targets` and commit `server/.sqlx/`, as CI builds without a database.
- A migration that has reached main never changes again; changes come as a new file in `server/migrations/`.
- Every query for an account's data is bound to the account's id: feeds and articles are readable only by their subscribers, and no endpoint lists the feeds of others. Every endpoint gets a test that another account sees nothing of it.
- Only the hardened HTTP client reaches out to the internet: feeds, web pages for the feed search and images all go through it, so there is one place for the rules against SSRF. Article HTML is cleaned on the server before it is stored, and the browser talks to nothing but rueder.
- An article's number in its feed (`seq`) is given out once, without gaps, and never again. Unread counts are always worked out from the read mark and its exceptions; nothing stores a counter.
- No `unsafe`: the crate forbids it. Nothing that comes from outside (a request, a feed, a web page, an image, the old database) may bring the server down: no `unwrap`, `expect` or `panic!` on it outside tests. Errors are `anyhow` inside and `AppError` outward, which answers without giving away internals.
- New code comes with tests, and no test is switched off to get CI green. Coverage goes to Codecov and is shown on pull requests, but no percentage is required. Tests of the server run against PostgreSQL with `#[sqlx::test]`; where a simple model can serve as a comparison (what was read, the tree), test against it with random input.
- Every parser of input from outside (feeds, web pages, images, OPML, the old database, the old frontend's `localStorage`) has limits for size and count and tests with input that breaks the rules.
- Every text users see is in `web/src/lib/i18n/`, in English and in German; components hold no texts. Colors come from the named variables (`bg-paper`, `text-soft`, `border-line`), so themes only recolor variables.
- To see the interface, e.g. after changing it, run `pnpm dev` in `web/`, or `pnpm build:single` for the whole of it as one HTML file.
- Tests and measurements use made-up data. No dump of the production database, no real account and no secret is ever committed; settings come from the environment, and `.env` stays out of the repository.
- Dependencies come from crates.io and the npm registry, with their lockfiles committed. Prefer crates in pure Rust, e.g. `rustls` over OpenSSL.
