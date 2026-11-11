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
	vi.unstubAllGlobals();
});

describe('Session', () => {
	it('finds the session the browser has', async () => {
		const session = new Session();
		expect(session.state).toBe('checking');
		await session.check();
		expect(session.state).toBe('in');
		expect(session.reader?.account).toEqual({ name: 'fiona', admin: true });
	});

	it('finds none, and asks how one can log in', async () => {
		server.signedIn = null;
		server.oidc = true;
		const session = new Session();
		await session.check();
		expect(session.state).toBe('out');
		expect(session.reader).toBeNull();
		await vi.waitFor(() => expect(session.ways).toEqual({ oidc: true }));
	});

	it('takes a server that does not say for one without other ways', async () => {
		server.signedIn = null;
		server.refuse('GET', /^\/login\/ways$/, 404);
		const session = new Session();
		await session.check();
		await vi.waitFor(() => expect(server.requests.at(-1)?.path).toBe('/login/ways'));
		expect(session.ways).toEqual({ oidc: false });
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
		expect(server.requests.at(-1)).toMatchObject({
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
