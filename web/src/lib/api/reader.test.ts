// The reader that talks to the server, against a made-up server: what it loads,
// what it sends, and that a change shows at once and is taken back when the server
// refuses it.

import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { TreeNode } from '#lib/model.ts';
import { MAX_ACCOUNT_FILE, MAX_OPML, type Resumed } from '#lib/reader.ts';
import { defaults, settings } from '#lib/settings.svelte.ts';
import { TAB } from './client.ts';
import { MAX_UNSENT, PAGE, ServerReader, UNREACHABLE_AFTER, toTree } from './reader.svelte.ts';
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
	vi.stubGlobal('EventSource', server.EventSource);
	session.signOut.mockClear();
	session.expired.mockClear();
	localStorage.clear();
});

/** Every reader a test started: they listen to the window, so each test stops its own. */
const running: (() => void)[] = [];
const start = ServerReader.prototype.start;
ServerReader.prototype.start = function (this: ServerReader) {
	const stop = start.call(this);
	running.push(stop);
	return stop;
};

afterEach(() => {
	for (const stop of running.splice(0)) stop();
	vi.unstubAllGlobals();
});

/** Lets every answer that is on its way arrive, and what follows from it. */
async function settle() {
	for (let i = 0; i < 20; i++) await new Promise((resolve) => setTimeout(resolve, 0));
	flushSync();
}

