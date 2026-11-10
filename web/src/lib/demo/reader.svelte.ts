// Everything the reader page shows and does, held in the browser. In the draft this
// stands in for the server: feeds, articles, what was read, the filter rules and the
// tree of folders live here, and "live updates" are made up on a timer. The rules for
// unread counts (unread.ts), filters (filters.ts) and the tree (tree.ts) are the ones
// the server will follow.

import { ruleMatches, type FilterRule, type RuleScope } from '#lib/filters.ts';
import type { Article, Feed, Stream, TreeNode } from '#lib/model.ts';
import {
	addFolder,
	canMove,
	feedIds,
	findNode,
	moveNode,
	nudge,
	removeFolder,
	renameFolder,
	type DropPosition,
	type NudgeDirection
} from '#lib/tree.ts';
import {
	emptyState,
	isHidden,
	isRead,
	isStarred,
	markAllRead,
	markRead,
	markUnread,
	setHidden,
	setStarred,
	unreadCount,
	type FeedState
} from '#lib/unread.ts';
import { demoData, random, type DemoData } from './data.ts';

const PAGE = 40;

/** Unread articles the made-up account starts with, feed by feed in the order of the data. */
const startUnread = [3, 0, 2, 7, 1, 12, 18, 9, 1, 0, 4, 0, 2, 5];

const startRules: FilterRule[] = [
	{
		id: 'rule-1',
		enabled: true,
		scope: { kind: 'all' },
		field: 'title',
		match: 'word',
		pattern: 'Anzeige',
		action: 'hide'
	},
	{
		id: 'rule-2',
		enabled: true,
		scope: { kind: 'all' },
		field: 'author',
		match: 'contains',
		pattern: 'Sponsored Desk',
		action: 'hide'
	},
	{
		id: 'rule-3',
		enabled: false,
		scope: { kind: 'folder', id: 'folder-news' },
		field: 'title',
		match: 'regex',
		pattern: 'Hafenfest|Stadtbad',
		action: 'read'
	}
];

export class Reader {
	tree = $state.raw<TreeNode[]>([]);
	feeds = $state.raw<Record<string, Feed>>({});
	/** Articles per feed, oldest first. */
	articles = $state.raw<Record<string, Article[]>>({});
	states = $state.raw<Record<string, FeedState>>({});
	rules = $state.raw<FilterRule[]>([]);
	/** Folders shown closed in the sidebar. */
	closed = $state.raw<ReadonlySet<string>>(new Set());

	stream = $state.raw<Stream>({ kind: 'home' });
	unreadOnly = $state(false);
	newestFirst = $state(true);
	query = $state('');
	/** The article the cursor is on, and the one that is open in the list view. */
	cursor = $state<string | null>(null);
	open = $state<string | null>(null);
	/** How many articles of the list are on the page; more come with scrolling. */
	limit = $state(PAGE);
	/** Whether made-up articles keep arriving. */
	live = $state(true);
	/** Counts the steps of the cursor, so the list can scroll to where it went. */
	moved = $state(0);

	/** Per feed, the newest article the list shows; later ones wait behind "N new articles". */
	private shownUpto = $state.raw<Record<string, number>>({});
	/** Read while this list was up; an unread-only list keeps them until it is refreshed. */
	private kept = $state.raw<ReadonlySet<string>>(new Set());
	/** Set back to unread by hand; scrolling past does not mark these as read again. */
	private pinned = new Set<string>();

	private data: DemoData;
	private now: () => number;
	private rng = random(42);
	private folderCount = 0;
	private feedCount = 0;

	constructor(now: () => number = Date.now, data: DemoData = demoData(now())) {
		this.now = now;
		this.data = data;
		this.tree = data.tree;
		this.feeds = data.feeds;
		this.articles = data.articles;
		this.rules = startRules;

		const states: Record<string, FeedState> = {};
		Object.values(data.feeds).forEach((feed, index) => {
			const unread = Math.min(startUnread[index % startUnread.length], feed.lastSeq);
			let state = markAllRead(emptyState(), feed.lastSeq - unread);
			// A few exceptions from the mark, as real reading leaves them.
			if (unread >= 4) state = markRead(state, feed.lastSeq - 1);
			if (index % 3 === 0 && state.readUpto > 4) state = markUnread(state, state.readUpto - 3);
			if (index % 2 === 0 && feed.lastSeq > 6) state = setStarred(state, feed.lastSeq - 5, true);
			states[feed.id] = state;
		});
		this.states = states;
		this.applyRules();
		this.showNew();
	}

