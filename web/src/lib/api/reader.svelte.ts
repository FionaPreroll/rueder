// The reader that talks to the server: the tree, the lists and the articles come
// from the API, and what the reader changes goes back to it.
//
// What somebody reads, stars or sorts shows at once and is sent in the background,
// one change after the other so they arrive in the order they were made. A change
// the server refuses is taken back and said in a line over the list. The unread
// counts are the server's; a change moves them here by one until the server is
// asked again.
//
// What the server cannot do yet is left out here and switched off in the page
// (`can`, `filters`): lists that start with the oldest article, marking as read by
// age, the filter rules and subscribing.

import type { Article, ArticleVersion, Feed, Stream, TreeNode } from '#lib/model.ts';
import type { Abilities, Account, Fresh, Notice, Reader } from '#lib/reader.ts';
import {
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
	api as theApi,
	ApiError,
	serverId,
	type Api,
	type ApiListItem,
	type ApiTree,
	type ListStream
} from './client.ts';

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

/** Where the closed folders are kept in the browser. */
const CLOSED_KEY = 'rueder.closed';

/** What the account did with an article. */
interface Mark {
	read: boolean;
	starred: boolean;
	hidden: boolean;
}

const unmarked: Mark = { read: false, starred: false, hidden: false };

/** The text of an article, as far as a list does not have it. */
interface Text {
	/** The version it is the text of. */
	changes: number;
	html: string;
	tags: string[];
}

/** What a session does for its reader. */
export interface SessionEnds {
	/** The reader asked to log out. */
	signOut(): void;
	/** The server says there is no session any more. */
	expired(): void;
}

const feedKey = (id: number) => `feed-${id}`;
const folderKey = (id: number) => `folder-${id}`;

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

	// What comes with later work packages: live updates (3.2), a count of the
	// hidden articles, the filter rules and subscribing (3.5).
	readonly pending = 0;
	readonly hiddenCount = 0;
	readonly filters = null;
	readonly demo = null;
	readonly can: Abilities = { oldestFirst: false, markOlder: false, subscribe: false };
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

	/** The newest article the list knew when it was loaded; "mark all as read" stops there. */
	private newest: number | null = null;
	/** Counts the loads of the list, so that an answer that comes late is dropped. */
	private listRun = 0;
	private loadingMore = false;
	/** Set back to unread by hand; scrolling past does not mark these as read again. */
	private pinned = new Set<string>();

	/** Texts being fetched, waiting to be fetched, and fetched. */
	private fetching = new Set<string>();
	private wanted: Article[] = [];
	private texts = new Map<string, Text>();

	/** Changes go to the server one after the other. */
	private sending: Promise<unknown> = Promise.resolve();
	/** How many changes of one thing are on their way, by what they change. */
	private waiting = new Map<string, number>();
	private resyncing = false;

	private api: Api;
	private session: SessionEnds;

	constructor(account: Account, session: SessionEnds, api: Api = theApi) {
		this.account = account;
		this.session = session;
		this.api = api;
	}

	/** Loads the tree and the start page. There is nothing to stop yet. */
	start() {
		void this.loadTree();
		void this.loadList();
		return () => {};
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

	/** The server's lists start with the newest article. */
	get newestFirst() {
		return true;
	}

	set newestFirst(_: boolean) {}

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
				error: feed.error
			};
			counts[id] = feed.unread;
		}
		this.feeds = feeds;
		this.counts = counts;
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
			return text?.changes === article.changes
				? { ...article, html: text.html, tags: text.tags }
				: article;
		});
		this.marks = marks;
		return articles;
	}

	private async loadList(more = false) {
		const stream = this.stream;
		const run = ++this.listRun;
		if (!more) this.loading = true;
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
				const rows = await this.api.list(stream, {
					limit: PAGE,
					before: last ? Number(last.id) : undefined,
					unread: this.onlyUnread && stream.kind !== 'starred' && stream.kind !== 'hidden'
				});
				if (run !== this.listRun) return;
				const articles = this.take(rows, stream.kind === 'hidden');
				this.items = more ? [...this.items, ...articles] : articles;
				this.hasMore = rows.length === PAGE;
				if (!more) this.newest = rows[0]?.id ?? null;
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
		this.cursor = null;
		this.open = null;
		this.query = '';
		this.items = [];
		this.hasMore = false;
		this.newest = null;
		this.reload();
	}

	/** Until live updates say what arrived (work package 3.2), this loads the list anew. */
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
				tags: [...new Set(opened.tags)]
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

	step(by: 1 | -1, open = true) {
		const list = this.visible;
		const at = list.findIndex((article) => article.id === this.cursor);
		const next = at === -1 ? (by === 1 ? 0 : -1) : at + by;
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
	 * stays unread. The lists of the starred and the hidden are not a place to mark
	 * everything from, and the server cannot mark by age yet.
	 */
	markAllRead(olderThan = 0) {
		const stream = this.stream;
		if (olderThan > 0 || stream.kind === 'home') return;
		if (stream.kind === 'starred' || stream.kind === 'hidden') return;
		const upto = this.newest;
		if (upto === null) return;
		this.sweep(stream, upto);
	}

	private sweep(stream: ListStream, upto: number) {
		const feeds = new Set(this.feedsOf(stream));
		const countsBefore = this.counts;
		const marks = { ...this.marks };
		const swept: Article[] = [];
		for (const article of [...this.items, ...this.homeUnread, ...this.homeStarred]) {
			const mark = marks[article.id] ?? unmarked;
			if (mark.read || !feeds.has(article.feedId) || Number(article.id) > upto) continue;
			marks[article.id] = { ...mark, read: true };
			swept.push(article);
		}
		const counts = { ...this.counts };
		for (const id of feeds) counts[id] = 0;
		this.marks = marks;
		this.counts = counts;

		this.write(
			'sweep',
			async () => {
				await this.api.readUpto(stream, upto);
				// What arrived after the list was loaded is still unread; the server counts.
				await this.loadTree();
			},
			() => {
				const marks = { ...this.marks };
				for (const article of swept) {
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

	renameFolder(id: string, title: string) {
		this.changeTree(renameFolder(this.tree, id, title), () =>
			this.api.renameFolder(serverId(id), title)
		);
	}

	removeFolder(id: string) {
		this.changeTree(removeFolder(this.tree, id), () => this.api.removeFolder(serverId(id)));
		if (this.stream.kind === 'folder' && this.stream.id === id) this.show({ kind: 'all' });
	}

	/** Comes with work package 3.5; `can.subscribe` keeps the page from asking. */
	subscribe() {}
}
