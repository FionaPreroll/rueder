#!/bin/sh
# Checks a running rueder the way docs/architecture.md expects it to be run:
# behind a reverse proxy that does TLS, with only the proxy reachable from
# outside. It changes nothing and logs nobody in. On Linux and macOS:
#
#   deploy/check.sh https://reader.example
#
# looks at the installation from outside, from any machine: TLS, the
# certificate, HSTS, the redirect from http, rueder's own headers, and that
# neither rueder's port nor PostgreSQL's answers from the internet.
#
#   deploy/check.sh --server [https://reader.example]
#
# looks from the server itself, in the folder of compose.yaml: the containers,
# what listens where, the settings in .env, the image and, on Debian or
# Ubuntu, the updates of the host. With an address it also does the outside
# checks, which from the server itself prove less about the ports.
#
# Each line says ok, warn or FAIL; the exit status is 1 if anything failed.
# It needs curl and openssl; --server also docker, and gh to check where the
# image comes from.
set -u

failed=0
ok() { printf '  ok    %s\n' "$1"; }
warn() { printf '  warn  %s\n' "$1"; }
bad() {
	printf '  FAIL  %s\n' "$1"
	failed=1
}
section() { printf '\n%s\n' "$1"; }
have() { command -v "$1" >/dev/null 2>&1; }

usage() {
	sed -n '2,22p' "$0" | sed 's/^# \{0,1\}//'
	exit 2
}

server=false
url=
for arg in "$@"; do
	case $arg in
	--server) server=true ;;
	-h | --help) usage ;;
	https://*) url=${arg%/} ;;
	*)
		printf 'not an https address or option: %s\n\n' "$arg" >&2
		usage
		;;
	esac
done
if [ -z "$url" ] && [ "$server" = false ]; then
	usage
fi

# The value of one header in a block of headers, lower-cased names.
header() {
	printf '%s\n' "$2" | tr -d '\r' | grep -i "^$1:" | head -n 1 | cut -d: -f2- | sed 's/^ *//'
}

# Whether a TCP port answers, without sending anything to it.
answers() {
	case $1 in
	*:*) target="[$1]" ;;
	*) target=$1 ;;
	esac
	connected=$(curl -s -o /dev/null -w '%{time_connect}' --connect-timeout 4 -m 5 \
		"telnet://$target:$2" </dev/null 2>/dev/null)
	[ -n "$connected" ] && [ "$connected" != "0.000000" ] && [ "$connected" != "0" ]
}

outside() {
	host=${url#https://}
	host=${host%%/*}
	port=443
	case $host in
	\[*\]:*) port=${host##*:} ;;
	\[*\]) ;;
	*:*) port=${host##*:} host=${host%:*} ;;
	esac
	name=$(printf '%s' "$host" | tr -d '[]')

	section "From outside: $url"
	if ! have curl; then
		bad "curl is missing"
		return
	fi

	health=$(curl -sS --proto '=https' -m 15 "$url/api/health" 2>&1)
	case $health in
	*'"status":"ok"'*) ok "rueder answers over https, with a certificate this machine trusts" ;;
	*) bad "no healthy answer from $url/api/health: $(printf '%s' "$health" | head -c 200)" ;;
	esac

	if have openssl; then
		cert=$(printf '' | openssl s_client -connect "$name:$port" -servername "$name" 2>/dev/null |
			openssl x509 2>/dev/null)
		if [ -z "$cert" ]; then
			warn "could not read the certificate with openssl (behind a proxy of your own?)"
		elif printf '%s\n' "$cert" | openssl x509 -noout -checkend 1209600 >/dev/null 2>&1; then
			until=$(printf '%s\n' "$cert" | openssl x509 -noout -enddate | cut -d= -f2)
			ok "the certificate is valid for more than 14 days (until $until)"
		else
			bad "the certificate expires within 14 days"
		fi
		for old in tls1 tls1_1; do
			if printf '' | openssl s_client -connect "$name:$port" -servername "$name" "-$old" \
				>/dev/null 2>&1; then
				bad "the proxy still accepts $old, which browsers dropped in 2020"
			fi
		done
	else
		warn "openssl is missing: certificate and old TLS versions not checked"
	fi

	headers=$(curl -sS -D - -o /dev/null -m 15 "$url/" 2>/dev/null)
	hsts=$(header strict-transport-security "$headers")
	age=$(printf '%s' "$hsts" | sed -n 's/.*max-age=\([0-9]*\).*/\1/p')
	if [ -z "$hsts" ]; then
		bad "no Strict-Transport-Security header: the proxy should set it (deploy/README.md)"
	elif [ "${age:-0}" -lt 15552000 ]; then
		warn "Strict-Transport-Security has max-age=${age:-0}, less than half a year"
	else
		ok "Strict-Transport-Security: $hsts"
	fi
	for expected in content-security-policy x-content-type-options x-frame-options \
		referrer-policy cross-origin-opener-policy; do
		if [ -z "$(header "$expected" "$headers")" ]; then
			bad "the page comes without $expected: is something between the proxy and rueder taking headers away?"
		fi
	done
	[ -n "$(header content-security-policy "$headers")" ] && ok "rueder's own security headers arrive"
	served_by=$(header server "$headers")
	case $served_by in
	*[0-9]*) warn "the Server header names a version: $served_by" ;;
	esac

	plain=$(curl -sS -D - -o /dev/null -m 10 "http://$host/" 2>/dev/null)
	location=$(header location "$plain")
	if [ -z "$plain" ]; then
		ok "plain http does not answer"
	else
		case $location in
		https://*) ok "plain http sends browsers to https" ;;
		*) bad "plain http answers without sending browsers to https" ;;
		esac
	fi

	for open in 3000 5432; do
		if answers "$name" "$open"; then
			bad "port $open answers from here: rueder and PostgreSQL should be reachable on the server only"
		else
			ok "port $open does not answer from here"
		fi
	done
}

