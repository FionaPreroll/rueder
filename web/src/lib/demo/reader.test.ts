import { afterEach, describe, expect, it, vi } from 'vitest';
import type { FilterRule } from '#lib/filters.ts';
import { MAX_FOLDER_DEPTH } from '#lib/model.ts';
import { feedIds, findNode } from '#lib/tree.ts';
import { DemoReader } from './reader.svelte.ts';

const start = Date.parse('2026-11-10T14:30:00Z');
const DAY = 86_400_000;

/** A reader at a fixed time, which the test can move on. */
function setup() {
	const clock = { now: start };
	const reader = new DemoReader(() => clock.now);
	return { reader, clock };
}

/** The unread count worked out the slow way, article by article. */
const counted = (reader: DemoReader, feedId: string) =>
	reader.articles[feedId].filter((article) => !reader.isHidden(article) && !reader.isRead(article))
		.length;

const rule = (over: Partial<FilterRule>): FilterRule => ({
	id: 'rule-test',
	enabled: true,
	scope: { kind: 'all' },
	field: 'title',
	match: 'contains',
	pattern: '',
	action: 'hide',
	...over
});

afterEach(() => vi.useRealTimers());

describe('unread counts', () => {
	it('agree with the articles, feed by feed and in sum', () => {
		const { reader } = setup();
		let sum = 0;
		for (const id of Object.keys(reader.feeds)) {
			expect(reader.unread(id), id).toBe(counted(reader, id));
			sum += reader.unread(id);
		}
		expect(sum).toBeGreaterThan(0);
		expect(reader.totalUnread).toBe(sum);
	});

	it('add up through the folders', () => {
		const { reader } = setup();
		const tech = findNode(reader.tree, 'folder-tech')!;
		const expected = reader
			.feedsOf({ kind: 'folder', id: 'folder-tech' })
			.reduce((total, id) => total + reader.unread(id), 0);
		expect(reader.unreadIn(tech)).toBe(expected);
		expect(reader.unreadIn({ kind: 'feed', id: 'feed-netz' })).toBe(reader.unread('feed-netz'));
		expect(reader.unread('nope')).toBe(0);
	});

	it('stay right through reading, arriving and marking all', () => {
		const { reader } = setup();
		const id = 'feed-tagesbote';
		reader.show({ kind: 'feed', id });
		const before = reader.unread(id);
		const unread = reader.list.find((article) => !reader.isRead(article))!;
		reader.select(unread);
		expect(reader.unread(id)).toBe(before - 1);
		reader.setRead(unread, false);
		expect(reader.unread(id)).toBe(before);
		reader.arrive(id);
		reader.arrive(id);
		expect(reader.unread(id)).toBe(counted(reader, id));
		// What arrived after the list was drawn is not marked with it.
		reader.markAllRead();
		expect(reader.unread(id)).toBe(counted(reader, id));
		expect(reader.unread(id)).toBeLessThanOrEqual(2);
		reader.showNew();
		reader.markAllRead();
		expect(reader.unread(id)).toBe(0);
	});
});

