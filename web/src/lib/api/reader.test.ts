// The reader that talks to the server, against a made-up server: what it loads,
// what it sends, and that a change shows at once and is taken back when the server
// refuses it.

import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { TreeNode } from '#lib/model.ts';
import { PAGE, ServerReader, toTree } from './reader.svelte.ts';
import { FakeServer } from './test/fake-server.ts';

let server: FakeServer;
let news: number;
let tech: number;
let blog: number;
let lone: number;
const session = { signOut: vi.fn(), expired: vi.fn() };

/** Feeds in two folders and one outside, each with a few articles. */
beforeEach(() => {
	server = new FakeServer();
	news = server.addFolder('News');
	tech = server.addFeed('Tech', news);
	blog = server.addFeed('Blog', news);
	lone = server.addFeed('Lone');
	server.addArticles(tech, 3);
	server.addArticles(blog, 2);
	server.addArticles(lone, 1);
	vi.stubGlobal('fetch', server.fetch);
	session.signOut.mockClear();
	session.expired.mockClear();
	localStorage.clear();
});

afterEach(() => {
	vi.unstubAllGlobals();
});

/** Lets every answer that is on its way arrive, and what follows from it. */
async function settle() {
	for (let i = 0; i < 20; i++) await new Promise((resolve) => setTimeout(resolve, 0));
	flushSync();
}

async function started() {
	const reader = new ServerReader({ name: 'fiona', admin: true }, session);
	reader.start();
	await settle();
	return reader;
}

const titles = (reader: ServerReader) => reader.visible.map((article) => article.title);
const shape = (nodes: TreeNode[]): unknown[] =>
	nodes.map((node) =>
		node.kind === 'feed' ? node.id : { [`${node.title} (${node.id})`]: shape(node.children) }
	);