	// What is where

	/** The feeds a stream draws from, in the order of the tree. */
	feedsOf(stream: Stream): string[] {
		switch (stream.kind) {
			case 'home':
				return [];
			case 'feed':
				return this.feeds[stream.id] ? [stream.id] : [];
			case 'folder': {
				const folder = findNode(this.tree, stream.id);
				return folder?.kind === 'folder' ? feedIds(folder.children) : [];
			}
			default:
				return feedIds(this.tree);
		}
	}

	private inScope(scope: RuleScope) {
		return new Set(this.feedsOf(scope.kind === 'all' ? { kind: 'all' } : scope));
	}

	unread(feedId: string) {
		const feed = this.feeds[feedId];
		return feed ? unreadCount(this.states[feedId], feed.lastSeq) : 0;
	}

	unreadIn(node: TreeNode): number {
		return node.kind === 'feed'
			? this.unread(node.id)
			: node.children.reduce((sum, child) => sum + this.unreadIn(child), 0);
	}

	totalUnread = $derived(feedIds(this.tree).reduce((sum, id) => sum + this.unread(id), 0));

	hiddenCount = $derived(
		Object.values(this.states).reduce((sum, state) => sum + state.hidden.size, 0)
	);

	isRead = (article: Article) => isRead(this.states[article.feedId], article.seq);
	isStarred = (article: Article) => isStarred(this.states[article.feedId], article.seq);
	isHidden = (article: Article) => isHidden(this.states[article.feedId], article.seq);

	/** The first enabled rule that hides the article, to say why it is hidden. */
	hidingRule(article: Article) {
		return (
			this.rules.find(
				(rule) =>
					rule.enabled &&
					rule.action === 'hide' &&
					this.inScope(rule.scope).has(article.feedId) &&
					ruleMatches(rule, article)
			) ?? null
		);
	}

	// The list in the main pane

	/** All articles of the stream that pass its filters, in reading order. */
	list = $derived.by(() => {
		const stream = this.stream;
		const query = this.query.trim().toLowerCase();
		const onlyUnread = this.unreadOnly && stream.kind !== 'starred' && stream.kind !== 'hidden';
		const found: Article[] = [];
		for (const feedId of this.feedsOf(stream)) {
			const state = this.states[feedId];
			const upto = this.shownUpto[feedId] ?? 0;
			for (const article of this.articles[feedId] ?? []) {
				if (article.seq > upto) continue;
				if (isHidden(state, article.seq) !== (stream.kind === 'hidden')) continue;
				if (stream.kind === 'starred' && !isStarred(state, article.seq)) continue;
				if (onlyUnread && isRead(state, article.seq) && !this.kept.has(article.id)) continue;
				if (
					query &&
					!article.title.toLowerCase().includes(query) &&
					!article.teaser.toLowerCase().includes(query) &&
					!article.author.toLowerCase().includes(query)
				) {
					continue;
				}
				found.push(article);
			}
		}
		const direction = this.newestFirst ? -1 : 1;
		return found.sort((a, b) => direction * (a.publishedAt - b.publishedAt));
	});

	/** The part of the list that is on the page. */
	visible = $derived(this.list.slice(0, this.limit));

	/** Articles that arrived since the list was drawn. */
	pending = $derived.by(() => {
		if (this.stream.kind === 'hidden' || this.stream.kind === 'starred') return 0;
		let count = 0;
		for (const feedId of this.feedsOf(this.stream)) {
			const state = this.states[feedId];
			const upto = this.shownUpto[feedId] ?? 0;
			for (const article of this.articles[feedId] ?? []) {
				if (article.seq > upto && !isHidden(state, article.seq)) count++;
			}
		}
		return count;
	});

	show(stream: Stream) {
		this.stream = stream;
		this.cursor = null;
		this.open = null;
		this.query = '';
		this.refresh();
	}

	/** Brings in the articles that arrived since the list was drawn. */
	showNew() {
		this.shownUpto = Object.fromEntries(
			Object.values(this.feeds).map((feed) => [feed.id, feed.lastSeq])
		);
	}

	refresh() {
		this.kept = new Set();
		this.pinned.clear();
		this.limit = PAGE;
		this.showNew();
	}

	showMore() {
		if (this.limit < this.list.length) this.limit += PAGE;
	}

	// Reading

	private setState(feedId: string, state: FeedState) {
		if (state !== this.states[feedId]) this.states = { ...this.states, [feedId]: state };
	}

