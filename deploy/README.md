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
version. Both tags carry an image for amd64 and one for arm64, and docker takes
the one for the machine.

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
everything on. rueder sets its security headers itself, including
`Strict-Transport-Security`, so a proxy that cannot add headers, such as
`tailscale serve`, is enough. With Caddy this is the whole configuration,
certificate included:

```caddyfile
reader.example {
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

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $remote_addr;
        proxy_set_header X-Forwarded-Proto $scheme;
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

## Metrics for Prometheus

rueder can give what its status page shows to a Prometheus of your own, at
`/metrics` in Prometheus' text format: requests and how long they took by
route, fetching, the feed search, the image proxy, the warnings and errors of
the log, and what PostgreSQL says (accounts, feeds, size, connections, feeds
that fail by why). Nothing in it names an account, a feed or an address.
Counters count since rueder started and begin at 0 after a restart, which
Prometheus handles by itself.

It is off until you set a token of 32 characters or more:

```sh
# in .env
METRICS_TOKEN=$(openssl rand -hex 32)   # write the value itself, not the command
```

```sh
docker compose up --detach
```

Without the token `/metrics` answers 404; with a wrong one or none, 401. A
session of an admin or an app password does not open it. Give Prometheus the
same token in a file of its own and let it ask rueder directly, not through the
proxy, here from the same machine:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: rueder
    authorization:
      credentials_file: /etc/prometheus/rueder-token
    static_configs:
      - targets: ["127.0.0.1:3000"]
```

Prometheus in a container of its own reaches rueder at `rueder:3000` when it
is on the network of this Compose project (`rueder_default`), or at the
address of the machine when `RUEDER_LISTEN` lets it.

The token protects the metrics wherever they are reached, but they are for
you, not for the internet. If Prometheus asks rueder directly, keep `/metrics`
out of the proxy. With Caddy, inside the site's block:

```caddyfile
	respond /metrics 404
```

With nginx, in the `server` block:

```nginx
    location = /metrics { return 404; }
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
| `FETCH_ROUND`, `FETCH_AT_ONCE`, `FETCH_EVERY`, `FETCH_PER_HOST` | How fast feeds are fetched, see [A large instance](#a-large-instance). |
| `DATABASE_CONNECTIONS`, `HEAVY_AT_ONCE`, `MAILS_PER_DAY` | How large the server is set up: connections to the database (10), costly requests at once (3) and mails a day in all (1,000), see [A large instance](#a-large-instance). |
| `DATABASE_WARN_GB` | From what size of the database in GB the status page warns, such as `50` on a disk of 64 GB. Off if not set; see [Keeping old articles, or not](#keeping-old-articles-or-not). |
| `CLIENT_REPORTS` | `off` takes no reports of errors from browsers, even of readers who agreed to send them. |
| `APP_API` | `off` lets no feed reader app of another maker in. On if not set: readers set up apps such as NetNewsWire and Read You under Settings, Apps, with an app password each, and the apps reach rueder at `https://<your rueder>/greader`. |
| `CONTENT_REPORTS` | `on` lets readers report a feed or an article to you, with a reason and a short note. The status page lists the reported feeds with their address, never who reported; `rueder feed block` blocks one. Off if not set. |
| `LEGAL_IMPRINT_URL`, `LEGAL_PRIVACY_URL`, `LEGAL_TERMS_URL` | Where your imprint, privacy policy and terms of use are, on your own website. rueder links to them on the login page, in the account menu and in the app; without them it shows none. |
| `TERMS_VERSION` | A name for the version of your terms of use, such as `2026-11`. Registering then takes the agreement to it, and every account is asked to agree to a new one at its next visit. Needs `LEGAL_TERMS_URL`. |
| `ADMIN_SECOND_FACTOR` | `required` makes admins admins only after logging in with a second factor (an authenticator app or a passkey); readers set one up under Account settings, Security. |
| `PUBLIC_URL` | The address browsers reach rueder at, such as `https://reader.example`: HTTPS, scheme and host only. With it, readers can add passkeys under Account settings, Security and log in with them, without name or password. A passkey belongs to this address; one made for another address does not work. Without it there are no passkeys. |
| `SMTP_URL`, `MAIL_FROM` | Mail to readers about their account, through your SMTP relay: `smtps://user:password@smtp.example:465` or `smtp://user:password@smtp.example:587?tls=required`; plain `smtp://` only to a relay on the same machine. `MAIL_FROM` is the address it comes from, such as `rueder <rueder@reader.example>`, and the links in it lead to `PUBLIC_URL`, which it needs too. Without them rueder sends no mail. See [Letting people register](#letting-people-register). |
| `PASSKEY_ANDROID_APPS` | The Android apps that may use the passkeys too, as `package=fingerprint` separated by commas, the fingerprint the SHA-256 of the app's signing certificate as `keytool -printcert` writes it (`AB:CD:…`). Without it, it is the app released on GitHub; `none` names none. rueder names them to Android at `/.well-known/assetlinks.json`, which Google reads from the internet, so the app's passkeys need rueder reachable from there at the host of `PUBLIC_URL`, on port 443. Add the fingerprint of Play App Signing for the Play Store's build, or your own for a build you sign yourself. |
| `OIDC_ISSUER`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET`, `OIDC_REDIRECT_URL` | Login through an OpenID Connect provider. |
| `METRICS_TOKEN` | Metrics for Prometheus under `/metrics`, for whoever sends this token, 32 characters or more. Off if not set; see [Metrics for Prometheus](#metrics-for-prometheus). |

Changing `POSTGRES_PASSWORD` later does not change the password of a database
that already exists. Change it there first
(`docker compose exec db psql -U rueder -c '\password'`), then in `.env`.

### A large instance

rueder fetches in rounds: every `FETCH_EVERY` seconds (30) it takes up to
`FETCH_ROUND` feeds that are due (100) and fetches `FETCH_AT_ONCE` of them at
the same time (8), `FETCH_PER_HOST` of one host at once (1). A round does
not wait for the one before: while a round still works through a host with
many feeds, the next ones leave that host out and fetch the others. That is
at most 12,000 fetches an hour, enough for a few thousand feeds: a feed is
fetched every 15 minutes to 12 hours, by how much it publishes. The server
refuses to start with a value out of bounds (round 1 to 10,000, at once 1 to
256, every 5 to 3,600 seconds, of one host 1 to 32).

The status page says the pace, the fetches of the last hour and how far
fetching is behind. When every round takes all it may and feeds still wait,
it says so; then raise the pace. For an instance with tens of thousands of
feeds:

```sh
FETCH_ROUND=1000
FETCH_AT_ONCE=64
FETCH_EVERY=10
```

That is up to 360,000 fetches an hour. On a machine with 4 cores and 15 GB
the rounds took about a second each, against made-up feeds on the same
machine (`rueder bench fetch`; the numbers are in `docs/implementation.md`
under "Messung"). Real sites answer slower than a source on the same
machine, so `FETCH_AT_ONCE` is what keeps rounds short: 64 fetches at once of
sites that take half a second each make about 130 a second. Where many readers
follow feeds of one large platform, such as a video site, `FETCH_PER_HOST=4`
fetches four of them at once; ask the platform's terms before you go higher.

For tens of thousands of accounts, the server itself can be set up larger:

- `DATABASE_CONNECTIONS` (10) is how many connections to the database rueder
  keeps. More help only where PostgreSQL has more cores to use them, about
  two for each core. PostgreSQL takes 100 as it comes, and rueder's commands,
  such as `rueder account list`, take a few of their own.
- `HEAVY_AT_ONCE` (3) is how many costly requests run at once, such as an
  export, an import or the preview of a filter rule; more wait for their
  turn. It stays fewer than the connections, so that reading goes on
  meanwhile.
- `MAILS_PER_DAY` (1,000) is how many mails rueder sends in a day, of which
  three in ten may be links to set a new password. Set it to what your relay
  allows.

The server refuses to start with a value out of bounds (connections 2 to 90,
costly requests 1 to 64 and fewer than the connections, mails 1 to
1,000,000). A starting point:

| | 10,000 accounts | 100,000 accounts |
|---|---|---|
| Machine | 2 cores, 8 GB | 4 cores, 16 GB |
| PostgreSQL | `POSTGRES_SHARED_BUFFERS=2GB` | `POSTGRES_SHARED_BUFFERS=4GB` |
| Sizes | as rueder comes | `MAILS_PER_DAY=10000`; with 8 cores also `DATABASE_CONNECTIONS=20` and `HEAVY_AT_ONCE=6` |
| Fetching | `FETCH_ROUND=300`, `FETCH_AT_ONCE=32`, `FETCH_EVERY=15`: 72,000 an hour | `FETCH_ROUND=1000`, `FETCH_AT_ONCE=64`, `FETCH_EVERY=10`: 360,000 an hour |
| Retention | `--articles 24 --remove text --versions 12 --orphans 90`, see [Keeping old articles, or not](#keeping-old-articles-or-not) | the same |
| Disk | by the feeds, not the accounts: see below | the same |

With 100,000 made-up readers, a machine with 4 cores and 15 GB answered about
250 visits of a reader a second, each the tree, a page of unread articles,
three articles opened and marked read and a page of one feed; 100,000
accounts make 15 to 35 a second at their busiest. Twice the connections
made no difference there, as its cores were busy. The column for 10,000
accounts is worked out from these numbers, not measured on its own. The
numbers are in `docs/implementation.md` under "Messung".

**Memory.** What readers read has to fit in memory: their subscriptions, the
lines of the articles they read and what they marked, about 1.3 GB for
100,000 readers with 50 subscriptions each; of the texts, only those opened.
With less, as with PostgreSQL's 128 MB, a page of unread articles took over
half a minute, as everything came from the disk. The `POSTGRES_SHARED_BUFFERS`
in the table leave room for that and more. After a restart, reading was as
fast as before within half a minute.

**Disk.** The space goes with the feeds, not with the accounts. An article
takes about 7.5 KB with its text, the words to search it by and its
versions. A feed brings 57 articles a year at the median and 4,500 on
average, while a few news feeds bring over 100,000; on the instance this was
measured on, two feeds brought 60 % of all articles. An account with 50
subscriptions takes about 7 KB, and every star, label or article its filter
rules hide about 110 bytes more. So 1,000 feeds like those take about 34 GB
a year. With the retention in the table, an article that is two years old
keeps about 1.2 KB, its line in the list and the words of its title and
tags: the
same 1,000 feeds then hold about 57 GB and grow by about 5 GB a year. The
image proxy's cache stays within `IMAGE_CACHE_MB`.

Turning the retention on keeps the database from growing by old texts, but
does not make it smaller: PostgreSQL reuses the space of the texts that went,
while the versions give theirs back at once. To give back the rest, once:

```sh
docker compose exec db psql -U rueder -c 'VACUUM FULL article_content'
```

It locks the table while it runs, about a minute for 300,000 articles, so
readers wait meanwhile. The same is worth it once for `subscriptions` after an
update on an instance with many subscriptions: rueder leaves a tenth of every
page of that table free, so that moving the read mark needs no new entry in
its indexes, and a table written before gets that room only bit by bit.

## Commands

The server's commands run inside its container:

```sh
docker compose exec rueder rueder account list
docker compose exec rueder rueder registration show        # who may register or invite, see below
docker compose exec rueder rueder admin add fiona          # sees the server status; admin remove, admin list
docker compose exec rueder rueder account password fiona   # also ends the account's sessions
docker compose exec rueder rueder account second-factor-off fiona   # lost the app and the recovery codes
docker compose exec rueder rueder account link fiona <subject>   # for OpenID Connect
docker compose exec rueder rueder account disable mallory  # no login, sessions end, feeds only it reads rest; `enable` undoes it
docker compose exec rueder rueder account remove mallory   # deletes the account and all that is its own
docker compose exec rueder rueder feed block https://spam.example/feed.xml   # not fetched, hidden for all, its images gone; `unblock` undoes it
docker compose exec rueder rueder feed show 130            # what is known of a feed a reader asks about, by number or address
docker compose exec rueder rueder feed agent 130 browser   # fetch it as a browser for a site that turns rueder away; `rueder` undoes it
docker compose exec rueder rueder reclean                  # after the cleaning's rules changed
docker compose exec rueder rueder retention show           # how long what is kept, see below
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

`none` lifts a limit (`--max-accounts none`), but not the one per day while
registration is open. `registration show` says what is set and how many
accounts there are. Readers make their links in the account
settings, in the menu of their account, under "Invitations".

Registration without an invitation first has the browser compute for a
moment: a captcha that asks nothing of the reader and no service but rueder.
`registration set --captcha false` turns it off. It only stops scripts that do
not solve it: code of one's own solves one in milliseconds. What limits how
many accounts anybody can make is `--per-day`, which open registration needs,
`--max-accounts`, and on a public instance best `--email required` too, so
that every account needs a mailbox of its own. Registrations are also
throttled per network and, without an invitation, for the whole server, and
they never make logins wait.

With `SMTP_URL`, `MAIL_FROM` and `PUBLIC_URL` set (see [Settings](#settings)),
accounts can have an email address, which a link mailed to it confirms:

```sh
# Registration asks for an address, and readers can give, change or remove one
# under Account settings, Email.
docker compose exec rueder rueder registration set --email optional

# Every account needs one: a new account logs in once its link is opened, and
# one from before is asked for its address when it logs in.
docker compose exec rueder rueder registration set --email required
```

With an address, readers who forgot their password get a link from "Forgot
your password?" on the login page, good for 30 minutes and once. It sets a new
password and logs out every browser and device of the account; logging in
still asks for a second factor where the account has one. Without addresses
you set a forgotten password yourself with `rueder account password`.

`--email off`, the default, asks nobody. A link is good for 24 hours and once;
an account made with `required` that never opens its link goes after 7 days.
rueder sends at most 5 mails an hour for an account, 3 to an address and one
more every two hours, and 1,000 a day in all (`MAILS_PER_DAY`), of which three
in ten may be links to set a new password. Mail goes out in the language the
account was made in. The status page names no address, and moving an account
to another rueder takes none along.

## Keeping old articles, or not

Out of the box rueder keeps everything for good, and it never lets anything
go by itself, not even when the disk fills up. So that you hear of it in
time, `DATABASE_WARN_GB` names a size of the database: from it on, the status
page warns and says what the database takes. Leave room below the size of the
disk for what PostgreSQL needs besides, and for `VACUUM FULL`, which writes a
table anew before it gives back the old one.

A large instance, or one whose disk fills up, can let old things go, each on
its own, and the server clears them within the hour, a little at a time:

```sh
# Articles that arrived more than 24 months ago. Say what goes of them:
# `article` takes them from the lists, the unread counts and the search;
# `text` takes only their text and leaves their line in the lists.
docker compose exec rueder rueder retention set --articles 24 --remove text

# Versions after 12 months, by whole months; feeds nobody has subscribed to
# for 90 days, with their articles.
docker compose exec rueder rueder retention set --versions 12 --orphans 90

# Clear it all now instead of waiting for the server.
docker compose exec rueder rueder retention run
```

Articles a reader starred or labelled stay, with their text. `never` keeps
something for good again (`--articles never`), and `retention show` says what
is set. Age counts from when rueder first fetched an article, not from the date
its feed gives it. A blocked feed nobody reads loses its articles but keeps its
block. If you bring over read state and labels from the old rueder's browsers,
do that first: the takeover finds the old articles, which the retention may
take away.

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

The new server applies new migrations when it starts. Make a backup first. A
migration that rewrites a large table can take minutes and needs free disk for a
second copy of that table; while it runs, `docker compose logs -f rueder` says
regularly which one it is and how far it has come.

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
docker compose run --rm rueder account disable gone   # an old account nobody uses any more
docker compose start rueder
```

The dry run says which feeds and accounts would be merged and which added. An
old account that nobody uses any more is best disabled before the server
starts: the feeds no other enabled account reads are then never fetched, and
`account enable` brings them back. While the import runs, it says regularly
how many of the old articles it has brought and about how long it still needs.
If it stops halfway, run it again.
The way back is `before-import.dump`, put in as under [Backup](#backup).
Afterwards `rueder_old` can go:
`docker compose exec db dropdb -U rueder rueder_old`.

The import reads the old database at any address it can reach, so a backup
need not go into rueder's PostgreSQL. If the old rueder's volume is still
there, the old database can instead run next to rueder in a container of its
own, with the version that wrote it (PostgreSQL 13, `postgres:13.6-alpine` in
the old compose file), on rueder's network. PostgreSQL writes to its volume
when it starts, so it runs on a copy, and nothing of the old rueder may still
use the volume. `docker volume ls` shows its name, after the folder the old
compose file ran in:

```sh
docker volume create rueder3_db_copy
docker run --rm -v rueder3_db:/from:ro -v rueder3_db_copy:/to postgres:13.6-alpine cp -a /from/. /to/
docker run -d --name rueder3-db --network rueder_default \
  -v rueder3_db_copy:/var/lib/postgresql/data postgres:13.6-alpine
docker exec rueder3-db psql -U postgres -c '\du' -c '\l'   # its users and databases
export LEGACY_DATABASE_URL="postgres://postgres:postgres@rueder3-db:5432/rueder_production"
```

User, password and database are the ones the old compose file set. Then stop
rueder, take the backup and run the import as above. Afterwards the old
database can go: `docker rm -f rueder3-db` and
`docker volume rm rueder3_db_copy`.

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