describe('the tree', () => {
	it('comes from the server, with the unread counts', async () => {
		const reader = await started();
		expect(shape(reader.tree)).toEqual([
			{ [`News (folder-${news})`]: [`feed-${tech}`, `feed-${blog}`] },
			`feed-${lone}`
		]);
		expect(reader.feeds[`feed-${tech}`]).toMatchObject({ title: 'Tech', lastSeq: 3, error: null });
		expect(reader.unreadIn(reader.tree[0])).toBe(5);
		expect(reader.unreadIn(reader.tree[1])).toBe(1);
		expect(reader.totalUnread).toBe(6);
		expect(reader.title({ kind: 'feed', id: `feed-${tech}` })).toBe('Tech');
		expect(reader.title({ kind: 'folder', id: `folder-${news}` })).toBe('News');
		expect(reader.title({ kind: 'all' })).toBeNull();
		expect(reader.account).toEqual({ name: 'fiona', admin: true });
	});

	it('sorts a level by position, folders first where two are the same', () => {
		const tree = toTree({
			folders: [
				{ id: 2, parent_id: null, title: 'B', position: 2 },
				{ id: 1, parent_id: null, title: 'A', position: 2 },
				{ id: 3, parent_id: 1, title: 'Inside', position: 1 },
				// Rows that name each other as parent are not drawn in circles.
				{ id: 8, parent_id: 9, title: 'Loop', position: 1 },
				{ id: 9, parent_id: 8, title: 'Loop', position: 1 }
			],
			feeds: [
				{ id: 5, folder_id: null, position: 2 },
				{ id: 4, folder_id: null, position: 1 },
				{ id: 6, folder_id: 3, position: 1 }
			].map((feed) => ({
				...feed,
				title: '',
				feed_title: '',
				feed_url: '',
				site_url: null,
				icon_url: null,
				fetch_agent: '',
				error: null,
				last_seq: 0,
				read_upto: 0,
				unread: 0
			}))
		});
		expect(shape(tree)).toEqual([
			'feed-4',
			{ 'A (folder-1)': [{ 'Inside (folder-3)': ['feed-6'] }] },
			{ 'B (folder-2)': [] },
			'feed-5'
		]);
	});

	it('shows a feed that fails, and its icon at the proxy', async () => {
		server.feeds[0].error = 'timed out';
		server.feeds[0].icon_url = '/img/sig/icon/abc';
		const reader = await started();
		expect(reader.feeds[`feed-${tech}`]).toMatchObject({
			error: 'timed out',
			icon: '/img/sig/icon/abc'
		});
	});

	it('moves a node at once and tells the server', async () => {
		const reader = await started();
		reader.move(`feed-${lone}`, `folder-${news}`, 'inside');
		expect(shape(reader.tree)).toEqual([
			{ [`News (folder-${news})`]: [`feed-${tech}`, `feed-${blog}`, `feed-${lone}`] }
		]);
		await settle();
		expect(server.requests.at(-1)).toMatchObject({
			method: 'POST',
			path: '/tree/move',
			body: {
				node: { kind: 'feed', id: lone },
				target: { kind: 'folder', id: news },
				position: 'inside'
			}
		});
		expect(server.feeds.find((feed) => feed.id === lone)?.folder_id).toBe(news);
		// A move that is not allowed is not even asked for.
		const asked = server.requests.length;
		reader.move(`folder-${news}`, `feed-${lone}`, 'inside');
		expect(reader.canMove(`folder-${news}`, `feed-${lone}`, 'inside')).toBe(false);
		await settle();
		expect(server.requests.length).toBe(asked);
	});

	it('takes a move back when the server refuses it', async () => {
		const reader = await started();
		const before = shape(reader.tree);
		server.refuse('POST', /^\/tree\/move$/, 400);
		expect(reader.nudge(`feed-${lone}`, 'up')).toBe(true);
		expect(shape(reader.tree)).not.toEqual(before);
		await settle();
		expect(shape(reader.tree)).toEqual(before);
		expect(reader.notice).toBe('notSaved');
		reader.dismissNotice();
		expect(reader.notice).toBeNull();
		expect(reader.nudge(`folder-${news}`, 'up')).toBe(false);
	});

	it('adds, renames and removes folders', async () => {
		const reader = await started();
		reader.addFolder('Later');
		await settle();
		const later = server.folders.find((folder) => folder.title === 'Later')!;
		expect(shape(reader.tree).at(-1)).toEqual({ [`Later (folder-${later.id})`]: [] });

		reader.renameFolder(`folder-${later.id}`, 'Sooner');
		expect(reader.title({ kind: 'folder', id: `folder-${later.id}` })).toBe('Sooner');
		await settle();
		expect(later.title).toBe('Sooner');

		reader.show({ kind: 'folder', id: `folder-${news}` });
		reader.removeFolder(`folder-${news}`);
		// What was in the folder moves to where it was, and the list goes to all items.
		expect(shape(reader.tree).slice(0, 2)).toEqual([`feed-${tech}`, `feed-${blog}`]);
		expect(reader.stream).toEqual({ kind: 'all' });
		await settle();
		expect(server.folders.map((folder) => folder.title)).toEqual(['Sooner']);
		expect(server.writes).toEqual([
			'POST /folders',
			`PATCH /folders/${later.id}`,
			`DELETE /folders/${news}`
		]);
	});

	it('remembers closed folders in the browser', async () => {
		const reader = await started();
		reader.setClosed(`folder-${news}`, true);
		expect(reader.isClosed(`folder-${news}`)).toBe(true);
		expect((await started()).isClosed(`folder-${news}`)).toBe(true);
		reader.setClosed(`folder-${news}`, false);
		expect((await started()).isClosed(`folder-${news}`)).toBe(false);
		// Moving something into a closed folder opens it.
		reader.setClosed(`folder-${news}`, true);
		reader.move(`feed-${lone}`, `folder-${news}`, 'inside');
		expect(reader.isClosed(`folder-${news}`)).toBe(false);
		localStorage.setItem('rueder.closed', 'not json');
		expect((await started()).isClosed(`folder-${news}`)).toBe(false);
	});

	it('leaves a feed or folder that is gone after the tree was loaded anew', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${lone}` });
		await settle();
		server.feeds = server.feeds.filter((feed) => feed.id !== lone);
		reader.refresh();
		await settle();
		expect(reader.stream).toEqual({ kind: 'all' });
		expect(reader.feeds[`feed-${lone}`]).toBeUndefined();
	});
});

describe('the lists', () => {
	it('show a feed, a folder and everything, newest first', async () => {
		const reader = await started();
		expect(reader.loading).toBe(false);
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		expect(reader.loading).toBe(true);
		expect(reader.visible).toEqual([]);
		await settle();
		expect(reader.loading).toBe(false);
		expect(titles(reader)).toEqual([
			`Article 3 of feed ${tech}`,
			`Article 2 of feed ${tech}`,
			`Article 1 of feed ${tech}`
		]);
		const [first] = reader.visible;
		expect(first).toMatchObject({ feedId: `feed-${tech}`, seq: 3, html: null, changes: 0 });
		expect(first.link).toBe(`https://feed-${tech}.example/3`);

		reader.show({ kind: 'folder', id: `folder-${news}` });
		await settle();
		expect(reader.visible.map((a) => a.feedId)).toEqual(
			[blog, blog, tech, tech, tech].map((id) => `feed-${id}`)
		);
		reader.show({ kind: 'all' });
		await settle();
		expect(reader.visible.length).toBe(6);
		expect(reader.hasMore).toBe(false);
	});

	it('fetch the next page when asked, and only once at a time', async () => {
		server.addArticles(lone, PAGE + 5);
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${lone}` });
		await settle();
		expect(reader.visible.length).toBe(PAGE);
		expect(reader.hasMore).toBe(true);
		const oldest = reader.visible.at(-1)!;

		reader.showMore();
		reader.showMore();
		await settle();
		expect(reader.visible.length).toBe(PAGE + 6);
		expect(reader.hasMore).toBe(false);
		const pages = server.requests.filter((r) => r.path.includes('before='));
		expect(pages.map((r) => r.path)).toEqual([
			`/feeds/${lone}/articles?limit=${PAGE}&before=${oldest.id}`
		]);
		reader.showMore();
		await settle();
		expect(server.requests.filter((r) => r.path.includes('before=')).length).toBe(1);
	});

	it('leave out what was read when asked to, and keep what is read while they are up', async () => {
		server.article(tech, 2)!.read = true;
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		expect(reader.visible.length).toBe(3);

		reader.unreadOnly = true;
		await settle();
		expect(server.requests.at(-1)?.path).toBe(`/feeds/${tech}/articles?limit=${PAGE}&unread=true`);
		expect(reader.visible.map((a) => a.seq)).toEqual([3, 1]);
		// Reading one does not take it from under the reader's eyes.
		reader.select(reader.visible[0]);
		await settle();
		expect(reader.visible.map((a) => a.seq)).toEqual([3, 1]);
		reader.refresh();
		await settle();
		expect(reader.visible.map((a) => a.seq)).toEqual([1]);

		// Setting it to what it is asks nothing; the server's lists start with the newest.
		const asked = server.requests.length;
		reader.unreadOnly = true;
		reader.newestFirst = false;
		expect(reader.newestFirst).toBe(true);
		await settle();
		expect(server.requests.length).toBe(asked);
	});

	it('show the starred and the hidden, where unread-only does not count', async () => {
		server.article(tech, 1)!.starred = true;
		server.article(tech, 1)!.read = true;
		server.article(blog, 1)!.hidden = true;
		const reader = await started();
		reader.unreadOnly = true;
		reader.show({ kind: 'starred' });
		await settle();
		expect(server.requests.at(-1)?.path).toBe(`/starred?limit=${PAGE}`);
		expect(reader.visible.map((a) => [a.feedId, a.seq])).toEqual([[`feed-${tech}`, 1]]);
		expect(reader.isStarred(reader.visible[0])).toBe(true);
		expect(reader.isHidden(reader.visible[0])).toBe(false);

		reader.show({ kind: 'hidden' });
		await settle();
		expect(reader.visible.map((a) => [a.feedId, a.seq])).toEqual([[`feed-${blog}`, 1]]);
		expect(reader.isHidden(reader.visible[0])).toBe(true);
		// A hidden article never counts as unread, read or not.
		const unread = reader.totalUnread;
		reader.setRead(reader.visible[0], true);
		expect(reader.totalUnread).toBe(unread);
	});

	it('search what is loaded', async () => {
		server.article(tech, 2)!.title = 'A Rare Word';
		server.article(blog, 1)!.author = 'Rarely Seen';
		const reader = await started();
		reader.show({ kind: 'all' });
		await settle();
		reader.query = ' rare ';
		expect(reader.visible.map((a) => [a.feedId, a.seq])).toEqual([
			[`feed-${blog}`, 1],
			[`feed-${tech}`, 2]
		]);
		reader.query = 'teaser 3';
		expect(reader.visible.length).toBe(1);
		// Another list starts without the search.
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		expect(reader.query).toBe('');
	});

	it('drop an answer that comes after another list was asked for', async () => {
		const reader = await started();
		const release = server.hold(new RegExp(`^/feeds/${tech}/articles`));
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		reader.show({ kind: 'feed', id: `feed-${lone}` });
		await settle();
		expect(reader.visible.map((a) => a.feedId)).toEqual([`feed-${lone}`]);
		release();
		await settle();
		expect(reader.visible.map((a) => a.feedId)).toEqual([`feed-${lone}`]);
		expect(reader.loading).toBe(false);
	});

	it('say when the server does not answer', async () => {
		const reader = await started();
		server.down = true;
		reader.show({ kind: 'all' });
		await settle();
		expect(reader.notice).toBe('unreachable');
		expect(reader.loading).toBe(false);
		expect(reader.visible).toEqual([]);
		server.down = false;
		reader.dismissNotice();
		reader.refresh();
		await settle();
		expect(reader.notice).toBeNull();
		expect(reader.visible.length).toBe(6);
	});
});

describe('the start page', () => {
	it('has the feeds with something new and the latest stars', async () => {
		server.article(tech, 3)!.read = true;
		server.article(lone, 1)!.starred = true;
		const reader = await started();
		expect(reader.stream).toEqual({ kind: 'home' });
		expect(server.requests.map((r) => r.path)).toContain('/articles?limit=60&unread=true');
		// The freshest feed first, each with its two newest unread articles.
		expect(
			reader.fresh.map(({ feed, count, top }) => [feed.title, count, top.map((a) => a.seq)])
		).toEqual([
			['Lone', 1, [1]],
			['Blog', 2, [2, 1]],
			['Tech', 2, [2, 1]]
		]);
		expect(reader.latestStarred.map((a) => a.feedId)).toEqual([`feed-${lone}`]);

		// Reading an article takes it from the start page at once.
		reader.setRead(reader.fresh[0].top[0], true);
		expect(reader.fresh.map(({ feed }) => feed.title)).toEqual(['Blog', 'Tech']);
		reader.toggleStar(reader.latestStarred[0]);
		expect(reader.latestStarred).toEqual([]);
		// There is no page to turn here, and nothing to mark as read.
		reader.showMore();
		reader.markAllRead();
		await settle();
		expect(server.writes.filter((write) => write.includes('/read'))).toEqual([]);
	});
});

describe('reading', () => {
	it('marks an opened article as read at once and tells the server', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		const article = reader.visible[0];
		const release = server.hold(/\/state$/);

		reader.select(article);
		expect(reader.isRead(article)).toBe(true);
		expect(reader.unreadIn(reader.tree[0])).toBe(4);
		expect(reader.totalUnread).toBe(5);
		expect(reader.cursor).toBe(article.id);
		expect(reader.open).toBe(article.id);
		expect(reader.current).toBe(article);
		expect(server.article(tech, 3)!.read).toBe(false);

		release();
		await settle();
		expect(server.article(tech, 3)!.read).toBe(true);
		expect(server.requests.at(-1)).toMatchObject({
			method: 'PUT',
			path: `/feeds/${tech}/articles/3/state`,
			body: { read: true }
		});
		expect(reader.notice).toBeNull();
		// Reading what is read asks nothing.
		const asked = server.requests.length;
		reader.setRead(article, true);
		await settle();
		expect(server.requests.length).toBe(asked);
	});

	it('takes a change back when the server refuses it', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		const article = reader.visible[0];
		server.refuse('PUT', /\/state$/, 500);

		reader.setRead(article, true);
		expect(reader.isRead(article)).toBe(true);
		expect(reader.totalUnread).toBe(5);
		await settle();
		expect(reader.isRead(article)).toBe(false);
		expect(reader.totalUnread).toBe(6);
		expect(reader.notice).toBe('notSaved');
		// The counts were asked for anew, so they are the server's again.
		expect(server.requests.at(-1)?.path).toBe('/tree');

		server.refuse('PUT', /\/state$/, 404);
		reader.toggleStar(article);
		expect(reader.isStarred(article)).toBe(true);
		await settle();
		expect(reader.isStarred(article)).toBe(false);
	});

	it('sends changes in the order they were made', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		const article = reader.visible[0];
		const release = server.hold(/\/state$/);
		reader.setRead(article, true);
		reader.toggleStar(article);
		reader.setRead(article, false);
		release();
		await settle();
		const sent = server.requests.filter((r) => r.method === 'PUT').map((r) => r.body);
		expect(sent).toEqual([{ read: true }, { starred: true }, { read: false }]);
		expect(server.article(tech, 3)).toMatchObject({ read: false, starred: true });
		expect(reader.isRead(article)).toBe(false);
		expect(reader.totalUnread).toBe(6);
	});

	it('does not undo a refused change that a later one already replaced', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		const article = reader.visible[0];
		// The first of the two is refused, the second gets through.
		server.refuse('PUT', /\/state$/, 500);
		const release = server.hold(/\/state$/);
		reader.setRead(article, true);
		reader.setRead(article, false);
		release();
		await settle();
		expect(reader.isRead(article)).toBe(false);
		expect(server.article(tech, 3)!.read).toBe(false);
		expect(reader.totalUnread).toBe(6);
	});

	it('keeps a change that is on its way when a list from before it arrives', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		const article = reader.visible[0];
		const release = server.hold(/\/state$/);
		reader.setRead(article, true);
		reader.toggleStar(article);
		// The list is loaded anew while the server does not know of the changes yet.
		reader.refresh();
		await settle();
		expect(reader.isRead(reader.visible[0])).toBe(true);
		expect(reader.isStarred(reader.visible[0])).toBe(true);
		release();
		await settle();
		expect(server.article(tech, 3)).toMatchObject({ read: true, starred: true });
	});

	it('marks what scrolls past as read, but not what was set back by hand', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		const [first, second] = reader.visible;
		reader.scrolledPast(first);
		expect(reader.isRead(first)).toBe(true);
		reader.setRead(second, true);
		reader.setRead(second, false);
		reader.scrolledPast(second);
		expect(reader.isRead(second)).toBe(false);
		// After a refresh it is an article like any other again.
		reader.refresh();
		await settle();
		reader.scrolledPast(reader.visible[1]);
		expect(reader.isRead(reader.visible[1])).toBe(true);
	});

	it('opens and closes an article, and steps through the list', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		const [first, second] = reader.visible;
		reader.step(-1);
		expect(reader.cursor).toBeNull();
		reader.step(1);
		expect([reader.cursor, reader.open, reader.moved]).toEqual([first.id, first.id, 1]);
		reader.step(1, false);
		expect([reader.cursor, reader.open]).toEqual([second.id, null]);
		expect(reader.isRead(second)).toBe(false);
		reader.toggleOpen(second);
		expect(reader.open).toBe(second.id);
		expect(reader.isRead(second)).toBe(true);
		reader.toggleOpen(second);
		expect([reader.cursor, reader.open]).toEqual([second.id, null]);
		reader.step(1);
		reader.step(1);
		await settle();
		expect(reader.current?.seq).toBe(1);
		reader.step(-1);
		expect(reader.current?.seq).toBe(2);
	});

	it('goes on to the next page when the cursor steps past the end', async () => {
		server.addArticles(lone, PAGE + 2);
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${lone}` });
		await settle();
		reader.select(reader.visible.at(-1)!, false);
		reader.step(1, false);
		expect(reader.visible.length).toBe(PAGE);
		await settle();
		expect(reader.visible.length).toBe(PAGE + 3);
		expect(reader.current).toBe(reader.visible[PAGE]);
	});

	it('marks all as read up to what the list knew, and asks for the counts', async () => {
		const reader = await started();
		reader.show({ kind: 'folder', id: `folder-${news}` });
		await settle();
		const newest = Number(reader.visible[0].id);
		// An article arrives after the list was loaded.
		server.addArticles(tech, 1);
		const release = server.hold(/\/read$/);

		reader.markAllRead();
		expect(reader.visible.every((article) => reader.isRead(article))).toBe(true);
		expect(reader.unreadIn(reader.tree[0])).toBe(0);
		expect(reader.totalUnread).toBe(1);
		release();
		await settle();
		expect(server.requests.find((r) => r.path.endsWith('/read'))).toMatchObject({
			method: 'POST',
			path: `/folders/${news}/read`,
			body: { upto_id: newest }
		});
		// The late one stays unread, and the count says so.
		expect(reader.unreadIn(reader.tree[0])).toBe(1);
		expect(server.unread(tech)).toBe(1);
		expect(server.unread(lone)).toBe(1);

		reader.show({ kind: 'feed', id: `feed-${lone}` });
		await settle();
		reader.markAllRead();
		await settle();
		expect(server.writes.at(-1)).toBe(`POST /feeds/${lone}/read`);
		reader.show({ kind: 'all' });
		await settle();
		reader.markAllRead();
		await settle();
		expect(server.writes.at(-1)).toBe('POST /read');
		expect(reader.totalUnread).toBe(0);
	});

	it('takes "mark all as read" back when the server refuses it', async () => {
		const reader = await started();
		reader.show({ kind: 'all' });
		await settle();
		server.refuse('POST', /^\/read$/, 500);
		reader.markAllRead();
		expect(reader.totalUnread).toBe(0);
		await settle();
		expect(reader.totalUnread).toBe(6);
		expect(reader.visible.some((article) => reader.isRead(article))).toBe(false);
		expect(reader.notice).toBe('notSaved');
	});

	it('does not mark by age, nor from the starred or an empty list', async () => {
		server.article(tech, 1)!.starred = true;
		const reader = await started();
		reader.show({ kind: 'all' });
		await settle();
		reader.markAllRead(86_400_000);
		reader.show({ kind: 'starred' });
		await settle();
		reader.markAllRead();
		const empty = server.addFeed('Empty');
		reader.refresh();
		await settle();
		reader.show({ kind: 'feed', id: `feed-${empty}` });
		await settle();
		reader.markAllRead();
		await settle();
		expect(server.writes).toEqual([]);
		expect(reader.totalUnread).toBe(6);
		expect(reader.can).toEqual({ oldestFirst: false, markOlder: false, subscribe: false });
	});
});

