// Where the reader was: what the main pane showed, which article was open, and how
// far the sidebar, the list and the open article were scrolled. It is kept in the
// browser, so a page that is loaded anew, or an app the phone closed, starts where
// it was left. It holds ids and pixels only, never what an article says, and it
// belongs to one account: another one starts from the start page.
//
// The page notes what changes (`noteView`); the parts that scroll take their place
// once, when what they show is there again (`takeListSpot`, `takePaneSpot`).

import { sameStream, type Stream } from './model.ts';

/** Where the view is kept in the browser. */
export const VIEW_KEY = 'rueder.view';
/** The form of what is kept; another form is not read. */
const FORM = 1;
/** What is kept is small; anything longer is not read. */
const MAX_STORED = 4096;
/** How many articles below the top one the list names, for when the top one is gone. */
export const SPOT_IDS = 5;
const MAX_ID = 64;
/** Scrolled further than this is not a place anybody was. */
const MAX_PIXELS = 10_000_000;
/** What is noted is written once it rests this long, and when the page goes away. */
const WRITE_AFTER = 300;

/** Where the list was scrolled to. */
export interface ListSpot {
	/** The article at the top of the list, and the ones below it, in the list's order. */
	ids: string[];
	/** How far the list was scrolled into the first of them, in pixels. */
	offset: number;
	/** How far the list was scrolled, in pixels: for where there are no rows to count. */
	top: number;
}

export interface View {
	stream: Stream;
	unreadOnly: boolean;
	newestFirst: boolean;
	cursor: string | null;
	open: string | null;
	/** How far the sidebar was scrolled, in pixels. */
	sidebar: number;
	list: ListSpot;
	/** How far the article in its own pane was scrolled. */
	pane: { id: string; top: number } | null;
}

const blank = (): View => ({
	stream: { kind: 'home' },
	unreadOnly: false,
	newestFirst: true,
	cursor: null,
	open: null,
	sidebar: 0,
	list: { ids: [], offset: 0, top: 0 },
	pane: null
});

const isId = (value: unknown): value is string =>
	typeof value === 'string' && value.length > 0 && value.length <= MAX_ID && /^[\w-]+$/.test(value);

const pixels = (value: unknown) =>
	typeof value === 'number' && Number.isFinite(value)
		? Math.min(MAX_PIXELS, Math.max(0, Math.round(value)))
		: 0;

export function streamOf(value: unknown): Stream | null {
	if (!value || typeof value !== 'object') return null;
	const { kind, id } = value as Record<string, unknown>;
	switch (kind) {
		case 'home':
		case 'all':
		case 'unread':
		case 'starred':
		case 'hidden':
			return { kind };
		case 'folder':
		case 'feed':
		case 'label':
			return isId(id) ? { kind, id } : null;
		default:
			return null;
	}
}

/**
 * The view as it was kept for `account`; null for nothing, for another account,
 * and for anything that is not understood. What is understood in part is taken in
 * part: a scroll position that is not one is the top.
 */
export function parseView(stored: string | null, account: string): View | null {
	if (!stored || stored.length > MAX_STORED) return null;
	let found: unknown;
	try {
		found = JSON.parse(stored);
	} catch {
		return null;
	}
	if (!found || typeof found !== 'object' || Array.isArray(found)) return null;
	const kept = found as Record<string, unknown>;
	if (kept.form !== FORM || kept.account !== account) return null;
	const stream = streamOf(kept.stream);
	if (!stream) return null;
	const list = (kept.list && typeof kept.list === 'object' ? kept.list : {}) as Record<
		string,
		unknown
	>;
	const pane = (kept.pane && typeof kept.pane === 'object' ? kept.pane : {}) as Record<
		string,
		unknown
	>;
	return {
		stream,
		unreadOnly: kept.unreadOnly === true,
		newestFirst: kept.newestFirst !== false,
		cursor: isId(kept.cursor) ? kept.cursor : null,
		open: isId(kept.open) ? kept.open : null,
		sidebar: pixels(kept.sidebar),
		list: {
			ids: Array.isArray(list.ids) ? list.ids.filter(isId).slice(0, SPOT_IDS) : [],
			offset: pixels(list.offset),
			top: pixels(list.top)
		},
		pane: isId(pane.id) ? { id: pane.id, top: pixels(pane.top) } : null
	};
}

/** Whose view is kept; null until a reader page opens it. */
let account: string | null = null;
let now: View = blank();
/** Where the list and the open article were, until they take it. */
let listSpot: { stream: Stream; spot: ListSpot } | null = null;
let paneSpot: { id: string; top: number } | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;
let listening = false;

function write() {
	clearTimeout(timer);
	timer = undefined;
	if (account === null) return;
	try {
		localStorage.setItem(VIEW_KEY, JSON.stringify({ form: FORM, account, ...now }));
	} catch {
		// Without storage the page starts on the start page.
	}
}

/** Writes what is noted now, as when the page goes away. */
export function flushView() {
	if (timer !== undefined) write();
}

function listen() {
	if (listening || typeof window === 'undefined') return;
	listening = true;
	window.addEventListener('pagehide', flushView);
	document.addEventListener('visibilitychange', () => {
		if (document.visibilityState === 'hidden') flushView();
	});
}

/**
 * Starts keeping the view of an account (the name, or '' where there is none) and
 * says where it was left; null if there is nothing to go back to.
 */
export function openView(name: string): View | null {
	clearTimeout(timer);
	timer = undefined;
	account = name;
	let kept: View | null = null;
	try {
		kept = parseView(localStorage.getItem(VIEW_KEY), name);
	} catch {
		kept = null;
	}
	now = kept ? { ...kept, list: { ...kept.list } } : blank();
	listSpot = kept && kept.stream.kind !== 'home' ? { stream: kept.stream, spot: kept.list } : null;
	paneSpot = kept?.pane ?? null;
	listen();
	return kept;
}

/** The view as it is now. */
export function currentView(): Readonly<View> {
	return now;
}

/** Notes a change of the view; it is written once changes rest. */
export function noteView(change: Partial<View>) {
	if (account === null) return;
	now = { ...now, ...change };
	clearTimeout(timer);
	timer = setTimeout(write, WRITE_AFTER);
}

/** Where the list of `stream` was left, once; null for another stream, and the second time. */
export function takeListSpot(stream: Stream): ListSpot | null {
	const kept = listSpot;
	listSpot = null;
	if (!kept || !sameStream(kept.stream, stream)) return null;
	return kept.spot;
}

/** How far the article `id` was scrolled in its pane, once; null for another article. */
export function takePaneSpot(id: string): number | null {
	const kept = paneSpot;
	paneSpot = null;
	return kept && kept.id === id ? kept.top : null;
}

/** The list or the pane no longer goes back to where it was: the reader went elsewhere. */
export function dropSpots() {
	listSpot = null;
	paneSpot = null;
}

/** Stops keeping the view, as when the reader page goes; what was not written yet is dropped. */
export function closeView() {
	clearTimeout(timer);
	timer = undefined;
	account = null;
	dropSpots();
}

/** Forgets the view, as when the account logs out. */
export function forgetView() {
	clearTimeout(timer);
	timer = undefined;
	account = null;
	now = blank();
	dropSpots();
	try {
		localStorage.removeItem(VIEW_KEY);
	} catch {
		// Nothing was kept, then.
	}
}
