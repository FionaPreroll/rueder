// Who is logged in, against a made-up server.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError, api, serverId } from './client.ts';
import { Session } from './session.svelte.ts';
import { FakeServer } from './test/fake-server.ts';

let server: FakeServer;

beforeEach(() => {
	server = new FakeServer();
	vi.stubGlobal('fetch', server.fetch);
});

afterEach(() => {
	history.replaceState(null, '', '/');
	vi.unstubAllGlobals();
});

describe('Session', () => {
	it('finds the session the browser has', async () => {
		const session = new Session();
		expect(session.state).toBe('checking');
		await session.check();
		expect(session.state).toBe('in');
		expect(session.reader?.account).toEqual({
			name: 'fiona',
			admin: true,
			password: true,
			adminNeedsSecondFactor: false,
			termsToAccept: null,
			contentReports: false,
			appApi: true
		});
	});

	it('finds none, and asks how one can log in', async () => {
		server.signedIn = null;
		server.oidc = true;
		const session = new Session();
		await session.check();
		expect(session.state).toBe('out');
		expect(session.reader).toBeNull();
		await vi.waitFor(() => expect(session.ways).toMatchObject({ oidc: true, registration: false }));
	});

	it('takes a server that does not say for one without other ways', async () => {
		server.signedIn = null;
		server.refuse('GET', /^\/login\/ways$/, 404);
		const session = new Session();
		await session.check();
		await vi.waitFor(() => expect(server.requests.at(-1)?.path).toBe('/login/ways'));
		expect(session.ways).toMatchObject({ oidc: false, registration: false });
	});

	it('says when the server does not answer, and can ask again', async () => {
		server.down = true;
		const session = new Session();
		await session.check();
		expect(session.state).toBe('unreachable');
		server.down = false;
		await session.check();
		expect(session.state).toBe('in');
		server.refuse('GET', /^\/me$/, 502);
		await session.check();
		expect(session.state).toBe('unreachable');
	});

	it('logs in with name and password', async () => {
		server.signedIn = null;
		const session = new Session();
		await session.check();
		expect(await session.login('Fiona', 'correct horse')).toBeNull();
		expect(session.state).toBe('in');
		expect(server.requests.find((request) => request.path === '/login')).toMatchObject({
			method: 'POST',
			path: '/login',
			body: { username: 'Fiona', password: 'correct horse' }
		});
	});

	it('says why a login did not work', async () => {
		server.signedIn = null;
		const session = new Session();
		await session.check();
		expect(await session.login('fiona', 'wrong')).toEqual({ kind: 'wrong' });
		expect(session.state).toBe('out');
		// A request the server cannot read is a wrong name or password, too.
		server.refuse('POST', /^\/login$/, 422);
		expect(await session.login('fiona', 'x')).toEqual({ kind: 'wrong' });

		server.lockedFor = 15 * 60;
		expect(await session.login('fiona', 'correct horse')).toEqual({ kind: 'tooMany', minutes: 15 });
		server.lockedFor = 61;
		expect(await session.login('fiona', 'correct horse')).toEqual({ kind: 'tooMany', minutes: 2 });
		server.lockedFor = 0;

		// Only who knows the password hears that the account is disabled.
		server.disabled.add('fiona');
		expect(await session.login('fiona', 'wrong')).toEqual({ kind: 'wrong' });
		expect(await session.login('fiona', 'correct horse')).toEqual({ kind: 'disabled' });
		expect(session.state).toBe('out');
		server.disabled.clear();

		server.refuse('POST', /^\/login$/, 500);
		expect(await session.login('fiona', 'correct horse')).toEqual({ kind: 'unreachable' });
		server.down = true;
		expect(await session.login('fiona', 'correct horse')).toEqual({ kind: 'unreachable' });
	});

	it('logs out, whether or not the server hears it', async () => {
		const session = new Session();
		await session.check();
		session.reader?.signOut();
		await vi.waitFor(() => expect(session.state).toBe('out'));
		expect(server.signedIn).toBeNull();
		expect(session.reader).toBeNull();

		await session.login('fiona', 'correct horse');
		server.down = true;
		await session.signOut();
		expect(session.state).toBe('out');
	});

	it('deletes the account with its password and says so at the login', async () => {
		const session = new Session();
		await session.check();
		const reader = session.reader!;
		expect(reader.can.remove).toBe(true);
		expect(await reader.removeAccount('wrong')).toEqual({ problem: 'wrong' });
		expect(session.state).toBe('in');
		server.refuse('DELETE', /^\/account$/, 429, 1, 'too many', { 'Retry-After': '90' });
		expect(await reader.removeAccount('correct horse')).toEqual({ problem: 'busy', minutes: 2 });
		server.refuse('DELETE', /^\/account$/, 500);
		expect(await reader.removeAccount('correct horse')).toEqual({ problem: 'unreachable' });

		expect(session.farewell).toBe(false);
		expect(await reader.removeAccount('correct horse')).toBeNull();
		expect(server.requests.findLast((request) => request.method === 'DELETE')).toMatchObject({
			path: '/account',
			body: { password: 'correct horse' }
		});
		expect(session.state).toBe('out');
		expect(session.reader).toBeNull();
		expect(session.farewell).toBe(true);
		expect(server.accounts.has('fiona')).toBe(false);
		// Whoever logs in next is not told again.
		server.accounts.set('grace', 'a long password');
		await session.login('grace', 'a long password');
		expect(session.farewell).toBe(false);
	});

	it('deletes an account without a password by its name', async () => {
		// As one that logs in only through single sign-on.
		server.accounts.delete('fiona');
		const session = new Session();
		await session.check();
		const reader = session.reader!;
		expect(reader.account?.password).toBe(false);
		expect(await reader.removeAccount('grace')).toEqual({ problem: 'wrong' });
		expect(await reader.removeAccount('Fiona')).toBeNull();
		const asked = server.requests.findLast((request) => request.method === 'DELETE');
		expect(asked?.body).toEqual({ name: 'Fiona' });
		expect(session.state).toBe('out');
	});

	it('shows the login when the server no longer knows the session', async () => {
		const session = new Session();
		await session.check();
		const reader = session.reader!;
		server.signedIn = null;
		reader.refresh();
		await vi.waitFor(() => expect(session.state).toBe('out'));
		expect(session.reader).toBeNull();
	});
});

