// A made-up rueder server for the tests: it answers `fetch` the way the real one
// does, from feeds and articles held in memory. Tests put it in the place of
// `fetch`, look at what was asked (`requests`), and can make it refuse or hold
// back answers.

import type {
	ApiArticle,
	ApiFeed,
	ApiFolder,
	ApiLabel,
	ApiListItem,
	ApiNewRule,
	ApiNode,
	ApiRule,
	ApiVersion
} from '#lib/api/client.ts';
import type { EventStream } from '#lib/api/client.ts';
import type { ServerEvent } from '#lib/api/types.ts';
import { demoStatus } from '#lib/demo/status.ts';
import { compile, ruleMatches } from '#lib/filters.ts';

interface FakeFeed {
	id: number;
	folder_id: number | null;
	position: number;
	/** The feed's own name, and the one the reader gave it. */
	title: string;
	name: string | null;
	url: string;
	error: string | null;
	icon_url: string | null;
	/** Whether the server fetched it yet; a feed from an OPML file waits. */
	fetched: boolean;
	/** The operator blocked it. */
	blocked?: boolean;
}

/** A feed the made-up web has, at the address of a site or of the feed itself. */
interface FakeSiteFeed {
	url: string;
	title: string;
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
	updated_what: ApiListItem['updated_what'];
	version: number;
	html: string;
	tags: string[];
	read: boolean;
	starred: boolean;
	hidden: boolean;
	versions: ApiVersion[];
	labels: number[];
	enclosures: ApiArticle['enclosures'];
	image_url: string | null;
	/** The picture a list shows the article by, at the proxy. */
	thumbnail_url?: string | null;
	image_title: string | null;
	/** The retention took its text; its line stays. */
	text_dropped: boolean;
}

/** An invitation, with the token the real server keeps only the hash of. */
export interface FakeInvite {
	id: number;
	token: string;
	max_uses: number;
	uses: number;
	created_at: string;
	expires_at: string;
	revoked: boolean;
}

interface Request {
	method: string;
	/** The path after /api, with its query. */
	path: string;
	body: unknown;
	/** The type of the body, as the request says it. */
	type: string | null;
	/** The tab that sent it, as it names itself. */
	tab: string | null;
}

type Listener = (event: MessageEvent<string>) => void;

interface Refusal {
	method: string;
	path: RegExp;
	status: number;
	/** How many requests it still refuses. */
	times: number;
	reason: string;
	headers: Record<string, string>;
}

const START = Date.parse('2026-11-01T08:00:00Z');
const MINUTE = 60_000;

const json = (body: unknown, status = 200) =>
	new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });
const empty = (status = 204) => new Response(null, { status });
const refused = (status: number, reason = '', headers: Record<string, string> = {}) =>
	new Response(reason, { status, headers });
const text = (body: string, type: string) =>
	new Response(body, { status: 200, headers: { 'Content-Type': type } });

export class FakeServer {
	/** Passwords by name. */
	accounts = new Map([['fiona', 'correct horse']]);
	/** Who the browser's session belongs to. */
	signedIn: string | null = 'fiona';
	/** Accounts the operator disabled, by name. */
	disabled = new Set<string>();
	/** Whether the account that is logged in is an admin, who sees the status. */
	admin = true;
	/** An admin whose right waits for a login with a second factor. */
	adminNeedsSecondFactor = false;
	/**
	 * The second factor of the account: the code its app shows now, and the
	 * recovery codes left. A secret that waits for its code is `pending`.
	 */
	secondFactor: { code: string; recovery: string[] } | null = null;
	pendingSecondFactor = false;
	/** Whether the server has passkeys, and the passkeys of the accounts, by the device's key id. */
	passkeysOn = false;
	passkeys: {
		id: number;
		owner: string;
		key: string;
		name: string;
		created_at: string;
		used_at: string | null;
	}[] = [];
	/** The recovery codes a first passkey gives where the second factor was off. */
	recoveryOfPasskeys: string[] = [];
	private ceremonies = 0;
	/** The code the made-up app shows, and the recovery codes a confirmation gives. */
	readonly appCode = '246810';
	readonly recoveryCodes = ['AAAAA-BBBBB', 'CCCCC-DDDDD'];
	/** Whether the server takes reports of errors, whether the account sends them, and what it sent. */
	reportsAvailable = true;
	sendsReports = false;
	reports: unknown[] = [];
	/** Whether readers can report feeds and articles, and what they reported. */
	contentReports = false;
	contentReported: { feed_id: number; seq: number | null; reason: unknown; note: unknown }[] = [];
	/** Whether apps of other makers may log in, and the account's app passwords. */
	appApi = true;
	appPasswords: { id: number; name: string; created_at: string; used_at: string | null }[] = [];
	/** The secret the next app password gets. */
	nextSecret = 'abcd-efgh-ijkl-mnop-qrst-uvwx';
	oidc = false;
	/** Whether anybody can register, and why nobody can right now: no room, or enough for today. */
	registrationOpen = false;
	/** The operator's legal texts, and the version of the terms an account agrees to. */
	legal: {
		imprint: string | null;
		privacy: string | null;
		terms: string | null;
		terms_version: string | null;
	} = { imprint: null, privacy: null, terms: null, terms_version: null };
	/** The version of the terms the account agreed to. */
	agreedTerms: string | null = null;
	noRoom: 'full' | 'today' | null = null;
	/** The invitations of the account that is logged in, the newest first. */
	invites: FakeInvite[] = [];
	/** For how many accounts a reader who is no admin may invite in all, and how long a link is good. */
	inviteQuota = 0;
	inviteDays = 7;
	/** Wrong tries are refused for this many seconds. */
	lockedFor = 0;
	/**
	 * Searches only this many ids back per request and then says where to go on,
	 * as the real server does when a search takes long; 0 searches everything.
	 */
	searchWindow = 0;

