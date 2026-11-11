// The reader that talks to the server: the tree, the lists and the articles come
// from the API, and what the reader changes goes back to it.
//
// What somebody reads, stars or sorts shows at once and is sent in the background,
// one change after the other so they arrive in the order they were made. A change
// the server refuses is taken back and said in a line over the list. The unread
// counts are the server's; a change moves them here by one until the server is
// asked again. It can do everything the page offers (`can`).
//
// What changes elsewhere comes as live updates (`GET /api/events`): the
// connection opens before anything is loaded, so nothing that happens in between
// is lost. An event says what changed, and the reader asks the server anew for
// that part; its own changes it knows by the tab's name and leaves alone.

import type {
	Article,
	ArticleVersion,
	Enclosure,
	Feed,
	Picture,
	Stream,
	TreeNode
} from '#lib/model.ts';
import type { FilterRule } from '#lib/filters.ts';
import {
	MAX_OPML,
	type Abilities,
	type Account,
	type Found,
	type Fresh,
	type Imported,
	type Notice,
	type Reader,
	type Refusal,
	type RulePreview,
	type Subscribed
} from '#lib/reader.ts';
import {
	ancestorIds,
	canGroup,
	canMove,
	feedIds,
	findNode,
	moveNode,
	nudge,
	removeFeed,
	removeFolder,
	renameFolder,
	type DropPosition,
	type NudgeDirection
} from '#lib/tree.ts';
import {
	api as theApi,
	ApiError,
	serverId,
	CLOSED,
	type Api,
	type ApiListItem,
	type EventStream,
	type ApiNewRule,
	type ApiRule,
	type ApiTree,
	type ListStream
} from './client.ts';
import type { ServerEvent } from './types.ts';

/** How many articles a page of a list has. */
export const PAGE = 40;
/** How many unread articles the start page looks at, and how many stars it shows. */
const HOME_UNREAD = 60;
const HOME_STARRED = 5;
/** How many texts are fetched at once, and how many are kept for a list that is drawn again. */
const TEXTS_AT_ONCE = 4;
const TEXTS_KEPT = 300;
/** The cursor this close to the end of the page fetches the next one. */
const AHEAD = 5;
/** A rule that is being typed is tried once the typing has paused this long. */
const PREVIEW_AFTER = 250;
/** Events that come close together ask the server once, after this long. */
const SETTLE_AFTER = 300;
/** A connection for live updates that the server ended is opened anew after this long. */
const RECONNECT_AFTER = 30_000;
/** The longest list the server gives at once, for loading what is shown anew. */
const MAX_PAGE = 200;

/** Where the closed folders are kept in the browser. */
const CLOSED_KEY = 'rueder.closed';

/** What the account did with an article. */
interface Mark {
	read: boolean;
	starred: boolean;
	hidden: boolean;
}

const unmarked: Mark = { read: false, starred: false, hidden: false };

/** The text of an article and what comes with it, as far as a list does not have it. */
interface Text {
	/** The version it is the text of. */
	changes: number;
	html: string;
	tags: string[];
	enclosures: Enclosure[];
	image: Picture | null;
	hiddenBy: string | null;
}

/** What a session does for its reader. */
export interface SessionEnds {
	/** The reader asked to log out. */
	signOut(): void;
	/** The server says there is no session any more. */
	expired(): void;
}

const feedKey = (id: number) => `feed-${id}`;

/** The events of the live updates this reader listens to. */
const EVENTS: ServerEvent['event'][] = [
	'hello',
	'resync',
	'articles',
	'updated',
	'feed',
	'reading',
	'tree',
	'labels',
	'settings',
	'rules',
	'session'
];
const folderKey = (id: number) => `folder-${id}`;
const ruleKey = (id: number) => `rule-${id}`;

function toRule(rule: ApiRule): FilterRule {
	const { scope } = rule;
	return {
		id: ruleKey(rule.id),
		enabled: rule.enabled,
		scope:
			scope.kind === 'all'
				? scope
				: { kind: scope.kind, id: scope.kind === 'feed' ? feedKey(scope.id) : folderKey(scope.id) },
		field: rule.field,
		match: rule.match,
		pattern: rule.pattern,
		action: rule.action
	};
}

/** A rule as the server takes it: without its id, its scope by the server's numbers. */
function fromRule(rule: FilterRule): ApiNewRule {
	const { scope } = rule;
	return {
		enabled: rule.enabled,
		scope: scope.kind === 'all' ? scope : { kind: scope.kind, id: serverId(scope.id) },
		field: rule.field,
		match: rule.match,
		pattern: rule.pattern,
		action: rule.action
	};
}

/** The hue of the stand-in for a feed's icon: the same for the same feed every time. */
const hueOf = (id: number) => (id * 67) % 360;