/** Waits until a search that was typed is asked for and answered. */
async function searched() {
	await new Promise((resolve) => setTimeout(resolve, 350));
	await settle();
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
			hidden: 0,
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
				fetch_agent: 'rueder',
				fetched: true,
				blocked: false,
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
		server.article(tech, 3)!.thumbnail_url = '/img/sig/thumbnail/pic';
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
		// The picture of the list, for the magazine.
		expect(reader.visible.map((article) => article.thumbnail)).toEqual([
			'/img/sig/thumbnail/pic',
			null,
			null
		]);

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

		// Setting it to what it is asks nothing.
		const asked = server.requests.length;
		reader.unreadOnly = true;
		reader.newestFirst = true;
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

	it('show the unread items, where what was read goes once the reader leaves it', async () => {
		server.article(tech, 2)!.read = true;
		const reader = await started();
		reader.show({ kind: 'unread' });
		await settle();
		expect(server.requests.at(-1)?.path).toBe(`/articles?limit=${PAGE}&unread=true`);
		const ids = reader.visible.map((article) => article.id);
		expect(ids).toHaveLength(5);
		const [first, second, third, fourth] = reader.visible;
		const shown = () => reader.visible.map((article) => article.id);

		// Read, it stays while the reader is on it; opening another leaves it.
		reader.step(1);
		expect(reader.open).toBe(first.id);
		expect(reader.isRead(first)).toBe(true);
		expect(shown()).toEqual(ids);
		reader.step(1);
		expect(reader.open).toBe(second.id);
		expect(shown()).toEqual(ids.slice(1));
		// Closing it leaves it too; the next step opens the one that took its
		// place, as if it were still there.
		reader.toggleOpen(second);
		expect(reader.open).toBeNull();
		expect(shown()).toEqual(ids.slice(2));
		reader.step(1);
		expect(reader.open).toBe(third.id);
		// Set back to unread by hand, it stays.
		reader.setRead(third, false);
		reader.step(1);
		expect(reader.open).toBe(fourth.id);
		expect(shown()).toEqual(ids.slice(2));

		// Loaded anew because something changed, the list keeps what the reader is on.
		await settle();
		server.emit({ event: 'updated', data: { feed_id: Number(fourth.feedId.slice(5)), count: 1 } });
		await new Promise((resolve) => setTimeout(resolve, 600));
		await settle();
		expect(shown()).toContain(fourth.id);
		expect(reader.current?.id).toBe(fourth.id);

		// Marking all as read leaves the open one only, and closing it nothing.
		reader.markAllRead();
		await settle();
		expect(shown()).toEqual([fourth.id]);
		reader.open = null;
		expect(reader.visible).toEqual([]);
		reader.refresh();
		await settle();
		expect(reader.visible).toEqual([]);
		expect(reader.totalUnread).toBe(0);
	});

	it('search the unread items within what is unread', async () => {
		const reader = await started();
		reader.show({ kind: 'unread' });
		await settle();
		reader.query = 'Tech';
		await searched();
		expect(server.requests.at(-1)?.path).toMatch(/^\/search\?.*unread=true/);
	});

	it('start with the oldest when asked to, page by page, and still know the newest', async () => {
		server.addArticles(lone, PAGE);
		const reader = await started();
		reader.show({ kind: 'all' });
		await settle();
		const newest = Number(reader.visible[0].id);
		expect(reader.newestFirst).toBe(true);

		reader.newestFirst = false;
		await settle();
		expect(reader.newestFirst).toBe(false);
		expect(titles(reader).slice(0, 2)).toEqual([
			`Article 1 of feed ${tech}`,
			`Article 2 of feed ${tech}`
		]);
		expect(reader.visible.length).toBe(PAGE);
		expect(reader.hasMore).toBe(true);
		reader.showMore();
		await settle();
		expect(reader.visible.length).toBe(PAGE + 6);
		const ids = reader.visible.map((article) => Number(article.id));
		expect(ids).toEqual([...ids].sort((a, b) => a - b));
		expect(reader.hasMore).toBe(false);
		expect(server.requests.at(-1)?.path).toBe(`/articles?limit=${PAGE}&after=${ids[PAGE - 1]}`);

		// "Mark all as read" stops at the newest article there was, not at the end of the page.
		reader.newestFirst = false;
		reader.refresh();
		await settle();
		expect(reader.visible.length).toBe(PAGE);
		reader.markAllRead();
		await settle();
		expect(server.requests.find((r) => r.path === '/read')?.body).toEqual({ upto_id: newest });
		expect(reader.totalUnread).toBe(0);

		// Only what is unread, oldest first: nothing is left.
		reader.unreadOnly = true;
		await settle();
		expect(server.requests.at(-2)?.path).toBe(`/articles?limit=${PAGE}&after=0&unread=true`);
		expect(reader.visible).toEqual([]);
	});

	it('search on the server, in what is shown, once the typing pauses', async () => {
		server.article(tech, 2)!.title = 'A Rare Word';
		server.article(blog, 1)!.author = 'Rarely Seen';
		server.article(lone, 1)!.html = '<p>Rarer still.</p>';
		const reader = await started();
		reader.show({ kind: 'all' });
		await settle();
		const searches = () => server.requests.filter((r) => r.path.startsWith('/search'));

		reader.query = ' ra';
		reader.query = ' rare ';
		expect(reader.loading).toBe(true);
		expect(reader.visible).toEqual([]);
		await searched();
		expect(searches().map((r) => r.path)).toEqual([`/search?q=rare&limit=${PAGE}`]);
		expect(reader.loading).toBe(false);
		expect(reader.visible.map((a) => [a.feedId, a.seq])).toEqual([
			[`feed-${lone}`, 1],
			[`feed-${blog}`, 1],
			[`feed-${tech}`, 2]
		]);
		expect(reader.hasMore).toBe(false);
		// What is found is read like any list.
		reader.step(1);
		expect(reader.current?.seq).toBe(1);
		await settle();
		expect(server.article(lone, 1)!.read).toBe(true);

		// Only the unread, again.
		reader.unreadOnly = true;
		await settle();
		expect(searches().at(-1)?.path).toBe(`/search?q=rare&limit=${PAGE}&unread=true`);
		expect(reader.visible.length).toBe(2);
		reader.unreadOnly = false;

		// In a feed, only there; another list starts without the search.
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		expect(reader.query).toBe('');
		await settle();
		reader.query = 'RARE word';
		await searched();
		expect(searches().at(-1)?.path).toBe(`/search?q=RARE+word&limit=${PAGE}&feed=${tech}`);
		expect(reader.visible.map((a) => a.seq)).toEqual([2]);
		// No words, no search: the feed's list is back.
		reader.query = '  ';
		expect(reader.visible.length).toBe(3);
	});

	it('search page by page, and ask again where the server ran out of time', async () => {
		server.addArticles(lone, 60);
		server.searchWindow = 25;
		const reader = await started();
		reader.show({ kind: 'all' });
		await settle();
		reader.query = 'article';
		// A step while the hits are on their way waits for them.
		reader.step(1);
		await searched();
		expect(reader.current?.id).toBe(reader.visible[0].id);
		// 66 articles, 25 at a time: the first page takes two answers.
		const asked = () => server.requests.filter((r) => r.path.startsWith('/search'));
		expect(asked()).toHaveLength(2);
		expect(asked()[1].path).toMatch(
			new RegExp(`^/search\\?q=article&limit=${PAGE - 25}&before=\\d+$`)
		);
		expect(reader.visible.length).toBe(PAGE);
		expect(reader.hasMore).toBe(true);
		for (let page = 0; page < 5 && reader.hasMore; page++) {
			reader.showMore();
			await settle();
		}
		const ids = reader.visible.map((a) => Number(a.id));
		expect(ids).toHaveLength(66);
		expect(ids).toEqual([...ids].sort((a, b) => b - a));
		expect(reader.hasMore).toBe(false);
	});

	it('search in what the server does not search: what is loaded', async () => {
		const reader = await started();
		server.article(tech, 2)!.starred = true;
		server.article(tech, 2)!.title = 'A Rare Word';
		server.article(tech, 3)!.starred = true;
		reader.show({ kind: 'starred' });
		await settle();
		reader.query = ' rare ';
		expect(reader.visible.map((a) => a.seq)).toEqual([2]);
		await searched();
		expect(server.requests.some((r) => r.path.startsWith('/search'))).toBe(false);
	});

	it('say so when the server cannot search', async () => {
		const reader = await started();
		reader.show({ kind: 'all' });
		await settle();
		server.refuse('GET', /^\/search/, 503, 1, 'the search took too long');
		reader.query = 'article';
		await searched();
		expect(reader.loading).toBe(false);
		expect(reader.visible).toEqual([]);
		expect(reader.notice).toBe('unreachable');
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

describe('resuming after a reload', () => {
	function resumed(view: Partial<Resumed> & Pick<Resumed, 'stream'>) {
		const reader = new ServerReader({ name: 'fiona', admin: true }, session);
		reader.resume({
			unreadOnly: false,
			newestFirst: true,
			cursor: null,
			open: null,
			wanted: [],
			...view
		});
		return reader;
	}

	it('shows the list again with the article open, loading pages until it is there', async () => {
		server.addArticles(lone, PAGE + 20);
		const deep = server.article(lone, 3)!;
		const reader = resumed({
			stream: { kind: 'feed', id: `feed-${lone}` },
			open: String(deep.id),
			cursor: String(deep.id)
		});
		expect(reader.resuming).toBe(true);
		reader.start();
		await settle();
		expect(reader.resuming).toBe(false);
		expect(reader.visible.length).toBe(PAGE + 21);
		expect(reader.open).toBe(String(deep.id));
		expect(reader.cursor).toBe(String(deep.id));
		expect(reader.current?.html).toContain('Text 3 of feed');
		// The start page was never asked for.
		expect(server.requests.some((r) => r.path.includes('unread=true&limit=60'))).toBe(false);
	});

	it('stops looking once the list is past what it wants, or after a few pages', async () => {
		server.addArticles(lone, PAGE * 20);
		const reader = resumed({ stream: { kind: 'feed', id: `feed-${lone}` }, wanted: ['1'] });
		reader.start();
		await settle();
		expect(reader.resuming).toBe(false);
		expect(reader.visible.length).toBe(PAGE * 11);
		expect(reader.open).toBeNull();
	});

	it('puts the open article back into a list of the unread, where it was read', async () => {
		server.addArticles(tech, 3);
		const open = server.article(tech, 4)!;
		open.read = true;
		server.article(tech, 5)!.read = true;
		const reader = resumed({
			stream: { kind: 'feed', id: `feed-${tech}` },
			unreadOnly: true,
			newestFirst: false,
			open: String(open.id)
		});
		reader.start();
		await settle();
		expect(reader.unreadOnly).toBe(true);
		expect(reader.newestFirst).toBe(false);
		// The oldest first: 1, 2, 3, the open 4, then 6; 5 was read and stays out.
		expect(reader.visible.map((article) => article.seq)).toEqual([1, 2, 3, 4, 6]);
		expect(reader.open).toBe(String(open.id));
		expect(reader.isRead(reader.visible[3])).toBe(true);
	});

	it('shows all items when the feed is gone, without saying the server failed', async () => {
		const reader = resumed({ stream: { kind: 'feed', id: 'feed-999' }, open: '5' });
		reader.start();
		await settle();
		expect(reader.stream).toEqual({ kind: 'all' });
		expect(reader.resuming).toBe(false);
		expect(reader.open).toBeNull();
		expect(reader.notice).toBeNull();
		expect(reader.visible.length).toBe(6);
	});

	it('gives up when the reader goes elsewhere first', async () => {
		const release = server.hold(new RegExp(`/feeds/${tech}/articles`));
		const article = server.article(tech, 1)!;
		const reader = resumed({
			stream: { kind: 'feed', id: `feed-${tech}` },
			open: String(article.id)
		});
		reader.start();
		reader.show({ kind: 'feed', id: `feed-${blog}` });
		expect(reader.resuming).toBe(false);
		release();
		await settle();
		expect(reader.open).toBeNull();
		expect(titles(reader)).toEqual([`Article 2 of feed ${blog}`, `Article 1 of feed ${blog}`]);
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
		// Without a cursor, a step back starts at the top too.
		reader.step(-1);
		expect([reader.cursor, reader.open, reader.moved]).toEqual([first.id, first.id, 1]);
		reader.step(-1);
		expect([reader.cursor, reader.moved]).toEqual([first.id, 1]);
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

	it('takes a step that comes while the list is on its way once it is there', async () => {
		const reader = await started();
		for (const by of [1, -1] as const) {
			const release = server.hold(/^\/feeds\/\d+\/articles\?/);
			reader.show({ kind: 'feed', id: `feed-${tech}` });
			reader.step(by);
			expect(reader.cursor).toBeNull();
			release();
			await settle();
			expect([reader.current?.seq, reader.open]).toEqual([3, reader.cursor]);
		}
		// A step for a list that is no longer shown is forgotten.
		const release = server.hold(/^\/feeds\/\d+\/articles\?/);
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		reader.step(1);
		reader.show({ kind: 'feed', id: `feed-${blog}` });
		release();
		await settle();
		expect(reader.cursor).toBeNull();
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

	it('marks as read by age what is old enough, here and on the server', async () => {
		// The articles came a minute apart; now is half a minute after the fourth.
		const fourth = Date.parse(server.article(blog, 1)!.published_at);
		const now = vi.spyOn(Date, 'now').mockReturnValue(fourth + 30_000);
		server.now = () => fourth + 30_000;
		const reader = await started();
		reader.show({ kind: 'folder', id: `folder-${news}` });
		await settle();
		const newest = Number(reader.visible[0].id);
		const release = server.hold(/\/read$/);

		// Older than two minutes: the first two of the five in the folder.
		reader.markAllRead(120_000);
		expect(reader.visible.map((article) => reader.isRead(article))).toEqual([
			false,
			false,
			false,
			true,
			true
		]);
		expect(reader.unreadIn(reader.tree[0])).toBe(3);
		release();
		await settle();
		expect(server.requests.find((r) => r.path.endsWith('/read'))).toMatchObject({
			path: `/folders/${news}/read`,
			body: { upto_id: newest, older_than: 120 }
		});
		expect(server.unread(tech)).toBe(1);
		expect(server.unread(blog)).toBe(2);
		expect(server.unread(lone)).toBe(1);
		expect(reader.totalUnread).toBe(4);

		// Refused, it is taken back like any other change.
		server.refuse('POST', /\/read$/, 500);
		reader.markAllRead(30_000);
		expect(reader.unreadIn(reader.tree[0])).toBe(1);
		await settle();
		expect(reader.unreadIn(reader.tree[0])).toBe(3);
		expect(reader.notice).toBe('notSaved');
		now.mockRestore();
	});

	it('does not mark from the starred or an empty list', async () => {
		server.article(tech, 1)!.starred = true;
		const reader = await started();
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
		// An admin sees the status of the server too.
		expect(reader.can).toEqual({
			oldestFirst: true,
			markOlder: true,
			subscribe: true,
			opml: true,
			move: true,
			status: true,
			invites: true,
			reports: true,
			secondFactor: true,
			password: true,
			remove: true,
			reportContent: false,
			apps: false,
			// The server here has no passkeys.
			passkeys: false,
			// Nor does it keep addresses.
			email: false
		});
	});
});

describe('invitations', () => {
	it('are listed with how many more the reader can make', async () => {
		server.admin = false;
		server.inviteQuota = 3;
		const reader = await started();
		expect(await reader.invites()).toEqual({ invites: [], left: 3, days: 7 });

		// The link leads to this rueder and is shown this once.
		const made = await reader.invite(2);
		expect(made).toEqual({ link: `${location.origin}/?invite=${server.invites[0].token}` });
		expect(server.requests.at(-1)).toMatchObject({
			method: 'POST',
			path: '/invites',
			body: { uses: 2 }
		});
		const list = await reader.invites();
		expect(list).toMatchObject({ left: 1, invites: [{ max_uses: 2, uses: 0, usable: true }] });
		expect(JSON.stringify(list)).not.toContain(server.invites[0].token);

		// More than are left cannot be made, and taking one back gives its places back.
		expect(await reader.invite(2)).toBe('none-left');
		expect(await reader.revokeInvite(list!.invites[0].id)).toBe(true);
		expect(await reader.invites()).toMatchObject({ left: 3, invites: [{ usable: false }] });
		expect(await reader.revokeInvite(4711)).toBe(false);
	});

	it('have no quota for an admin', async () => {
		const reader = await started();
		expect(await reader.invite(50)).toMatchObject({ link: expect.stringContaining('?invite=') });
		const list = await reader.invites();
		expect(list?.left).toBeUndefined();
	});

	it('say nothing when the server does not answer', async () => {
		const reader = await started();
		server.down = true;
		expect(await reader.invites()).toBeNull();
		expect(await reader.invite(1)).toBeNull();
		expect(await reader.revokeInvite(1)).toBe(false);
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
		reader.load(reader.visible[0]);
		await settle();
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
		expect(reader.hiddenCount).toBe(1);
		// The server names the rule with the article's text.
		expect(reader.hidingRule(reader.visible[0])).toBeNull();
		reader.load(reader.visible[0]);
		await settle();
		expect(reader.hidingRule(reader.visible[0])?.id).toBe('rule-7');
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		reader.load(reader.visible[0]);
		await settle();
		// The rule that marks as read hides nothing, and the other is not about this feed.
		expect(reader.hidingRule(reader.visible[0])).toBeNull();
	});

	it('count what they find once somebody asks, and again after a change', async () => {
		server.rules = [
			{ ...adRule(), id: 7, scope: { kind: 'feed', id: lone }, pattern: 'Article' },
			{ ...adRule(), id: 8, enabled: false, pattern: 'Article 1' }
		];
		const reader = await started();
		const asked = () => server.requests.filter((r) => r.path === '/rules/hits').length;
		expect(reader.rules.map((rule) => reader.hitsOf(rule))).toEqual([null, null]);
		expect(asked()).toBe(0);

		reader.countHits();
		await settle();
		expect(reader.rules.map((rule) => reader.hitsOf(rule))).toEqual([1, 3]);

		reader.saveRule({ ...reader.rules[1], pattern: 'Article 2' });
		await settle();
		expect(asked()).toBe(2);
		expect(reader.rules.map((rule) => reader.hitsOf(rule))).toEqual([1, 2]);
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

	it('says what changed as the text says it, when it is newer than the list', async () => {
		const article = server.article(tech, 3)!;
		Object.assign(article, {
			version: 2,
			updated_what: 'title',
			updated_at: '2026-11-02T10:00:00Z'
		});
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		expect(reader.visible[0]).toMatchObject({ changes: 1, changed: 'title' });

		// The feed corrects the text before the article is opened.
		Object.assign(article, {
			version: 3,
			updated_what: 'text',
			updated_at: '2026-11-03T10:00:00Z'
		});
		reader.load(reader.visible[0]);
		await settle();
		expect(reader.visible[0]).toMatchObject({
			changes: 2,
			changed: 'text',
			updatedAt: Date.parse('2026-11-03T10:00:00Z')
		});
	});

	it('comes with the files and the picture of the article', async () => {
		const episode = { url: 'https://pod.example/1.mp3', type: 'audio/mpeg', length: 1200 };
		Object.assign(server.article(lone, 1)!, {
			// A feed may name a file twice.
			enclosures: [
				{ ...episode, inline: false },
				{ ...episode, inline: true }
			],
			image_url: '/img/abc/image/def',
			image_title: 'A picture'
		});
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${lone}` });
		await settle();
		expect(reader.visible[0]).toMatchObject({ enclosures: [], image: null });
		reader.load(reader.visible[0]);
		await settle();
		expect(reader.visible[0]).toMatchObject({
			enclosures: [episode],
			image: { url: '/img/abc/image/def', title: 'A picture' }
		});
		// A list that is drawn again has them still.
		reader.refresh();
		await settle();
		expect(reader.visible[0].enclosures).toEqual([episode]);
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

	it('of the next article is fetched with the one that is opened', async () => {
		server.addArticles(lone, 5);
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${lone}` });
		await settle();
		const asked = () =>
			server.requests
				.map((r) => /\/articles\/(\d+)$/.exec(r.path)?.[1])
				.filter((seq) => seq !== undefined);
		const [newest, second, third] = reader.visible;
		reader.select(second);
		await settle();
		// The opened one first, then the one below it.
		expect(asked()).toEqual([`${second.seq}`, `${third.seq}`]);
		expect(reader.visible[2].html).not.toBeNull();
		// A step down shows a text that is there, and fetches the next.
		reader.step(1);
		await settle();
		expect(reader.current?.html).not.toBeNull();
		expect(asked()).toHaveLength(3);
		// A step up fetches the one above.
		reader.select(second);
		reader.step(-1);
		await settle();
		expect(reader.current?.id).toBe(newest.id);
		expect(asked().slice(3)).toEqual([`${newest.seq}`]);
		// Moving the cursor without opening fetches nothing.
		reader.step(1, false);
		await settle();
		expect(asked()).toHaveLength(4);
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
		reader.showNew();
		await settle();
		expect(server.writes).toEqual([]);
	});
});

describe('the email address', () => {
	const asking = () =>
		new ServerReader({ name: 'fiona', admin: false, emailMode: 'optional' }, session);

	it('is there only where the server keeps addresses', () => {
		expect(new ServerReader({ name: 'fiona', admin: false }, session).can.email).toBe(false);
		expect(
			new ServerReader({ name: 'fiona', admin: false, emailMode: 'off' }, session).can.email
		).toBe(false);
		expect(asking().can.email).toBe(true);
	});

	it('mails a link to a new address, with the password and the language of the page', async () => {
		server.emailMode = 'optional';
		const reader = asking();
		expect(await reader.changeEmail(' fiona@example.org ', 'correct horse')).toBeNull();
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
		const reader = asking();
		expect(await reader.changeEmail('new@example.org', 'wrong')).toEqual({ problem: 'wrong' });
		expect(await reader.changeEmail('nowhere', 'correct horse')).toEqual({ problem: 'invalid' });
		expect(await reader.changeEmail('FIONA@example.org', 'correct horse')).toEqual({
			problem: 'same'
		});
		server.refuse('PUT', /^\/email$/, 429, 1, 'too many mails', { 'Retry-After': '600' });
		expect(await reader.changeEmail('new@example.org', 'correct horse')).toEqual({
			problem: 'busy',
			minutes: 10
		});
		server.refuse('PUT', /^\/email$/, 503);
		expect(await reader.changeEmail('new@example.org', 'correct horse')).toEqual({
			problem: 'unsent'
		});
		server.down = true;
		expect(await reader.changeEmail('new@example.org', 'correct horse')).toEqual({
			problem: 'unreachable'
		});
		server.down = false;

		// With a second factor the code is asked for, then checked.
		server.secondFactor = { code: '123456', recovery: [] };
		expect(await reader.changeEmail('new@example.org', 'correct horse')).toEqual({
			problem: 'code'
		});
		expect(await reader.changeEmail('new@example.org', 'correct horse', '000000')).toEqual({
			problem: 'wrong'
		});
		expect(await reader.changeEmail('new@example.org', 'correct horse', '123456')).toBeNull();
		expect(server.mails.at(-1)?.to).toBe('new@example.org');
		expect(reader.account.email).toBeUndefined();
	});

	it('takes the address away where it is optional, and not where it is required', async () => {
		server.emailMode = 'optional';
		server.emails.set('fiona', 'fiona@example.org');
		const reader = asking();
		reader.account = { ...reader.account, email: 'fiona@example.org' };
		expect(await reader.removeEmail('wrong')).toEqual({ problem: 'wrong' });
		expect(await reader.removeEmail('correct horse')).toBeNull();
		expect(reader.account).toMatchObject({ email: null, emailPending: null });
		expect(server.emails.has('fiona')).toBe(false);

		server.emailMode = 'required';
		expect(await reader.removeEmail('correct horse')).toEqual({ problem: 'required' });
	});
});

describe('subscribing', () => {
	const site = { url: 'https://site.example/feed.xml', title: 'A site' };
	const other = { url: 'https://site.example/comments.xml', title: 'Its comments' };

	it('finds the feeds of an address, and what is not there', async () => {
		server.addSite('https://site.example', site, other);
		const reader = await started();
		const fresh = (feed: typeof site) => ({ ...feed, feedId: null });
		expect(await reader.findFeeds('site.example')).toEqual({
			feeds: [fresh(site), fresh(other)],
			movedTo: null
		});
		expect(await reader.findFeeds('https://site.example/feed.xml')).toEqual({
			feeds: [fresh(site)],
			movedTo: null
		});
		expect(await reader.findFeeds('nothing.example')).toEqual({
			problem: 'refused',
			reason: 'the address cannot be fetched: no such site'
		});
		expect(server.requests.filter((r) => r.path === '/feeds/find').map((r) => r.body)).toEqual([
			{ url: 'site.example' },
			{ url: 'https://site.example/feed.xml' },
			{ url: 'nothing.example' }
		]);
		// Looking changes nothing.
		expect(server.writes.filter((write) => write !== 'POST /feeds/find')).toEqual([]);
	});

	it('marks what is subscribed already, and says where a site sent the request', async () => {
		server.addSite('https://site.example', site, other);
		server.moves.set('https://members.example', 'https://site.example');
		const reader = await started();
		const subscribed = await reader.subscribe(site.url, null);
		const feedId = 'feedId' in subscribed ? subscribed.feedId : '';
		expect(await reader.findFeeds('members.example')).toEqual({
			feeds: [
				{ ...site, feedId },
				{ ...other, feedId: null }
			],
			movedTo: 'https://site.example'
		});
	});

	it('says why the server would not look', async () => {
		const reader = await started();
		server.refuse('POST', /^\/feeds\/find$/, 429, 1, 'too many', { 'Retry-After': '2' });
		expect(await reader.findFeeds('site.example')).toEqual({ problem: 'busy', retryAfter: 2 });
		server.refuse('POST', /^\/feeds\/find$/, 503);
		expect(await reader.findFeeds('site.example')).toEqual({ problem: 'noFetching' });
		server.refuse('POST', /^\/feeds\/find$/, 500);
		expect(await reader.findFeeds('site.example')).toEqual({ problem: 'unreachable' });
		server.down = true;
		expect(await reader.findFeeds('site.example')).toEqual({ problem: 'unreachable' });
		server.down = false;
		server.signedIn = null;
		expect(await reader.findFeeds('site.example')).toEqual({ problem: 'unreachable' });
		expect(session.expired).toHaveBeenCalled();
		expect(reader.notice).toBeNull();
	});

	it('subscribes into a folder and shows the new feed', async () => {
		server.addSite('https://site.example', site);
		const reader = await started();
		reader.setClosed(`folder-${news}`, true);
		const subscribed = await reader.subscribe(site.url, `folder-${news}`);
		await settle();
		if (!('feedId' in subscribed)) throw new Error('not subscribed');
		const { feedId } = subscribed;
		expect(subscribed.known).toBe(false);
		expect(server.requests.find((r) => r.method === 'POST' && r.path === '/feeds')?.body).toEqual({
			url: site.url,
			folder_id: news
		});
		expect(shape(reader.tree)).toEqual([
			{ [`News (folder-${news})`]: [`feed-${tech}`, `feed-${blog}`, feedId] },
			`feed-${lone}`
		]);
		expect(reader.feeds[feedId].title).toBe('A site');
		expect(reader.isClosed(`folder-${news}`)).toBe(false);
		expect(reader.stream).toEqual({ kind: 'feed', id: feedId });

		// Once more, it is known; at the top level, the folder is null.
		expect(await reader.subscribe(site.url, null)).toEqual({ feedId, known: true });
		const second = server.requests.filter((r) => r.method === 'POST' && r.path === '/feeds')[1];
		expect(second.body).toEqual({ url: site.url, folder_id: null });
	});

	it('says why the server would not subscribe, and changes nothing', async () => {
		server.addSite('https://site.example', site);
		const reader = await started();
		const before = shape(reader.tree);
		expect(await reader.subscribe('https://site.example', null)).toEqual({
			problem: 'refused',
			reason: 'this is not a feed that can be read: it is a web page'
		});
		expect(await reader.subscribe(site.url, 'folder-999')).toMatchObject({ problem: 'refused' });
		server.refuse('POST', /^\/feeds$/, 429);
		expect(await reader.subscribe(site.url, null)).toEqual({ problem: 'busy', retryAfter: null });
		await settle();
		expect(shape(reader.tree)).toEqual(before);
		expect(reader.notice).toBeNull();
		expect(reader.stream).toEqual({ kind: 'home' });
	});

	it('subscribes after the changes on their way', async () => {
		server.addSite('https://site.example', site);
		const reader = await started();
		const release = server.hold(/^\/tree\/move$/);
		reader.move(`feed-${lone}`, `folder-${news}`, 'inside');
		const subscribing = reader.subscribe(site.url, null);
		await settle();
		expect(server.writes).toEqual(['POST /tree/move']);
		release();
		await subscribing;
		expect(server.writes).toEqual(['POST /tree/move', 'POST /feeds']);
	});
});

describe('the subscriptions', () => {
	it("are renamed at once, and an empty name is the feed's own", async () => {
		const reader = await started();
		reader.renameFeed(`feed-${tech}`, 'Technik');
		expect(reader.feeds[`feed-${tech}`].title).toBe('Technik');
		await settle();
		expect(server.requests.at(-1)).toMatchObject({
			method: 'PATCH',
			path: `/feeds/${tech}`,
			body: { title: 'Technik' }
		});
		reader.renameFeed(`feed-${tech}`, '');
		await settle();
		expect(reader.feeds[`feed-${tech}`].title).toBe('Tech');
		reader.renameFeed('feed-999', 'Nothing');
		await settle();
		expect(server.writes).toEqual([`PATCH /feeds/${tech}`, `PATCH /feeds/${tech}`]);
	});

	it('go two into a new folder where the one dropped on was', async () => {
		const reader = await started();
		expect(reader.canGroup(`feed-${blog}`, `feed-${tech}`)).toBe(true);
		reader.group(`feed-${blog}`, `feed-${tech}`, 'Together');
		await settle();
		const folder = server.folders.find((f) => f.title === 'Together')!;
		expect(server.writes).toEqual([
			'POST /folders',
			'POST /tree/move',
			'POST /tree/move',
			'POST /tree/move'
		]);
		expect(server.requests.find((r) => r.path === '/folders')?.body).toEqual({
			title: 'Together',
			parent_id: news
		});
		expect(shape(reader.tree)).toEqual([
			{
				[`News (folder-${news})`]: [
					{ [`Together (folder-${folder.id})`]: [`feed-${tech}`, `feed-${blog}`] }
				]
			},
			`feed-${lone}`
		]);
		// At the top, the new folder is made there.
		reader.group(`feed-${tech}`, `feed-${lone}`, 'Top');
		await settle();
		expect(server.requests.filter((r) => r.path === '/folders').at(-1)?.body).toEqual({
			title: 'Top',
			parent_id: null
		});
		expect(shape(reader.tree)[1]).toEqual({
			[`Top (folder-${server.folders.at(-1)?.id})`]: [`feed-${lone}`, `feed-${tech}`]
		});
	});

	it('get their old name back when the server refuses the new one', async () => {
		const reader = await started();
		server.refuse('PATCH', /^\/feeds\/\d+$/, 400);
		reader.renameFeed(`feed-${tech}`, 'Technik');
		await settle();
		expect(reader.feeds[`feed-${tech}`].title).toBe('Tech');
		expect(reader.notice).toBe('notSaved');
	});

	it("end at once, and the list loses the feed's articles", async () => {
		const reader = await started();
		reader.show({ kind: 'all' });
		await settle();
		expect(titles(reader)).toContain(`Article 1 of feed ${tech}`);
		reader.unsubscribe(`feed-${tech}`);
		expect(reader.feeds[`feed-${tech}`]).toBeUndefined();
		expect(shape(reader.tree)).toEqual([
			{ [`News (folder-${news})`]: [`feed-${blog}`] },
			`feed-${lone}`
		]);
		expect(reader.totalUnread).toBe(3);
		await settle();
		expect(server.writes).toEqual([`DELETE /feeds/${tech}`]);
		expect(titles(reader).some((title) => title.endsWith(`of feed ${tech}`))).toBe(false);
	});

	it('that is shown ends, and everything is shown', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${lone}` });
		await settle();
		reader.unsubscribe(`feed-${lone}`);
		expect(reader.stream).toEqual({ kind: 'all' });
		reader.unsubscribe('feed-999');
		await settle();
		expect(server.writes).toEqual([`DELETE /feeds/${lone}`]);
	});

	it('come back when the server refuses to end one', async () => {
		const reader = await started();
		server.refuse('DELETE', /^\/feeds\/\d+$/, 500);
		reader.unsubscribe(`feed-${tech}`);
		await settle();
		expect(reader.feeds[`feed-${tech}`]).toMatchObject({ title: 'Tech' });
		expect(shape(reader.tree)[0]).toEqual({
			[`News (folder-${news})`]: [`feed-${tech}`, `feed-${blog}`]
		});
		expect(reader.notice).toBe('notSaved');
	});
});

describe('Moving the account', () => {
	afterEach(() => {
		Object.assign(settings, defaults);
	});

	const account = (over: object = {}) =>
		new Blob(
			[
				JSON.stringify({
					format: 'rueder-account',
					version: 1,
					settings: { theme: 'dark' },
					tree: [
						{ kind: 'feed', url: 'https://moved.example/feed.xml', title: 'Moved' },
						{ kind: 'feed', url: `https://feed-${tech}.example/feed.xml`, title: 'Tech' }
					],
					...over
				})
			],
			{ type: 'application/json' }
		);

	it('sends the file as JSON and shows everything anew', async () => {
		const reader = await started();
		const moved = await reader.importAccount(account());
		expect(moved).toMatchObject({ feeds: 1, known: 1, settings: true });
		const sent = server.requests.find((r) => r.path === '/import');
		expect(sent?.type).toBe('application/json');
		await settle();
		expect(Object.values(reader.feeds).map((feed) => feed.title)).toContain('Moved');
		expect(settings.theme).toBe('dark');
		// The rules and the labels were asked for anew, too.
		const asked = server.requests.slice(server.requests.indexOf(sent!)).map((r) => r.path);
		expect(asked).toEqual(expect.arrayContaining(['/tree', '/settings', '/labels', '/rules']));
	});

	it('says why the server would not take a file', async () => {
		const reader = await started();
		expect(await reader.importAccount(account({ format: 'opml' }))).toEqual({
			problem: 'refused',
			reason: 'this is not a file of a rueder account'
		});
		const large = new Blob(['x'.repeat(MAX_ACCOUNT_FILE + 1)]);
		expect(await reader.importAccount(large)).toEqual({ problem: 'tooLarge' });
		server.refuse('POST', /^\/import$/, 429, 1, 'too many imports', { 'Retry-After': '600' });
		expect(await reader.importAccount(account())).toEqual({ problem: 'busy', retryAfter: 600 });
		server.down = true;
		expect(await reader.importAccount(account())).toEqual({ problem: 'unreachable' });
	});

	it('gives out the account as a file', async () => {
		const reader = await started();
		const file = await reader.exportAccount();
		expect(JSON.parse(file ?? '')).toMatchObject({ format: 'rueder-account', version: 1 });
		server.down = true;
		expect(await reader.exportAccount()).toBeNull();
	});
});

describe('OPML', () => {
	const file = (...outlines: string[]) =>
		new Blob([`<?xml version="1.0"?><opml version="2.0"><body>${outlines.join('')}</body></opml>`]);

	it('takes a file in as it is and loads the tree anew', async () => {
		const reader = await started();
		const imported = await reader.importOpml(
			file(
				'<outline text="New" xmlUrl="https://new.example/feed.xml"/>',
				`<outline text="Tech" xmlUrl="https://feed-${tech}.example/feed.xml"/>`,
				'<outline text="Not one" xmlUrl="javascript:alert(1)"/>'
			)
		);
		expect(imported).toEqual({ feeds: 1, folders: 0, known: 1, skipped: 1 });
		const sent = server.requests.find((r) => r.path === '/opml');
		expect(sent?.type).toBe('text/x-opml');
		expect(String(sent?.body)).toContain('https://new.example/feed.xml');
		await settle();
		expect(Object.values(reader.feeds).map((feed) => feed.title)).toContain('New');
	});

	it('refuses a file that is too large before sending it', async () => {
		const reader = await started();
		const large = new Blob(['x'.repeat(MAX_OPML + 1)]);
		expect(await reader.importOpml(large)).toEqual({ problem: 'tooLarge' });
		expect(server.writes).toEqual([]);
	});

	it('says why the server would not take a file', async () => {
		const reader = await started();
		expect(await reader.importOpml(new Blob(['<html></html>']))).toEqual({
			problem: 'refused',
			reason: 'this is not an OPML file'
		});
		server.refuse('POST', /^\/opml$/, 413);
		expect(await reader.importOpml(file())).toEqual({ problem: 'tooLarge' });
		server.refuse('POST', /^\/opml$/, 415, 1, 'not XML');
		expect(await reader.importOpml(file())).toEqual({ problem: 'refused', reason: 'not XML' });
		server.down = true;
		expect(await reader.importOpml(file())).toEqual({ problem: 'unreachable' });
		expect(reader.notice).toBeNull();
	});

	it('gives out the subscriptions as a file', async () => {
		const reader = await started();
		const opml = await reader.exportOpml();
		expect(opml).toContain('<opml version="2.0">');
		expect(opml).toContain(`xmlUrl="https://feed-${tech}.example/feed.xml"`);
		server.down = true;
		expect(await reader.exportOpml()).toBeNull();
		server.down = false;
		server.signedIn = null;
		expect(await reader.exportOpml()).toBeNull();
		expect(session.expired).toHaveBeenCalled();
	});
});

describe('live updates', () => {
	/** Lets the events that came close together be answered, and what follows from it. */
	async function settled() {
		await new Promise((resolve) => setTimeout(resolve, 350));
		await settle();
	}

	const unread = (reader: ServerReader, feed: number) =>
		reader.unreadIn({ kind: 'feed', id: `feed-${feed}` });

	it('opens the connection before anything is loaded, and closes it again', async () => {
		const reader = new ServerReader({ name: 'fiona', admin: true }, session);
		const stop = reader.start();
		expect(server.streams).toHaveLength(1);
		expect(server.streams[0].after).toBe(0);
		await settle();
		// Every request names the tab, so its own changes come back as its own.
		expect(server.requests.length).toBeGreaterThan(0);
		expect(server.requests.every((request) => request.tab === TAB)).toBe(true);
		stop();
		expect(server.streams).toHaveLength(0);
	});

	it('says how many articles arrived in what is shown, and brings them in', async () => {
		const reader = await started();
		reader.show({ kind: 'folder', id: `folder-${news}` });
		await settle();
		server.addArticles(tech, 2);
		server.emit({ event: 'articles', data: { feed_id: tech, last_seq: 5, new: 2 } });
		// Somewhere else, it is not counted here.
		server.addArticles(lone, 1);
		server.emit({ event: 'articles', data: { feed_id: lone, last_seq: 2, new: 1 } });
		expect(reader.pending).toBe(2);
		expect(reader.feeds[`feed-${tech}`].lastSeq).toBe(5);
		// The list stays as it is until the reader asks for what is new.
		expect(reader.visible).toHaveLength(5);
		await settled();
		expect([unread(reader, tech), unread(reader, lone)]).toEqual([5, 2]);
		reader.showNew();
		await settle();
		expect(reader.visible).toHaveLength(7);
		expect(reader.pending).toBe(0);
	});

	it('loads the start page anew when articles arrive, and the list when they change', async () => {
		const reader = await started();
		server.addArticles(blog, 1, { title: 'Fresh' });
		server.emit({ event: 'articles', data: { feed_id: blog, last_seq: 3, new: 1 } });
		await settled();
		expect(reader.fresh.flatMap((fresh) => fresh.top.map((a) => a.title))).toContain('Fresh');
		expect(reader.pending).toBe(0);

		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		const article = server.article(tech, 3)!;
		article.version = 2;
		article.title = 'Changed';
		server.emit({ event: 'updated', data: { feed_id: blog, count: 1 } });
		await settled();
		expect(titles(reader)).not.toContain('Changed');
		server.emit({ event: 'updated', data: { feed_id: tech, count: 1 } });
		await settled();
		expect(titles(reader)).toContain('Changed');
		expect(reader.visible.find((a) => a.title === 'Changed')?.changes).toBe(1);
	});

	it('takes what was read on another device, and leaves its own alone', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		const first = reader.visible[0];
		server.article(tech, first.seq)!.read = true;
		server.emit({
			event: 'reading',
			data: {
				feed: { feed_id: tech, last_seq: 3, read_upto: 0, unread: 2 },
				seq: first.seq,
				by: 'another-tab'
			}
		});
		expect(unread(reader, tech)).toBe(2);
		await settled();
		expect(reader.isRead(reader.visible[0])).toBe(true);

		// What this tab did itself comes back too, and changes nothing.
		const asked = server.requests.length;
		server.emit({
			event: 'reading',
			data: { feed: { feed_id: tech, last_seq: 3, read_upto: 0, unread: 0 }, seq: 2, by: TAB }
		});
		server.emit({ event: 'tree', data: { by: TAB } });
		server.emit({ event: 'rules', data: { by: TAB } });
		await settled();
		expect(server.requests.length).toBe(asked);
		expect(unread(reader, tech)).toBe(2);

		// Marked all as read elsewhere: the tree says the counts.
		for (const article of server.articles) article.read = true;
		server.emit({ event: 'reading', data: { feed: null, seq: null, by: 'another-tab' } });
		await settled();
		expect(reader.totalUnread).toBe(0);
	});

	it('follows the tree, the rules and the errors of feeds', async () => {
		const reader = await started();
		server.addFeed('Elsewhere');
		server.emit({ event: 'tree', data: { by: 'another-tab' } });
		await settled();
		expect(Object.values(reader.feeds).map((feed) => feed.title)).toContain('Elsewhere');

		server.rules.push({
			id: 99,
			enabled: true,
			scope: { kind: 'all' },
			field: 'title',
			match: 'contains',
			pattern: 'Article 1 of',
			action: 'hide'
		});
		server.applyRules();
		server.emit({ event: 'rules', data: { by: null } });
		await settled();
		expect(reader.rules.map((rule) => rule.id)).toEqual(['rule-99']);
		expect(reader.hiddenCount).toBe(3);

		server.emit({ event: 'feed', data: { feed_id: lone, error: 'timed out' } });
		expect(reader.feeds[`feed-${lone}`].error).toBe('timed out');
		server.emit({ event: 'feed', data: { feed_id: lone, error: null } });
		expect(reader.feeds[`feed-${lone}`].error).toBeNull();
		// Events of feeds it does not read, and nonsense, change nothing.
		server.emit({ event: 'feed', data: { feed_id: 999, error: 'gone' } });
		server.emit({ event: 'articles', raw: 'not json' });
		server.emit({ event: 'labels', data: { by: null } });
		server.emit({ event: 'settings', data: { by: null } });
		expect(reader.feeds['feed-999']).toBeUndefined();
	});

	it('asks whether the session holds when one of the account ends', async () => {
		await started();
		server.emit({ event: 'session', data: { by: 'another-tab' } });
		await settle();
		expect(session.expired).not.toHaveBeenCalled();
		server.signedIn = null;
		server.emit({ event: 'session', data: { by: 'another-tab' } });
		await settle();
		expect(session.expired).toHaveBeenCalledOnce();
	});

	it('shows an address confirmed in another tab or by the link', async () => {
		server.emailMode = 'required';
		const reader = await started();
		server.emails.set('fiona', 'fiona@example.org');
		server.emit({ event: 'session', data: { by: null } });
		await settle();
		expect(reader.account).toMatchObject({ emailMode: 'required', email: 'fiona@example.org' });
	});

	it('loads everything anew when the server cannot say what was missed', async () => {
		const reader = await started();
		server.addArticles(lone, 3);
		server.emit({ event: 'resync', data: {} });
		await settled();
		expect(unread(reader, lone)).toBe(4);
		expect(server.requests.some((r) => r.path === '/rules' && r.method === 'GET')).toBe(true);
	});

	it('opens a connection the server ended anew after a while, and catches up', async () => {
		vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
		try {
			const reader = new ServerReader({ name: 'fiona', admin: true }, session);
			reader.start();
			await vi.advanceTimersByTimeAsync(10);
			expect(reader.connection).toEqual({ state: 'live' });
			server.dropStreams();
			expect(server.streams).toHaveLength(0);
			expect(reader.connection.state).toBe('reconnecting');
			server.addArticles(lone, 2);
			await vi.advanceTimersByTimeAsync(1_900);
			expect(server.streams).toHaveLength(0);
			await vi.advanceTimersByTimeAsync(100);
			expect(server.streams).toHaveLength(1);
			// The new connection starts afresh, so what came meanwhile is asked for.
			await vi.advanceTimersByTimeAsync(400);
			expect(reader.connection).toEqual({ state: 'live' });
			expect(unread(reader, lone)).toBe(3);
			expect(session.expired).not.toHaveBeenCalled();
		} finally {
			vi.useRealTimers();
		}
	});

	it('waits longer each time the server ends it again, up to a minute', async () => {
		vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
		try {
			const reader = new ServerReader({ name: 'fiona', admin: true }, session);
			reader.start();
			await vi.advanceTimersByTimeAsync(10);
			server.silent = true;
			const waited: number[] = [];
			for (let round = 0; round < 6; round++) {
				server.dropStreams();
				let ms = 0;
				while (server.streams.length === 0) {
					await vi.advanceTimersByTimeAsync(500);
					ms += 500;
				}
				waited.push(ms);
			}
			expect(waited).toEqual([2_000, 5_000, 15_000, 30_000, 60_000, 60_000]);
			expect(reader.connection.state).toBe('unreachable');
			// Once it is up, the next break starts short again.
			server.silent = false;
			server.reopenStreams();
			expect(reader.connection).toEqual({ state: 'live' });
			server.dropStreams();
			await vi.advanceTimersByTimeAsync(2_000);
			expect(server.streams).toHaveLength(1);
		} finally {
			vi.useRealTimers();
		}
	});

	it('says the server does not answer once the connection stays away, and not for each load', async () => {
		vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'Date'] });
		try {
			const reader = new ServerReader({ name: 'fiona', admin: true }, session);
			reader.start();
			await vi.advanceTimersByTimeAsync(10);
			const broke = Date.now();
			server.down = true;
			server.breakStreams();
			expect(reader.connection).toEqual({ state: 'reconnecting', since: broke });
			await vi.advanceTimersByTimeAsync(9_000);
			expect(reader.connection.state).toBe('reconnecting');
			await vi.advanceTimersByTimeAsync(1_000);
			expect(reader.connection).toEqual({ state: 'unreachable', since: broke, offline: false });
			reader.show({ kind: 'all' });
			await vi.advanceTimersByTimeAsync(10);
			expect(reader.notice).toBeNull();
			expect(reader.visible).toEqual([]);

			// The browser opens it again by itself; what failed meanwhile is loaded anew.
			server.down = false;
			server.reopenStreams();
			expect(reader.connection).toEqual({ state: 'live' });
			await vi.advanceTimersByTimeAsync(400);
			expect(reader.visible.length).toBe(6);
			expect(reader.notice).toBeNull();
		} finally {
			vi.useRealTimers();
		}
	});

	it('says so when a load fails at once, before the connection is missed', async () => {
		const reader = await started();
		server.down = true;
		server.breakStreams();
		reader.show({ kind: 'all' });
		await settle();
		expect(reader.connection).toMatchObject({ state: 'unreachable', offline: false });
		expect(reader.notice).toBeNull();
	});

	it('is not live at the start while the server does not answer', async () => {
		vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
		try {
			server.silent = true;
			const reader = new ServerReader({ name: 'fiona', admin: true }, session);
			reader.start();
			await vi.advanceTimersByTimeAsync(10);
			expect(reader.connection).toEqual({ state: 'connecting' });
			await vi.advanceTimersByTimeAsync(UNREACHABLE_AFTER);
			expect(reader.connection.state).toBe('unreachable');
		} finally {
			vi.useRealTimers();
		}
	});

	it('tries at once when the network is back, the page is shown again, or when asked', async () => {
		const reader = await started();
		const opened = () => server.streams.length;
		window.dispatchEvent(new Event('offline'));
		expect(reader.connection).toMatchObject({ state: 'unreachable', offline: true });

		server.silent = true;
		window.dispatchEvent(new Event('online'));
		expect(opened()).toBe(1);
		const [first] = server.streams;
		Object.defineProperty(document, 'visibilityState', { value: 'visible', configurable: true });
		document.dispatchEvent(new Event('visibilitychange'));
		expect(server.streams).toHaveLength(1);
		expect(server.streams[0]).not.toBe(first);
		const [second] = server.streams;
		reader.reconnect();
		expect(server.streams).toHaveLength(1);
		expect(server.streams[0]).not.toBe(second);

		// What the old connections say is not heard any more.
		server.silent = false;
		reader.reconnect();
		await settle();
		expect(reader.connection).toEqual({ state: 'live' });
		const requests = server.requests.length;
		reader.reconnect();
		document.dispatchEvent(new Event('visibilitychange'));
		window.dispatchEvent(new Event('online'));
		await settle();
		// Live already: nothing is opened anew, nothing loaded.
		expect(server.requests.length).toBe(requests);
	});

	it('stops listening to the browser once it is stopped', async () => {
		const reader = new ServerReader({ name: 'fiona', admin: true }, session);
		const stop = reader.start();
		await settle();
		stop();
		window.dispatchEvent(new Event('offline'));
		window.dispatchEvent(new Event('online'));
		expect(reader.connection).toEqual({ state: 'live' });
		expect(server.streams).toHaveLength(0);
	});

	it('ends the session when the server refused the connection for want of one', async () => {
		await started();
		server.signedIn = null;
		server.dropStreams();
		await settle();
		expect(session.expired).toHaveBeenCalledOnce();
	});
});

