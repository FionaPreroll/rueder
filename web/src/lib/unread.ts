// What somebody has read of one feed, and how many articles that leaves unread.
//
// Articles of a feed are numbered as they arrive. The state is a mark ("everything up
// to number N is read") and the exceptions from it, so "mark all as read" is one
// number, and the unread count follows from the state and the feed's newest number.
// There is no counter to add to or take from, so nothing can drift. The server will
// keep the same state per user and feed.

export interface FeedState {
	/** Everything up to this number is read, except `keptUnread`. */
	readUpto: number;
	/** Read articles above the mark. */
	read: ReadonlySet<number>;
	/** Articles at or below the mark that were set back to unread. */
	keptUnread: ReadonlySet<number>;
	/** Articles a filter rule hides; they count neither as read nor as unread. */
	hidden: ReadonlySet<number>;
	starred: ReadonlySet<number>;
}

export const emptyState = (): FeedState => ({
	readUpto: 0,
	read: new Set(),
	keptUnread: new Set(),
	hidden: new Set(),
	starred: new Set()
});

export const isHidden = (state: FeedState, seq: number) => state.hidden.has(seq);

export const isRead = (state: FeedState, seq: number) =>
	seq <= state.readUpto ? !state.keptUnread.has(seq) : state.read.has(seq);

export const isStarred = (state: FeedState, seq: number) => state.starred.has(seq);

/** Unread articles of a feed whose newest article has the number `lastSeq`. */
export function unreadCount(state: FeedState, lastSeq: number) {
	let above = Math.max(0, lastSeq - state.readUpto);
	for (const seq of state.hidden) {
		if (seq > state.readUpto && seq <= lastSeq) above--;
	}
	for (const seq of state.read) {
		if (seq > state.readUpto && seq <= lastSeq && !state.hidden.has(seq)) above--;
	}
	let below = 0;
	for (const seq of state.keptUnread) {
		if (seq <= state.readUpto && !state.hidden.has(seq)) below++;
	}
	return above + below;
}

const without = (set: ReadonlySet<number>, seq: number) => {
	const copy = new Set(set);
	copy.delete(seq);
	return copy;
};

const withAlso = (set: ReadonlySet<number>, seq: number) => new Set(set).add(seq);

/** Moves the mark over the read articles right above it, so the exceptions stay few. */
function compact(state: FeedState): FeedState {
	let readUpto = state.readUpto;
	while (state.read.has(readUpto + 1)) readUpto++;
	if (readUpto === state.readUpto) return state;
	return { ...state, readUpto, read: new Set([...state.read].filter((seq) => seq > readUpto)) };
}

export function markRead(state: FeedState, seq: number): FeedState {
	if (isRead(state, seq)) return state;
	return seq <= state.readUpto
		? { ...state, keptUnread: without(state.keptUnread, seq) }
		: compact({ ...state, read: withAlso(state.read, seq) });
}

export function markUnread(state: FeedState, seq: number): FeedState {
	if (!isRead(state, seq)) return state;
	return seq <= state.readUpto
		? { ...state, keptUnread: withAlso(state.keptUnread, seq) }
		: { ...state, read: without(state.read, seq) };
}

/** Marks everything up to `upto` as read, usually the feed's newest number. */
export function markAllRead(state: FeedState, upto: number): FeedState {
	const readUpto = Math.max(state.readUpto, upto);
	return compact({
		...state,
		readUpto,
		read: new Set([...state.read].filter((seq) => seq > readUpto)),
		keptUnread: new Set([...state.keptUnread].filter((seq) => seq > upto))
	});
}

export function setStarred(state: FeedState, seq: number, on: boolean): FeedState {
	if (state.starred.has(seq) === on) return state;
	return { ...state, starred: on ? withAlso(state.starred, seq) : without(state.starred, seq) };
}

/** Replaces what the filter rules hide. */
export const setHidden = (state: FeedState, hidden: Iterable<number>): FeedState => ({
	...state,
	hidden: new Set(hidden)
});
