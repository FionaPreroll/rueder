#!/bin/sh
# Builds the image, starts rueder and its database as compose.yaml describes
# them, and checks that what comes up is a working rueder: it answers, serves
# the frontend, lets an account log in, keeps its data over a restart, and
# runs the way the file promises (not root, read-only). For CI, and for trying
# a change to the image:
#
#   deploy/smoke-test.sh
#
# It uses a project and a port of its own and takes both away again, so it can
# run next to a real installation.
set -eu
cd "$(dirname "$0")"

export COMPOSE_PROJECT_NAME=rueder-smoke
export POSTGRES_PASSWORD=smoke
export RUEDER_LISTEN=127.0.0.1:3999
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
# --wait returns once both containers call themselves healthy: PostgreSQL
# accepts connections, and `rueder health` got an answer from the server.
compose up --build --detach --wait

say "The server answers and reaches its database"
curl --fail --silent --show-error "$at/api/health" | grep -q '"status":"ok"' ||
	fail "/api/health does not say ok"

say "The frontend is in the image"
curl --fail --silent --show-error "$at/" | grep -qi '<!doctype html' ||
	fail "/ is not the frontend"
# A route of the single-page app gets the app too, so a reload works.
curl --fail --silent --show-error "$at/feed/12" | grep -qi '<!doctype html' ||
	fail "an unknown path is not answered with the frontend"

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
container=$(compose ps --quiet rueder)
user=$(docker inspect --format '{{.Config.User}}' "$container")
case "$user" in
'' | root | 0 | 0:*) fail "the container runs as root" ;;
esac
[ "$(docker inspect --format '{{.HostConfig.ReadonlyRootfs}}' "$container")" = true ] ||
	fail "the container's file system is not read-only"
# The server made its key for the image proxy, so it can write where it must.
compose logs rueder | grep -q 'listening on' || fail "the server did not say it listens"

say "Data outlives the containers"
compose down
compose up --detach --wait
curl --fail --silent --show-error --header "Cookie: $cookie" "$at/api/me" |
	grep -q '"username":"smoke"' || fail "the session is gone after a restart"

say "All good"