describe('settings', () => {
	afterEach(() => {
		Object.assign(settings, defaults);
	});

	/** Lets settings that changed be sent, and what follows from it. */
	async function saved() {
		await new Promise((resolve) => setTimeout(resolve, 550));
		await settle();
	}

	const puts = () => server.requests.filter((r) => r.method === 'PUT' && r.path === '/settings');

	it('takes the settings of the account into the page', async () => {
		server.settings = { theme: 'dark', accent: 'green', view: 'nonsense', unknown: 1 };
		const reader = await started();
		expect([settings.theme, settings.accent, settings.view]).toEqual(['dark', 'green', 'list']);
		// Taking them in sends nothing back.
		reader.saveSettings({ ...settings });
		await saved();
		expect(puts()).toEqual([]);
	});

	it('gives a new account the settings this browser had', async () => {
		server.settings = {};
		settings.theme = 'dark';
		const reader = await started();
		reader.saveSettings({ ...settings });
		await saved();
		expect(puts()).toHaveLength(1);
		expect(server.settings).toMatchObject({ theme: 'dark', accent: 'red' });
	});

	it('sends what changes once it rests, and nothing before those of the account are there', async () => {
		server.settings = { theme: 'light' };
		const release = server.hold(/^\/settings$/);
		const reader = new ServerReader({ name: 'fiona', admin: true }, session);
		reader.start();
		reader.saveSettings({ ...settings, theme: 'dark' });
		release();
		await settle();
		expect(settings.theme).toBe('light');
		for (const density of ['comfortable', 'compact', 'comfortable'] as const) {
			settings.density = density;
			reader.saveSettings({ ...settings });
		}
		await saved();
		expect(puts().map((r) => (r.body as { density: string }).density)).toEqual(['comfortable']);
		expect(server.settings).toMatchObject({ theme: 'light', density: 'comfortable' });
	});

	it('leave the text size to each device', async () => {
		// The account has a size from another device; this one keeps its own.
		server.settings = { theme: 'dark', textSize: 'small' };
		settings.textSize = 'large';
		const reader = await started();
		expect([settings.theme, settings.textSize]).toEqual(['dark', 'large']);
		// Changing it here sends nothing to the account.
		settings.textSize = 'medium';
		reader.saveSettings({ ...settings });
		await saved();
		expect(puts()).toEqual([]);
		// What else changes goes without it.
		settings.theme = 'light';
		reader.saveSettings({ ...settings });
		await saved();
		expect(puts()).toHaveLength(1);
		expect(puts()[0].body).not.toHaveProperty('textSize');
		// Another tab's change of the account's settings leaves it too.
		server.settings = { theme: 'dark', textSize: 'small' };
		server.emit({ event: 'settings', data: { by: 'another-tab' } });
		await settle();
		expect([settings.theme, settings.textSize]).toEqual(['dark', 'medium']);
	});

	it('loads them anew when another tab changed them', async () => {
		server.settings = { theme: 'light' };
		await started();
		server.settings = { theme: 'dark' };
		server.emit({ event: 'settings', data: { by: TAB } });
		await settle();
		expect(settings.theme).toBe('light');
		server.emit({ event: 'settings', data: { by: 'another-tab' } });
		await settle();
		expect(settings.theme).toBe('dark');
	});
});

