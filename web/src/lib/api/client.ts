// The server's API as functions: one per endpoint the page uses. The shapes of
// what the server sends and reads come from types.ts, which the server writes
// out of its Rust types (see server/src/typescript.rs).
//
// The session is a cookie the browser keeps and scripts cannot read, so nothing
// here knows a token. Every address is rueder's own.

import type { Stream } from '#lib/model.ts';
import type { DropPosition } from '#lib/tree.ts';
import type {
	Article,
	ArticleVersion,
	Created,
	Credentials,
	Feed,
	FindFeeds,
	Folder,
	FoundFeeds,
	Label,
	LabelChange,
	List,
	ListItem,
	LoginWays,
	Me,
	Move,
	NewFolder,
	NewLabel,
	NewSubscription,
	NodeRef,
	OpmlImported,
	Rename,
	Rule,
	RuleOutcome,
	RulePreview,
	RuleScope,
	RulesHits,
	StateChange,
	Subscribed,
	Tree,
	Upto,
	Versions
} from './types.ts';

/** A request the server refused, or one it did not answer (`status` 0). */
export class ApiError extends Error {
	constructor(
		readonly status: number,
		message: string,
		/** Seconds to wait, if the server said so (429). */
		readonly retryAfter: number | null = null
	) {
		super(message);
		this.name = 'ApiError';
	}

	/** The session is over, or there never was one. */
	get signedOut() {
		return this.status === 401;
	}
}

// The shapes of what the server sends, under the names the page knows them by.
// They are the server's own: types.ts is written out of its Rust types.
export type ApiAccount = Me;
export type ApiLoginWays = LoginWays;
export type ApiFolder = Folder;
export type ApiFeed = Feed;
export type ApiTree = Tree;
export type ApiListItem = ListItem;
export type ApiArticle = Article;
export type ApiVersion = ArticleVersion;
export type ApiRuleScope = RuleScope;
export type ApiRule = Rule;
/** A new rule is sent without its id. */
export type ApiNewRule = Omit<Rule, 'id'>;
export type ApiRulePreview = RulePreview;
export type ApiNode = NodeRef;
export type ApiLabel = Label;

/** Which part of a list is asked for. */
export interface PageQuery {
	/** Only what arrived before the article of this id. */
	before?: number;
	/** Only what arrived after the article of this id, the oldest first; 0 starts at the oldest. */
	after?: number;
	limit: number;
	/** Leave out what was read. */
	unread?: boolean;
}

/** The lists of articles: a feed's, a folder's, everything, the starred and the hidden. */
export type ListStream = Exclude<Stream, { kind: 'home' }>;

/** The number in an id of the page: 12 for "feed-12", which is feed 12 of the server. */
export const serverId = (id: string) => Number(id.slice(id.lastIndexOf('-') + 1));

const listPath = (stream: ListStream) => {
	switch (stream.kind) {
		case 'all':
			return '/articles';
		case 'starred':
			return '/starred';
		case 'hidden':
			return '/hidden';
		case 'feed':
			return `/feeds/${serverId(stream.id)}/articles`;
		case 'folder':
			return `/folders/${serverId(stream.id)}/articles`;
		case 'label':
			return `/labels/${serverId(stream.id)}/articles`;
	}
};

/** Marking as read goes by feed, by folder or for everything. */
const readPath = (stream: ListStream) => {
	switch (stream.kind) {
		case 'feed':
			return `/feeds/${serverId(stream.id)}/read`;
		case 'folder':
			return `/folders/${serverId(stream.id)}/read`;
		default:
			return '/read';
	}
};

/** How much of a refusal's text is kept. */
const MAX_REASON = 300;

/** An OPML file as the server takes it. */
const OPML = 'text/x-opml';

/**
 * This tab's name, which every request carries in `X-Rueder-Tab`. The server
 * names it again in the event a change brings (`by`), so the tab knows its own
 * changes when they come back.
 */
export const TAB = Array.from(crypto.getRandomValues(new Uint8Array(8)), (byte) =>
	byte.toString(16).padStart(2, '0')
).join('');

/** What the page needs of an `EventSource`; the tests put a made-up one in its place. */
export interface EventStream {
	addEventListener(type: string, listener: (event: MessageEvent<string>) => void): void;
	close(): void;
	readonly readyState: number;
	onerror: ((event: Event) => void) | null;
}

/** The `readyState` of a stream that will not come back by itself. */
export const CLOSED = 2;

/**
 * Sends a request and gives back the answer, if the server took it. A body is
 * sent as JSON, a `Blob` as it is, as an OPML file.
 */
async function send(method: string, path: string, body?: unknown): Promise<Response> {
	const file = body instanceof Blob;
	let response: Response;
	try {
		response = await fetch(`/api${path}`, {
			method,
			credentials: 'same-origin',
			headers: {
				'X-Rueder-Tab': TAB,
				...(body === undefined ? {} : { 'Content-Type': file ? OPML : 'application/json' })
			},
			body: body === undefined ? undefined : file ? body : JSON.stringify(body)
		});
	} catch {
		throw new ApiError(0, 'the server did not answer');
	}
	if (!response.ok) {
		const reason = (await response.text().catch(() => '')).slice(0, MAX_REASON);
		const wait = Number(response.headers.get('Retry-After'));
		throw new ApiError(response.status, reason, wait > 0 ? wait : null);
	}
	return response;
}

