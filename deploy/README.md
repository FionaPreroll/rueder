# Installing rueder

How to run the new rueder (`server/` and `web/`) on a machine of your own: one
container with the server and the frontend, PostgreSQL next to it, and your
reverse proxy in front for TLS.

rueder is not finished. What it can do today is in
[docs/implementation.md](../docs/implementation.md#stand).

## What you need

- A machine with Docker and its compose plugin (`docker compose version`).
- A name for it that browsers reach over HTTPS, and a reverse proxy that holds
  the certificate: Caddy, nginx or whatever already runs there. Without HTTPS
  nobody can log in, because browsers keep the session cookie for HTTPS only.
  The one exception is `http://localhost`, which is enough for a look.

## Install

```sh
git clone https://github.com/FionaPreroll/rueder.git
cd rueder/deploy
cp .env.example .env
```

Give the database a password in `.env`, for example the output of
`openssl rand -hex 24`:

```sh
POSTGRES_PASSWORD=…
```

Build the image and start both containers. The first build compiles the server
and takes a few minutes.

```sh
docker compose up --detach --build
```

Or take the image that CI builds and tests for every change on `main`,
instead of building one yourself: name it in `.env`, then fetch it and start.

```sh
# in .env
RUEDER_IMAGE=ghcr.io/fionapreroll/rueder:latest
```

```sh
docker compose pull rueder
docker compose up --detach
```

`:latest` is the newest change on `main`; every change also has a tag of its
own, `:sha-` and the first seven characters of its commit, for staying on one
version. The image is for amd64 machines only.

rueder now listens on `127.0.0.1:3000` and applies its database migrations on
every start. `docker compose ps` shows both containers as `healthy` once they
are up.

There is no registration. Make an account for everybody who reads, the first
one as admin; the command asks for the password:

```sh
docker compose exec rueder rueder account add fiona --admin
```

## The reverse proxy

rueder speaks plain HTTP on `127.0.0.1:3000`; the proxy does TLS and passes
everything on. With Caddy that is the whole configuration, certificate
included:

```caddyfile
reader.example {
	# Browsers that were here once come back over HTTPS only.
	header Strict-Transport-Security "max-age=31536000"
	reverse_proxy 127.0.0.1:3000
}
```

With nginx:

```nginx
server {
    listen 443 ssl;
    http2 on;
    server_name reader.example;
    ssl_certificate     /etc/letsencrypt/live/reader.example/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/reader.example/privkey.pem;

    # Browsers that were here once come back over HTTPS only.
    add_header Strict-Transport-Security "max-age=31536000" always;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $remote_addr;
        # Live updates are one long answer; do not collect it before passing it on.
        proxy_buffering off;
        proxy_read_timeout 1h;
    }
}
```

rueder sends the other security headers itself, a Content-Security-Policy
among them; the proxy need not add any. It has to pass the `Host` header on
unchanged, as both examples do: rueder compares it with the `Origin` of a
request to refuse changes that a page of another site asks for.

Behind a proxy every connection comes from the proxy. Tell rueder which header
carries the reader's address, so that the limits on requests and on wrong
passwords count per reader and not for everybody at once. Both examples above set
`X-Forwarded-For`:

```sh
# in .env
CLIENT_IP_HEADER=X-Forwarded-For
```

```sh
docker compose up --detach
```

Set it only when a proxy really is in front: without one, anybody could write
that header themselves.

## Checking an installation

`check.sh` looks at a running installation the way it is meant to run and
changes nothing. From any machine, it checks TLS, the certificate, HSTS, the
redirect from http, rueder's own headers, and that neither rueder's port nor
PostgreSQL's answers from outside:

```sh
deploy/check.sh https://reader.example
```

On the server, in this folder, it also checks the containers, what listens
where, the settings in `.env`, where the image comes from, and on Debian or
Ubuntu the updates of the host:

```sh
deploy/check.sh --server https://reader.example
```

It needs `curl` and `openssl`, on the server also `docker`, and `gh` (logged
in with `gh auth login`) to check an image's attestation. Every line says
`ok`, `warn` or `FAIL`; it ends with status 1 if anything failed. Run it after
an installation, after changing the proxy, and now and then.

To check an image yourself before you pull it:

```sh
gh attestation verify oci://ghcr.io/fionapreroll/rueder:latest --repo FionaPreroll/rueder
```

## Settings

All settings are in `.env`; `.env.example` lists them with what they do. After
a change, `docker compose up --detach` starts the containers anew.

| Setting | What it is for |
|---|---|
| `POSTGRES_PASSWORD` | The password of the database. Required. Letters and digits only. |
| `RUEDER_LISTEN` | Where on the machine rueder listens, `127.0.0.1:3000` if not set. |
| `CLIENT_IP_HEADER` | The header the reverse proxy names the reader's address in. |
| `POSTGRES_SHARED_BUFFERS` | Memory for PostgreSQL's cache, `128MB` if not set. |
| `IMAGE_CACHE_MB`, `IMAGE_PROXY_KEY` | Size of the image proxy's cache, and its secret if you want to set it yourself. |
| `FETCH_ALLOW_NETS`, `FETCH` | Internal networks feeds may come from; `FETCH=off` stops fetching. |
| `CLIENT_REPORTS` | `off` takes no reports of errors from browsers, even of readers who agreed to send them. |
| `OIDC_ISSUER`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET`, `OIDC_REDIRECT_URL` | Login through an OpenID Connect provider. |

Changing `POSTGRES_PASSWORD` later does not change the password of a database
that already exists. Change it there first
(`docker compose exec db psql -U rueder -c '\password'`), then in `.env`.

## Commands

The server's commands run inside its container:

```sh
docker compose exec rueder rueder account list
docker compose exec rueder rueder registration show        # who may register or invite, see below
docker compose exec rueder rueder admin add fiona          # sees the server status; admin remove, admin list
docker compose exec rueder rueder account password fiona   # also ends the account's sessions
docker compose exec rueder rueder account link fiona <subject>   # for OpenID Connect
docker compose exec rueder rueder reclean                  # after the cleaning's rules changed
docker compose exec -T rueder rueder account export fiona > fiona.json            # move an account away
docker compose exec -T rueder rueder account import fiona /dev/stdin < fiona.json  # and in, here or elsewhere
docker compose logs --follow rueder
```

`docker compose exec rueder rueder --help` lists all of them.

## Letting people register

Out of the box nobody can make an account but you. Three switches change that,
each on the command line and at once, without a restart:

```sh
# Anybody can register, up to 50 accounts in all and 5 new ones a day.
docker compose exec rueder rueder registration set --open true --max-accounts 50 --per-day 5

# Registration stays closed, but every reader may invite 3 others with a link
# that is good for 14 days. The limits above count for these too.
docker compose exec rueder rueder registration set --invites 3 --invite-days 14
docker compose exec rueder rueder account invites fiona 10   # a quota of fiona's own

# Admins can always make invitation links. The accounts these bring count in a
# pool of their own, which you can cap.
docker compose exec rueder rueder registration set --admin-invite-accounts 20
```

`none` lifts a limit (`--max-accounts none`), `registration show` says what is
set and how many accounts there are. Readers make their links in the settings
under "Invitations". There is no confirmation by email and no CAPTCHA: if you
open registration to everybody, set `--per-day`.

## Updates

```sh
git pull
docker compose up --detach --build
```

With the published image (`RUEDER_IMAGE` in `.env`):

```sh
git pull
docker compose pull rueder
docker compose up --detach
```

The new server applies new migrations when it starts. Make a backup first.

## Backup

Everything rueder knows is in PostgreSQL. The cache of the image proxy fills
itself again and needs no backup.

```sh
docker compose exec -T db pg_dump -U rueder --format=custom rueder > rueder.dump
```

To put a backup into an empty database, stop the server so that nothing else
writes, and start it afterwards:

```sh
docker compose stop rueder
docker compose exec -T db pg_restore -U rueder --dbname=rueder --clean --if-exists < rueder.dump
docker compose start rueder
```

## Bringing over the old rueder

The data of the old rueder comes once from a backup of its database and is
merged with the accounts and feeds that are here
([server/README.md](../server/README.md#importing-the-old-database) says how).
Put the backup into a second database next to rueder's, stop the server, take
a backup of rueder's own database, and run the import from a container of its
own:

```sh
docker compose exec -T db createdb -U rueder rueder_old
docker compose exec -T db pg_restore -U rueder --dbname=rueder_old --no-owner < old.dump
# (a backup in plain SQL goes in with: docker compose exec -T db psql -U rueder rueder_old < old.sql)
docker compose stop rueder
docker compose exec -T db pg_dump -U rueder --format=custom rueder > before-import.dump
export LEGACY_DATABASE_URL="postgres://rueder:$(grep ^POSTGRES_PASSWORD= .env | cut -d= -f2-)@db:5432/rueder_old"
docker compose run --rm -e LEGACY_DATABASE_URL rueder import-legacy --dry-run
docker compose run --rm -e LEGACY_DATABASE_URL rueder import-legacy --account oldname=newname
docker compose start rueder
```

The dry run says which feeds and accounts would be merged and which added. If
the import stops halfway, run it again. The way back is `before-import.dump`,
put in as under [Backup](#backup). Afterwards `rueder_old` can go:
`docker compose exec db dropdb -U rueder rueder_old`.

## What the container may do

- It holds the program, the frontend and the C library, with no shell and no
  package manager, and runs as a user that is not root.
- Its file system is read-only. It writes to one volume, the cache of the
  image proxy, where the key for the proxy's addresses lies too.
- It has no Linux capabilities and cannot gain privileges.
- The database is reachable from the rueder container only; no port of it is
  open on the machine.
- Secrets come from `.env` when the container starts. None is in the image or
  in the frontend.

`deploy/smoke-test.sh` builds the image, starts everything under another name
and port, checks these points and that an account can log in, and takes it
all away again. CI runs it on every pull request, against the image it built; `RUEDER_IMAGE=… deploy/smoke-test.sh` tests a finished image
without building one.

## Without Docker

rueder is one program that needs a PostgreSQL 16 and the built frontend. How
to build and run it by hand is in [server/README.md](../server/README.md) and
[web/README.md](../web/README.md).