/** The tree as the page draws it, from the rows the server sends. */
export function toTree(tree: ApiTree): TreeNode[] {
	type Placed = { node: TreeNode; position: number; folder: boolean; id: number };
	// A level holds folders and feeds in one order; where positions are equal,
	// folders come first, then the older node, as the server sorts them.
	const inOrder = (a: Placed, b: Placed) =>
		a.position - b.position || Number(b.folder) - Number(a.folder) || a.id - b.id;
	const seen = new Set<number>();
	const level = (parent: number | null): TreeNode[] => {
		const nodes: Placed[] = [];
		for (const folder of tree.folders) {
			// A folder is drawn once, whatever the rows claim about circles.
			if (folder.parent_id !== parent || seen.has(folder.id)) continue;
			seen.add(folder.id);
			nodes.push({
				node: {
					kind: 'folder',
					id: folderKey(folder.id),
					title: folder.title,
					children: level(folder.id)
				},
				position: folder.position,
				folder: true,
				id: folder.id
			});
		}
		for (const feed of tree.feeds) {
			if (feed.folder_id !== parent) continue;
			nodes.push({
				node: { kind: 'feed', id: feedKey(feed.id) },
				position: feed.position,
				folder: false,
				id: feed.id
			});
		}
		return nodes.sort(inOrder).map((placed) => placed.node);
	};
	return level(null);
}

function toArticle(item: ApiListItem): Article {
	return {
		id: String(item.id),
		feedId: feedKey(item.feed_id),
		seq: item.seq,
		title: item.title,
		teaser: item.teaser,
		author: item.author,
		tags: [],
		link: item.url ?? '',
		publishedAt: Date.parse(item.published_at),
		html: null,
		enclosures: [],
		image: null,
		changes: Math.max(0, item.version - 1),
		updatedAt: item.updated_at ? Date.parse(item.updated_at) : null
	};
}

function readClosed(): ReadonlySet<string> {
	try {
		const kept: unknown = JSON.parse(localStorage.getItem(CLOSED_KEY) ?? '[]');
		return new Set(Array.isArray(kept) ? kept.filter((id) => typeof id === 'string') : []);
	} catch {
		return new Set();
	}
}

export class ServerReader implements Reader {
	tree = $state.raw<TreeNode[]>([]);
	feeds = $state.raw<Record<string, Feed>>({});
	/** Folders shown closed in the sidebar; kept in the browser. */
	closed = $state.raw<ReadonlySet<string>>(readClosed());

	stream = $state.raw<Stream>({ kind: 'home' });
	query = $state('');
	cursor = $state<string | null>(null);
	open = $state<string | null>(null);
	moved = $state(0);
	hasMore = $state(false);
	loading = $state(true);
	notice = $state<Notice | null>(null);

	/** The filter rules, in the order the server keeps them. */
	rules = $state.raw<FilterRule[]>([]);

	/** How many articles the rules hide, as the server counted them. */
	hiddenCount = $state(0);

	/** Articles that arrived in what is shown since the list was loaded. */
	pending = $state(0);
	/** The filter rules are this reader's own methods. */
	readonly filters = this;
	readonly demo = null;
	readonly can: Abilities = { oldestFirst: true, markOlder: true, subscribe: true, opml: true };
	readonly account: Account;

	/** Unread articles per feed, as the server counted them and as changes here moved them. */
	private counts = $state.raw<Record<string, number>>({});
	/** The pages of the list that are loaded, newest first. */
	private items = $state.raw<Article[]>([]);
	/** For the start page: the newest unread articles and the latest stars. */
	private homeUnread = $state.raw<Article[]>([]);
	private homeStarred = $state.raw<Article[]>([]);
	/** By the id of the article. */
	private marks = $state.raw<Record<string, Mark>>({});
	private onlyUnread = $state(false);
	private oldestFirst = $state(false);
	/** What each saved rule finds, by its id, once somebody asked. */
	private hits = $state.raw<Record<string, number>>({});
	private hitsAsked = false;

	/** The newest article the list knew when it was loaded; "mark all as read" stops there. */
	private newest: number | null = null;
	/** Counts the loads of the list, so that an answer that comes late is dropped. */
	private listRun = 0;
	private loadingMore = false;
	/** Set back to unread by hand; scrolling past does not mark these as read again. */
	private pinned = new Set<string>();
	/** A step with the keyboard that waits for the list to arrive. */
	private waitingStep: { by: 1 | -1; open: boolean } | null = null;

	/** The connection for live updates, and what keeps it going. */
	private events: EventStream | null = null;
	private stopped = false;
	/** The connection was opened anew and may have missed events. */
	private reopened = false;
	private reconnectTimer: ReturnType<typeof setTimeout> | undefined;
	/** Asking anew for what events said changed, a moment after the last of them. */
	private settling = new Map<string, ReturnType<typeof setTimeout>>();

	/** Texts being fetched, waiting to be fetched, and fetched. */
	private fetching = new Set<string>();
	private wanted: Article[] = [];
	private texts = new Map<string, Text>();

	/** Changes go to the server one after the other. */
	private sending: Promise<unknown> = Promise.resolve();
	/** How many changes of one thing are on their way, by what they change. */
	private waiting = new Map<string, number>();
	private resyncing = false;
	private newRules = 0;
	/** Counts the rules that were tried, so that only the last one typed is asked for. */
	private previewRun = 0;

	private api: Api;
	private session: SessionEnds;

