// Where the pieces of an end-to-end run listen. All on this machine, on ports
// of their own, so a run can stand next to a rueder that is being worked on.

/** The rueder server under test. `localhost`, as browsers keep its session cookie only for HTTPS and for that name. */
export const RUEDER = 'http://localhost:3210';
export const RUEDER_LISTEN = '127.0.0.1:3210';

/** The made-up website with its feeds (site.js). */
export const SITE_PORT = 3211;
export const SITE = `http://127.0.0.1:${SITE_PORT}`;

/**
 * The made-up mail relay (mail.js): rueder hands its mail over by SMTP on the
 * one port, the tests read it over HTTP on the other.
 */
export const SMTP_PORT = 3212;
export const MAILBOX_PORT = 3213;
export const MAILBOX = `http://127.0.0.1:${MAILBOX_PORT}`;

/** What rueder needs to send mail to that relay. */
export const MAIL_ENV = {
	SMTP_URL: `smtp://127.0.0.1:${SMTP_PORT}`,
	MAIL_FROM: 'rueder <rueder@rueder.test>',
	PUBLIC_URL: RUEDER
};

/** The database of a run; it is made anew every time. */
export const DATABASE = 'rueder_e2e';

/** The server PostgreSQL runs on, with an account that may create databases. */
export const postgres = () =>
	new URL(process.env.DATABASE_URL ?? 'postgres://postgres@127.0.0.1:5432/postgres');

/** The address of the run's database on that server. */
export function database() {
	const url = postgres();
	url.pathname = `/${DATABASE}`;
	return url.toString();
}

/** The accounts the tests log in as; each spec has its own, so one spec's reading does not show in another. */
export const PASSWORD = 'correct horse battery';
export const ACCOUNTS = [
	'login',
	'reading',
	'tree',
	'rules',
	'live',
	'other',
	'inviter',
	'moving',
	'moved',
	'search',
	'darkbars',
	'guarded',
	'leaving',
	'barred',
	'reporting',
	'panes',
	'apps',
	'secrets',
	'keyholder',
	'unverified',
	'unreader',
	'cutoff',
	'waiter',
	'addressee'
];