describe('reports of errors', () => {
	/** What the reports the server got say, by kind and message. */
	const got = () =>
		(server.reports as { kind: string; message: string }[]).map((r) => `${r.kind}: ${r.message}`);

	/** The page is left, which sends what waits. */
	async function leave() {
		window.dispatchEvent(new Event('pagehide'));
		await settle();
	}

	it('are sent only once the reader agreed, and stop with no', async () => {
		const reader = await started();
		expect(reader.reportsConsent).toEqual({ available: true, on: false });
		window.dispatchEvent(new ErrorEvent('error', { error: new TypeError('before') }));
		await leave();
		expect(got()).toEqual([]);

		reader.setReportsConsent(true);
		await settle();
		expect(server.sendsReports).toBe(true);
		window.dispatchEvent(new ErrorEvent('error', { error: new TypeError('x is undefined') }));
		// What the server fails at is reported; what it refuses is not.
		server.refuse('GET', /^\/feeds\/\d+\/articles\?/, 502);
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		server.refuse('GET', /^\/feeds\/\d+\/articles\?/, 404);
		reader.show({ kind: 'feed', id: `feed-${blog}` });
		await settle();
		await leave();
		expect(got()).toEqual([
			'error: TypeError: x is undefined',
			'request: GET /feeds/<n>/articles: 502'
		]);

		reader.setReportsConsent(false);
		await settle();
		expect([server.sendsReports, server.reports]).toEqual([false, []]);
		window.dispatchEvent(new ErrorEvent('error', { error: new TypeError('after') }));
		await leave();
		expect(got()).toEqual([]);
	});

	it('follow what another tab agreed to, and stay off where the server takes none', async () => {
		const reader = await started();
		server.sendsReports = true;
		server.emit({ event: 'settings', data: { by: 'another-tab' } });
		await settle();
		expect(reader.reportsConsent?.on).toBe(true);
		reader.noteError('component', new Error('a part failed'));
		await leave();
		expect(got()).toEqual(['component: Error: a part failed']);

		server.reportsAvailable = false;
		server.sendsReports = false;
		const other = await started();
		expect(other.reportsConsent).toEqual({ available: false, on: false });
		other.setReportsConsent(true);
		await settle();
		expect(server.requests.some((r) => r.method === 'PUT' && r.path === '/reports/consent')).toBe(
			false
		);
	});
});