	constructor(account: Account, session: SessionEnds, api: Api = theApi) {
		this.account = account;
		this.session = session;
		this.api = api;
	}

	/**
	 * Opens the connection for live updates, then loads the tree, the rules and
	 * the start page. Returns how to close the connection again.
	 */
	start() {
		this.stopped = false;
		this.connect();
		void this.loadTree();
		void this.loadRules();
		void this.loadList();
		return () => {
			this.stopped = true;
			this.events?.close();
			this.events = null;
			clearTimeout(this.reconnectTimer);
			for (const timer of this.settling.values()) clearTimeout(timer);
			this.settling.clear();
		};
	}

	signOut() {
		this.session.signOut();
	}

	dismissNotice() {
		this.notice = null;
	}

	/** Says what went wrong, or ends the session if the server no longer knows it. */
	private failed(error: unknown, notice: Notice) {
		if (error instanceof ApiError && error.signedOut) this.session.expired();
		else this.notice = notice;
	}

	// What is where

	get unreadOnly() {
		return this.onlyUnread;
	}

	set unreadOnly(only: boolean) {
		if (only === this.onlyUnread) return;
		this.onlyUnread = only;
		this.reload();
	}

	get newestFirst() {
		return !this.oldestFirst;
	}

	set newestFirst(newest: boolean) {
		if (newest === !this.oldestFirst) return;
		this.oldestFirst = !newest;
		this.reload();
	}

