// A made-up rueder server for the tests: it answers `fetch` the way the real one
// does, from feeds and articles held in memory. Tests put it in the place of
// `fetch`, look at what was asked (`requests`), and can make it refuse or hold
// back answers.

import type {
	ApiFeed,
	ApiFolder,
	ApiListItem,
	ApiNewRule,
	ApiNode,
	ApiRule,
	ApiVersion
} from '#lib/api/client.ts';
import { compile, ruleMatches } from '#lib/filters.ts';

interface FakeFeed {
	id: number;
	folder_id: number | null;
	position: number;
	title: string;
	error: string | null;
	icon_url: string | null;
}

export interface FakeArticle {
	id: number;
	feed_id: number;
	seq: number;
	title: string;
	teaser: string;
	author: string;
	url: string | null;
	published_at: string;
	updated_at: string | null;
	version: number;
	html: string;
	tags: string[];
	read: boolean;
	starred: boolean;
	hidden: boolean;
	versions: ApiVersion[];
}

interface Request {
	method: string;
	/** The path after /api, with its query. */
	path: string;
	body: unknown;
}

interface Refusal {
	method: string;
	path: RegExp;
	status: number;
	/** How many requests it still refuses. */
	times: number;
}

const START = Date.parse('2026-11-01T08:00:00Z');
const MINUTE = 60_000;

const json = (body: unknown, status = 200) =>
	new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });
const empty = (status = 204) => new Response(null, { status });
const refused = (status: number, reason = '', headers: Record<string, string> = {}) =>
	new Response(reason, { status, headers });

export class FakeServer {
	/** Passwords by name. */
	accounts = new Map([['fiona', 'correct horse']]);
	/** Who the browser's session belongs to. */
	signedIn: string | null = 'fiona';
	oidc = false;
	/** Wrong tries are refused for this many seconds. */
	lockedFor = 0;

	folders: ApiFolder[] = [];
	feeds: FakeFeed[] = [];
	articles: FakeArticle[] = [];
	rules: ApiRule[] = [];

	requests: Request[] = [];
	/** The server is not there at all: `fetch` fails. */
	down = false;
	private refusals: Refusal[] = [];
	private held: { path: RegExp; wait: Promise<void> }[] = [];
	private nextId = 1;

	// Filling it

	addFolder(title: string, parent: number | null = null) {
		const id = this.nextId++;
		this.folders.push({ id, parent_id: parent, title, position: this.level(parent).length + 1 });
		return id;
	}

	addFeed(title: string, folder: number | null = null, over: Partial<FakeFeed> = {}) {
		const id = this.nextId++;
		const position = this.level(folder).length + 1;
		this.feeds.push({
			id,
			folder_id: folder,
			position,
			title,
			error: null,
			icon_url: null,
			...over
		});
		return id;
	}

	/** Adds articles to a feed, as if they had just arrived; returns the last one. */
	addArticles(feedId: number, count: number, over: Partial<FakeArticle> = {}) {
		let article!: FakeArticle;
		for (let i = 0; i < count; i++) {
			const id = this.nextId++;
			const seq = this.articles.filter((a) => a.feed_id === feedId).length + 1;
			article = {
				id,
				feed_id: feedId,
				seq,
				title: `Article ${seq} of feed ${feedId}`,
				teaser: `Teaser ${seq}`,
				author: 'Ada',
				url: `https://feed-${feedId}.example/${seq}`,
				published_at: new Date(START + id * MINUTE).toISOString(),
				updated_at: null,
				version: 1,
				html: `<p>Text ${seq} of feed ${feedId}.</p>`,
				tags: ['news'],
				read: false,
				starred: false,
				hidden: false,
				versions: [],
				...over
			};
			this.articles.push(article);
		}
		return article;
	}

	article(feedId: number, seq: number) {
		return this.articles.find((a) => a.feed_id === feedId && a.seq === seq);
	}

