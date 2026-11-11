// The server's API as functions: one per endpoint the page uses, with the shapes
// of what the server sends. The shapes are written by hand after the Rust types in
// server/src/ until they are generated from them (work package 2.8).
//
// The session is a cookie the browser keeps and scripts cannot read, so nothing
// here knows a token. Every address is rueder's own.

import type { Stream } from '#lib/model.ts';
import type { DropPosition } from '#lib/tree.ts';

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

export interface ApiAccount {
	id: number;
	username: string;
	is_admin: boolean;
}

/** The ways to log in this server has, besides name and password. */
export interface ApiLoginWays {
	oidc: boolean;
}

export interface ApiFolder {
	id: number;
	parent_id: number | null;
	title: string;
	position: number;
}

/** A subscription with what the tree shows of its feed. */
export interface ApiFeed {
	id: number;
	folder_id: number | null;
	position: number;
	/** The name the reader gave it, else the feed's own. */
	title: string;
	feed_title: string;
	feed_url: string;
	site_url: string | null;
	icon_url: string | null;
	fetch_agent: string;
	error: string | null;
	last_seq: number;
	read_upto: number;
	unread: number;
}

export interface ApiTree {
	folders: ApiFolder[];
	feeds: ApiFeed[];
}

/** One line of a list. */
export interface ApiListItem {
	feed_id: number;
	seq: number;
	/** Where the article stands in the order of arrival; the anchor for the next page. */
	id: number;
	published_at: string;
	updated_at: string | null;
	version: number;
	title: string;
	teaser: string;
	author: string;
	url: string | null;
	thumbnail_url: string | null;
	read: boolean;
	starred: boolean;
	labels: number[];
}

/** An article opened for reading. */
export interface ApiArticle {
	feed_id: number;
	seq: number;
	id: number;
	published_at: string;
	updated_at: string | null;
	version: number;
	title: string;
	author: string;
	url: string | null;
	html: string;
	tags: string[];
	enclosures: unknown;
	image_url: string | null;
	image_title: string | null;
	labels: number[];
}

export interface ApiVersion {
	version: number;
	fetched_at: string;
	title: string;
	html: string;
	authors: string[];
	tags: string[];
	url: string | null;
}

/** A folder or a subscription, as the tree's changes name them. */
export interface ApiNode {
	kind: 'folder' | 'feed';
	id: number;
}

/** Which part of a list is asked for. */
export interface PageQuery {
	/** Only what arrived before the article of this id. */
	before?: number;
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

async function call<T>(method: string, path: string, body?: unknown): Promise<T> {
	let response: Response;
	try {
		response = await fetch(`/api${path}`, {
			method,
			credentials: 'same-origin',
			headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
			body: body === undefined ? undefined : JSON.stringify(body)
		});
	} catch {
		throw new ApiError(0, 'the server did not answer');
	}
	if (!response.ok) {
		const reason = (await response.text().catch(() => '')).slice(0, MAX_REASON);
		const wait = Number(response.headers.get('Retry-After'));
		throw new ApiError(response.status, reason, wait > 0 ? wait : null);
	}
	if (response.status === 204) return undefined as T;
	try {
		return (await response.json()) as T;
	} catch {
		// Something else answered in the server's place, a proxy's error page for one.
		throw new ApiError(0, 'the answer was not what the server sends');
	}
}

export const api = {
	me: () => call<ApiAccount>('GET', '/me'),
	loginWays: () => call<ApiLoginWays>('GET', '/login/ways'),
	login: (username: string, password: string) =>
		call<ApiAccount>('POST', '/login', { username, password }),
	logout: () => call<void>('POST', '/logout'),

	tree: () => call<ApiTree>('GET', '/tree'),

	list: async (stream: ListStream, page: PageQuery) => {
		const query = new URLSearchParams({ limit: String(page.limit) });
		if (page.before !== undefined) query.set('before', String(page.before));
		if (page.unread) query.set('unread', 'true');
		const list = await call<{ articles: ApiListItem[] }>('GET', `${listPath(stream)}?${query}`);
		return list.articles;
	},

	article: (feedId: number, seq: number) =>
		call<ApiArticle>('GET', `/feeds/${feedId}/articles/${seq}`),

	versions: async (feedId: number, seq: number) => {
		const kept = await call<{ versions: ApiVersion[] }>(
			'GET',
			`/feeds/${feedId}/articles/${seq}/versions`
		);
		return kept.versions;
	},

	setState: (feedId: number, seq: number, change: { read?: boolean; starred?: boolean }) =>
		call<void>('PUT', `/feeds/${feedId}/articles/${seq}/state`, change),

	/** Marks as read what arrived in the stream up to the article of this id. */
	readUpto: (stream: ListStream, uptoId: number) =>
		call<void>('POST', readPath(stream), { upto_id: uptoId }),

	move: (node: ApiNode, target: ApiNode, position: DropPosition) =>
		call<void>('POST', '/tree/move', { node, target, position }),
	addFolder: (title: string) => call<{ id: number }>('POST', '/folders', { title }),
	renameFolder: (id: number, title: string) => call<void>('PATCH', `/folders/${id}`, { title }),
	removeFolder: (id: number) => call<void>('DELETE', `/folders/${id}`)
};

export type Api = typeof api;