	/** The feeds a stream draws from, in the order of the tree. */
	private feedsOf(stream: Stream): string[] {
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

	unreadIn(node: TreeNode): number {
		return node.kind === 'feed'
			? (this.counts[node.id] ?? 0)
			: node.children.reduce((sum, child) => sum + this.unreadIn(child), 0);
	}

	totalUnread = $derived(feedIds(this.tree).reduce((sum, id) => sum + (this.counts[id] ?? 0), 0));

	private markOf = (article: Article) => this.marks[article.id] ?? unmarked;

	isRead = (article: Article) => this.markOf(article).read;
	isStarred = (article: Article) => this.markOf(article).starred;
	isHidden = (article: Article) => this.markOf(article).hidden;

	title(stream: Stream = this.stream) {
		if (stream.kind === 'feed') return this.feeds[stream.id]?.title ?? '';
		if (stream.kind === 'folder') {
			const folder = findNode(this.tree, stream.id);
			return folder?.kind === 'folder' ? folder.title : '';
		}
		return null;
	}

	// The tree from the server

	private async loadTree() {
		try {
			this.takeTree(await this.api.tree());
		} catch (error) {
			this.failed(error, 'unreachable');
		}
	}

	private takeTree(tree: ApiTree) {
		const feeds: Record<string, Feed> = {};
		const counts: Record<string, number> = {};
		for (const feed of tree.feeds) {
			const id = feedKey(feed.id);
			feeds[id] = {
				id,
				title: feed.title,
				siteUrl: feed.site_url ?? feed.feed_url,
				hue: hueOf(feed.id),
				icon: feed.icon_url,
				lastSeq: feed.last_seq,
				error: feed.error,
				waiting: !feed.fetched
			};
			counts[id] = feed.unread;
		}
		this.feeds = feeds;
		this.counts = counts;
		this.hiddenCount = tree.hidden;
		this.tree = toTree(tree);
		// What is shown may be gone: unsubscribed or removed on another device.
		const stream = this.stream;
		const gone =
			(stream.kind === 'feed' && !feeds[stream.id]) ||
			(stream.kind === 'folder' && findNode(this.tree, stream.id)?.kind !== 'folder');
		if (gone) this.show({ kind: 'all' });
	}

	/** Asks the server anew for the counts, once the changes on their way are through. */
	private resync() {
		if (this.resyncing) return;
		this.resyncing = true;
		this.sending = this.sending.then(async () => {
			this.resyncing = false;
			await this.loadTree();
		});
	}

	// The list in the main pane

	/** The loaded part of the list that passes the search. */
	visible = $derived.by(() => {
		const query = this.query.trim().toLowerCase();
		if (!query) return this.items;
		return this.items.filter(
			(article) =>
				article.title.toLowerCase().includes(query) ||
				article.teaser.toLowerCase().includes(query) ||
				article.author.toLowerCase().includes(query)
		);
	});

	get current() {
		return this.cursor
			? (this.visible.find((article) => article.id === this.cursor) ?? null)
			: null;
	}

	/** Feeds with something unread among the newest unread articles, the freshest first. */
	fresh: Fresh[] = $derived.by(() => {
		const tops = new Map<string, Article[]>();
		for (const article of this.homeUnread) {
			if (this.isRead(article) || this.isHidden(article)) continue;
			const top = tops.get(article.feedId) ?? [];
			if (top.length < 2) top.push(article);
			tops.set(article.feedId, top);
		}
		const fresh: Fresh[] = [];
		for (const [feedId, top] of tops) {
			const feed = this.feeds[feedId];
			if (feed) fresh.push({ feed, count: Math.max(this.counts[feedId] ?? 0, top.length), top });
		}
		return fresh;
	});

	latestStarred = $derived(this.homeStarred.filter((article) => this.isStarred(article)));

	/** Articles from the rows of a list, with what the account did with them noted. */
	private take(rows: ApiListItem[], hidden: boolean): Article[] {
		const marks = { ...this.marks };
		const articles = rows.map((row) => {
			const article = toArticle(row);
			// A change that is still on its way knows better than a list from before it.
			const read = this.waiting.has(`${article.id}:read`) ? marks[article.id]?.read : undefined;
			const starred = this.waiting.has(`${article.id}:starred`)
				? marks[article.id]?.starred
				: undefined;
			marks[article.id] = { read: read ?? row.read, starred: starred ?? row.starred, hidden };
			const text = this.texts.get(article.id);
			return text?.changes === article.changes ? { ...article, ...text } : article;
		});
		this.marks = marks;
		return articles;
	}

	/**
	 * Loads the list from its start, or with `more` its next page. With `again`
	 * the list is loaded anew as long as it is, without saying it is on its way:
	 * an event said that something in it changed.
	 */
	private async loadList(more = false, again = false) {
		const stream = this.stream;
		const run = ++this.listRun;
		if (!more && !again) this.loading = true;
		const length = again ? Math.min(MAX_PAGE, Math.max(PAGE, this.items.length)) : PAGE;
		try {
			if (stream.kind === 'home') {
				const [unread, starred] = await Promise.all([
					this.api.list({ kind: 'all' }, { limit: HOME_UNREAD, unread: true }),
					this.api.list({ kind: 'starred' }, { limit: HOME_STARRED })
				]);
				if (run !== this.listRun) return;
				this.homeUnread = this.take(unread, false);
				this.homeStarred = this.take(starred, false);
			} else {
				const last = more ? this.items.at(-1) : undefined;
				const oldestFirst = this.oldestFirst;
				const page = oldestFirst
					? { after: last ? Number(last.id) : 0 }
					: { before: last ? Number(last.id) : undefined };
				// A list that starts with the oldest does not say what the newest is.
				const [rows, newest] = await Promise.all([
					this.api.list(stream, {
						...page,
						limit: length,
						unread: this.onlyUnread && stream.kind !== 'starred' && stream.kind !== 'hidden'
					}),
					oldestFirst && !more ? this.api.list(stream, { limit: 1 }) : null
				]);
				if (run !== this.listRun) return;
				const articles = this.take(rows, stream.kind === 'hidden');
				this.items = more ? [...this.items, ...articles] : articles;
				this.hasMore = rows.length === length;
				if (!more) {
					this.newest = (newest ?? rows)[0]?.id ?? null;
					// What arrived is in the list now.
					this.pending = 0;
				}
				// A step that was asked for while the list was on its way is taken now.
				const waiting = this.waitingStep;
				this.waitingStep = null;
				if (!more && waiting) this.step(waiting.by, waiting.open);
			}
		} catch (error) {
			if (run !== this.listRun) return;
			this.failed(error, 'unreachable');
			// The feed or the folder is not there any more; the tree knows what is.
			if (error instanceof ApiError && error.status === 404) void this.loadTree();
		} finally {
			if (run === this.listRun) this.loading = false;
		}
	}

	/** Loads what is shown anew, from its start. */
	private reload() {
		this.pinned.clear();
		void this.loadList();
	}

	show(stream: Stream) {
		this.stream = stream;
		this.waitingStep = null;
		this.pending = 0;
		this.cursor = null;
		this.open = null;
		this.query = '';
		this.items = [];
		this.hasMore = false;
		this.newest = null;
		this.reload();
	}

	/** Brings in what arrived: the list is loaded anew from its start. */
	showNew() {
		this.reload();
	}

	refresh() {
		void this.loadTree();
		this.reload();
	}

	showMore() {
		void this.more();
	}

	/** Fetches the next page; false if there was none to fetch. */
	private async more() {
		if (!this.hasMore || this.loadingMore || this.stream.kind === 'home') return false;
		this.loadingMore = true;
		const before = this.items.length;
		try {
			await this.loadList(true);
		} finally {
			this.loadingMore = false;
		}
		return this.items.length > before;
	}

	// The text of an article

	/** Puts another article in the place of one, wherever it is listed. */
	private replace(id: string, change: (article: Article) => Article) {
		const within = (list: Article[]) =>
			list.some((article) => article.id === id)
				? list.map((article) => (article.id === id ? change(article) : article))
				: list;
		this.items = within(this.items);
		this.homeUnread = within(this.homeUnread);
		this.homeStarred = within(this.homeStarred);
	}

	load(article: Article) {
		if (article.html !== null || this.fetching.has(article.id)) return;
		this.fetching.add(article.id);
		this.wanted.push(article);
		this.fetchTexts();
	}

	/** A few at a time: the expanded view asks for a whole page of texts at once. */
	private fetchTexts() {
		while (this.fetching.size - this.wanted.length < TEXTS_AT_ONCE) {
			const article = this.wanted.shift();
			if (!article) return;
			void this.fetchText(article).finally(() => {
				this.fetching.delete(article.id);
				this.fetchTexts();
			});
		}
	}

	private async fetchText(article: Article) {
		try {
			const opened = await this.api.article(serverId(article.feedId), article.seq);
			const text: Text = {
				changes: Math.max(0, opened.version - 1),
				html: opened.html,
				// A feed may name a tag twice; the page shows each once.
				tags: [...new Set(opened.tags)],
				// A feed may name a file twice, too.
				enclosures: [...new Map(opened.enclosures.map((file) => [file.url, file])).values()].map(
					({ url, type, length }) => ({ url, type, length })
				),
				image: opened.image_url ? { url: opened.image_url, title: opened.image_title ?? '' } : null,
				hiddenBy: opened.hidden_by === null ? null : ruleKey(opened.hidden_by)
			};
			this.texts.set(article.id, text);
			// The oldest go first; a Map keeps the order they were put in.
			for (const id of this.texts.keys()) {
				if (this.texts.size <= TEXTS_KEPT) break;
				this.texts.delete(id);
			}
			this.replace(article.id, (listed) => ({
				...listed,
				...text,
				title: opened.title,
				updatedAt: opened.updated_at ? Date.parse(opened.updated_at) : null
			}));
		} catch (error) {
			this.failed(error, 'unreachable');
		}
	}

	/** The article's earlier versions, oldest first: all that are kept but the current one. */
	async versions(article: Article): Promise<ArticleVersion[]> {
		try {
			const kept = await this.api.versions(serverId(article.feedId), article.seq);
			return kept.slice(0, -1).map((version) => ({
				rev: version.version,
				fetchedAt: Date.parse(version.fetched_at),
				title: version.title,
				html: version.html
			}));
		} catch (error) {
			this.failed(error, 'unreachable');
			return [];
		}
	}

	// Reading

	/** Sends a change. If the server refuses it, `undo` takes it back, unless a later change of the same thing is on its way. */
	private write(key: string, send: () => Promise<unknown>, undo: () => void) {
		this.waiting.set(key, (this.waiting.get(key) ?? 0) + 1);
		const settled = () => {
			const left = (this.waiting.get(key) ?? 1) - 1;
			if (left > 0) this.waiting.set(key, left);
			else this.waiting.delete(key);
			return left === 0;
		};
		this.sending = this.sending.then(send).then(settled, (error: unknown) => {
			if (settled()) undo();
			this.failed(error, 'notSaved');
			// Whatever else the refusal means, the server's counts are the right ones.
			this.resync();
		});
	}

	/** Notes a change here; the unread count of the feed moves with it. */
	private mark(article: Article, change: Partial<Mark>) {
		const before = this.markOf(article);
		const after = { ...before, ...change };
		this.marks = { ...this.marks, [article.id]: after };
		if (before.read !== after.read && !after.hidden) {
			const count = (this.counts[article.feedId] ?? 0) + (after.read ? -1 : 1);
			this.counts = { ...this.counts, [article.feedId]: Math.max(0, count) };
		}
	}

	private change(article: Article, field: 'read' | 'starred', value: boolean) {
		if (this.markOf(article)[field] === value) return;
		this.mark(article, { [field]: value });
		this.write(
			`${article.id}:${field}`,
			() => this.api.setState(serverId(article.feedId), article.seq, { [field]: value }),
			() => this.mark(article, { [field]: !value })
		);
	}

	setRead(article: Article, read: boolean) {
		if (!read) this.pinned.add(article.id);
		this.change(article, 'read', read);
	}

	scrolledPast(article: Article) {
		if (!this.pinned.has(article.id)) this.setRead(article, true);
	}

	toggleStar(article: Article) {
		this.change(article, 'starred', !this.isStarred(article));
	}

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

	/**
	 * Without a cursor, either step goes to the first article. A step that comes
	 * while the list is still on its way, as from the start page, waits for it.
	 */
	step(by: 1 | -1, open = true) {
		const list = this.visible;
		if (this.loading && this.stream.kind !== 'home' && list.length === 0) {
			this.waitingStep = { by, open };
			return;
		}
		const at = list.findIndex((article) => article.id === this.cursor);
		const next = at === -1 ? 0 : at + by;
		const article = list[next];
		if (!article) {
			// Past the end of the page: the step goes on once the next page is there.
			if (by === 1 && next >= list.length) {
				void this.more().then((grew) => grew && this.step(by, open));
			}
			return;
		}
		if (by === 1 && next >= list.length - AHEAD) void this.more();
		this.select(article, open);
		if (!open) this.open = null;
		this.moved++;
	}

	/**
	 * Marks as read what the list knew of when it was loaded; what arrived since
	 * stays unread. With `olderThan`, only what was published more than that many
	 * milliseconds ago. The lists of the starred and the hidden are not a place to
	 * mark everything from.
	 */
	markAllRead(olderThan = 0) {
		const stream = this.stream;
		if (stream.kind === 'home' || stream.kind === 'starred' || stream.kind === 'hidden') return;
		const upto = this.newest;
		if (upto === null) return;
		this.sweep(stream, upto, Math.max(0, olderThan));
	}

	private sweep(stream: ListStream, upto: number, olderThan: number) {
		const feeds = new Set(this.feedsOf(stream));
		const countsBefore = this.counts;
		const marks = { ...this.marks };
		const counts = { ...this.counts };
		const before = Date.now() - olderThan;
		const swept = new Map<string, Article>();
		for (const article of [...this.items, ...this.homeUnread, ...this.homeStarred]) {
			const mark = marks[article.id] ?? unmarked;
			if (mark.read || !feeds.has(article.feedId) || Number(article.id) > upto) continue;
			if (olderThan > 0 && article.publishedAt > before) continue;
			marks[article.id] = { ...mark, read: true };
			swept.set(article.id, article);
			// By age, only the server knows how many are left; here go those the page has.
			if (!mark.hidden) counts[article.feedId] = Math.max(0, (counts[article.feedId] ?? 0) - 1);
		}
		if (olderThan === 0) for (const id of feeds) counts[id] = 0;
		this.marks = marks;
		this.counts = counts;

		this.write(
			'sweep',
			async () => {
				// The server counts an age in whole seconds.
				const age = olderThan > 0 ? Math.max(1, Math.round(olderThan / 1000)) : undefined;
				await this.api.readUpto(stream, upto, age);
				// What arrived after the list was loaded is still unread; the server counts.
				await this.loadTree();
			},
			() => {
				const marks = { ...this.marks };
				for (const article of swept.values()) {
					const mark = marks[article.id];
					if (mark && !this.waiting.has(`${article.id}:read`)) {
						marks[article.id] = { ...mark, read: false };
					}
				}
				this.marks = marks;
				this.counts = countsBefore;
			}
		);
	}

	// The tree of folders

	canMove = (id: string, targetId: string, position: DropPosition) =>
		canMove(this.tree, id, targetId, position);

	/**
	 * A change to the tree shows at once. If the server refuses it, the tree is
	 * loaded anew, which takes the change back.
	 */
	private changeTree(tree: TreeNode[], send: () => Promise<unknown>) {
		this.tree = tree;
		this.write('tree', send, () => {});
	}

	move(id: string, targetId: string, position: DropPosition) {
		const node = findNode(this.tree, id);
		const target = findNode(this.tree, targetId);
		if (!node || !target || !this.canMove(id, targetId, position)) return;
		if (position === 'inside') this.setClosed(targetId, false);
		this.changeTree(moveNode(this.tree, id, targetId, position), () =>
			this.api.move(
				{ kind: node.kind, id: serverId(id) },
				{ kind: target.kind, id: serverId(targetId) },
				position
			)
		);
	}

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
		try {
			localStorage.setItem(CLOSED_KEY, JSON.stringify([...next]));
		} catch {
			// Without storage the folders are open again after a reload.
		}
	}

