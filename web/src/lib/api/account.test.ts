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

describe('refusals', () => {
	const fiona = () => opened({ name: 'fiona', admin: false });
	/** What the settings make of the server's next answer to this request. */
	async function heard(
		method: string,
		path: RegExp,
		status: number,
		reason: string,
		ask: () => Promise<unknown>,
		headers: Record<string, string> = {}
	) {
		server.refuse(method, path, status, 1, reason, headers);
		return ask();
	}

	it('of an app password say what to do about it', async () => {
		const { settings } = fiona();
		const make = () => settings.createAppPassword('Reeder', 'correct horse', '');
		const path = /^\/app-passwords$/;
		for (const [status, reason, refusal] of [
			[403, 'second factor needed', 'codeNeeded'],
			[403, 'the code is wrong', 'wrongCode'],
			[403, 'the passkey is wrong', 'wrongCode'],
			[403, 'wrong password', 'wrongPassword'],
			[400, 'an account has at most 20 app passwords', 'tooManyApps'],
			[429, 'too many wrong tries, try again later', 'tooMany'],
			[500, 'internal server error', 'unreachable']
		] as const) {
			expect(await heard('POST', path, status, reason, make), `${status} ${reason}`).toBe(refusal);
		}
		server.down = true;
		expect(await make()).toBe('unreachable');
		expect(session.expired).not.toHaveBeenCalled();
	});

	it('of the second factor say what to do about it', async () => {
		const { settings } = fiona();
		const setUp = () => settings.setUpSecondFactor('correct horse');
		const setup = /^\/second-factor\/setup$/;
		for (const [status, reason, refusal] of [
			[403, 'second factor needed', 'codeNeeded'],
			[403, 'the code is wrong', 'wrongCode'],
			[403, 'the passkey is wrong', 'wrongCode'],
			[403, 'wrong password', 'wrongPassword'],
			[429, 'too many wrong tries, try again later', 'tooMany'],
			[500, 'internal server error', 'unreachable']
		] as const) {
			expect(await heard('POST', setup, status, reason, setUp), `${status} ${reason}`).toBe(
				refusal
			);
		}

		const confirm = () => settings.confirmSecondFactor(' 123456 ');
		const confirming = /^\/second-factor\/confirm$/;
		expect(await heard('POST', confirming, 403, 'the code is wrong', confirm)).toBe('wrongCode');
		expect(await heard('POST', confirming, 429, 'too many wrong tries', confirm)).toBe('tooMany');

		const off = () => settings.switchOffSecondFactor('correct horse', '123456');
		const switching = /^\/second-factor$/;
		for (const [reason, refusal] of [
			['the code is wrong', 'wrongCode'],
			['the passkey is wrong', 'wrongCode'],
			['wrong password', 'wrongPassword']
		] as const) {
			expect(await heard('DELETE', switching, 403, reason, off), reason).toBe(refusal);
		}
		expect(await heard('DELETE', switching, 429, 'too many', off)).toBe('tooMany');

		// Asked while the session is over, the page logs out.
		server.refuse('GET', /^\/second-factor$/, 401, 1, 'not logged in');
		expect(await settings.secondFactor()).toBeNull();
		expect(session.expired).toHaveBeenCalledTimes(1);
	});

	it('of passkeys say what to do about it', async () => {
		server.passkeysOn = true;
		const { settings } = fiona();
		const answers = [
			[403, 'second factor needed', 'codeNeeded'],
			[403, 'the code is wrong', 'wrongCode'],
			[403, 'the passkey is wrong', 'wrong'],
			[403, 'log in again first', 'loginAgain'],
			[403, 'wrong password', 'wrongPassword'],
			[400, 'an account has at most 10 passkeys', 'tooManyPasskeys'],
			[400, 'the passkey could not be checked', 'failed'],
			[409, 'this passkey is known already', 'known'],
			[409, 'no such ceremony', 'wrong'],
			[429, 'too many passkey ceremonies, try again later', 'tooMany'],
			[500, 'internal server error', 'unreachable']
		] as const;
		for (const [method, path, ask] of [
			['POST', /^\/passkeys\/register$/, () => settings.addPasskey('Phone', 'correct horse', '')],
			['DELETE', /^\/passkeys\/7$/, () => settings.removePasskey(7, 'correct horse', '')],
			['POST', /^\/passkeys\/proof$/, () => settings.provePasskey()]
		] as const) {
			for (const [status, reason, refusal] of answers) {
				expect(await heard(method, path, status, reason, ask), `${method} ${reason}`).toBe(refusal);
			}
		}
		server.refuse('DELETE', /^\/passkeys\/7$/, 401, 1, 'not logged in');
		expect(await settings.removePasskey(7, 'correct horse', '')).toBe('unreachable');
		expect(session.expired).toHaveBeenCalledTimes(1);
	});

	it('of deleting the account say what to do about it', async () => {
		const { settings } = fiona();
		const path = /^\/account$/;
		const remove = (code?: string) => () => settings.removeAccount('correct horse', code);
		expect(await heard('DELETE', path, 403, 'second factor needed', remove())).toEqual({
			problem: 'code'
		});
		// With a code given, a refusal means that something given was wrong.
		expect(await heard('DELETE', path, 403, 'second factor needed', remove('123456'))).toEqual({
			problem: 'wrong'
		});
		expect(await heard('DELETE', path, 403, 'wrong password', remove())).toEqual({
			problem: 'wrong'
		});
		const busy = await heard('DELETE', path, 429, 'too many', remove(), { 'Retry-After': '61' });
		expect(busy).toEqual({ problem: 'busy', minutes: 2 });
		expect(await heard('DELETE', path, 429, 'too many', remove())).toEqual({
			problem: 'busy',
			minutes: 1
		});
		expect(await heard('DELETE', path, 500, 'internal server error', remove())).toEqual({
			problem: 'unreachable'
		});
		expect(session.expired).not.toHaveBeenCalled();
		// An account without a password says its name instead.
		const { settings: nameless } = opened({ name: 'fiona', admin: false, password: false });
		await heard('DELETE', path, 403, 'wrong name', () => nameless.removeAccount('fiona'));
		expect(server.requests.at(-1)).toMatchObject({ body: { name: 'fiona' } });
	});

	it('of a new password say what to do about it', async () => {
		const { settings } = fiona();
		const path = /^\/password$/;
		const change = () => settings.changePassword('correct horse', 'short');
		expect(await heard('PUT', path, 400, 'the password is too short', change)).toEqual({
			problem: 'invalid'
		});
		expect(await heard('PUT', path, 429, 'too many', change, { 'Retry-After': '300' })).toEqual({
			problem: 'busy',
			minutes: 5
		});
		expect(await heard('PUT', path, 500, 'internal server error', change)).toEqual({
			problem: 'unreachable'
		});
	});
});