async function call<T>(method: string, path: string, body?: unknown): Promise<T> {
	const response = await send(method, path, body);
	if (response.status === 204) return undefined as T;
	try {
		return (await response.json()) as T;
	} catch {
		// Something else answered in the server's place, a proxy's error page for one.
		throw new ApiError(0, 'the answer was not what the server sends');
	}
}

/** An answer that is a text, not JSON. */
async function text(method: string, path: string): Promise<string> {
	const response = await send(method, path);
	try {
		return await response.text();
	} catch {
		throw new ApiError(0, 'the answer was cut short');
	}
}

export const api = {
	/** This tab's name, as the requests carry it. */
	tab: TAB,
	/**
	 * The live updates: one connection that stays open and comes back by itself
	 * after a break, saying the last event it got. Null where the browser has none.
	 */
	events: (): EventStream | null =>
		typeof EventSource === 'undefined' ? null : new EventSource('/api/events'),

	me: () => call<ApiAccount>('GET', '/me'),
	loginWays: () => call<ApiLoginWays>('GET', '/login/ways'),
	login: (username: string, password: string) =>
		call<ApiAccount>('POST', '/login', { username, password } satisfies Credentials),
	logout: () => call<void>('POST', '/logout'),

	tree: () => call<ApiTree>('GET', '/tree'),

	list: async (stream: ListStream, page: PageQuery) => {
		const query = new URLSearchParams({ limit: String(page.limit) });
		if (page.before !== undefined) query.set('before', String(page.before));
		if (page.after !== undefined) query.set('after', String(page.after));
		if (page.unread) query.set('unread', 'true');
		const list = await call<List>('GET', `${listPath(stream)}?${query}`);
		return list.articles;
	},

	article: (feedId: number, seq: number) =>
		call<ApiArticle>('GET', `/feeds/${feedId}/articles/${seq}`),

	versions: async (feedId: number, seq: number) => {
		const kept = await call<Versions>('GET', `/feeds/${feedId}/articles/${seq}/versions`);
		return kept.versions;
	},

	setState: (feedId: number, seq: number, change: { read?: boolean; starred?: boolean }) =>
		call<void>('PUT', `/feeds/${feedId}/articles/${seq}/state`, change satisfies StateChange),

	/**
	 * Marks as read what arrived in the stream up to the article of this id; with
	 * `olderThan`, only what was published more than that many seconds ago.
	 */
	readUpto: (stream: ListStream, uptoId: number, olderThan?: number) =>
		call<void>(
			'POST',
			readPath(stream),
			(olderThan === undefined
				? { upto_id: uptoId }
				: { upto_id: uptoId, older_than: olderThan }) satisfies Upto
		),

	move: (node: ApiNode, target: ApiNode, position: DropPosition) =>
		call<void>('POST', '/tree/move', { node, target, position } satisfies Move),
	addFolder: (title: string, parentId: number | null = null) =>
		call<Created>('POST', '/folders', { title, parent_id: parentId } satisfies NewFolder),
	renameFolder: (id: number, title: string) =>
		call<void>('PATCH', `/folders/${id}`, { title } satisfies Rename),
	removeFolder: (id: number) => call<void>('DELETE', `/folders/${id}`),

	/** The feeds at an address: the feed itself, or those a web page names. */
	findFeeds: (url: string) => call<FoundFeeds>('POST', '/feeds/find', { url } satisfies FindFeeds),
	/** Subscribes to the feed at an address and gives back its number. */
	subscribe: async (url: string, folderId: number | null) => {
		const body: NewSubscription = { url, folder_id: folderId };
		return (await call<Subscribed>('POST', '/feeds', body)).id;
	},
	/** An empty title gives the feed its own name back. */
	renameFeed: (id: number, title: string) =>
		call<void>('PATCH', `/feeds/${id}`, { title } satisfies Rename),
	unsubscribe: (id: number) => call<void>('DELETE', `/feeds/${id}`),
	importOpml: (file: Blob) => call<OpmlImported>('POST', '/opml', file),
	exportOpml: () => text('GET', '/opml'),

	/** The settings as the account keeps them; what is in them is the page's business. */
	settings: () => call<unknown>('GET', '/settings'),
	saveSettings: (settings: unknown) => call<void>('PUT', '/settings', settings),

	labels: () => call<ApiLabel[]>('GET', '/labels'),
	addLabel: (title: string, color: string) =>
		call<Created>('POST', '/labels', { title, color } satisfies NewLabel),
	changeLabel: (id: number, change: LabelChange) => call<void>('PATCH', `/labels/${id}`, change),
	removeLabel: (id: number) => call<void>('DELETE', `/labels/${id}`),
	/** Puts a label on an article, or takes it off. */
	setLabel: (feedId: number, seq: number, labelId: number, on: boolean) =>
		call<void>(on ? 'PUT' : 'DELETE', `/feeds/${feedId}/articles/${seq}/labels/${labelId}`),

	rules: () => call<ApiRule[]>('GET', '/rules'),
	addRule: (rule: ApiNewRule) => call<RuleOutcome>('POST', '/rules', rule),
	changeRule: (id: number, rule: ApiNewRule) => call<RuleOutcome>('PUT', `/rules/${id}`, rule),
	removeRule: (id: number) => call<RuleOutcome>('DELETE', `/rules/${id}`),
	/** Tries a rule without storing it; a pattern that cannot be used is refused with 400. */
	previewRule: (rule: ApiNewRule) => call<ApiRulePreview>('POST', '/rules/preview', rule),
	/** What each of the stored rules finds. */
	ruleHits: async () => (await call<RulesHits>('GET', '/rules/hits')).hits
};

export type Api = typeof api;