	/** The server gives a new folder its id, so it shows once the server has it. */
	addFolder(title: string) {
		this.write(
			'tree',
			async () => {
				await this.api.addFolder(title);
				await this.loadTree();
			},
			() => {}
		);
	}

	canGroup = (id: string, targetId: string) => canGroup(this.tree, id, targetId);

	/**
	 * The server gives the new folder its number, so the folder shows once it has
	 * it: made in the target's folder, put where the target is, and the two feeds
	 * moved into it.
	 */
	group(id: string, targetId: string, title: string) {
		if (!this.canGroup(id, targetId)) return;
		const parent = ancestorIds(this.tree, targetId).at(-1);
		const feed = (key: string) => ({ kind: 'feed' as const, id: serverId(key) });
		this.write(
			'tree',
			async () => {
				const { id: folderId } = await this.api.addFolder(
					title,
					parent === undefined ? null : serverId(parent)
				);
				const folder = { kind: 'folder' as const, id: folderId };
				await this.api.move(folder, feed(targetId), 'before');
				await this.api.move(feed(targetId), folder, 'inside');
				await this.api.move(feed(id), folder, 'inside');
				await this.loadTree();
			},
			() => {}
		);
	}

	renameFolder(id: string, title: string) {
		this.changeTree(renameFolder(this.tree, id, title), () =>
			this.api.renameFolder(serverId(id), title)
		);
	}

