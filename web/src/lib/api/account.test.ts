// The account's own settings with the server, against a made-up server: what
// they send, and in which words the page hears why the server refused.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Account } from '#lib/reader.ts';
import { ServerAccount } from './account.svelte.ts';
import { ServerReader } from './reader.svelte.ts';
import { FakeServer } from './test/fake-server.ts';

let server: FakeServer;
const session = { signOut: vi.fn(), expired: vi.fn() };

beforeEach(() => {
	server = new FakeServer();
	vi.stubGlobal('fetch', server.fetch);
	session.signOut.mockClear();
	session.expired.mockClear();
});

afterEach(() => vi.unstubAllGlobals());

/** The reader of an account and its settings, as the page has them. */
function opened(account: Account) {
	const reader = new ServerReader(account, session);
	return { reader, settings: new ServerAccount(reader, session) };
}

describe('invitations', () => {
	it('are listed with how many more the reader can make', async () => {
		server.admin = false;
		server.inviteQuota = 3;
		const { settings } = opened({ name: 'fiona', admin: true });
		expect(await settings.invites()).toEqual({ invites: [], left: 3, days: 7 });

		// The link leads to this rueder and is shown this once.
		const made = await settings.invite(2);
		expect(made).toEqual({ link: `${location.origin}/#invite=${server.invites[0].token}` });
		expect(server.requests.at(-1)).toMatchObject({
			method: 'POST',
			path: '/invites',
			body: { uses: 2 }
		});
		const list = await settings.invites();
		expect(list).toMatchObject({ left: 1, invites: [{ max_uses: 2, uses: 0, usable: true }] });
		expect(JSON.stringify(list)).not.toContain(server.invites[0].token);

		// More than are left cannot be made, and taking one back gives its places back.
		expect(await settings.invite(2)).toBe('none-left');
		expect(await settings.revokeInvite(list!.invites[0].id)).toBe(true);
		expect(await settings.invites()).toMatchObject({ left: 3, invites: [{ usable: false }] });
		expect(await settings.revokeInvite(4711)).toBe(false);
	});

	it('have no quota for an admin', async () => {
		const { settings } = opened({ name: 'fiona', admin: true });
		expect(await settings.invite(50)).toMatchObject({ link: expect.stringContaining('#invite=') });
		const list = await settings.invites();
		expect(list?.left).toBeUndefined();
	});

	it('say nothing when the server does not answer', async () => {
		const { settings } = opened({ name: 'fiona', admin: true });
		server.down = true;
		expect(await settings.invites()).toBeNull();
		expect(await settings.invite(1)).toBeNull();
		expect(await settings.revokeInvite(1)).toBe(false);
	});
});