describe('labels', () => {
	let later: number;
	let work: number;

	beforeEach(() => {
		later = server.addLabel('Later', '#1a73e8');
		work = server.addLabel('Work', '#1e8e3e');
		server.article(tech, 3)!.labels = [work, later];
	});

	it('come from the server, and the lists say which articles have them', async () => {
		const reader = await started();
		expect(reader.labels).toEqual([
			{ id: `label-${later}`, title: 'Later', color: '#1a73e8' },
			{ id: `label-${work}`, title: 'Work', color: '#1e8e3e' }
		]);
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		const [third, second] = reader.visible;
		// In the order of the labels, whatever order the article has them in.
		expect(reader.labelsOf(third)).toEqual([`label-${later}`, `label-${work}`]);
		expect(reader.labelsOf(second)).toEqual([]);

		reader.show({ kind: 'label', id: `label-${work}` });
		await settle();
		expect(titles(reader)).toEqual([`Article 3 of feed ${tech}`]);
		expect(reader.title()).toBe('Work');
		expect(server.requests.at(-1)?.path).toMatch(new RegExp(`^/labels/${work}/articles\\?`));
		// A list of a label is no place to mark everything from.
		reader.markAllRead();
		await settle();
		expect(server.writes).toEqual([]);
	});

	it('go on and off an article at once, and back when the server refuses', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		const second = reader.visible[1];
		reader.setLabel(second, `label-${work}`, true);
		expect(reader.labelsOf(second)).toEqual([`label-${work}`]);
		await settle();
		expect(server.article(tech, 2)?.labels).toEqual([work]);
		reader.setLabel(second, `label-${work}`, false);
		reader.setLabel(second, `label-${work}`, false);
		await settle();
		expect(server.article(tech, 2)?.labels).toEqual([]);
		expect(server.writes).toEqual([
			`PUT /feeds/${tech}/articles/2/labels/${work}`,
			`DELETE /feeds/${tech}/articles/2/labels/${work}`
		]);

		server.refuse('PUT', /\/labels\/\d+$/, 500);
		reader.setLabel(second, `label-${later}`, true);
		expect(reader.labelsOf(second)).toEqual([`label-${later}`]);
		await settle();
		expect(reader.labelsOf(second)).toEqual([]);
		expect(reader.notice).toBe('notSaved');
	});

	it('are made from an article, and go on it', async () => {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		const second = reader.visible[1];
		reader.addLabel('Read later', '#d93025', second);
		await settle();
		const made = server.labels.at(-1)!;
		expect(made).toMatchObject({ title: 'Read later', color: '#d93025' });
		expect(server.article(tech, 2)?.labels).toEqual([made.id]);
		expect(reader.labelsOf(second)).toEqual([`label-${made.id}`]);
		expect(server.writes).toEqual([
			'POST /labels',
			`PUT /feeds/${tech}/articles/2/labels/${made.id}`
		]);
	});

	it('are made, renamed, recolored and deleted', async () => {
		const reader = await started();
		reader.addLabel('New', '#9334e6');
		await settle();
		expect(reader.labels.map((label) => label.title)).toEqual(['Later', 'Work', 'New']);

		reader.changeLabel(`label-${later}`, { title: 'Soon', color: '#d93025' });
		expect(reader.labels[0]).toMatchObject({ title: 'Soon', color: '#d93025' });
		await settle();
		expect(server.labels[0]).toMatchObject({ title: 'Soon', color: '#d93025' });

		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		reader.show({ kind: 'label', id: `label-${work}` });
		await settle();
		const article = reader.visible[0];
		reader.removeLabel(`label-${work}`);
		expect(reader.labels.map((label) => label.title)).toEqual(['Soon', 'New']);
		expect(reader.labelsOf(article)).toEqual([`label-${later}`]);
		expect(reader.stream).toEqual({ kind: 'all' });
		await settle();
		expect(server.labels.map((label) => label.title)).toEqual(['Soon', 'New']);
	});

	it('follow what another tab did with them', async () => {
		const reader = await started();
		server.addLabel('Elsewhere');
		server.emit({ event: 'labels', data: { by: TAB } });
		await settle();
		expect(reader.labels).toHaveLength(2);
		server.emit({ event: 'labels', data: { by: 'another-tab' } });
		await settle();
		expect(reader.labels.map((label) => label.title)).toContain('Elsewhere');
	});
});