	removeFolder(id: string) {
		this.changeTree(removeFolder(this.tree, id), () => this.api.removeFolder(serverId(id)));
		if (this.stream.kind === 'folder' && this.stream.id === id) this.show({ kind: 'all' });
	}

	// Filter rules

	private async loadRules() {
		try {
			this.rules = (await this.api.rules()).map(toRule);
		} catch (error) {
			this.failed(error, 'unreachable');
		}
	}

	/**
	 * A change to the rules shows in their list at once. What it hides and marks as
	 * read is the server's to work out, so the tree and the list are asked for anew
	 * once it has taken the change; a change it refuses is gone with the rules as
	 * the server has them.
	 */
	private changeRules(rules: FilterRule[], send: () => Promise<unknown>) {
		this.rules = rules;
		this.write(
			'rules',
			async () => {
				await send();
				await Promise.all([this.loadRules(), this.loadTree()]);
				this.reload();
				if (this.hitsAsked) this.countHits();
			},
			() => void this.loadRules()
		);
	}

	newRuleId = () => `rule-new-${++this.newRules}`;

	saveRule(rule: FilterRule) {
		const known = this.rules.some((r) => r.id === rule.id);
		this.changeRules(
			known ? this.rules.map((r) => (r.id === rule.id ? rule : r)) : [...this.rules, rule],
			() =>
				known
					? this.api.changeRule(serverId(rule.id), fromRule(rule))
					: this.api.addRule(fromRule(rule))
		);
	}

