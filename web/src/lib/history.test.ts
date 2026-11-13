// The entries the reader's way makes in the browser's history.

import { describe, expect, it } from 'vitest';
import { Journey, placeOf, type BrowserHistory } from './history.ts';
import type { Stream } from './model.ts';

/** A history like the browser's: entries, where it stands, and what back finds. */
class FakeHistory implements BrowserHistory {
	entries: unknown[] = [null];
	at = 0;
	scrollRestoration: ScrollRestoration = 'auto';
	popped: unknown[] = [];

	get state() {
		return this.entries[this.at];
	}
	pushState(state: unknown) {
		this.entries = [...this.entries.slice(0, this.at + 1), state];
		this.at++;
	}
	replaceState(state: unknown) {
		this.entries[this.at] = state;
	}
	back() {
		if (this.at === 0) return;
		this.at--;
		this.popped.push(this.state);
	}
	go(by: number) {
		this.at = Math.min(this.entries.length - 1, Math.max(0, this.at + by));
		this.popped.push(this.state);
	}
}

const feed = (id: string): Stream => ({ kind: 'feed', id });
const all: Stream = { kind: 'all' };

describe('Journey', () => {
	it('makes an entry for every list and every article opened in one', () => {
		const history = new FakeHistory();
		const journey = new Journey(history);
		journey.note({ stream: { kind: 'home' }, open: null });
		expect(history.entries).toHaveLength(1);
		journey.note({ stream: all, open: null });
		journey.note({ stream: all, open: null });
		journey.note({ stream: feed('feed-1'), open: null });
		journey.note({ stream: feed('feed-1'), open: '7' });
		// Stepping on takes the place of the entry.
		journey.note({ stream: feed('feed-1'), open: '6' });
		journey.note({ stream: feed('feed-1'), open: '5' });
		expect(history.entries.map((entry) => placeOf(entry))).toEqual([
			{ stream: { kind: 'home' }, open: null, opened: false },
			{ stream: all, open: null, opened: false },
			{ stream: feed('feed-1'), open: null, opened: false },
			{ stream: feed('feed-1'), open: '5', opened: true }
		]);

		// Back goes to the list, then to all items; the journey says where.
		history.back();
		expect(journey.returned(history.popped.at(-1))).toEqual({ stream: feed('feed-1'), open: null });
		history.back();
		expect(journey.returned(history.popped.at(-1))).toEqual({ stream: all, open: null });
		// And forward back to the article.
		history.go(2);
		expect(journey.returned(history.popped.at(-1))).toEqual({ stream: feed('feed-1'), open: '5' });
	});

	it('goes back an entry when the article that made it closes', () => {
		const history = new FakeHistory();
		const journey = new Journey(history);
		journey.note({ stream: all, open: null });
		journey.note({ stream: all, open: '3' });
		journey.note({ stream: all, open: null });
		expect(history.at).toBe(0);
		// That return is the journey's own and changes nothing.
		expect(journey.returned(history.popped.at(-1))).toBeNull();
		// The next one is the reader's again.
		journey.note({ stream: all, open: '4' });
		expect(history.at).toBe(1);
	});

	it('closes in place an article that was open when the page came', () => {
		const history = new FakeHistory();
		const journey = new Journey(history);
		journey.note({ stream: all, open: '3' });
		journey.note({ stream: all, open: null });
		expect(history.entries).toHaveLength(1);
		expect(placeOf(history.state)).toEqual({ stream: all, open: null, opened: false });
	});

	it('leaves entries alone that are not its own, and reads nothing odd', () => {
		const journey = new Journey(new FakeHistory());
		for (const state of [
			null,
			'text',
			{ rueder: 2, stream: all },
			{ rueder: 1, stream: { kind: 'feed', id: '<script>' } },
			{ rueder: 1, stream: { kind: 'nonsense' } }
		]) {
			expect(journey.returned(state)).toBeNull();
		}
		expect(placeOf({ rueder: 1, stream: all, open: 'x'.repeat(65), opened: true })).toEqual({
			stream: all,
			open: null,
			opened: false
		});
	});
});