	unread(feedId: number) {
		return this.articles.filter((a) => a.feed_id === feedId && !a.read && !a.hidden).length;
	}

	// Misbehaving

	/** The next `times` requests of this kind are refused with `status`. */
	refuse(method: string, path: RegExp, status: number, times = 1) {
		this.refusals.push({ method, path, status, times });
	}

	/** Answers to these paths wait until the returned function is called. */
	hold(path: RegExp) {
		let release!: () => void;
		const wait = new Promise<void>((resolve) => (release = resolve));
		this.held.push({ path, wait });
		return () => {
			this.held = this.held.filter((held) => held.wait !== wait);
			release();
		};
	}

	/** The requests that changed something, as "METHOD path". */
	get writes() {
		return this.requests
			.filter((request) => request.method !== 'GET')
			.map((request) => `${request.method} ${request.path}`);
	}

	// Answering

	fetch = async (input: RequestInfo | URL, init: RequestInit = {}): Promise<Response> => {
		const url = new URL(String(input), 'https://reader.example');
		const method = init.method ?? 'GET';
		const path = url.pathname.replace(/^\/api/, '') + url.search;
		const body: unknown = typeof init.body === 'string' ? JSON.parse(init.body) : undefined;
		this.requests.push({ method, path, body });
		if (this.down) throw new TypeError('fetch failed');
		for (const held of this.held) if (held.path.test(path)) await held.wait;
		const refusal = this.refusals.find((r) => r.method === method && r.path.test(path));
		if (refusal) {
			if (--refusal.times === 0) this.refusals = this.refusals.filter((r) => r !== refusal);
			return refused(refusal.status, 'refused');
		}
		return this.answer(method, url.pathname.replace(/^\/api/, ''), url.searchParams, body);
	};