on_server() {
	section "On the server: $(pwd)"
	if [ ! -f compose.yaml ]; then
		bad "no compose.yaml here: run this in deploy/ of the installation"
		return
	fi
	if ! have docker; then
		bad "docker is missing"
		return
	fi

	for service in rueder db; do
		state=$(docker compose ps --format '{{.State}} {{.Health}}' "$service" 2>/dev/null | head -n 1)
		case $state in
		"running healthy") ok "$service runs and is healthy" ;;
		running*) warn "$service runs, health: ${state#running }" ;;
		*) bad "$service does not run (${state:-not found})" ;;
		esac
	done

	if [ -f .env ]; then
		value() { sed -n "s/^$1=//p" .env | tail -n 1 | tr -d '"'"'"; }
		password=$(value POSTGRES_PASSWORD)
		if [ "${#password}" -lt 20 ]; then
			bad "POSTGRES_PASSWORD has fewer than 20 characters"
		else
			ok "POSTGRES_PASSWORD is long enough"
		fi
		mode=$(stat -c '%a' .env 2>/dev/null || stat -f '%Lp' .env 2>/dev/null)
		case $mode in
		*[1-7]) bad ".env can be read by every user of this machine (mode $mode): chmod 600 .env" ;;
		*) ok ".env is not readable by everybody (mode $mode)" ;;
		esac
		listen=$(value RUEDER_LISTEN)
		case ${listen:-127.0.0.1:3000} in
		127.* | localhost:* | \[::1\]:*) ok "rueder listens on ${listen:-127.0.0.1:3000} only" ;;
		*) warn "RUEDER_LISTEN=$listen: rueder is reachable beyond this machine, make sure a firewall stops that" ;;
		esac
		ip_header=$(value CLIENT_IP_HEADER)
		if [ -n "$ip_header" ]; then
			ok "CLIENT_IP_HEADER=$ip_header: the proxy has to set it itself, never pass on what a browser sent"
		else
			warn "CLIENT_IP_HEADER is not set: behind a proxy, all readers count as one address for the limits"
		fi
		key=$(value IMAGE_PROXY_KEY)
		if [ -n "$key" ] && [ "${#key}" -lt 32 ]; then
			bad "IMAGE_PROXY_KEY has fewer than 32 characters"
		fi
		image=$(value RUEDER_IMAGE)
	else
		bad "no .env here"
		image=
	fi

	if have ss; then
		listening=$(ss -Hltn 2>/dev/null | awk '{print $4}')
	elif have netstat; then
		listening=$(netstat -an 2>/dev/null | awk '/LISTEN/ {print $4}')
	else
		listening=
		warn "neither ss nor netstat: open ports not checked"
	fi
	if [ -n "$listening" ]; then
		wide=$(printf '%s\n' "$listening" | grep -E '[:.](3000|5432)$' |
			grep -vE '^(127\.|\[?::1\]?|localhost)' || true)
		if [ -n "$wide" ]; then
			bad "rueder or PostgreSQL listens beyond this machine: $(printf '%s' "$wide" | tr '\n' ' ')"
		else
			ok "ports 3000 and 5432 are not open beyond this machine"
		fi
	fi

	running=$(docker compose images --format '{{.Repository}}:{{.Tag}} {{.ID}}' rueder 2>/dev/null | head -n 1)
	created=$(docker inspect --format '{{.Created}}' "$(docker compose ps -q rueder 2>/dev/null | head -n 1)" 2>/dev/null | cut -c1-10)
	[ -n "$running" ] && ok "rueder runs ${running%% *}, started from an image created on ${created:-?}"
	case $image in
	ghcr.io/*)
		if have gh; then
			if gh attestation verify "oci://$image" --repo FionaPreroll/rueder >/dev/null 2>&1; then
				ok "$image was built by rueder's CI (its attestation checks out)"
			else
				bad "$image has no attestation from rueder's CI that checks out: images from before 2026-11-12 have none (docker compose pull), and gh needs gh auth login"
			fi
		else
			warn "gh is missing: where $image comes from is not checked"
		fi
		;;
	'') ok "the image is built here from this repository" ;;
	esac

	if have apt-get; then
		if have systemctl && systemctl is-enabled unattended-upgrades >/dev/null 2>&1; then
			ok "unattended-upgrades is on"
		else
			warn "unattended-upgrades is off: security updates of the host wait for somebody"
		fi
		pending=$(apt-get -s upgrade 2>/dev/null | grep -c '^Inst' || true)
		if [ "${pending:-0}" -gt 0 ]; then
			warn "$pending updates of the host are waiting (apt list --upgradable)"
		else
			ok "the host has no updates waiting"
		fi
		if [ -f /var/run/reboot-required ]; then
			warn "the host waits for a reboot"
		fi
	fi
}

[ "$server" = true ] && on_server
[ -n "$url" ] && outside

printf '\n'
if [ "$failed" = 1 ]; then
	printf 'Something failed; see the FAIL lines above.\n'
	exit 1
fi
printf 'Nothing failed.\n'