describe('the filter rules', () => {
	const adRule = (over = {}) => ({
		id: 'rule-new-1',
		enabled: true,
		scope: { kind: 'all' as const },
		field: 'title' as const,
		match: 'contains' as const,
		pattern: 'Article 1 of',
		action: 'hide' as const,
		...over
	});

	it('come from the server, with their scopes named as the tree names them', async () => {
		server.rules = [
			{ ...adRule(), id: 7, scope: { kind: 'feed', id: tech } },
			{ ...adRule(), id: 8, scope: { kind: 'folder', id: news }, enabled: false },
			{ ...adRule(), id: 9 }
		];
		const reader = await started();
		expect(reader.filters).toBe(reader);
		expect(reader.rules.map((rule) => [rule.id, rule.scope, rule.enabled])).toEqual([
			['rule-7', { kind: 'feed', id: `feed-${tech}` }, true],
			['rule-8', { kind: 'folder', id: `folder-${news}` }, false],
			['rule-9', { kind: 'all' }, true]
		]);
	});

	it('are saved at once, and the server says what they hide', async () => {
		const reader = await started();
		reader.show({ kind: 'all' });
		await settle();
		expect(reader.visible.length).toBe(6);

		const id = reader.newRuleId();
		reader.saveRule(adRule({ id, scope: { kind: 'folder', id: `folder-${news}` } }));
		expect(reader.rules.map((rule) => rule.id)).toEqual([id]);
		await settle();
		expect(server.requests.find((r) => r.method === 'POST' && r.path === '/rules')?.body).toEqual({
			enabled: true,
			scope: { kind: 'folder', id: news },
			field: 'title',
			match: 'contains',
			pattern: 'Article 1 of',
			action: 'hide'
		});
		// The rule has the server's id now; what it hides is gone from the list and the counts.
		const [saved] = reader.rules;
		expect(saved.id).toBe(`rule-${server.rules[0].id}`);
		expect(reader.visible.length).toBe(4);
		expect(reader.totalUnread).toBe(4);

		reader.show({ kind: 'hidden' });
		await settle();
		expect(reader.visible.map((a) => a.feedId).sort()).toEqual(
			[blog, tech].map((n) => `feed-${n}`).sort()
		);
		expect(reader.hidingRule(reader.visible[0])).toEqual(saved);

		// Switched off, it hides nothing and says so for no article.
		reader.saveRule({ ...saved, enabled: false });
		expect(reader.rules[0].enabled).toBe(false);
		await settle();
		expect(server.writes.at(-1)).toBe(`PUT /rules/${server.rules[0].id}`);
		expect(reader.visible).toEqual([]);
		expect(reader.totalUnread).toBe(6);

		reader.deleteRule(saved.id);
		expect(reader.rules).toEqual([]);
		await settle();
		expect(server.writes.at(-1)).toMatch(/^DELETE \/rules\/\d+$/);
		expect(server.rules).toEqual([]);
	});

	it('are as the server has them again when it refuses a change', async () => {
		server.rules = [{ ...adRule(), id: 7 }];
		server.applyRules();
		const reader = await started();
		server.refuse('PUT', /^\/rules\/7$/, 400);
		reader.saveRule({ ...reader.rules[0], pattern: 'something else' });
		expect(reader.rules[0].pattern).toBe('something else');
		await settle();
		expect(reader.rules[0].pattern).toBe('Article 1 of');
		expect(reader.notice).toBe('notSaved');

		server.refuse('DELETE', /^\/rules\/7$/, 500);
		reader.deleteRule('rule-7');
		expect(reader.rules).toEqual([]);
		await settle();
		expect(reader.rules.map((rule) => rule.id)).toEqual(['rule-7']);
	});

	it('say only for a rule in whose scope the feed lies why an article is hidden', async () => {
		server.rules = [
			{ ...adRule(), id: 7, scope: { kind: 'feed', id: lone }, pattern: 'Article' },
			{ ...adRule(), id: 8, action: 'read', pattern: 'Article' }
		];
		server.applyRules();
		const reader = await started();
		reader.show({ kind: 'hidden' });
		await settle();
		expect(reader.visible.map((a) => a.feedId)).toEqual([`feed-${lone}`]);
		expect(reader.hidingRule(reader.visible[0])?.id).toBe('rule-7');
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		// The rule that marks as read hides nothing, and the other is not about this feed.
		expect(reader.hidingRule(reader.visible[0])).toBeNull();
	});

	it('are tried on the server while they are written, the last one typed only', async () => {
		const reader = await started();
		const asked = () => server.requests.filter((r) => r.path === '/rules/preview');
		const typed = ['A', 'Ar', 'Article 1 of'].map((pattern) => reader.preview(adRule({ pattern })));
		expect(asked()).toEqual([]);
		const answers = await Promise.all(typed);
		expect(asked().length).toBe(1);
		expect(asked()[0].body).toMatchObject({ pattern: 'Article 1 of', scope: { kind: 'all' } });
		expect(answers.at(-1)).toEqual({ matches: 3, of: 6 });

		const inFeed = adRule({ scope: { kind: 'feed' as const, id: `feed-${tech}` } });
		expect(await reader.preview(inFeed)).toEqual({ matches: 1, of: 3 });
	});

	it('learn from the server whether a pattern is one', async () => {
		const reader = await started();
		expect(await reader.preview(adRule({ match: 'regex', pattern: '(Art' }))).toBe('invalid');
		server.down = true;
		expect(await reader.preview(adRule())).toBe('unreachable');
		server.down = false;
		server.signedIn = null;
		expect(await reader.preview(adRule())).toBe('unreachable');
		expect(session.expired).toHaveBeenCalled();
	});
});