describe('reports of feeds and articles', () => {
	async function reporting(on: boolean) {
		server.contentReports = on;
		const reader = new ServerReader({ name: 'fiona', admin: false, contentReports: on }, session);
		reader.start();
		await settle();
		return reader;
	}

	it('send a reason and a note, and a second report replaces the first', async () => {
		const reader = await reporting(true);
		expect(reader.can.reportContent).toBe(true);
		const feedId = `feed-${tech}`;
		expect(await reader.reportContent({ feedId }, 'spam', '  ads only \n')).toBe('sent');
		expect(await reader.reportContent({ feedId, seq: 2 }, 'illegal', '')).toBe('sent');
		expect(await reader.reportContent({ feedId }, 'malicious', 'phishing')).toBe('sent');
		expect(server.contentReported).toEqual([
			{ feed_id: tech, seq: 2, reason: 'illegal', note: '' },
			{ feed_id: tech, seq: null, reason: 'malicious', note: 'phishing' }
		]);
		expect(server.requests.filter((r) => r.path.endsWith('/report')).map((r) => r.path)).toEqual([
			`/feeds/${tech}/report`,
			`/feeds/${tech}/articles/2/report`,
			`/feeds/${tech}/report`
		]);
	});

	it('say when there were too many or the server did not take them', async () => {
		const reader = await reporting(true);
		server.refuse('POST', /\/report$/, 429);
		expect(await reader.reportContent({ feedId: `feed-${lone}` }, 'other', '')).toBe('tooMany');
		server.refuse('POST', /\/report$/, 502);
		expect(await reader.reportContent({ feedId: `feed-${lone}` }, 'other', '')).toBe('unreachable');
		expect(server.contentReported).toEqual([]);
	});

	it('are offered only where the server takes them', async () => {
		const reader = await reporting(false);
		expect(reader.can.reportContent).toBe(false);
		expect(await reader.reportContent({ feedId: `feed-${lone}` }, 'spam', '')).toBe('unreachable');
	});

	it('leave a blocked feed without articles and unread ones', async () => {
		server.feeds.find((feed) => feed.id === tech)!.blocked = true;
		const reader = await started();
		expect(reader.feeds[`feed-${tech}`]?.blocked).toBe(true);
		expect(reader.feeds[`feed-${blog}`]?.blocked).toBe(false);
		expect(reader.unreadIn({ kind: 'feed', id: `feed-${tech}` } as TreeNode)).toBe(0);
		reader.show({ kind: 'folder', id: `folder-${news}` });
		await settle();
		expect(titles(reader).length).toBe(2);
		expect(new Set(reader.visible.map((article) => article.feedId))).toEqual(
			new Set([`feed-${blog}`])
		);
	});
});

