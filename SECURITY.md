# Security policy

## Reporting a vulnerability

Please do not open a public issue for a security problem. Report it privately
through GitHub instead: on the repository's **Security** tab, choose
**Report a vulnerability**. Only the maintainers see the report, and the fix
can be discussed and prepared there before anything is public.

A useful report says what is affected (the server, the web page, the Android
app or the container), how to reproduce it, and what an attacker gains. Test
against an installation of your own, never against somebody else's rueder or
their data.

rueder is maintained in spare time. You will get an answer as soon as
possible, usually within a week; there is no bug bounty.

## What is covered

The new rueder, which replaces the old one:

- the server in `server/` and the web page in `web/`, as in the image
  `ghcr.io/fionapreroll/rueder:latest`, built from `main`;
- the Android app in `app/`, as in the release
  [`android-latest`](https://github.com/FionaPreroll/rueder/releases/tag/android-latest);
- the container setup in `deploy/` and the `Dockerfile`.

Only the newest state of `main` gets fixes. The old rueder (Go backend and
Svelte frontend), which was removed from the repository, gets none.

## How rueder is meant to be safe

What the server protects and how, and the risks that are knowingly left open,
are written down (in German) under "Security" in
[`docs/architecture.md`](docs/architecture.md#security). A report about one of
those open risks is welcome too, especially if it is easier to exploit than
described there.