describe('the list', () => {
	it('shows a feed, a folder or everything', () => {
		const { reader } = setup();
		expect(reader.list).toEqual([]);
		reader.show({ kind: 'feed', id: 'feed-ferris' });
		expect(new Set(reader.list.map((article) => article.feedId))).toEqual(new Set(['feed-ferris']));
		reader.show({ kind: 'folder', id: 'folder-rust' });
		expect(new Set(reader.list.map((article) => article.feedId))).toEqual(
			new Set(['feed-ferris', 'feed-borrow'])
		);
		reader.show({ kind: 'all' });
		expect(new Set(reader.list.map((article) => article.feedId)).size).toBe(
			Object.keys(reader.feeds).length
		);
		expect(reader.feedsOf({ kind: 'feed', id: 'nope' })).toEqual([]);
		expect(reader.feedsOf({ kind: 'folder', id: 'feed-ferris' })).toEqual([]);
	});

	it('sorts by time, either way', () => {
		const { reader } = setup();
		reader.show({ kind: 'all' });
		const newest = reader.list.map((article) => article.publishedAt);
		expect(newest).toEqual([...newest].sort((a, b) => b - a));
		reader.newestFirst = false;
		const oldest = reader.list.map((article) => article.publishedAt);
		expect(oldest).toEqual([...newest].reverse());
	});

	it('shows a page at a time', () => {
		const { reader } = setup();
		reader.show({ kind: 'all' });
		expect(reader.visible).toHaveLength(40);
		reader.showMore();
		expect(reader.visible).toHaveLength(80);
		reader.show({ kind: 'feed', id: 'feed-wort' });
		const all = reader.list.length;
		reader.showMore();
		reader.showMore();
		expect(reader.visible).toHaveLength(all);
	});

	it('keeps what was just read in an unread-only list until it is refreshed', () => {
		const { reader } = setup();
		reader.show({ kind: 'feed', id: 'feed-tagesbote' });
		reader.unreadOnly = true;
		const length = reader.list.length;
		expect(length).toBe(reader.unread('feed-tagesbote'));
		reader.select(reader.list[0]);
		expect(reader.list).toHaveLength(length);
		reader.refresh();
		expect(reader.list).toHaveLength(length - 1);
	});

	it('searches titles, texts and authors', () => {
		const { reader } = setup();
		reader.show({ kind: 'all' });
		const all = reader.list.length;
		const sample = reader.list[3];
		reader.query = sample.title.toUpperCase();
		expect(reader.list.length).toBeLessThan(all);
		expect(reader.list).toContain(sample);
		reader.query = sample.author;
		expect(reader.list.every((article) => article.author === sample.author)).toBe(true);
		reader.query = 'no article says this';
		expect(reader.list).toEqual([]);
		reader.show({ kind: 'all' });
		expect(reader.query).toBe('');
	});

	it('lists what is starred, and what the rules hide', () => {
		const { reader } = setup();
		reader.show({ kind: 'starred' });
		const starred = reader.list.length;
		expect(starred).toBeGreaterThan(0);
		expect(reader.list.every((article) => reader.isStarred(article))).toBe(true);
		reader.toggleStar(reader.list[0]);
		expect(reader.list).toHaveLength(starred - 1);

		reader.show({ kind: 'hidden' });
		expect(reader.list).toHaveLength(reader.hiddenCount);
		expect(reader.hiddenCount).toBeGreaterThan(0);
		expect(reader.list.every((article) => reader.hidingRule(article) !== null)).toBe(true);
		// Unread-only does not apply to these two lists.
		reader.unreadOnly = true;
		expect(reader.list).toHaveLength(reader.hiddenCount);
	});

	it('names what is shown', () => {
		const { reader } = setup();
		expect(reader.title()).toBeNull();
		expect(reader.title({ kind: 'feed', id: 'feed-ferris' })).toBe('Ferris Wochenblatt');
		expect(reader.title({ kind: 'folder', id: 'folder-news' })).toBe('Nachrichten');
		expect(reader.title({ kind: 'folder', id: 'nope' })).toBe('');
		expect(reader.title({ kind: 'feed', id: 'nope' })).toBe('');
	});
});

