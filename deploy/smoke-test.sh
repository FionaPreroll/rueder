#!/bin/sh
# Builds the images, starts rueder, its database and RSSifier as compose.yaml
# describes them, and checks that what comes up is a working rueder: it
# answers, serves the frontend, lets an account log in, reaches RSSifier,
# keeps its data over a restart, and runs the way the file promises (not
# root, read-only). For CI, and for trying a change to the images:
#
#   deploy/smoke-test.sh
#
# With RUEDER_IMAGE and RSSIFIER_IMAGE set it starts those images instead of
# building them, as CI does with the images it built before:
#
#   RUEDER_IMAGE=rueder:ci RSSIFIER_IMAGE=rssifier:ci deploy/smoke-test.sh
#
# It uses a project and a port of its own and takes both away again, so it can
# run next to a real installation.
set -eu
cd "$(dirname "$0")"

export COMPOSE_PROJECT_NAME=rueder-smoke
export POSTGRES_PASSWORD=smoke
export RUEDER_LISTEN=127.0.0.1:3999
# Made for this run, as the setup in README.md makes one.
export RSSIFIER_TOKEN="rsf_$(od -An -N16 -tx1 /dev/urandom | tr -d " \n")"
at=http://$RUEDER_LISTEN
password=correct-horse-battery

compose() {
	docker compose --file compose.yaml "$@"
}

say() {
	printf '\n== %s\n' "$1"
}

fail() {
	printf 'FAILED: %s\n' "$1" >&2
	exit 1
}

finish() {
	status=$?
	if [ "$status" -ne 0 ]; then
		compose logs --tail 50 || true
	fi
	compose down --volumes --remove-orphans || true
	rm -f "$headers"
	exit "$status"
}
headers=$(mktemp)
trap finish EXIT

say "Build and start"
if [ -n "${RUEDER_IMAGE:-}" ] && [ -n "${RSSIFIER_IMAGE:-}" ]; then
	build=--no-build
else
	build=--build
fi
# --wait returns once the containers call themselves healthy: PostgreSQL
# accepts connections, and `rueder health` and `rssifier health` got an
# answer from their servers.
compose up "$build" --detach --wait

say "The server answers and reaches its database"
curl --fail --silent --show-error "$at/api/health" | grep -q '"status":"ok"' ||
	fail "/api/health does not say ok"

say "The frontend is in the image"
curl --fail --silent --show-error "$at/" | grep -qi '<!doctype html' ||
	fail "/ is not the frontend"
# A route of the single-page app gets the app too, so a reload works.
curl --fail --silent --show-error "$at/feed/12" | grep -qi '<!doctype html' ||
	fail "an unknown path is not answered with the frontend"

say "The frontend's scripts may run, and no others"
# The policy names every script inside the page by its hash. Without them the
# app would not start in a browser, which curl would never notice.
policy=$(curl --fail --silent --show-error --head "$at/" | tr -d '\r' |
	sed -n 's/^[Cc]ontent-[Ss]ecurity-[Pp]olicy: //p')
# One directive of the policy, without its name.
directive() { printf '%s' "$policy" | tr ';' '\n' | sed -n "s/^ *$1 //p"; }
script_src=$(directive script-src)
scripts=$(curl --fail --silent --show-error "$at/" | grep -c '<script>')
hashes=$(printf '%s' "$script_src" | grep -o "'sha256-" | wc -l)
[ "$scripts" -gt 0 ] && [ "$hashes" -eq "$scripts" ] ||
	fail "the policy allows $hashes scripts inside the page, which has $scripts"
case "$script_src" in *unsafe*) fail "the policy allows more than the page's own scripts" ;; esac
# No style attribute is allowed, not even SvelteKit's on its announcer.
[ -z "$(directive style-src-attr)" ] || fail "the policy allows style attributes"
case "$policy" in *unsafe-inline* | *unsafe-eval*) fail "the policy allows inline code" ;; esac

say "A change that another site asks for is refused"
code=$(curl --silent --output /dev/null --write-out '%{http_code}' --request POST \
	--header 'Sec-Fetch-Site: cross-site' "$at/api/logout")
[ "$code" = 403 ] || fail "a request from another site got $code, not 403"

say "An account can be made and logs in"
printf '%s\n' "$password" | compose exec -T rueder rueder account add smoke --admin
curl --fail --silent --show-error --dump-header "$headers" --output /dev/null \
	--header 'Content-Type: application/json' \
	--data "{\"username\":\"smoke\",\"password\":\"$password\"}" "$at/api/login"
# The cookie is for https only, so curl would not send it back on its own.
cookie=$(tr -d '\r' <"$headers" | sed -n 's/^[Ss]et-[Cc]ookie: \(__Host-rueder=[^;]*\).*/\1/p')
[ -n "$cookie" ] || fail "the login set no session cookie"
curl --fail --silent --show-error --header "Cookie: $cookie" "$at/api/me" |
	grep -q '"username":"smoke"' || fail "/api/me does not know the session"
code=$(curl --silent --output /dev/null --write-out '%{http_code}' "$at/api/me")
[ "$code" = 401 ] || fail "/api/me answers $code without a session, not 401"

say "It runs as promised"
for service in rueder rssifier; do
	container=$(compose ps --quiet "$service")
	user=$(docker inspect --format '{{.Config.User}}' "$container")
	case "$user" in
	'' | root | 0 | 0:*) fail "the container of $service runs as root" ;;
	esac
	[ "$(docker inspect --format '{{.HostConfig.ReadonlyRootfs}}' "$container")" = true ] ||
		fail "the file system of $service is not read-only"
done
# RSSifier's image holds bash and env to look around in.
compose exec -T rssifier env bash -c 'test -n "$BASH_VERSION"' ||
	fail "RSSifier's container has no bash or env"
# `docker port` lists only the ports published on the machine.
[ -z "$(docker port "$(compose ps --quiet rssifier)")" ] || fail "RSSifier is open on the machine"
# The server made its key for the image proxy, so it can write where it must.
compose logs rueder | grep -q 'listening on' || fail "the server did not say it listens"

say "It says what it is"
# The admin's status page carries what this build is. The commit comes from the
# build, which only whoever builds knows, so it is only looked at when it was
# given (CI always gives it).
status=$(curl --fail --silent --show-error --header "Cookie: $cookie" "$at/api/status")
printf '%s' "$status" | grep -q '"container":true' ||
	fail "the status does not say that it runs in a container"
if [ -n "${RUEDER_COMMIT:-}" ]; then
	printf '%s' "$status" | grep -q "\"commit\":\"$RUEDER_COMMIT\"" ||
		fail "the status does not name the commit it was built from"
fi
printf '%s' "$status" | grep -q '"profile":"release"' ||
	fail "the image was not built as a release"

say "rueder reaches RSSifier with the token they share"
printf '%s' "$status" | grep -q '"rssifier":{"on":true' || fail "the status says RSSifier is off"
printf '%s' "$status" | grep -q '"usage":{"name":"env"' ||
	fail "RSSifier did not tell rueder what its token used"

say "Data outlives the containers"
compose down
compose up --detach --wait
curl --fail --silent --show-error --header "Cookie: $cookie" "$at/api/me" |
	grep -q '"username":"smoke"' || fail "the session is gone after a restart"

say "All good"