	deleteRule(id: string) {
		this.changeRules(
			this.rules.filter((rule) => rule.id !== id),
			() => this.api.removeRule(serverId(id))
		);
	}

	/** Asks the server what a rule would catch, once the typing has paused. */
	async preview(rule: FilterRule): Promise<RulePreview> {
		const run = ++this.previewRun;
		await new Promise((resolve) => setTimeout(resolve, PREVIEW_AFTER));
		// Another rule was typed since; whoever asked for this one has moved on.
		if (run !== this.previewRun) return 'unreachable';
		try {
			return await this.api.previewRule(fromRule(rule));
		} catch (error) {
			// 400 is the server's word for a pattern it cannot use.
			if (error instanceof ApiError && error.status === 400) return 'invalid';
			if (error instanceof ApiError && error.signedOut) this.session.expired();
			return 'unreachable';
		}
	}

	hitsOf = (rule: FilterRule) => this.hits[rule.id] ?? null;

	/** Asks the server what the saved rules find. The page shows the counts as they come. */
	countHits() {
		this.hitsAsked = true;
		void this.api.ruleHits().then(
			(hits) => {
				this.hits = Object.fromEntries(hits.map((hit) => [ruleKey(hit.id), hit.matches]));
			},
			(error: unknown) => this.failed(error, 'unreachable')
		);
	}

	/** The rule that hides the article, as the server named it with the article's text. */
	hidingRule(article: Article) {
		return this.rules.find((rule) => rule.id === article.hiddenBy) ?? null;
	}

	// Live updates

	/**
	 * Opens the connection. The browser opens it again by itself after a break
	 * and says the last event it got, so nothing is missed. One that the server
	 * ended for good, refusing it, is opened anew after a while, and what came
	 * meanwhile is asked for anew.
	 */
	private connect() {
		if (this.stopped) return;
		const events = this.api.events();
		if (!events) return;
		this.events = events;
		for (const name of EVENTS) {
			events.addEventListener(name, (message) => {
				let data: unknown;
				try {
					data = JSON.parse(message.data);
				} catch {
					return;
				}
				// The name of the event says which shape its data has.
				this.hear({ event: name, data } as ServerEvent);
			});
		}
		events.onerror = () => {
			if (events.readyState !== CLOSED || this.events !== events) return;
			this.events = null;
			// The server may have refused it because the session is over.
			void this.checkSession();
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = setTimeout(() => {
				this.reopened = true;
				this.connect();
			}, RECONNECT_AFTER);
		};
	}

	/** Runs `ask` once, a moment after the last time it was asked for. */
	private soon(key: string, ask: () => void) {
		clearTimeout(this.settling.get(key));
		this.settling.set(
			key,
			setTimeout(() => {
				this.settling.delete(key);
				if (!this.stopped) ask();
			}, SETTLE_AFTER)
		);
	}

	/** Whether the list on show has articles of this feed. */
	private shows(feedId: string) {
		return this.stream.kind === 'home' || this.feedsOf(this.stream).includes(feedId);
	}

	/** Loads the list on show anew, as long as it is, once the changes on their way are through. */
	private relist() {
		this.soon('list', () => {
			this.sending = this.sending.then(() => this.loadList(false, true));
		});
	}

	/** Loads everything anew: events were missed. */
	private catchUp() {
		void this.loadRules();
		this.resync();
		this.relist();
	}

	private async checkSession() {
		try {
			await this.api.me();
		} catch (error) {
			if (error instanceof ApiError && error.signedOut) this.session.expired();
		}
	}