describe('the cursor', () => {
	it('steps through the list and opens what it lands on', () => {
		const { reader } = setup();
		reader.show({ kind: 'feed', id: 'feed-tagesbote' });
		expect(reader.current).toBeNull();
		const first = reader.step(1);
		expect(first).toBe(reader.list[0]);
		expect(reader.open).toBe(first?.id);
		expect(reader.isRead(first!)).toBe(true);
		expect(reader.step(1)).toBe(reader.list[1]);
		expect(reader.step(-1)).toBe(first);
		expect(reader.step(-1)).toBeNull();
		expect(reader.current).toBe(first);
		expect(reader.moved).toBe(3);
	});

	it('steps without opening', () => {
		const { reader } = setup();
		reader.show({ kind: 'feed', id: 'feed-tagesbote' });
		const unread = reader.unread('feed-tagesbote');
		const first = reader.step(1, false);
		expect(reader.cursor).toBe(first?.id);
		expect(reader.open).toBeNull();
		expect(reader.unread('feed-tagesbote')).toBe(unread);
	});

	it('stays put going up from nowhere, and brings more of the list going down', () => {
		const { reader } = setup();
		reader.show({ kind: 'all' });
		expect(reader.step(-1, false)).toBeNull();
		expect(reader.cursor).toBeNull();
		for (let i = 0; i <= 40; i++) reader.step(1, false);
		expect(reader.current).toBe(reader.list[40]);
		expect(reader.visible).toContain(reader.current);
		reader.show({ kind: 'feed', id: 'nope' });
		expect(reader.step(1)).toBeNull();
	});

	it('opens and closes an article', () => {
		const { reader } = setup();
		reader.show({ kind: 'feed', id: 'feed-ferris' });
		const article = reader.list[2];
		reader.toggleOpen(article);
		expect(reader.open).toBe(article.id);
		reader.toggleOpen(article);
		expect(reader.open).toBeNull();
		expect(reader.cursor).toBe(article.id);
	});

	it('leaves an article unread that was set back by hand', () => {
		const { reader } = setup();
		reader.show({ kind: 'feed', id: 'feed-tagesbote' });
		const [kept, passed] = reader.list.filter((article) => !reader.isRead(article));
		reader.select(kept);
		reader.setRead(kept, false);
		reader.scrolledPast(kept);
		reader.scrolledPast(passed);
		expect(reader.isRead(kept)).toBe(false);
		expect(reader.isRead(passed)).toBe(true);
	});
});

describe('marking all as read', () => {
	it('marks the shown stream only', () => {
		const { reader } = setup();
		const other = reader.unread('feed-tagesbote');
		reader.show({ kind: 'folder', id: 'folder-tech' });
		reader.markAllRead();
		expect(reader.unreadIn(findNode(reader.tree, 'folder-tech')!)).toBe(0);
		expect(reader.unread('feed-tagesbote')).toBe(other);
	});

	it('can leave the last day alone', () => {
		const { reader } = setup();
		const id = 'feed-tagesbote';
		reader.show({ kind: 'feed', id });
		const recent = reader.list.filter(
			(article) => article.publishedAt > start - DAY && !reader.isRead(article)
		).length;
		expect(recent).toBeGreaterThan(0);
		reader.markAllRead(DAY);
		expect(reader.unread(id)).toBe(recent);
	});
});

describe('live updates', () => {
	it('hold new articles back until they are asked for', () => {
		const { reader, clock } = setup();
		const id = 'feed-harbor';
		reader.show({ kind: 'feed', id });
		const length = reader.list.length;
		const unread = reader.unread(id);
		clock.now += 60_000;
		const article = reader.arrive(id)!;
		expect(article.publishedAt).toBe(clock.now);
		expect(reader.unread(id)).toBe(unread + (reader.isHidden(article) ? 0 : 1));
		expect(reader.list).toHaveLength(length);
		expect(reader.pending).toBe(reader.isHidden(article) ? 0 : 1);
		reader.showNew();
		expect(reader.pending).toBe(0);
		expect(reader.list.length).toBe(length + (reader.isHidden(article) ? 0 : 1));
	});

	it('hide an advertisement as it arrives', () => {
		const { reader } = setup();
		reader.saveRule(rule({ field: 'any', match: 'regex', pattern: '.' }));
		const unread = reader.totalUnread;
		const hidden = reader.hiddenCount;
		expect(unread).toBe(0);
		reader.arrive('feed-harbor');
		expect(reader.totalUnread).toBe(0);
		expect(reader.hiddenCount).toBe(hidden + 1);
	});

	it('pick a feed by themselves, never one that does not post', () => {
		const { reader } = setup();
		const total = () => Object.values(reader.feeds).reduce((sum, feed) => sum + feed.lastSeq, 0);
		const before = total();
		const broken = reader.feeds['feed-wort'].lastSeq;
		for (let i = 0; i < 40; i++) reader.arrive();
		expect(total()).toBe(before + 40);
		expect(reader.feeds['feed-wort'].lastSeq).toBe(broken);
		expect(reader.arrive('nope')).toBeNull();
	});

	it('run on a timer while live is on', () => {
		vi.useFakeTimers();
		const { reader } = setup();
		const total = () => Object.values(reader.feeds).reduce((sum, feed) => sum + feed.lastSeq, 0);
		const before = total();
		const stop = reader.startLive(1000);
		vi.advanceTimersByTime(3000);
		const arrived = total() - before;
		expect(arrived).toBeGreaterThanOrEqual(3);
		reader.live = false;
		vi.advanceTimersByTime(3000);
		expect(total() - before).toBe(arrived);
		reader.live = true;
		stop();
		vi.advanceTimersByTime(3000);
		expect(total() - before).toBe(arrived);
	});
});