describe('registering', () => {
	const invitation = (uses = 1) => {
		const invite = {
			id: 1,
			token: 'a-token',
			max_uses: uses,
			uses: 0,
			created_at: new Date().toISOString(),
			expires_at: new Date(Date.now() + 86_400_000).toISOString(),
			revoked: false
		};
		server.invites = [invite];
		return invite;
	};

	beforeEach(() => {
		server.signedIn = null;
	});

	it('is offered where the server lets anybody register', async () => {
		server.registrationOpen = true;
		const session = new Session();
		await session.check();
		await vi.waitFor(() => expect(session.ways.registration).toBe(true));
		expect(session.invitation).toBeNull();

		expect(await session.register(' Grace ', 'a long password')).toBeNull();
		expect(session.state).toBe('in');
		expect(session.reader?.account).toEqual({
			name: 'grace',
			admin: false,
			password: true,
			adminNeedsSecondFactor: false,
			termsToAccept: null,
			contentReports: false,
			appApi: true
		});
		expect(server.requests.find((request) => request.method === 'POST')).toMatchObject({
			method: 'POST',
			path: '/register',
			body: { username: ' Grace ', password: 'a long password' }
		});
	});

	it('takes the invitation from the address the page was opened with', async () => {
		const invite = invitation();
		history.replaceState(null, '', '/?invite=a-token');
		const session = new Session();
		await session.check();
		await vi.waitFor(() => expect(session.invitation).toEqual({ token: 'a-token', valid: true }));
		expect(session.ways.registration).toBe(false);

		expect(await session.register('grace', 'a long password')).toBeNull();
		const made = server.requests.find((r) => r.method === 'POST' && r.path === '/register');
		expect(made?.body).toMatchObject({ invite: 'a-token' });
		expect(invite.uses).toBe(1);
		expect(session.invitation).toBeNull();
	});

	it('says when the invitation cannot be used', async () => {
		invitation().revoked = true;
		history.replaceState(null, '', '/?invite=a-token');
		const session = new Session();
		await session.check();
		await vi.waitFor(() => expect(session.invitation).toEqual({ token: 'a-token', valid: false }));
		expect(await session.register('grace', 'a long password')).toEqual({ kind: 'invite' });
		expect(session.state).toBe('out');

		// A server that does not answer the question leaves the invitation unusable.
		server.down = true;
		const other = new Session();
		await other.check();
		server.down = false;
		expect(other.state).toBe('unreachable');
	});

	it('says why nobody was registered', async () => {
		const session = new Session();
		await session.check();
		expect(await session.register('grace', 'a long password')).toEqual({ kind: 'closed' });
		server.registrationOpen = true;
		server.noRoom = 'full';
		expect(await session.register('grace', 'a long password')).toEqual({ kind: 'full' });
		server.noRoom = 'today';
		expect(await session.register('grace', 'a long password')).toEqual({ kind: 'today' });
		server.noRoom = null;
		expect(await session.register('Fiona', 'a long password')).toEqual({ kind: 'taken' });
		expect(await session.register('grace', 'short')).toEqual({
			kind: 'invalid',
			reason: 'the password is shorter than 8 characters'
		});
		server.refuse('POST', /^\/register$/, 500);
		expect(await session.register('grace', 'a long password')).toEqual({ kind: 'unreachable' });
		server.down = true;
		expect(await session.register('grace', 'a long password')).toEqual({ kind: 'unreachable' });
		expect(session.state).toBe('out');
	});
});

describe('the API client', () => {
	it('tells a refusal from no answer and from an answer that is not the server’s', async () => {
		server.refuse('GET', /^\/tree$/, 503);
		const refused = await api.tree().catch((error: unknown) => error);
		expect(refused).toBeInstanceOf(ApiError);
		expect(refused).toMatchObject({ status: 503, message: 'refused', signedOut: false });

		server.signedIn = null;
		expect(await api.me().catch((error: unknown) => error)).toMatchObject({
			status: 401,
			signedOut: true
		});

		// A proxy's error page in the place of the server's JSON.
		vi.stubGlobal('fetch', async () => new Response('<html>Bad gateway</html>', { status: 200 }));
		expect(await api.tree().catch((error: unknown) => error)).toMatchObject({ status: 0 });
	});

	it('asks only its own server, with the session cookie', async () => {
		const fetch = vi.fn(server.fetch);
		vi.stubGlobal('fetch', fetch);
		const feed = server.addFeed('Feed');
		server.addArticles(feed, 2);
		await api.setState(feed, 2, { starred: true });
		const [address, options] = fetch.mock.calls[0];
		expect(address).toBe(`/api/feeds/${feed}/articles/2/state`);
		expect(options).toMatchObject({
			method: 'PUT',
			credentials: 'same-origin',
			headers: { 'Content-Type': 'application/json' },
			body: '{"starred":true}'
		});
	});

	it('finds the server’s number in an id of the page', () => {
		expect(serverId('feed-12')).toBe(12);
		expect(serverId('folder-3')).toBe(3);
	});
});
