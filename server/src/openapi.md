The HTTP interface of a rueder server: the API the web page lives on, under
`/api`, and the Google Reader API for feed reader apps of other makers, under
`/greader`. This description is written from the server's code
(`server/src/openapi.rs` and the `#[utoipa::path]` of each handler), so it
lists every route the server answers, besides the files of the page.

## Logging in

- **The page's API** (`/api`) knows a reader by the session cookie
  `__Host-rueder`, which `POST /api/login` (or a passkey, or OpenID Connect)
  sets. It is `HttpOnly`, `Secure` and `SameSite=Lax`, so it needs HTTPS, or
  `http://localhost` while developing.
- **A request that changes something** (anything but `GET`, `HEAD` and
  `OPTIONS`) is refused with 403 if a browser says it comes from a page of
  another site: by `Sec-Fetch-Site`, or by an `Origin` that is not this
  server. A request without either, such as one from a script, passes this
  check.
- **The Google Reader API** (`/greader`) knows an app by an app password,
  which the reader makes in the page's settings (Settings, Apps).
  `POST /greader/accounts/ClientLogin` takes the account's name and the app
  password as a form and answers with `Auth=<token>`; every other request
  names that token in `Authorization: GoogleLogin auth=<token>`. No cookie
  counts there.

## Answers

- What the page's API sends and takes is JSON, except where an operation says
  otherwise (OPML, the file of an account). The Google Reader API takes forms
  or query parameters, as the apps send them, and answers with JSON or plain
  text.
- **A refusal** has its status and a short plain-text reason, never the
  details of what went wrong inside: 400 for a request the server cannot
  use, 401 without a valid session or app password, 403 for what the account
  may not do, 404 for what is not there or not the account's, 413 for a body
  over the limit (256 KiB, larger only where an operation says so), 429 with
  `Retry-After` (in seconds) over a limit, 500 for a fault of the server.
- **Limits:** every sender and every account has limits on how much it asks
  of the server; logins, lookups of addresses on the internet and what an
  account can make more of (feeds, folders, labels, rules, app passwords)
  have limits of their own.
- **Live updates:** what changes for an account through either API reaches
  its open pages over `GET /api/events` (server-sent events).

## Images

Every image in an article, every thumbnail and icon goes through the server's
image proxy, at addresses the server signs: `/img/...` for the page (with the
session), `/greader/img/...` for an app (signed for its app password).
Addresses are never made by hand; they come in what the API sends.