	folders: ApiFolder[] = [];
	feeds: FakeFeed[] = [];
	articles: FakeArticle[] = [];
	rules: ApiRule[] = [];
	labels: ApiLabel[] = [];
	/** The settings of the account, as the page stored them; `{}` for a new account. */
	settings: unknown = { theme: 'system' };
	/** The feeds at an address of the made-up web, by the address with its scheme. */
	sites = new Map<string, FakeSiteFeed[]>();
	/** Addresses that send the request on to another, like a site's login. */
	moves = new Map<string, string>();

	requests: Request[] = [];
	/** The connections for live updates that are open, and how many requests came before each. */
	streams: { stream: EventStream; after: number }[] = [];

	/** What the page opens as `EventSource`: a connection this server sends its events to. */
	readonly EventSource = (() => {
		const server = this;
		return class implements EventStream {
			readyState = 1;
			onerror: ((event: Event) => void) | null = null;
			listeners = new Map<string, Listener[]>();
			constructor(readonly url: string) {
				server.streams.push({ stream: this, after: server.requests.length });
			}
			addEventListener(type: string, listener: Listener) {
				this.listeners.set(type, [...(this.listeners.get(type) ?? []), listener]);
			}
			close() {
				this.readyState = 2;
				server.streams = server.streams.filter((open) => open.stream !== this);
			}
		};
	})();
	/** What time it is for the server, for what it does by age. */
	now = () => Date.now();
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
			name: null,
			url: `https://feed-${id}.example/feed.xml`,
			error: null,
			icon_url: null,
			fetched: true,
			...over
		});
		return id;
	}

	/** A site of the made-up web: the feed at `url` itself, and a page that names it. */
	addSite(page: string, ...feeds: FakeSiteFeed[]) {
		this.sites.set(page, feeds);
		for (const feed of feeds) this.sites.set(feed.url, [feed]);
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
				updated_what: null,
				version: 1,
				html: `<p>Text ${seq} of feed ${feedId}.</p>`,
				tags: ['news'],
				read: false,
				starred: false,
				hidden: false,
				versions: [],
				labels: [],
				enclosures: [],
				image_url: null,
				image_title: null,
				text_dropped: false,
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

	/** The next `times` requests of this kind are refused with `status`, and why. */
	refuse(
		method: string,
		path: RegExp,
		status: number,
		times = 1,
		reason = 'refused',
		headers: Record<string, string> = {}
	) {
		this.refusals.push({ method, path, status, times, reason, headers });
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

	// Live updates

	/** Sends an event to every connection that is open; `raw` as it is, for nonsense. */
	emit(event: ServerEvent | { event: string; raw: string }) {
		const data = 'raw' in event ? event.raw : JSON.stringify(event.data);
		for (const { stream } of this.streams) {
			const listeners = (stream as unknown as { listeners: Map<string, Listener[]> }).listeners;
			for (const listener of listeners.get(event.event) ?? []) {
				listener(new MessageEvent(event.event, { data }));
			}
		}
	}

	/** Ends every connection for good, as a server that refuses it does. */
	dropStreams() {
		for (const { stream } of [...this.streams]) {
			stream.close();
			stream.onerror?.(new Event('error'));
		}
	}

	// Answering

	fetch = async (input: RequestInfo | URL, init: RequestInit = {}): Promise<Response> => {
		const url = new URL(String(input), 'https://reader.example');
		const method = init.method ?? 'GET';
		const path = url.pathname.replace(/^\/api/, '') + url.search;
		const body: unknown =
			typeof init.body === 'string'
				? JSON.parse(init.body)
				: init.body instanceof Blob
					? await init.body.text()
					: undefined;
		const headers = new Headers(init.headers);
		const type = headers.get('Content-Type');
		this.requests.push({ method, path, body, type, tab: headers.get('X-Rueder-Tab') });
		if (this.down) throw new TypeError('fetch failed');
		for (const held of this.held) if (held.path.test(path)) await held.wait;
		const refusal = this.refusals.find((r) => r.method === method && r.path.test(path));
		if (refusal) {
			if (--refusal.times === 0) this.refusals = this.refusals.filter((r) => r !== refusal);
			return refused(refusal.status, refusal.reason, refusal.headers);
		}
		return this.answer(method, url.pathname.replace(/^\/api/, ''), url.searchParams, body, type);
	};

	private answer(
		method: string,
		path: string,
		query: URLSearchParams,
		body: unknown,
		type: string | null
	): Response {
		const data = (body ?? {}) as Record<string, unknown>;
		if (method === 'GET' && path === '/login/ways') {
			return json({
				oidc: this.oidc,
				registration: this.registrationOpen && !this.noRoom,
				passkeys: this.passkeysOn,
				legal: this.legal
			});
		}
		if (method === 'POST' && path === '/login/passkey') {
			if (!this.passkeysOn) return refused(404);
			return json(this.challenge('login', 'get'));
		}
		if (method === 'POST' && path === '/login/passkey/finish') {
			if (!this.passkeysOn) return refused(404);
			const key = this.answered(data, 'login');
			if (!key) return refused(401, 'the passkey is wrong');
			if (this.disabled.has(key.owner)) return refused(403, 'disabled');
			key.used_at = new Date().toISOString();
			this.signedIn = key.owner;
			return json(this.me());
		}
		if (method === 'GET' && path === '/register') {
			const token = query.get('invite');
			const open = this.registrationOpen && !this.noRoom;
			return json(token === null ? { open } : { open, invite: !!this.usable(token) });
		}
		if (method === 'POST' && path === '/register') {
			const invite = typeof data.invite === 'string' ? this.usable(data.invite) : undefined;
			const terms = this.legal.terms_version;
			if (terms && data.accepted_terms !== terms) return refused(403, 'terms');
			if (typeof data.invite === 'string' && !invite) return refused(403, 'invite');
			if (!invite && !this.registrationOpen) return refused(403, 'closed');
			if (this.noRoom) return refused(403, this.noRoom);
			const name = String(data.username).trim();
			if (!name) return refused(400, 'the name is empty');
			if (String(data.password).length < 8) {
				return refused(400, 'the password is shorter than 8 characters');
			}
			if (this.accounts.has(name.toLowerCase())) {
				return refused(409, 'there is an account with this name already');
			}
			this.accounts.set(name.toLowerCase(), String(data.password));
			if (invite) invite.uses++;
			this.signedIn = name.toLowerCase();
			if (terms) this.agreedTerms = terms;
			this.admin = false;
			return json(this.me(), 201);
		}
		if (method === 'POST' && path === '/login') {
			if (this.lockedFor > 0)
				return refused(429, 'too many', { 'Retry-After': `${this.lockedFor}` });
			const name = String(data.username).trim().toLowerCase();
			if (this.accounts.get(name) !== data.password) return refused(401, 'wrong name or password');
			if (this.disabled.has(name)) return refused(403, 'disabled');
			if (this.secondFactor) {
				const code = typeof data.code === 'string' ? data.code.trim().toUpperCase() : '';
				if (!code) return refused(401, 'second factor needed');
				const recovery = this.secondFactor.recovery.indexOf(code);
				if (recovery !== -1) this.secondFactor.recovery.splice(recovery, 1);
				else if (code !== this.secondFactor.code) return refused(401, 'wrong code');
			}
			this.signedIn = name;
			return json(this.me());
		}
		if (method === 'POST' && path === '/logout') {
			this.signedIn = null;
			return empty();
		}
		if (!this.signedIn) return refused(401);
		const invites = this.answerInvites(method, path, data);
		if (invites) return invites;
		if (method === 'GET' && path === '/me') return json(this.me());
		if (method === 'DELETE' && path === '/account') {
			const name = this.signedIn;
			const password = this.accounts.get(name);
			if (data.passkey !== undefined) {
				if (!this.answered(data.passkey, 'proof', name))
					return refused(403, 'the passkey is wrong');
				this.accounts.delete(name);
				this.signedIn = null;
				return empty();
			}
			// An account that logs in only through single sign-on sends its name.
			const proven =
				password === undefined
					? String(data.name ?? '')
							.trim()
							.toLowerCase() === name
					: data.password === password;
			if (!proven) return refused(403, 'the password is wrong');
			if (this.secondFactor) {
				const code = typeof data.code === 'string' ? data.code.trim().toUpperCase() : '';
				if (!code) return refused(403, 'second factor needed');
				if (code !== this.secondFactor.code && !this.secondFactor.recovery.includes(code)) {
					return refused(403, 'wrong code');
				}
			}
			this.accounts.delete(name);
			this.signedIn = null;
			return empty();
		}
		if (method === 'PUT' && path === '/password') {
			const name = this.signedIn;
			const password = this.accounts.get(name);
			// An account that logs in only through single sign-on has none to give.
			if (password !== undefined && data.current !== password) {
				return refused(403, 'the current password is wrong');
			}
			const fresh = String(data.new ?? '');
			if (fresh.length < 8) return refused(400, 'the password is shorter than 8 characters');
			this.accounts.set(name, fresh);
			return empty();
		}
		if (method === 'PUT' && path === '/terms') {
			if (!this.legal.terms_version) return refused(404);
			if (data.version !== this.legal.terms_version) return refused(409, 'not these terms');
			this.agreedTerms = this.legal.terms_version;
			return empty();
		}
		const secondFactor = this.answerSecondFactor(method, path, data);
		if (secondFactor) return secondFactor;
		const apps = this.answerAppPasswords(method, path, data);
		if (apps) return apps;
		const passkeys = this.answerPasskeys(method, path, data);
		if (passkeys) return passkeys;
		if (method === 'GET' && path === '/tree') return json(this.tree());
		if (path === '/reports/consent') {
			if (method === 'GET')
				return json({ available: this.reportsAvailable, on: this.sendsReports });
			if (!this.reportsAvailable && data.on) return refused(404, 'this server takes no reports');
			this.sendsReports = Boolean(data.on);
			if (!this.sendsReports) this.reports = [];
			return empty();
		}
		if (method === 'POST' && path === '/reports') {
			if (!this.sendsReports) return refused(403, 'this account does not send reports');
			this.reports.push(...((data.reports as unknown[]) ?? []));
			return empty();
		}
		const reported = /^\/feeds\/(\d+)(?:\/articles\/(\d+))?\/report$/.exec(path);
		if (method === 'POST' && reported) {
			const feedId = Number(reported[1]);
			const seq = reported[2] === undefined ? null : Number(reported[2]);
			const known =
				this.contentReports &&
				this.feeds.some((feed) => feed.id === feedId) &&
				(seq === null || this.article(feedId, seq) !== undefined);
			if (!known) return refused(404, 'not found');
			this.contentReported = this.contentReported.filter(
				(r) => r.feed_id !== feedId || r.seq !== seq
			);
			this.contentReported.push({ feed_id: feedId, seq, reason: data.reason, note: data.note });
			return empty();
		}
		if (method === 'GET' && path === '/status') {
			const hours = Number(query.get('hours') ?? 1);
			return this.admin
				? json(demoStatus(Date.now(), hours))
				: refused(403, 'only an admin can do this');
		}

		const labelled = this.labelling(method, path, query, body);
		if (labelled) return labelled;

		if (method === 'GET' && path === '/search') return this.search(query);

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
				const hiding = this.rules.find(
					(rule) => rule.enabled && rule.action === 'hide' && this.catches(rule, article)
				);
				return json({
					...opened,
					hidden_by: article.hidden ? (hiding?.id ?? null) : null
				});
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
			const age = typeof data.older_than === 'number' ? data.older_than : null;
			if (age !== null && age <= 0) return refused(400, 'older_than is out of range');
			const before = age === null ? Infinity : this.now() - age * 1000;
			for (const article of this.articles) {
				if (!feeds.has(article.feed_id) || article.id > data.upto_id) continue;
				if (Date.parse(article.published_at) <= before) article.read = true;
			}
			return empty();
		}

		if (method === 'GET' && path === '/settings') return json(this.settings);
		if (method === 'PUT' && path === '/settings') {
			this.settings = body;
			return empty();
		}
		if (method === 'GET' && path === '/rules') return json(this.rules);
		if (method === 'GET' && path === '/rules/hits') {
			const hits = this.rules.map((rule) => ({
				id: rule.id,
				matches: this.articles.filter((article) => this.catches(rule, article)).length,
				of: this.articles.filter((article) => this.inScope(rule, article.feed_id)).length
			}));
			return json({ hits });
		}
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
			const parent = typeof data.parent_id === 'number' ? data.parent_id : null;
			if (parent !== null && !this.folders.some((f) => f.id === parent)) return refused(404);
			return json({ id: this.addFolder(title, parent) }, 201);
		}
		const subscribing = this.subscribing(method, path, data, body, type);
		if (subscribing) return subscribing;

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

	/** The invitation of this token, if it is good for one more account. */
	private usable(token: string) {
		return this.invites.find(
			(invite) =>
				invite.token === token &&
				!invite.revoked &&
				invite.uses < invite.max_uses &&
				Date.parse(invite.expires_at) > Date.now()
		);
	}

	/** For how many more accounts a reader can invite; an admin has no quota. */
	private invitesLeft() {
		const taken = this.invites.reduce(
			(sum, invite) =>
				sum +
				(invite.revoked || Date.parse(invite.expires_at) <= Date.now()
					? invite.uses
					: invite.max_uses),
			0
		);
		return Math.max(0, this.inviteQuota - taken);
	}

	private answerInvites(method: string, path: string, data: Record<string, unknown>) {
		if (method === 'GET' && path === '/invites') {
			return json({
				invites: this.invites.map(({ token: _token, ...invite }) => ({
					...invite,
					usable: !!this.usable(_token)
				})),
				...(this.admin ? {} : { left: this.invitesLeft() }),
				days: this.inviteDays
			});
		}
		if (method === 'POST' && path === '/invites') {
			const uses = typeof data.uses === 'number' ? data.uses : 1;
			if (uses < 1 || uses > 1000) return refused(400, 'a link is for 1 to 1000 accounts');
			if (!this.admin && this.invitesLeft() < uses) return refused(403, 'no invitations left');
			const id = this.nextId++;
			const made = {
				id,
				token: `token-${id}`,
				max_uses: uses,
				uses: 0,
				created_at: new Date().toISOString(),
				expires_at: new Date(Date.now() + this.inviteDays * 86_400_000).toISOString(),
				revoked: false
			};
			this.invites.unshift(made);
			return json({ id, token: made.token, expires_at: made.expires_at }, 201);
		}
		const one = /^\/invites\/(\d+)$/.exec(path);
		const invite = this.invites.find((invite) => invite.id === Number(one?.[1]));
		if (method === 'DELETE' && one) {
			if (!invite) return refused(404);
			invite.revoked = true;
			return empty();
		}
		return null;
	}

	private me() {
		return {
			id: 1,
			username: this.signedIn,
			is_admin: this.admin,
			has_password: this.accounts.has(this.signedIn ?? ''),
			admin_needs_second_factor: this.adminNeedsSecondFactor,
			content_reports: this.contentReports,
			app_api: this.appApi,
			passkeys: this.passkeysOn,
			terms_to_accept:
				this.legal.terms_version && this.agreedTerms !== this.legal.terms_version
					? this.legal.terms_version
					: null
		};
	}

	/** An app logs in with an app password for the first time. */
	appLogsIn(id: number) {
		this.appPasswords = this.appPasswords.map((app) =>
			app.id === id ? { ...app, used_at: new Date().toISOString() } : app
		);
		this.emit({ event: 'apps', data: { by: null } });
	}

	private answerAppPasswords(method: string, path: string, data: Record<string, unknown>) {
		if (!path.startsWith('/app-passwords')) return null;
		if (!this.appApi) return refused(404);
		if (method === 'GET' && path === '/app-passwords') {
			return json({ app_passwords: this.appPasswords, max: 20 });
		}
		if (method === 'POST' && path === '/app-passwords') {
			const refusal = this.proven(data);
			if (refusal) return refusal;
			if (this.appPasswords.length >= 20) return refused(400, 'at most 20');
			const made = {
				id: this.appPasswords.length + 1,
				name: String(data.name ?? '').trim(),
				created_at: new Date().toISOString(),
				used_at: null
			};
			this.appPasswords = [...this.appPasswords, made];
			return json({ ...made, secret: this.nextSecret }, 201);
		}
		const one = /^\/app-passwords\/(\d+)$/.exec(path);
		if (method === 'DELETE' && one) {
			const id = Number(one[1]);
			if (!this.appPasswords.some((app) => app.id === id)) return refused(404);
			this.appPasswords = this.appPasswords.filter((app) => app.id !== id);
			return empty();
		}
		return refused(404);
	}

	private answerSecondFactor(method: string, path: string, data: Record<string, unknown>) {
		if (!path.startsWith('/second-factor')) return null;
		const passwordRight = () => this.accounts.get(this.signedIn ?? '') === (data.password ?? '');
		if (method === 'GET' && path === '/second-factor') {
			const keys = this.ownPasskeys().length;
			return json({
				on: this.secondFactor !== null || keys > 0,
				app: this.secondFactor !== null,
				passkeys: keys,
				recovery_codes_left: this.secondFactor?.recovery.length ?? this.recoveryOfPasskeys.length,
				required_for_admins: this.adminNeedsSecondFactor
			});
		}
		if (method === 'POST' && path === '/second-factor/setup') {
			const refusal = this.proven(data);
			if (refusal) return refusal;
			this.pendingSecondFactor = true;
			return json({
				secret: 'JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP',
				uri: 'otpauth://totp/rueder%3Afiona?secret=JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP',
				qr_svg: '<svg xmlns="http://www.w3.org/2000/svg"/>'
			});
		}
		if (method === 'POST' && path === '/second-factor/confirm') {
			if (!this.pendingSecondFactor) return refused(409, 'set it up first');
			if (data.code !== this.appCode) return refused(403, 'the code is wrong');
			this.pendingSecondFactor = false;
			this.secondFactor = { code: this.appCode, recovery: [...this.recoveryCodes] };
			this.adminNeedsSecondFactor = false;
			return json({ codes: [...this.recoveryCodes] });
		}
		if (method === 'DELETE' && path === '/second-factor') {
			if (data.passkey === undefined) {
				if (!passwordRight()) return refused(403, 'the password is wrong');
				const code = String(data.code ?? '');
				const known = this.secondFactor;
				if (known && code !== known.code && !known.recovery.includes(code))
					return refused(403, 'the code is wrong');
			} else if (!this.answered(data.passkey, 'proof', this.signedIn ?? '')) {
				return refused(403, 'the passkey is wrong');
			}
			if (this.ownPasskeys().length > 0)
				this.recoveryOfPasskeys = this.secondFactor?.recovery ?? [];
			this.secondFactor = null;
			return empty();
		}
		return null;
	}

	private ownPasskeys() {
		return this.passkeys.filter((key) => key.owner === this.signedIn);
	}

	/** A ceremony of a kind, with what the device is asked, as the server writes it. */
	private challenge(kind: string, ask: 'create' | 'get') {
		const ceremony = `${kind}-${++this.ceremonies}`;
		const publicKey =
			ask === 'create'
				? {
						rp: { id: 'localhost', name: 'rueder' },
						user: { id: 'dXNlcg', name: this.signedIn, displayName: this.signedIn },
						challenge: 'Y2hhbGxlbmdl',
						pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
						timeout: 300000,
						excludeCredentials: this.ownPasskeys().map((key) => ({
							type: 'public-key',
							id: key.key
						})),
						authenticatorSelection: { residentKey: 'required', userVerification: 'required' },
						attestation: 'none'
					}
				: {
						challenge: 'Y2hhbGxlbmdl',
						rpId: 'localhost',
						timeout: 300000,
						userVerification: 'required',
						allowCredentials:
							kind === 'proof'
								? this.ownPasskeys().map((key) => ({ type: 'public-key', id: key.key }))
								: []
					};
		return { ceremony, options: { publicKey } };
	}

	/** The passkey a device's answer to a ceremony of this kind is, if it is one of `owner`'s. */
	private answered(answer: unknown, kind: string, owner?: string) {
		const { ceremony, credential } = (answer ?? {}) as {
			ceremony?: string;
			credential?: { id?: string };
		};
		if (!ceremony?.startsWith(`${kind}-`)) return undefined;
		return this.passkeys.find(
			(key) => key.key === credential?.id && (owner === undefined || key.owner === owner)
		);
	}

	/** Whether a change that asks for it is shown to come from the owner; else the refusal. */
	private proven(data: Record<string, unknown>) {
		if (data.passkey !== undefined) {
			return this.answered(data.passkey, 'proof', this.signedIn ?? '')
				? null
				: refused(403, 'the passkey is wrong');
		}
		if (this.accounts.get(this.signedIn ?? '') !== (data.password ?? '')) {
			return refused(403, 'the password is wrong');
		}
		if (this.secondFactor || this.ownPasskeys().length > 0) {
			const code = String(data.code ?? '')
				.trim()
				.toUpperCase();
			if (!code) return refused(403, 'second factor needed');
			const codes = [...(this.secondFactor?.recovery ?? []), ...this.recoveryOfPasskeys];
			if (code !== this.secondFactor?.code && !codes.includes(code)) {
				return refused(403, 'the code is wrong');
			}
		}
		return null;
	}

	private answerPasskeys(method: string, path: string, data: Record<string, unknown>) {
		if (!path.startsWith('/passkeys')) return null;
		if (!this.passkeysOn) return refused(404);
		const listed = (key: (typeof this.passkeys)[number]) => ({
			id: key.id,
			name: key.name,
			created_at: key.created_at,
			used_at: key.used_at
		});
		if (method === 'GET' && path === '/passkeys') {
			return json({ passkeys: this.ownPasskeys().map(listed), max: 10 });
		}
		if (method === 'POST' && path === '/passkeys/register') {
			const refusal = this.proven(data);
			if (refusal) return refusal;
			if (this.ownPasskeys().length >= 10)
				return refused(400, 'an account has at most 10 passkeys');
			this.pendingPasskey = String(data.name ?? '').trim();
			return json(this.challenge('register', 'create'));
		}
		if (method === 'POST' && path === '/passkeys/proof') {
			if (this.ownPasskeys().length === 0) return refused(409, 'the account has no passkey');
			return json(this.challenge('proof', 'get'));
		}
		if (method === 'POST' && path === '/passkeys') {
			const { ceremony, credential } = data as { ceremony?: string; credential?: { id?: string } };
			if (!ceremony?.startsWith('register-') || this.pendingPasskey === null) {
				return refused(409, 'the passkey ceremony is over, start again');
			}
			const key = String(credential?.id ?? '');
			if (this.passkeys.some((known) => known.key === key)) {
				return refused(409, 'this passkey is known already');
			}
			const first = this.ownPasskeys().length === 0 && this.secondFactor === null;
			const made = {
				id: this.passkeys.length + 1,
				owner: this.signedIn ?? '',
				key,
				name: this.pendingPasskey,
				created_at: new Date().toISOString(),
				used_at: null
			};
			this.pendingPasskey = null;
			this.passkeys = [...this.passkeys, made];
			if (first) this.recoveryOfPasskeys = [...this.recoveryCodes];
			return json(
				{ passkey: listed(made), ...(first ? { recovery_codes: [...this.recoveryCodes] } : {}) },
				201
			);
		}
		const one = /^\/passkeys\/(\d+)$/.exec(path);
		if (method === 'DELETE' && one) {
			const refusal = this.proven(data);
			if (refusal) return refusal;
			const id = Number(one[1]);
			if (!this.ownPasskeys().some((key) => key.id === id)) return refused(404);
			this.passkeys = this.passkeys.filter((key) => key.id !== id);
			if (this.ownPasskeys().length === 0 && !this.secondFactor) this.recoveryOfPasskeys = [];
			return empty();
		}
		return refused(404);
	}

	private pendingPasskey: string | null = null;

	private tree(): { folders: ApiFolder[]; feeds: ApiFeed[]; hidden: number } {
		return {
			folders: this.folders,
			hidden: this.articles.filter((article) => article.hidden).length,
			feeds: this.feeds.map(({ name, url, ...feed }) => ({
				...feed,
				title: name ?? feed.title,
				feed_title: feed.title,
				feed_url: url,
				site_url: `https://feed-${feed.id}.example`,
				fetch_agent: 'rueder',
				last_seq: this.articles.filter((a) => a.feed_id === feed.id).length,
				read_upto: 0,
				unread: feed.blocked ? 0 : this.unread(feed.id),
				blocked: feed.blocked ?? false
			}))
		};
	}

	// Labels

	addLabel(title: string, color = '#1a73e8') {
		const id = this.nextId++;
		this.labels.push({ id, title, color, position: this.labels.length + 1 });
		return id;
	}

	private labelling(
		method: string,
		path: string,
		query: URLSearchParams,
		body: unknown
	): Response | null {
		const data = (body ?? {}) as Record<string, unknown>;
		if (method === 'GET' && path === '/labels') return json(this.labels);
		if (method === 'POST' && path === '/labels') {
			const title = String(data.title ?? '').trim();
			if (!title || !/^#[0-9a-f]{6}$/i.test(String(data.color))) return refused(400, 'no label');
			return json({ id: this.addLabel(title, String(data.color)) }, 201);
		}
		const one = /^\/labels\/(\d+)(\/articles)?$/.exec(path);
		if (one) {
			const label = this.labels.find((l) => l.id === Number(one[1]));
			if (!label) return refused(404);
			if (method === 'GET' && one[2]) {
				const all = new Set(this.feeds.map((feed) => feed.id));
				return json({ articles: this.list('label', all, query, label.id) });
			}
			if (method === 'PATCH') {
				if (typeof data.title === 'string') label.title = data.title;
				if (typeof data.color === 'string') label.color = data.color;
				return empty();
			}
			if (method === 'DELETE') {
				this.labels = this.labels.filter((l) => l !== label);
				for (const article of this.articles) {
					article.labels = article.labels.filter((id) => id !== label.id);
				}
				return empty();
			}
		}
		const on = /^\/feeds\/(\d+)\/articles\/(\d+)\/labels\/(\d+)$/.exec(path);
		if (on) {
			const article = this.article(Number(on[1]), Number(on[2]));
			const label = Number(on[3]);
			if (!article || !this.labels.some((l) => l.id === label)) return refused(404);
			article.labels = article.labels.filter((id) => id !== label);
			if (method === 'PUT') article.labels.push(label);
			return empty();
		}
		return null;
	}

	// Subscribing and OPML

	private subscribing(
		method: string,
		path: string,
		data: Record<string, unknown>,
		body: unknown,
		type: string | null
	): Response | null {
		if (method === 'POST' && path === '/feeds/find') {
			const asked = this.address(String(data.url));
			const moved = this.moves.get(asked);
			const found = this.sites.get(moved ?? asked);
			if (!found) return refused(400, 'the address cannot be fetched: no such site');
			const feeds = found.map((feed) => ({
				...feed,
				feed_id: this.feeds.find((known) => known.url === feed.url)?.id ?? null
			}));
			return json({ feeds, moved_to: moved ?? null });
		}
		if (method === 'POST' && path === '/feeds') {
			const url = this.address(String(data.url));
			const folder = data.folder_id ?? null;
			if (folder !== null && !this.folders.some((f) => f.id === folder)) return refused(404);
			const known = this.feeds.find((feed) => feed.url === url);
			if (known) return json({ id: known.id });
			const feed = this.sites.get(url)?.find((found) => found.url === url);
			if (!feed) return refused(400, 'this is not a feed that can be read: it is a web page');
			return json({ id: this.addFeed(feed.title, folder as number | null, { url }) }, 201);
		}
		const feed = /^\/feeds\/(\d+)$/.exec(path);
		if (feed) {
			const found = this.feeds.find((f) => f.id === Number(feed[1]));
			if (!found) return refused(404);
			if (method === 'PATCH') {
				found.name = String(data.title ?? '').trim() || null;
				return empty();
			}
			if (method === 'DELETE') {
				const level = this.level(found.folder_id).filter((row) => row !== found);
				this.feeds = this.feeds.filter((f) => f !== found);
				this.articles = this.articles.filter((a) => a.feed_id !== found.id);
				this.renumber(level);
				return empty();
			}
		}
		if (method === 'GET' && path === '/opml') {
			const outlines = this.feeds
				.map((f) => `    <outline type="rss" text="${f.name ?? f.title}" xmlUrl="${f.url}"/>`)
				.join('\n');
			return text(
				`<?xml version="1.0" encoding="UTF-8"?>\n<opml version="2.0">\n  <body>\n${outlines}\n  </body>\n</opml>\n`,
				'text/x-opml; charset=utf-8'
			);
		}
		if (method === 'GET' && path === '/export') {
			const tree = this.feeds.map((f) => ({ kind: 'feed', id: f.id, url: f.url, title: f.title }));
			const file = { format: 'rueder-account', version: 1, settings: this.settings, tree };
			return text(JSON.stringify(file), 'application/json');
		}
		if (method === 'POST' && path === '/import') {
			if (type !== 'application/json')
				return refused(415, 'the file of an account comes as application/json');
			let file: { format?: string; settings?: unknown; tree?: { url?: string; title?: string }[] };
			try {
				file = JSON.parse(String(body));
			} catch {
				return refused(400, 'this is not a file of a rueder account');
			}
			if (file.format !== 'rueder-account')
				return refused(400, 'this is not a file of a rueder account');
			const moved = {
				...{ feeds: 0, known: 0, folders: 0, articles: 0, missing: 0 },
				...{ labels: 0, rules: 0, skipped: 0, settings: false }
			};
			for (const { url = '', title = '' } of file.tree ?? []) {
				if (this.feeds.some((f) => f.url === url)) moved.known++;
				else {
					this.addFeed(title, null, { url, fetched: false });
					moved.feeds++;
				}
			}
			if (file.settings && Object.keys(file.settings).length > 0) {
				this.settings = file.settings;
				moved.settings = true;
			}
			return json(moved);
		}
		if (method === 'POST' && path === '/opml') {
			if (type !== 'text/x-opml') return refused(415, 'an OPML file comes as text/x-opml or XML');
			const file = String(body);
			if (!file.includes('<opml')) return refused(400, 'this is not an OPML file');
			const imported = { feeds: 0, folders: 0, known: 0, skipped: 0 };
			for (const [, title, url] of file.matchAll(/<outline text="([^"]*)" xmlUrl="([^"]*)"/g)) {
				if (!/^https?:\/\//.test(url)) imported.skipped++;
				else if (this.feeds.some((f) => f.url === url)) imported.known++;
				else {
					this.addFeed(title, null, { url, fetched: false });
					imported.feeds++;
				}
			}
			return json(imported);
		}
		return null;
	}

	/** An address as the server takes it: without a scheme, it is https. */
	private address = (text: string) =>
		(/^\w+:\/\//.test(text.trim()) ? '' : 'https://') + text.trim();

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

	private list(kind: string, feeds: Set<number>, query: URLSearchParams, label = 0): ApiListItem[] {
		const before = Number(query.get('before') ?? Infinity);
		// With `after` the list starts with the oldest article.
		const after = query.has('after') ? Number(query.get('after')) : null;
		const limit = Number(query.get('limit') ?? 50);
		const unread = query.get('unread') === 'true';
		// Nobody sees the articles of a feed the operator blocked.
		const blocked = new Set(this.feeds.filter((feed) => feed.blocked).map((feed) => feed.id));
		return this.articles
			.filter(
				(a) =>
					feeds.has(a.feed_id) &&
					!blocked.has(a.feed_id) &&
					a.id < before &&
					a.id > (after ?? 0) &&
					a.hidden === (kind === 'hidden') &&
					(kind !== 'starred' || a.starred) &&
					(kind !== 'label' || a.labels.includes(label)) &&
					!(unread && a.read)
			)
			.sort((a, b) => (after === null ? b.id - a.id : a.id - b.id))
			.slice(0, limit)
			.map(listed);
	}

	/** Like the server: all the words, from three letters on also as a beginning. */
	private search(query: URLSearchParams): Response {
		const split = (text: string) =>
			text
				.toLowerCase()
				.split(/[^\p{L}\p{N}]+/u)
				.filter(Boolean);
		const words = split(query.get('q') ?? '');
		if (words.length === 0) return refused(400, 'there is no word to search for');
		const feed = query.get('feed');
		const folder = query.get('folder');
		const feeds = feed
			? this.scope('feeds', Number(feed))
			: folder
				? this.scope('folders', Number(folder))
				: this.scope('articles', 0);
		if (!feeds) return refused(404);
		const newest = Math.max(0, ...this.articles.map((a) => a.id)) + 1;
		const before = Math.min(Number(query.get('before') ?? newest), newest);
		const from = this.searchWindow ? before - this.searchWindow : 0;
		const limit = Number(query.get('limit') ?? 50);
		const unread = query.get('unread') === 'true';
		const hits = this.articles
			.filter((a) => {
				if (!feeds.has(a.feed_id) || a.hidden || (unread && a.read)) return false;
				if (a.id >= before || a.id < from) return false;
				const has = split([a.title, a.author, ...a.tags, a.html].join(' '));
				return words.every((word) =>
					has.some((one) => one === word || (word.length >= 3 && one.startsWith(word)))
				);
			})
			.sort((a, b) => b.id - a.id)
			.slice(0, limit);
		const next = hits.length === limit ? hits[limit - 1].id : from > 1 ? from : null;
		return json({ articles: hits.map(listed), next });
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
			{
				...article,
				id: '',
				feedId: '',
				link: '',
				publishedAt: 0,
				enclosures: [],
				image: null,
				changes: 0,
				updatedAt: null
			}
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

/** An article as a line of a list. */
function listed(a: FakeArticle): ApiListItem {
	return {
		feed_id: a.feed_id,
		seq: a.seq,
		id: a.id,
		published_at: a.published_at,
		updated_at: a.updated_at,
		updated_what: a.updated_what,
		version: a.version,
		title: a.title,
		teaser: a.teaser,
		author: a.author,
		url: a.url,
		thumbnail_url: a.thumbnail_url ?? null,
		read: a.read,
		starred: a.starred,
		labels: a.labels
	};
}