describe('versions', () => {
	it('keep the text so far when an article changes', () => {
		const { reader, clock } = setup();
		reader.show({ kind: 'feed', id: 'feed-ferris' });
		const article = reader.list.find((entry) => entry.versions.length === 0)!;
		reader.select(article);
		clock.now += 5 * 60_000;
		const revised = reader.revise(article, 'An added paragraph.');
		expect(revised.versions).toEqual([
			{ rev: 1, fetchedAt: article.publishedAt, title: article.title, html: article.html }
		]);
		expect(revised.html).toBe(`${article.html}<p>An added paragraph.</p>`);
		expect(revised.updatedAt).toBe(clock.now);
		// The updated article is the same article: still the cursor, still read.
		expect(reader.current).toEqual(revised);
		expect(reader.isRead(revised)).toBe(true);
		const again = reader.revise(revised, 'One more.');
		expect(again.versions.map((version) => version.rev)).toEqual([1, 2]);
		expect(again.versions[1].fetchedAt).toBe(clock.now);
	});
});

describe('filter rules', () => {
	it('start with rules that hide the advertisements', () => {
		const { reader } = setup();
		const [first, second, third] = reader.rules;
		expect(reader.countMatches(first)).toBeGreaterThan(0);
		expect(reader.countMatches(second)).toBeGreaterThan(0);
		expect(reader.hiddenCount).toBe(reader.countMatches(first) + reader.countMatches(second));
		// The third is off, but can still say what it would find.
		expect(third.enabled).toBe(false);
		expect(reader.countMatches(third)).toBeGreaterThan(0);
	});

	it('hide and bring back as rules change, and the counts follow', () => {
		const { reader } = setup();
		const hidden = reader.hiddenCount;
		const [first] = reader.rules;
		reader.saveRule({ ...first, enabled: false });
		expect(reader.hiddenCount).toBe(hidden - reader.countMatches(first));
		for (const id of Object.keys(reader.feeds))
			expect(reader.unread(id), id).toBe(counted(reader, id));
		reader.deleteRule(first.id);
		expect(reader.rules).toHaveLength(2);
		reader.saveRule(rule({ field: 'author', pattern: 'Mara Lindqvist' }));
		expect(reader.rules).toHaveLength(3);
		expect(reader.unread('feed-ferris')).toBe(counted(reader, 'feed-ferris'));
		reader.show({ kind: 'feed', id: 'feed-ferris' });
		expect(reader.list.some((article) => article.author === 'Mara Lindqvist')).toBe(false);
	});

	it('apply in their scope only', () => {
		const { reader } = setup();
		const everywhere = rule({ field: 'any', match: 'regex', pattern: '.' });
		const inRust = { ...everywhere, scope: { kind: 'folder', id: 'folder-rust' } as const };
		expect(reader.countIn(inRust.scope)).toBe(
			reader.articles['feed-ferris'].length + reader.articles['feed-borrow'].length
		);
		expect(reader.countMatches(inRust)).toBe(reader.countIn(inRust.scope));
		expect(reader.countIn({ kind: 'all' })).toBe(Object.values(reader.articles).flat().length);
		expect(reader.countIn({ kind: 'feed', id: 'feed-kernel' })).toBe(
			reader.articles['feed-kernel'].length
		);
		const kernel = reader.unread('feed-kernel');
		reader.saveRule(inRust);
		expect(reader.unread('feed-ferris')).toBe(0);
		expect(reader.unread('feed-borrow')).toBe(0);
		expect(reader.unread('feed-kernel')).toBe(kernel);
		// A feed dragged into the folder comes under the rule.
		reader.move('feed-kernel', 'folder-rust', 'inside');
		expect(reader.unread('feed-kernel')).toBe(0);
	});

	it('can mark as read instead of hiding', () => {
		const { reader } = setup();
		const hidden = reader.hiddenCount;
		reader.saveRule(
			rule({
				scope: { kind: 'feed', id: 'feed-tagesbote' },
				field: 'any',
				match: 'regex',
				pattern: '.',
				action: 'read'
			})
		);
		expect(reader.unread('feed-tagesbote')).toBe(0);
		expect(reader.hiddenCount).toBe(hidden);
	});

	it('makes ids for new rules', () => {
		const { reader } = setup();
		expect(reader.newRuleId()).toMatch(/^rule-\d+-\d+$/);
	});
});