	private answer(method: string, path: string, query: URLSearchParams, body: unknown): Response {
		const data = (body ?? {}) as Record<string, unknown>;
		if (method === 'GET' && path === '/login/ways') return json({ oidc: this.oidc });
		if (method === 'POST' && path === '/login') {
			if (this.lockedFor > 0)
				return refused(429, 'too many', { 'Retry-After': `${this.lockedFor}` });
			const name = String(data.username).trim().toLowerCase();
			if (this.accounts.get(name) !== data.password) return refused(401, 'wrong name or password');
			this.signedIn = name;
			return json(this.me());
		}
		if (method === 'POST' && path === '/logout') {
			this.signedIn = null;
			return empty();
		}
		if (!this.signedIn) return refused(401);
		if (method === 'GET' && path === '/me') return json(this.me());
		if (method === 'GET' && path === '/tree') return json(this.tree());

		const list = /^\/(articles|starred|hidden)$|^\/(feeds|folders)\/(\d+)\/articles$/.exec(path);
		if (method === 'GET' && list) {
			const scope = list[1] ?? list[2];
			const feeds = this.scope(scope, Number(list[3]));
			if (!feeds) return refused(404);
			return json({ articles: this.list(scope, feeds, query) });
		}

		const one = /^\/feeds\/(\d+)\/articles\/(\d+)(\/versions|\/state)?$/.exec(path);
		if (one) {
			const article = this.article(Number(one[1]), Number(one[2]));
			if (!article) return refused(404);
			if (method === 'GET' && !one[3]) {
				const {
					read: _read,
					starred: _starred,
					hidden: _hidden,
					versions: _versions,
					...opened
				} = article;
				return json({ ...opened, enclosures: [], image_url: null, image_title: null, labels: [] });
			}
			if (method === 'GET' && one[3] === '/versions') return json({ versions: article.versions });
			if (method === 'PUT' && one[3] === '/state') {
				if (typeof data.read === 'boolean') article.read = data.read;
				if (typeof data.starred === 'boolean') article.starred = data.starred;
				return empty();
			}
		}

		const read = /^\/read$|^\/(feeds|folders)\/(\d+)\/read$/.exec(path);
		if (method === 'POST' && read) {
			const feeds = this.scope(read[1] ?? 'articles', Number(read[2]));
			if (!feeds) return refused(404);
			if (typeof data.upto_id !== 'number') return refused(400, 'give upto_id');
			for (const article of this.articles) {
				if (feeds.has(article.feed_id) && article.id <= data.upto_id) article.read = true;
			}
			return empty();
		}

		if (method === 'GET' && path === '/rules') return json(this.rules);
		const rule = /^\/rules(?:\/(\d+|preview))?$/.exec(path);
		if (rule && method !== 'GET') {
			const known = this.rules.find((r) => r.id === Number(rule[1]));
			if (method === 'DELETE') {
				if (!known) return refused(404);
				this.rules = this.rules.filter((r) => r !== known);
				this.applyRules();
				return json({ id: known.id });
			}
			const sent = data as unknown as ApiNewRule;
			sent.pattern = sent.pattern.trim();
			if (compile(sent) === null) return refused(400, 'the pattern cannot be used');
			if (sent.scope.kind !== 'all' && !this.scope(`${sent.scope.kind}s`, sent.scope.id)) {
				return refused(404);
			}
			if (method === 'POST' && rule[1] === 'preview') {
				const matches = this.articles.filter((article) => this.catches(sent, article)).length;
				const of = this.articles.filter((a) => this.inScope(sent, a.feed_id)).length;
				return json({ matches, of });
			}
			if (method === 'POST' && !rule[1]) {
				const added = { ...sent, id: this.nextId++ };
				this.rules.push(added);
				this.applyRules();
				return json({ id: added.id }, 201);
			}
			if (method === 'PUT' && known) {
				Object.assign(known, sent);
				this.applyRules();
				return json({ id: known.id });
			}
			return refused(404);
		}

		if (method === 'POST' && path === '/tree/move') {
			return this.move(data.node as ApiNode, data.target as ApiNode, String(data.position));
		}
		if (method === 'POST' && path === '/folders') {
			const title = String(data.title).trim();
			if (!title) return refused(400, 'the name is empty');
			return json({ id: this.addFolder(title) }, 201);
		}
		const folder = /^\/folders\/(\d+)$/.exec(path);
		if (folder) {
			const found = this.folders.find((f) => f.id === Number(folder[1]));
			if (!found) return refused(404);
			if (method === 'PATCH') {
				found.title = String(data.title).trim();
				return empty();
			}
			if (method === 'DELETE') {
				// What was in it moves to where the folder was.
				const inside = this.level(found.id);
				const around = this.level(found.parent_id);
				around.splice(around.indexOf(found), 1, ...inside);
				for (const node of inside) this.place(node, found.parent_id);
				this.folders = this.folders.filter((f) => f !== found);
				this.renumber(around);
				return empty();
			}
		}
		return refused(404);
	}

	private me() {
		return { id: 1, username: this.signedIn, is_admin: true };
	}

	private tree(): { folders: ApiFolder[]; feeds: ApiFeed[] } {
		return {
			folders: this.folders,
			feeds: this.feeds.map((feed) => ({
				...feed,
				feed_title: feed.title,
				feed_url: `https://feed-${feed.id}.example/feed.xml`,
				site_url: `https://feed-${feed.id}.example`,
				fetch_agent: 'rueder',
				last_seq: this.articles.filter((a) => a.feed_id === feed.id).length,
				read_upto: 0,
				unread: this.unread(feed.id)
			}))
		};
	}

	/** The feeds a list or a "mark as read" is about; undefined for what is not there. */
	private scope(kind: string, id: number): Set<number> | undefined {
		if (kind === 'feeds') {
			return this.feeds.some((feed) => feed.id === id) ? new Set([id]) : undefined;
		}
		if (kind === 'folders') {
			if (!this.folders.some((folder) => folder.id === id)) return undefined;
			const inside = (folder: number): number[] => [
				...this.feeds.filter((feed) => feed.folder_id === folder).map((feed) => feed.id),
				...this.folders.filter((f) => f.parent_id === folder).flatMap((f) => inside(f.id))
			];
			return new Set(inside(id));
		}
		return new Set(this.feeds.map((feed) => feed.id));
	}

