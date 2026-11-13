# rueder's site on GitHub Pages

What is published at https://fionapreroll.github.io/rueder/: an overview, the
demo and the documentation, put together from what the other parts of the
repository build. Everything on it is in English.

| Address                      | What                                     | Made from                         |
| ---------------------------- | ---------------------------------------- | --------------------------------- |
| `/rueder/`                   | the overview                             | `src/index.html`                  |
| `/rueder/demo/`              | the reader with made-up data             | `pnpm build:single` in `web/`     |
| `/rueder/docs/self-hosting/` | installing rueder                        | `deploy/README.md`                |
| `/rueder/docs/android-app/`  | the app, and its Java code under `code/` | `app/README.md`, Javadoc          |
| `/rueder/docs/rest-api/`     | every route of the server (OpenAPI)      | `server/openapi.json`, Swagger UI |
| `/rueder/docs/rust/`         | the server's code                        | rustdoc, private items included   |
| `/rueder/docs/frontend/`     | the page's TypeScript                    | TypeDoc over `web/src/lib`        |

`/rueder/docs/` leads to the overview, and an address with nothing there gets
`src/404.html`. Every page says at its foot which commit and day it was built
from (`footer.js`).

## Building it

The workflow `.github/workflows/pages.yml` does this on every pull request and
publishes the result from `main`. By hand, with what each part needs (see the
README of `web/`, `server/` and `app/`), from the top of the repository:

```sh
export RUEDER_DOCS_FOOTER="$(node pages/footer.js)"
node pages/footer.js --rustdoc > /tmp/footer.html
(cd web && pnpm build:single && pnpm build:docs)
(cd server && RUSTDOCFLAGS="-D warnings --html-after-content /tmp/footer.html" \
  cargo doc --no-deps --lib --document-private-items)
(cd app && pnpm run sync)
(cd app/android && ./gradlew --init-script ../../pages/javadoc.gradle javadoc)
(cd pages && pnpm install && node build.js)
```

The site is then in `_site/`, and its links are checked. To look at it, serve
the folder above it under the repository's name, e.g. with
`mkdir -p /tmp/site && ln -s "$PWD/_site" /tmp/site/rueder && python3 -m http.server -d /tmp/site`
and open http://localhost:8000/rueder/.

- `build.js` copies the parts to their places, makes pages of the READMEs
  (`markdown.js`, with GitHub's anchors; a link to another README on the site
  stays on the site, any other file of the repository leads to GitHub at the
  commit) and fails if a link leads nowhere (`links.js`: `href` and `src` of
  every page, `@import` and `url()` of every stylesheet). The only links let
  through are those of other crates' documentation that rustdoc shows on our
  types (`foreignLinks`).
- The REST API's page is `server/openapi.json` (written from the server's
  code, see `server/src/openapi.rs`) shown by Swagger UI from npm, to read
  only: no "Try it out", no other description from the address, and no
  validator elsewhere (`src/rest-api.js`).
- `javadoc.gradle` is a Gradle init script that gives the app the task
  `javadoc`. It lives here and not in the app's `build.gradle`, as CI releases
  the app anew whenever something in `app/` changes.
- The demo is the single-file build, which finds its pages by the part of the
  address after the `#`, so it runs from any folder.

## Publishing

Once, in the repository's Settings, Pages: Source "GitHub Actions". From then on
every push to `main` publishes the site; the environment `github-pages` lets
only the default branch do that. A fork gets its own address,
`https://<owner>.github.io/<repository>/`.

## Checks

```sh
pnpm lint
pnpm test
```
