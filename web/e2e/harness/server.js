// Starts the rueder under test for an end-to-end run: a database of its own,
// made anew, the accounts the tests log in as, and the server itself with the
// built frontend, allowed to fetch from the made-up site on this machine.
//
//   node e2e/harness/server.js
//
// It needs the server's binary (RUEDER_BIN, or `cargo build` in ../server), the
// frontend in build/ (`pnpm build`), and a PostgreSQL that DATABASE_URL names,
// with `psql` to make the database.

import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import {
	ACCOUNTS,
	DATABASE,
	PASSWORD,
	RUEDER,
	RUEDER_LISTEN,
	database,
	postgres
} from './ports.js';

const web = resolve(import.meta.dirname, '../..');
const rueder = resolve(web, process.env.RUEDER_BIN ?? '../server/target/debug/rueder');
for (const [path, how] of [
	[rueder, 'run `cargo build` in server/, or name the binary in RUEDER_BIN'],
	[join(web, 'build/index.html'), 'run `pnpm build` in web/']
]) {
	if (!existsSync(path)) {
		console.error(`${path} is not there: ${how}`);
		process.exit(1);
	}
}

const psql = (sql) =>
	execFileSync('psql', [postgres().toString(), '--quiet', '--set=ON_ERROR_STOP=1', '-c', sql], {
		stdio: ['ignore', 'inherit', 'inherit']
	});
psql(`DROP DATABASE IF EXISTS ${DATABASE} WITH (FORCE)`);
psql(`CREATE DATABASE ${DATABASE}`);

const env = {
	...process.env,
	DATABASE_URL: database(),
	LISTEN_ADDR: RUEDER_LISTEN,
	STATIC_DIR: join(web, 'build'),
	// The made-up site is on this machine, where rueder fetches nothing unasked.
	FETCH_ALLOW_NETS: '127.0.0.0/8',
	IMAGE_CACHE_DIR: mkdtempSync(join(tmpdir(), 'rueder-e2e-images-')),
	// Never the settings of a rueder that is being worked on.
	OIDC_ISSUER: '',
	CLIENT_IP_HEADER: '',
	// Readers may report feeds and articles (reporting.spec.ts).
	CONTENT_REPORTS: 'on',
	// Passkeys, for the address the browser reaches the server at
	// (passkeys.spec.ts); browsers allow them over HTTP for localhost.
	PUBLIC_URL: RUEDER
};

// The screenshots (screenshots/) name accounts of their own, all of them
// admins, so that every view an admin has is in them.
const own = process.env.E2E_ACCOUNTS?.split(',');

// The first command also applies the migrations.
for (const name of own ?? ACCOUNTS) {
	const admin = own !== undefined || name === 'login';
	execFileSync(rueder, ['account', 'add', name, ...(admin ? ['--admin'] : [])], {
		env,
		input: `${PASSWORD}\n`,
		stdio: ['pipe', 'inherit', 'inherit']
	});
}

const server = spawn(rueder, ['serve'], { env, stdio: 'inherit' });
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => server.kill(signal));
server.on('exit', (code) => process.exit(code ?? 0));