	private list(kind: string, feeds: Set<number>, query: URLSearchParams): ApiListItem[] {
		const before = Number(query.get('before') ?? Infinity);
		const limit = Number(query.get('limit') ?? 50);
		const unread = query.get('unread') === 'true';
		return this.articles
			.filter(
				(a) =>
					feeds.has(a.feed_id) &&
					a.id < before &&
					a.hidden === (kind === 'hidden') &&
					(kind !== 'starred' || a.starred) &&
					!(unread && a.read)
			)
			.sort((a, b) => b.id - a.id)
			.slice(0, limit)
			.map((a) => ({
				feed_id: a.feed_id,
				seq: a.seq,
				id: a.id,
				published_at: a.published_at,
				updated_at: a.updated_at,
				version: a.version,
				title: a.title,
				teaser: a.teaser,
				author: a.author,
				url: a.url,
				thumbnail_url: null,
				read: a.read,
				starred: a.starred,
				labels: []
			}));
	}

	// Filter rules

	private inScope(rule: ApiNewRule, feedId: number) {
		const { scope } = rule;
		return scope.kind === 'all' || !!this.scope(`${scope.kind}s`, scope.id)?.has(feedId);
	}

	/** Whether a rule finds an article, by the rules of the page's own model. */
	private catches(rule: ApiNewRule, article: FakeArticle) {
		if (!this.inScope(rule, article.feed_id)) return false;
		const scope = { kind: 'all' } as const;
		return ruleMatches(
			{ ...rule, id: '', scope },
			{ ...article, id: '', feedId: '', link: '', publishedAt: 0, changes: 0, updatedAt: null }
		);
	}

	/** An article is hidden exactly if a rule that is on hides it; "read" only ever marks. */
	applyRules() {
		const on = this.rules.filter((rule) => rule.enabled);
		for (const article of this.articles) {
			const found = on.filter((rule) => this.catches(rule, article));
			article.hidden = found.some((rule) => rule.action === 'hide');
			if (found.some((rule) => rule.action === 'read')) article.read = true;
		}
	}

	// The tree, as the server keeps it: rows with a parent and a position

	private level(parent: number | null): (ApiFolder | FakeFeed)[] {
		return [
			...this.folders.filter((folder) => folder.parent_id === parent),
			...this.feeds.filter((feed) => feed.folder_id === parent)
		].sort((a, b) => a.position - b.position);
	}

	private row(node: ApiNode) {
		return node.kind === 'folder'
			? this.folders.find((folder) => folder.id === node.id)
			: this.feeds.find((feed) => feed.id === node.id);
	}

	private parentOf = (row: ApiFolder | FakeFeed) =>
		'parent_id' in row ? row.parent_id : row.folder_id;

	private place(row: ApiFolder | FakeFeed, parent: number | null) {
		if ('parent_id' in row) row.parent_id = parent;
		else row.folder_id = parent;
	}

	private renumber(rows: (ApiFolder | FakeFeed)[]) {
		rows.forEach((row, index) => (row.position = index + 1));
	}

	private move(node: ApiNode, target: ApiNode, position: string) {
		const moved = this.row(node);
		const beside = this.row(target);
		if (!moved || !beside) return refused(404);
		if (position === 'inside' && target.kind !== 'folder') return refused(400);
		const parent = position === 'inside' ? target.id : this.parentOf(beside);
		this.place(moved, parent);
		const rows = this.level(parent).filter((row) => row !== moved);
		const at = rows.indexOf(beside);
		const index = at === -1 ? rows.length : position === 'after' ? at + 1 : at;
		rows.splice(index, 0, moved);
		this.renumber(rows);
		return empty();
	}
}