	setRead(article: Article, read: boolean) {
		const state = this.states[article.feedId];
		if (read) {
			if (!isRead(state, article.seq)) this.kept = new Set(this.kept).add(article.id);
			this.setState(article.feedId, markRead(state, article.seq));
		} else {
			this.pinned.add(article.id);
			this.setState(article.feedId, markUnread(state, article.seq));
		}
	}

	/** The article scrolled out at the top of the expanded view: read, unless it was set back by hand. */
	scrolledPast(article: Article) {
		if (!this.pinned.has(article.id)) this.setRead(article, true);
	}

	toggleStar(article: Article) {
		const state = this.states[article.feedId];
		this.setState(article.feedId, setStarred(state, article.seq, !isStarred(state, article.seq)));
	}

	/** Puts the cursor on an article; opening it marks it as read. */
	select(article: Article, open = true) {
		this.cursor = article.id;
		if (open) {
			this.open = article.id;
			this.setRead(article, true);
		}
	}

	toggleOpen(article: Article) {
		if (this.open === article.id) {
			this.cursor = article.id;
			this.open = null;
		} else {
			this.select(article);
		}
	}

	get current() {
		return this.cursor ? (this.list.find((article) => article.id === this.cursor) ?? null) : null;
	}

	/** Moves the cursor by one article, down the list for +1; returns where it landed. */
	step(by: 1 | -1, open = true) {
		const list = this.list;
		if (list.length === 0) return null;
		const at = list.findIndex((article) => article.id === this.cursor);
		const next = at === -1 ? (by === 1 ? 0 : -1) : at + by;
		const article = list[next];
		if (!article) return null;
		if (next >= this.limit) this.limit = next + PAGE;
		this.select(article, open);
		if (!open) this.open = null;
		this.moved++;
		return article;
	}

	/** Marks the stream's articles on the page as read; with `olderThan`, only those older than that many milliseconds. */
	markAllRead(olderThan = 0) {
		const before = this.now() - olderThan;
		const states = { ...this.states };
		for (const feedId of this.feedsOf(this.stream)) {
			const shown = this.shownUpto[feedId] ?? 0;
			// Articles are numbered as they arrive, so the old ones are the low numbers.
			const upto = (this.articles[feedId] ?? []).reduce(
				(max, article) =>
					article.seq <= shown && article.publishedAt <= before ? Math.max(max, article.seq) : max,
				0
			);
			states[feedId] = markAllRead(states[feedId], upto);
		}
		this.states = states;
	}

	// The tree of folders

	canMove = (id: string, targetId: string, position: DropPosition) =>
		canMove(this.tree, id, targetId, position);

	move(id: string, targetId: string, position: DropPosition) {
		this.tree = moveNode(this.tree, id, targetId, position);
		if (position === 'inside') this.setClosed(targetId, false);
		this.applyRules();
	}

	/** Moves a node one step with the keyboard; false when there is nowhere to go. */
	nudge(id: string, direction: NudgeDirection) {
		const step = nudge(this.tree, id, direction);
		if (!step || !this.canMove(id, step.targetId, step.position)) return false;
		this.move(id, step.targetId, step.position);
		return true;
	}

	isClosed = (folderId: string) => this.closed.has(folderId);

	setClosed(folderId: string, closed: boolean) {
		const next = new Set(this.closed);
		if (closed) next.add(folderId);
		else next.delete(folderId);
		this.closed = next;
	}

	addFolder(title: string) {
		const id = `folder-new-${++this.folderCount}`;
		this.tree = addFolder(this.tree, id, title);
		return id;
	}

	renameFolder(id: string, title: string) {
		this.tree = renameFolder(this.tree, id, title);
	}

	removeFolder(id: string) {
		this.tree = removeFolder(this.tree, id);
		if (this.stream.kind === 'folder' && this.stream.id === id) this.show({ kind: 'all' });
		this.applyRules();
	}