describe('the tree', () => {
	it('moves feeds and opens the folder they land in', () => {
		const { reader } = setup();
		reader.setClosed('folder-news', true);
		expect(reader.isClosed('folder-news')).toBe(true);
		expect(reader.canMove('feed-nebenan', 'folder-news', 'inside')).toBe(true);
		reader.move('feed-nebenan', 'folder-news', 'inside');
		expect(reader.feedsOf({ kind: 'folder', id: 'folder-news' })).toContain('feed-nebenan');
		expect(reader.isClosed('folder-news')).toBe(false);
		// Moving changes where things are, not how much is unread.
		expect(reader.totalUnread).toBe(
			feedIds(reader.tree).reduce((sum, id) => sum + counted(reader, id), 0)
		);
	});

	it('steps with the keyboard, and says when there is nowhere to go', () => {
		const { reader } = setup();
		const order = () => reader.tree.map((node) => node.id);
		expect(reader.nudge('folder-tech', 'up')).toBe(false);
		expect(reader.nudge('folder-tech', 'down')).toBe(true);
		expect(order().slice(0, 2)).toEqual(['folder-news', 'folder-tech']);
		expect(reader.nudge('feed-wort', 'in')).toBe(false);
		expect(reader.nudge('feed-wort', 'out')).toBe(false);
	});

	it('refuses folders nested deeper than the limit', () => {
		const { reader } = setup();
		let parent = 'folder-rust';
		// Rust is three levels deep; fill up to the limit.
		for (let level = 4; level <= MAX_FOLDER_DEPTH; level++) {
			const id = reader.addFolder(`Level ${level}`);
			expect(reader.canMove(id, parent, 'inside')).toBe(true);
			reader.move(id, parent, 'inside');
			parent = id;
		}
		const tooDeep = reader.addFolder('One too many');
		expect(reader.canMove(tooDeep, parent, 'inside')).toBe(false);
		expect(reader.canMove('feed-wort', parent, 'inside')).toBe(true);
	});

	it('adds, renames and removes folders', () => {
		const { reader } = setup();
		const id = reader.addFolder('Later');
		expect(reader.title({ kind: 'folder', id })).toBe('Later');
		reader.renameFolder(id, 'Someday');
		expect(reader.title({ kind: 'folder', id })).toBe('Someday');
		reader.show({ kind: 'folder', id: 'folder-news' });
		reader.removeFolder('folder-news');
		expect(findNode(reader.tree, 'folder-news')).toBeNull();
		expect(reader.tree.map((node) => node.id)).toContain('feed-tagesbote');
		// The folder that was shown is gone, so everything is shown.
		expect(reader.stream).toEqual({ kind: 'all' });
	});

	it('finds the feed at an address, which is the address itself', async () => {
		const { reader } = setup();
		const found = (feedId: string | null) => ({
			feeds: [{ url: 'https://blog.example/feed.xml', title: 'blog.example', feedId }],
			movedTo: null
		});
		expect(await reader.findFeeds(' blog.example/feed.xml ')).toEqual(found(null));
		// Once subscribed, it says so.
		const subscribed = await reader.subscribe('blog.example/feed.xml', null);
		const feedId = 'feedId' in subscribed ? subscribed.feedId : '';
		expect(await reader.findFeeds('https://blog.example/feed.xml')).toEqual(found(feedId));
		reader.unsubscribe(feedId);
		expect(await reader.findFeeds('blog.example/feed.xml')).toEqual(found(null));
		for (const nothing of ['', 'not an address', 'localhost', 'ftp://files.example/feed']) {
			expect(await reader.findFeeds(nothing)).toEqual({ problem: 'refused', reason: '' });
		}
	});

	it('subscribes to an address, into a folder or at the top', async () => {
		const { reader } = setup();
		const subscribed = await reader.subscribe(' blog.example/feed.xml ', null);
		if (!('feedId' in subscribed)) throw new Error('not subscribed');
		const id = subscribed.feedId;
		expect(subscribed.known).toBe(false);
		expect(reader.feeds[id]).toMatchObject({ title: 'blog.example', lastSeq: 0 });
		expect(reader.tree.at(-1)).toEqual({ kind: 'feed', id });
		expect(reader.stream).toEqual({ kind: 'feed', id });
		expect(reader.list).toEqual([]);
		expect(reader.unread(id)).toBe(0);
		// The same address again is the same feed.
		expect(await reader.subscribe('https://blog.example/feed.xml', null)).toEqual({
			feedId: id,
			known: true
		});

		reader.setClosed('folder-news', true);
		const other = await reader.subscribe('http://news.example/atom', 'folder-news');
		if (!('feedId' in other)) throw new Error('not subscribed');
		expect(reader.feeds[other.feedId].siteUrl).toBe('http://news.example');
		const news = findNode(reader.tree, 'folder-news');
		expect(news?.kind === 'folder' && news.children.at(-1)).toEqual({
			kind: 'feed',
			id: other.feedId
		});
		expect(reader.isClosed('folder-news')).toBe(false);
		// A folder that is gone is the top level.
		const third = await reader.subscribe('third.example', 'folder-gone');
		expect('feedId' in third && reader.tree.at(-1)?.id).toBe('feedId' in third && third.feedId);

		expect(await reader.subscribe('not an address', null)).toEqual({
			problem: 'refused',
			reason: ''
		});
		// The draft cannot fetch, so nothing arrives there.
		expect(reader.arrive(id)).toBeNull();
	});

	it('renames a feed, and gives it its own name back', async () => {
		const { reader } = setup();
		const before = reader.feeds['feed-tagesbote'].title;
		reader.renameFeed('feed-tagesbote', 'Morning paper');
		expect(reader.feeds['feed-tagesbote'].title).toBe('Morning paper');
		reader.renameFeed('feed-tagesbote', 'Evening paper');
		reader.renameFeed('feed-tagesbote', '  ');
		expect(reader.feeds['feed-tagesbote'].title).toBe(before);
		reader.renameFeed('feed-gone', 'Nothing');
		expect(reader.feeds['feed-gone']).toBeUndefined();
	});

	it('unsubscribes, and what was there goes with it', async () => {
		const { reader } = setup();
		const total = reader.totalUnread;
		const gone = reader.unread('feed-tagesbote');
		expect(gone).toBeGreaterThan(0);
		reader.show({ kind: 'feed', id: 'feed-tagesbote' });
		reader.unsubscribe('feed-tagesbote');
		expect(reader.feeds['feed-tagesbote']).toBeUndefined();
		expect(findNode(reader.tree, 'feed-tagesbote')).toBeNull();
		expect(feedIds(reader.tree)).not.toContain('feed-tagesbote');
		expect(reader.totalUnread).toBe(total - gone);
		expect(reader.stream).toEqual({ kind: 'all' });
		expect(reader.list.some((article) => article.feedId === 'feed-tagesbote')).toBe(false);
		// Subscribed again, it is a new and empty feed.
		const again = await reader.subscribe('tagesbote.example', null);
		expect('feedId' in again && again.feedId).not.toBe('feed-tagesbote');
		reader.unsubscribe('feed-gone');
	});

	it('has no OPML', async () => {
		const { reader } = setup();
		expect(reader.can.opml).toBe(false);
		expect(await reader.importOpml()).toEqual({ problem: 'unreachable' });
		expect(await reader.exportOpml()).toBeNull();
	});
});