describe('the email address', () => {
	const asking = () => opened({ name: 'fiona', admin: false, emailMode: 'optional' });

	it('is there only where the server keeps addresses', () => {
		expect(opened({ name: 'fiona', admin: false }).settings.can.email).toBe(false);
		expect(opened({ name: 'fiona', admin: false, emailMode: 'off' }).settings.can.email).toBe(
			false
		);
		expect(asking().settings.can.email).toBe(true);
	});

	it('mails a link to a new address, with the password and the language of the page', async () => {
		server.emailMode = 'optional';
		const { reader, settings } = asking();
		expect(await settings.changeEmail(' fiona@example.org ', 'correct horse')).toBeNull();
		expect(reader.account.emailPending).toBe('fiona@example.org');
		expect(server.requests.at(-1)).toMatchObject({
			method: 'PUT',
			path: '/email',
			body: { email: 'fiona@example.org', password: 'correct horse', language: 'en' }
		});
		expect(server.mails).toHaveLength(1);
	});

	it('says why an address was not taken', async () => {
		server.emailMode = 'optional';
		server.emails.set('fiona', 'fiona@example.org');
		const { reader, settings } = asking();
		expect(await settings.changeEmail('new@example.org', 'wrong')).toEqual({ problem: 'wrong' });
		expect(await settings.changeEmail('nowhere', 'correct horse')).toEqual({ problem: 'invalid' });
		expect(await settings.changeEmail('FIONA@example.org', 'correct horse')).toEqual({
			problem: 'same'
		});
		server.refuse('PUT', /^\/email$/, 429, 1, 'too many mails', { 'Retry-After': '600' });
		expect(await settings.changeEmail('new@example.org', 'correct horse')).toEqual({
			problem: 'busy',
			minutes: 10
		});
		server.refuse('PUT', /^\/email$/, 503);
		expect(await settings.changeEmail('new@example.org', 'correct horse')).toEqual({
			problem: 'unsent'
		});
		server.down = true;
		expect(await settings.changeEmail('new@example.org', 'correct horse')).toEqual({
			problem: 'unreachable'
		});
		server.down = false;

		// With a second factor the code is asked for, then checked.
		server.secondFactor = { code: '123456', recovery: [] };
		expect(await settings.changeEmail('new@example.org', 'correct horse')).toEqual({
			problem: 'code'
		});
		expect(await settings.changeEmail('new@example.org', 'correct horse', '000000')).toEqual({
			problem: 'wrong'
		});
		expect(await settings.changeEmail('new@example.org', 'correct horse', '123456')).toBeNull();
		expect(server.mails.at(-1)?.to).toBe('new@example.org');
		expect(reader.account.email).toBeUndefined();
	});

	it('says when only logging in again lets an account without password give one', async () => {
		server.emailMode = 'optional';
		server.refuse('PUT', /^\/email$/, 403, 1, 'log in again first');
		expect(await asking().settings.changeEmail('new@example.org', '')).toEqual({
			problem: 'loginAgain'
		});
	});

	it('takes the address away where it is optional, and not where it is required', async () => {
		server.emailMode = 'optional';
		server.emails.set('fiona', 'fiona@example.org');
		const { reader, settings } = asking();
		reader.account = { ...reader.account, email: 'fiona@example.org' };
		expect(await settings.removeEmail('wrong')).toEqual({ problem: 'wrong' });
		expect(await settings.removeEmail('correct horse')).toBeNull();
		expect(reader.account).toMatchObject({ email: null, emailPending: null });
		expect(server.emails.has('fiona')).toBe(false);

		server.emailMode = 'required';
		expect(await settings.removeEmail('correct horse')).toEqual({ problem: 'required' });
	});
});

describe('a new password', () => {
	const fiona = () => opened({ name: 'fiona', admin: false });

	it('takes the password so far, and the code where the second factor is on', async () => {
		const { reader, settings } = fiona();
		expect(await settings.changePassword('wrong', 'a new password')).toEqual({ problem: 'wrong' });
		server.secondFactor = { code: '123456', recovery: [] };
		expect(await settings.changePassword('correct horse', 'a new password')).toEqual({
			problem: 'code'
		});
		expect(await settings.changePassword('correct horse', 'a new password', '000000')).toEqual({
			problem: 'wrong'
		});
		expect(await settings.changePassword('correct horse', 'a new password', ' 123456 ')).toBeNull();
		expect(server.requests.at(-1)).toMatchObject({
			method: 'PUT',
			path: '/password',
			body: { current: 'correct horse', new: 'a new password', code: '123456' }
		});
		expect(reader.account.password).toBe(true);
	});

	it('says when only logging in again lets an account without password set one', async () => {
		server.refuse('PUT', /^\/password$/, 403, 1, 'log in again first');
		expect(await fiona().settings.changePassword('', 'a first password')).toEqual({
			problem: 'loginAgain'
		});
	});

	it('sends a passkey instead of the password so far and the code', async () => {
		const passkey = { ceremony: 'c', credential: {} };
		await fiona().settings.changePassword('ignored', 'a new password', undefined, passkey);
		const asked = server.requests.find((request) => request.path === '/password');
		expect(asked?.body).toEqual({ new: 'a new password', passkey });
	});
});

describe('young sessions', () => {
	it('say where an account without password and second factor has to log in again', async () => {
		const { settings } = opened({ name: 'fiona', admin: false });
		server.refuse('POST', /^\/app-passwords$/, 403, 1, 'log in again first');
		expect(await settings.createAppPassword('Reeder', '', '')).toBe('loginAgain');
		server.refuse('POST', /^\/second-factor\/setup$/, 403, 1, 'log in again first');
		expect(await settings.setUpSecondFactor('')).toBe('loginAgain');
	});
});