describe('changes without a connection', () => {
	/** The reader on the feed Tech, with its three articles, live. */
	async function onTech() {
		const reader = await started();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		return reader;
	}

	/** The server goes away, and the connection with it. */
	function goAway() {
		server.down = true;
		server.breakStreams();
	}

	/** The server is back, and the browser opened the connection again. */
	async function comeBack() {
		server.down = false;
		server.reopenStreams();
		await settle();
	}

	const unreadOf = (reader: ServerReader) => reader.unreadIn({ kind: 'feed', id: `feed-${tech}` });

	const onServer = (seq: number) => {
		const article = server.article(tech, seq)!;
		return { read: article.read, starred: article.starred };
	};

	it('keep what was read and starred, and send it once the server answers again', async () => {
		const reader = await onTech();
		goAway();
		const [third, second] = reader.visible;
		reader.setRead(third, true);
		reader.toggleStar(third);
		reader.setRead(second, true);
		await settle();
		// Nothing was taken back, and the sign says what waits.
		expect([reader.isRead(third), reader.isStarred(third), reader.isRead(second)]).toEqual([
			true,
			true,
			true
		]);
		expect(reader.unsentChanges).toBe(3);
		expect(reader.notice).toBeNull();
		expect(unreadOf(reader)).toBe(1);

		await comeBack();
		expect(reader.unsentChanges).toBe(0);
		expect([onServer(3), onServer(2)]).toEqual([
			{ read: true, starred: true },
			{ read: true, starred: false }
		]);
		expect(unreadOf(reader)).toBe(1);
		expect(reader.isRead(third)).toBe(true);
	});

	it('send what waits in its order, a later change of the same thing last', async () => {
		const reader = await onTech();
		goAway();
		const [third] = reader.visible;
		reader.setRead(third, true);
		await settle();
		reader.setRead(third, false);
		reader.setRead(third, true);
		reader.setRead(third, false);
		await settle();
		expect(reader.unsentChanges).toBe(4);
		const before = server.requests.length;
		await comeBack();
		const sent = server.requests
			.slice(before)
			.filter((request) => request.method === 'PUT' && request.path.endsWith('/state'))
			.map((request) => request.body);
		expect(sent).toEqual([{ read: true }, { read: false }, { read: true }, { read: false }]);
		expect(onServer(3).read).toBe(false);
		expect(reader.isRead(third)).toBe(false);
	});

	it('take back only what the server refuses once it answers', async () => {
		const reader = await onTech();
		goAway();
		const [third, second] = reader.visible;
		reader.setRead(third, true);
		reader.setRead(second, true);
		await settle();
		server.refuse('PUT', new RegExp(`/feeds/${tech}/articles/3/state$`), 400);
		await comeBack();
		expect(reader.notice).toBe('notSaved');
		expect([reader.isRead(third), reader.isRead(second)]).toEqual([false, true]);
		expect([onServer(3).read, onServer(2).read]).toEqual([false, true]);
		expect(unreadOf(reader)).toBe(2);
	});

	it('wait for a server behind a proxy that says it is not there', async () => {
		const reader = await onTech();
		server.refuse('PUT', /\/state$/, 502);
		reader.setRead(reader.visible[0], true);
		await settle();
		expect(reader.unsentChanges).toBe(1);
		expect(reader.notice).toBeNull();
		// The connection stayed up: there is no hello to wait for, so it is tried again.
		reader.reconnect();
		await settle();
		expect(reader.unsentChanges).toBe(0);
		expect(onServer(3).read).toBe(true);
	});

	it('try again by themselves while the connection stays up', async () => {
		vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
		try {
			const reader = new ServerReader({ name: 'fiona', admin: true }, session);
			reader.start();
			reader.show({ kind: 'feed', id: `feed-${tech}` });
			await vi.advanceTimersByTimeAsync(10);
			server.refuse('PUT', /\/state$/, 503, 2);
			reader.toggleStar(reader.visible[0]);
			await vi.advanceTimersByTimeAsync(10);
			expect(reader.unsentChanges).toBe(1);
			await vi.advanceTimersByTimeAsync(5_000);
			expect(reader.unsentChanges).toBe(1);
			await vi.advanceTimersByTimeAsync(5_000);
			expect(reader.unsentChanges).toBe(0);
			expect(onServer(3).starred).toBe(true);
		} finally {
			vi.useRealTimers();
		}
	});

	it('take back a change that cannot wait, as before', async () => {
		const reader = await onTech();
		goAway();
		reader.nudge(`feed-${lone}`, 'up');
		await settle();
		expect(reader.unsentChanges).toBe(0);
		expect(reader.notice).toBe('notSaved');
	});

	it('mark all as read while away, and the server counts once it answers', async () => {
		const reader = await onTech();
		goAway();
		reader.markAllRead();
		await settle();
		expect(unreadOf(reader)).toBe(0);
		expect(reader.unsentChanges).toBe(1);
		await comeBack();
		expect(server.unread(tech)).toBe(0);
		expect(unreadOf(reader)).toBe(0);
	});

	it('ask before the tab is closed while changes wait, and not after', async () => {
		const reader = await onTech();
		const leave = () => {
			const event = new Event('beforeunload', { cancelable: true });
			window.dispatchEvent(event);
			return event.defaultPrevented;
		};
		expect(leave()).toBe(false);
		goAway();
		reader.setRead(reader.visible[0], true);
		await settle();
		expect(leave()).toBe(true);
		await comeBack();
		expect(leave()).toBe(false);
	});

	it('are dropped when the reader stops, and never go out as the next account', async () => {
		const reader = new ServerReader({ name: 'fiona', admin: true }, session);
		const stop = reader.start();
		reader.show({ kind: 'feed', id: `feed-${tech}` });
		await settle();
		goAway();
		reader.setRead(reader.visible[0], true);
		reader.toggleStar(reader.visible[0]);
		await settle();
		expect(reader.unsentChanges).toBe(2);
		// Logging out, or the session running out, stops the reader.
		stop();
		expect(reader.unsentChanges).toBe(0);
		const next = new ServerReader({ name: 'grace', admin: false }, session);
		next.start();
		const before = server.requests.length;
		await comeBack();
		expect(server.requests.slice(before).filter((request) => request.method === 'PUT')).toEqual([]);
		expect(onServer(3)).toEqual({ read: false, starred: false });
	});

	it('end with the session, if the server says it is over', async () => {
		const reader = await onTech();
		goAway();
		reader.setRead(reader.visible[0], true);
		reader.setRead(reader.visible[1], true);
		await settle();
		server.signedIn = null;
		await comeBack();
		expect(session.expired).toHaveBeenCalled();
		expect(reader.unsentChanges).toBe(0);
		expect(
			server.requests.filter((r) => r.method === 'PUT' && r.path.endsWith('/state'))
		).toHaveLength(3);
	});

	it(`refuse more than ${MAX_UNSENT} and say so`, async () => {
		const reader = await onTech();
		goAway();
		const [third] = reader.visible;
		// Starring and taking it back again counts twice.
		for (let i = 0; i <= MAX_UNSENT / 2; i++) {
			reader.toggleStar(third);
			reader.toggleStar(third);
		}
		await settle();
		expect(reader.unsentChanges).toBe(MAX_UNSENT);
		expect(reader.notice).toBe('tooManyUnsent');
		expect(reader.isStarred(third)).toBe(false);
	});

	it('end as a simple model says, for random breaks and refusals', async () => {
		for (const seed of [1, 2, 3, 4, 5, 6]) {
			let state = seed;
			const random = (below: number) => {
				state = (state * 1103515245 + 12345) % 2 ** 31;
				return state % below;
			};
			server = new FakeServer();
			const feed = server.addFeed('Model');
			server.addArticles(feed, 4);
			vi.stubGlobal('fetch', server.fetch);
			vi.stubGlobal('EventSource', server.EventSource);
			const reader = new ServerReader({ name: 'fiona', admin: true }, session);
			const stop = reader.start();
			reader.show({ kind: 'feed', id: `feed-${feed}` });
			await settle();
			const articles = [...reader.visible];
			/** The changes made, in their order: each one should reach the server once. */
			const made: { seq: number; read?: boolean; starred?: boolean }[] = [];
			let refused = 0;
			for (let step = 0; step < 30; step++) {
				const roll = random(10);
				if (roll === 0 && !server.down) goAway();
				else if (roll === 1 && server.down) await comeBack();
				else if (roll === 2 && !server.down && refused === 0) {
					// The next change that reaches the server is refused.
					server.refuse('PUT', /\/state$/, 400);
					refused = 1;
				} else {
					const article = articles[random(articles.length)];
					if (random(2) === 0) {
						const read = !reader.isRead(article);
						reader.setRead(article, read);
						made.push({ seq: article.seq, read });
					} else {
						const starred = !reader.isStarred(article);
						reader.toggleStar(article);
						made.push({ seq: article.seq, starred });
					}
				}
				await settle();
			}
			await comeBack();
			await settle();
			expect(reader.unsentChanges).toBe(0);
			// Each change reached the server once and in its order, but for a refused one.
			const taken = server.stateChanges;
			if (taken.length === made.length) expect(taken, `seed ${seed}`).toEqual(made);
			else {
				// The refused one is missing, and only it.
				expect(refused, `seed ${seed}`).toBe(1);
				const gone = made.findIndex(
					(change, i) => JSON.stringify(change) !== JSON.stringify(taken[i])
				);
				expect(taken, `seed ${seed}`).toEqual(made.filter((_, i) => i !== gone));
			}
			// What the reader shows is what the server has.
			for (const article of articles) {
				const kept = server.article(feed, article.seq)!;
				expect([reader.isRead(article), reader.isStarred(article)], `seed ${seed}`).toEqual([
					kept.read,
					kept.starred
				]);
			}
			stop();
		}
	}, 30_000);
});