	/** Adds a feed for an address. The draft cannot fetch, so the feed starts empty. */
	subscribe(address: string) {
		let host = address.trim();
		try {
			host = new URL(/^[a-z]+:\/\//i.test(host) ? host : `https://${host}`).hostname;
		} catch {
			// Not an address; the text itself is the title.
		}
		const id = `feed-new-${++this.feedCount}`;
		const feed: Feed = {
			id,
			title: host,
			siteUrl: `https://${host}`,
			hue: (this.feedCount * 67) % 360,
			lastSeq: 0,
			error: null
		};
		this.feeds = { ...this.feeds, [id]: feed };
		this.articles = { ...this.articles, [id]: [] };
		this.states = { ...this.states, [id]: emptyState() };
		this.tree = [...this.tree, { kind: 'feed', id }];
		this.show({ kind: 'feed', id });
		return id;
	}

	// Filter rules

	/** Works out again which articles the rules hide, and marks what they mark as read. */
	private applyRules(only?: Article) {
		const rules = this.rules
			.filter((rule) => rule.enabled)
			.map((rule) => ({ rule, feeds: this.inScope(rule.scope) }));
		const states = { ...this.states };
		for (const feedId of only ? [only.feedId] : Object.keys(this.feeds)) {
			const mine = rules.filter(({ feeds }) => feeds.has(feedId));
			let state = states[feedId];
			const hidden = only ? new Set(state.hidden) : new Set<number>();
			for (const article of only ? [only] : (this.articles[feedId] ?? [])) {
				for (const { rule } of mine) {
					if (!ruleMatches(rule, article)) continue;
					if (rule.action === 'hide') hidden.add(article.seq);
					else state = markRead(state, article.seq);
				}
			}
			states[feedId] = setHidden(state, hidden);
		}
		this.states = states;
	}

	/** How many of the articles in its scope a rule finds, whether it is on or not. */
	countMatches(rule: FilterRule) {
		let count = 0;
		for (const feedId of this.inScope(rule.scope)) {
			for (const article of this.articles[feedId] ?? []) {
				if (ruleMatches(rule, article)) count++;
			}
		}
		return count;
	}

	/** How many articles there are in a rule's scope. */
	countIn(scope: RuleScope) {
		let count = 0;
		for (const feedId of this.inScope(scope)) count += (this.articles[feedId] ?? []).length;
		return count;
	}

	saveRule(rule: FilterRule) {
		this.rules = this.rules.some((r) => r.id === rule.id)
			? this.rules.map((r) => (r.id === rule.id ? rule : r))
			: [...this.rules, rule];
		this.applyRules();
	}

	deleteRule(id: string) {
		this.rules = this.rules.filter((rule) => rule.id !== id);
		this.applyRules();
	}

	newRuleId = () => `rule-${this.now()}-${Math.floor(this.rng() * 1000)}`;

	// Made-up live updates

	/** A new article arrives in `feedId`, or in a feed picked by how often it posts. */
	arrive(feedId = this.pickFeed()) {
		const feed = this.feeds[feedId];
		if (!feed || !(feedId in this.data.pace)) return null;
		const seq = feed.lastSeq + 1;
		const article = this.data.nextArticle(feedId, seq, this.now());
		this.articles = { ...this.articles, [feedId]: [...this.articles[feedId], article] };
		this.feeds = { ...this.feeds, [feedId]: { ...feed, lastSeq: seq } };
		this.applyRules(article);
		return article;
	}

	private pickFeed() {
		const pace = Object.entries(this.data.pace).filter(([id]) => this.feeds[id]);
		let at = this.rng() * pace.reduce((sum, [, perDay]) => sum + perDay, 0);
		for (const [id, perDay] of pace) {
			at -= perDay;
			if (at <= 0) return id;
		}
		return pace[0][0];
	}

	/** An article changes at its source: the text so far becomes an earlier version. */
	revise(article: Article, note: string) {
		const revised: Article = {
			...article,
			versions: [
				...article.versions,
				{
					rev: article.versions.length + 1,
					fetchedAt: article.updatedAt ?? article.publishedAt,
					title: article.title,
					html: article.html
				}
			],
			html: `${article.html}<p>${note}</p>`,
			updatedAt: this.now()
		};
		this.articles = {
			...this.articles,
			[article.feedId]: this.articles[article.feedId].map((a) =>
				a.id === article.id ? revised : a
			)
		};
		return revised;
	}

	/** Lets articles arrive every few seconds while `live` is on; returns how to stop. */
	startLive(every = 9000) {
		const timer = setInterval(() => {
			if (!this.live) return;
			const count = this.rng() < 0.3 ? 2 : 1;
			for (let i = 0; i < count; i++) this.arrive();
		}, every);
		return () => clearInterval(timer);
	}

	title(stream: Stream = this.stream) {
		if (stream.kind === 'feed') return this.feeds[stream.id]?.title ?? '';
		if (stream.kind === 'folder') {
			const folder = findNode(this.tree, stream.id);
			return folder?.kind === 'folder' ? folder.title : '';
		}
		return null;
	}
}