	/** What an event means for what this tab shows. */
	private hear(event: ServerEvent) {
		const own = 'by' in event.data && event.data.by === this.api.tab;
		switch (event.event) {
			case 'hello':
				if (this.reopened) {
					this.reopened = false;
					this.catchUp();
				}
				return;
			case 'resync':
				this.catchUp();
				return;
			case 'articles': {
				const { feed_id, last_seq, new: count } = event.data;
				const id = feedKey(feed_id);
				const feed = this.feeds[id];
				if (!feed) return;
				this.feeds = { ...this.feeds, [id]: { ...feed, lastSeq: last_seq, waiting: false } };
				// The counts are the server's, rules and all.
				this.soon('tree', () => this.resync());
				if (this.stream.kind === 'home') this.relist();
				else if (
					this.shows(id) &&
					this.stream.kind !== 'starred' &&
					this.stream.kind !== 'hidden'
				) {
					this.pending += count;
				}
				return;
			}
			case 'updated':
				if (this.shows(feedKey(event.data.feed_id))) this.relist();
				return;
			case 'feed': {
				const id = feedKey(event.data.feed_id);
				const feed = this.feeds[id];
				if (feed) {
					const error = event.data.error;
					this.feeds = { ...this.feeds, [id]: { ...feed, error, waiting: false } };
				}
				return;
			}
			case 'reading': {
				if (own) return;
				const { feed } = event.data;
				if (feed) {
					const id = feedKey(feed.feed_id);
					if (!this.feeds[id] || this.waiting.size > 0) this.resync();
					else this.counts = { ...this.counts, [id]: feed.unread };
				} else {
					this.soon('tree', () => this.resync());
				}
				if (!feed || this.shows(feedKey(feed.feed_id))) this.relist();
				return;
			}
			case 'tree':
				if (!own) this.soon('tree', () => this.resync());
				return;
			case 'rules':
				if (own) return;
				void this.loadRules();
				this.soon('tree', () => this.resync());
				this.relist();
				return;
			case 'session':
				void this.checkSession();
				return;
			// Labels (3.4) and settings on the server (3.3) are not in the page yet.
			case 'labels':
			case 'settings':
				return;
		}
	}

	// Subscribing and OPML

	/**
	 * Asks the server after the changes on their way, for an answer the page waits
	 * for: a folder that was just made is there for a feed that goes into it.
	 */
	private after<T>(ask: () => Promise<T>): Promise<T> {
		const answer = this.sending.then(ask);
		this.sending = answer.then(
			() => {},
			() => {}
		);
		return answer;
	}

	/** Why the server did not do what was asked, for the page to say. */
	private refusal(error: unknown): Refusal {
		if (!(error instanceof ApiError)) return { problem: 'unreachable' };
		if (error.signedOut) {
			this.session.expired();
			return { problem: 'unreachable' };
		}
		switch (error.status) {
			// Not an address, nothing there, not a feed, not OPML; a folder that is gone.
			case 400:
			case 404:
			case 415:
				return { problem: 'refused', reason: error.message };
			case 413:
				return { problem: 'tooLarge' };
			case 429:
				return { problem: 'busy', retryAfter: error.retryAfter };
			case 503:
				return { problem: 'noFetching' };
			default:
				return { problem: 'unreachable' };
		}
	}

	async findFeeds(address: string): Promise<Found> {
		try {
			const found = await this.after(() => this.api.findFeeds(address));
			const feeds = found.feeds.map(({ url, title, feed_id }) => ({
				url,
				title,
				feedId: feed_id === null ? null : feedKey(feed_id)
			}));
			return { feeds, movedTo: found.moved_to };
		} catch (error) {
			return this.refusal(error);
		}
	}

	/**
	 * The server fetches a feed nobody reads yet before it subscribes, which can
	 * take a while; the feed shows once it is there.
	 */
	async subscribe(url: string, folderId: string | null): Promise<Subscribed> {
		const folder = folderId === null ? null : serverId(folderId);
		try {
			const id = await this.after(() => this.api.subscribe(url, folder));
			const feedId = feedKey(id);
			const known = feedId in this.feeds;
			await this.loadTree();
			if (folderId !== null) this.setClosed(folderId, false);
			this.show({ kind: 'feed', id: feedId });
			return { feedId, known };
		} catch (error) {
			return this.refusal(error);
		}
	}

	/**
	 * A new name shows at once. An empty one gives the feed its own name back,
	 * which only the server knows, so the tree is asked for anew.
	 */
	renameFeed(id: string, title: string) {
		const feed = this.feeds[id];
		if (!feed) return;
		if (title) this.feeds = { ...this.feeds, [id]: { ...feed, title } };
		this.write(
			'tree',
			async () => {
				await this.api.renameFeed(serverId(id), title);
				if (!title) await this.loadTree();
			},
			() => {}
		);
	}

	/**
	 * The feed goes from the tree at once; a mixed list is loaded anew once the
	 * server has ended the subscription. A rule that holds for the feed stays, as
	 * it does on the server, and holds again if the feed is subscribed again.
	 */
	unsubscribe(id: string) {
		if (!this.feeds[id]) return;
		const { [id]: _gone, ...feeds } = this.feeds;
		this.feeds = feeds;
		this.changeTree(removeFeed(this.tree, id), async () => {
			await this.api.unsubscribe(serverId(id));
			if (this.stream.kind !== 'feed') this.reload();
		});
		if (this.stream.kind === 'feed' && this.stream.id === id) this.show({ kind: 'all' });
	}

	/** The feeds of a file are fetched by the server's next round, not here. */
	async importOpml(file: Blob): Promise<Imported> {
		if (file.size > MAX_OPML) return { problem: 'tooLarge' };
		try {
			const imported = await this.after(() => this.api.importOpml(file));
			await this.loadTree();
			return imported;
		} catch (error) {
			return this.refusal(error);
		}
	}

	async exportOpml() {
		try {
			return await this.after(() => this.api.exportOpml());
		} catch (error) {
			this.refusal(error);
			return null;
		}
	}
}
