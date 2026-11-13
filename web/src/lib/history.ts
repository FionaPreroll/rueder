// Entries in the browser's history for where the reader goes, so the browser's back
// and forward buttons, and the back button of a phone, go to the list or the
// article that came before. Another list is a new entry; so is an article opened
// in a list, and closing it goes back that entry. Stepping from one open article
// to the next takes the place of the entry. The address of the page stays as it
// is: what the entries hold is in `history.state`, ids only.

import { sameStream, type Stream } from './model.ts';
import { streamOf } from './view.ts';

/** Where the reader is: what the main pane shows, and the article open in it. */
export interface Place {
	stream: Stream;
	open: string | null;
}

/** An entry of rueder's, with whether it was made by opening its article. */
interface Entry extends Place {
	rueder: 1;
	opened: boolean;
}

const MAX_ID = 64;

/** The place an entry of the history holds, if it is one of rueder's. */
export function placeOf(state: unknown): (Place & { opened: boolean }) | null {
	if (!state || typeof state !== 'object') return null;
	const entry = state as Record<string, unknown>;
	if (entry.rueder !== 1) return null;
	const stream = streamOf(entry.stream);
	if (!stream) return null;
	const open =
		typeof entry.open === 'string' && entry.open.length > 0 && entry.open.length <= MAX_ID
			? entry.open
			: null;
	return { stream, open, opened: entry.opened === true && open !== null };
}

const entry = (place: Place, opened: boolean): Entry => ({
	rueder: 1,
	stream: place.stream,
	open: place.open,
	opened: opened && place.open !== null
});

/** The part of the browser's history this needs. */
export type BrowserHistory = Pick<History, 'state' | 'pushState' | 'replaceState' | 'back'>;

export class Journey {
	/** A step back the journey took itself, to close an article; its return changes nothing. */
	private goingBack = false;

	constructor(private history: BrowserHistory = globalThis.history) {}

	/** Notes where the reader is now, after a change the reader made. */
	note(place: Place) {
		const here = placeOf(this.history.state);
		if (!here) {
			this.history.replaceState(entry(place, false), '');
			return;
		}
		if (sameStream(here.stream, place.stream) && here.open === place.open) return;
		if (!sameStream(here.stream, place.stream)) {
			this.history.pushState(entry(place, false), '');
		} else if (here.open === null) {
			// An article opened in the list: back closes it again.
			this.history.pushState(entry(place, true), '');
		} else if (place.open !== null) {
			// The next article in its place, so back does not walk through every one.
			this.history.replaceState(entry(place, here.opened), '');
		} else if (here.opened) {
			// Closed: the entry that opened it goes, as back would have done.
			this.goingBack = true;
			this.history.back();
		} else {
			this.history.replaceState(entry(place, false), '');
		}
	}

	/**
	 * The place a step back or forward came to, when the reader should go there;
	 * null for an entry that is not rueder's, or for the step back the journey
	 * took itself.
	 */
	returned(state: unknown): Place | null {
		if (this.goingBack) {
			this.goingBack = false;
			return null;
		}
		const place = placeOf(state);
		return place ? { stream: place.stream, open: place.open } : null;
	}
}