describe('the text of an article', () => {
	it('is fetched when asked for, once', async () => {
		// Feeds name tags twice now and then.
		server.article(tech, 3)!.tags = ['news', 'rust', 'news'];
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		const article = reader.visible[0];
		reader.load(article);
		reader.load(article);
		await settle();
		const loaded = reader.visible[0];
		expect(loaded.html).toBe(`<p>Text 3 of feed ${tech}.</p>`);
		expect(loaded.tags).toEqual(['news', 'rust']);
		expect(reader.visible[1].html).toBeNull();
		reader.load(loaded);
		await settle();
		const asked = server.requests.filter((r) => r.path === `/feeds/${tech}/articles/3`);
		expect(asked.length).toBe(1);

		// A list that is loaded again has the text still, unless the article changed.
		reader.refresh();
		await settle();
		expect(reader.visible[0].html).toBe(`<p>Text 3 of feed ${tech}.</p>`);
		server.article(tech, 3)!.version = 2;
		reader.refresh();
		await settle();
		expect(reader.visible[0]).toMatchObject({ html: null, changes: 1 });
	});

	it('is fetched a few at a time when a whole page asks at once', async () => {
		server.addArticles(lone, 11);
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${lone}` });
		await settle();
		const release = server.hold(/\/articles\/\d+$/);
		for (const article of reader.visible) reader.load(article);
		await settle();
		const texts = () => server.requests.filter((r) => /\/articles\/\d+$/.test(r.path)).length;
		expect(texts()).toBe(4);
		release();
		await settle();
		expect(texts()).toBe(12);
		expect(reader.visible.every((article) => article.html !== null)).toBe(true);
	});

	it('stays away with a notice when the server does not answer', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		server.refuse('GET', /\/articles\/3$/, 502);
		reader.load(reader.visible[0]);
		await settle();
		expect(reader.visible[0].html).toBeNull();
		expect(reader.notice).toBe('unreachable');
		// The next try gets it.
		reader.load(reader.visible[0]);
		await settle();
		expect(reader.visible[0].html).not.toBeNull();
	});

	it('has its earlier versions, without the current one', async () => {
		const article = server.article(tech, 3)!;
		article.version = 3;
		article.updated_at = '2026-11-02T10:00:00Z';
		article.versions = [1, 2, 3].map((version) => ({
			version,
			fetched_at: `2026-11-0${version}T09:00:00Z`,
			title: `Title ${version}`,
			html: `<p>Version ${version}</p>`,
			authors: [],
			tags: [],
			url: null
		}));
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		expect(reader.visible[0]).toMatchObject({
			changes: 2,
			updatedAt: Date.parse('2026-11-02T10:00:00Z')
		});
		expect(await reader.versions(reader.visible[0])).toEqual([
			{
				rev: 1,
				fetchedAt: Date.parse('2026-11-01T09:00:00Z'),
				title: 'Title 1',
				html: '<p>Version 1</p>'
			},
			{
				rev: 2,
				fetchedAt: Date.parse('2026-11-02T09:00:00Z'),
				title: 'Title 2',
				html: '<p>Version 2</p>'
			}
		]);
		server.refuse('GET', /\/versions$/, 500);
		expect(await reader.versions(reader.visible[0])).toEqual([]);
		expect(reader.notice).toBe('unreachable');
	});
});

describe('the session', () => {
	it('is over when the server says so, whatever was asked', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		server.signedIn = null;
		reader.setRead(reader.visible[0], true);
		await settle();
		expect(session.expired).toHaveBeenCalled();
		expect(reader.notice).toBeNull();

		session.expired.mockClear();
		reader.show({ kind: 'all' });
		await settle();
		expect(session.expired).toHaveBeenCalled();
	});

	it('ends when the reader logs out', async () => {
		const reader = await started();
		reader.signOut();
		expect(session.signOut).toHaveBeenCalledOnce();
	});

	it('has none of the switches of the made-up reader', async () => {
		const reader = await started();
		expect(reader.demo).toBeNull();
		expect(reader.pending).toBe(0);
		expect(reader.hiddenCount).toBe(0);
		reader.subscribe();
		reader.showNew();
		await settle();
		expect(server.writes).toEqual([]);
	});
});
