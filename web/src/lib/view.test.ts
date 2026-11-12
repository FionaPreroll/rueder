import { afterEach, describe, expect, it, vi } from 'vitest';
import {
	closeView,
	currentView,
	forgetView,
	noteView,
	openView,
	parseView,
	takeListSpot,
	takePaneSpot,
	VIEW_KEY,
	type View
} from '#lib/view.ts';

afterEach(() => {
	closeView();
	localStorage.clear();
	vi.useRealTimers();
});

const left: View = {
	stream: { kind: 'feed', id: 'feed-12' },
	unreadOnly: true,
	newestFirst: false,
	cursor: '345',
	open: '345',
	sidebar: 120,
	list: { ids: ['340', '345', '346'], offset: 18, top: 900 },
	pane: { id: '345', top: 640 }
};

const stored = (change: Record<string, unknown> = {}) =>
	JSON.stringify({ form: 1, account: 'fiona', ...left, ...change });

describe('parseView', () => {
	it('reads what was written for the account', () => {
		expect(parseView(stored(), 'fiona')).toEqual(left);
	});

	it('is nothing for another account, another form, and what is not a view', () => {
		expect(parseView(stored(), 'ada')).toBeNull();
		expect(parseView(stored({ form: 2 }), 'fiona')).toBeNull();
		expect(parseView(stored({ form: undefined }), 'fiona')).toBeNull();
		for (const broken of [null, '', 'not json', '[1]', 'null', '42', '"view"']) {
			expect(parseView(broken, 'fiona')).toBeNull();
		}
		expect(parseView(stored({ stream: { kind: 'feed' } }), 'fiona')).toBeNull();
		expect(parseView(stored({ stream: { kind: 'elsewhere' } }), 'fiona')).toBeNull();
		expect(parseView(stored({ stream: 'all' }), 'fiona')).toBeNull();
		expect(parseView(stored({ stream: { kind: 'feed', id: '../x' } }), 'fiona')).toBeNull();
	});

	it('reads nothing that is longer than a view can be', () => {
		expect(parseView(stored({ padding: 'x'.repeat(5000) }), 'fiona')).toBeNull();
	});

	it('takes what it understands and makes the rest the start', () => {
		const view = parseView(
			stored({
				stream: { kind: 'all', id: 'ignored' },
				unreadOnly: 'yes',
				newestFirst: 0,
				cursor: 7,
				open: 'x'.repeat(65),
				sidebar: -40,
				list: { ids: ['1', 2, '<b>', '3', '4', '5', '6', '7'], offset: Infinity, top: 1e12 },
				pane: { id: '', top: 5 }
			}),
			'fiona'
		);
		expect(view).toEqual({
			stream: { kind: 'all' },
			unreadOnly: false,
			newestFirst: true,
			cursor: null,
			open: null,
			sidebar: 0,
			list: { ids: ['1', '3', '4', '5', '6'], offset: 0, top: 10_000_000 },
			pane: null
		});
		expect(parseView(stored({ list: 'none', pane: 3 }), 'fiona')).toMatchObject({
			list: { ids: [], offset: 0, top: 0 },
			pane: null
		});
	});
});

describe('keeping the view', () => {
	it('starts from nothing, and writes what changes once it rests', () => {
		vi.useFakeTimers();
		expect(openView('fiona')).toBeNull();
		noteView({ stream: { kind: 'starred' } });
		noteView({ sidebar: 30 });
		expect(localStorage.getItem(VIEW_KEY)).toBeNull();
		vi.advanceTimersByTime(300);
		expect(parseView(localStorage.getItem(VIEW_KEY), 'fiona')).toMatchObject({
			stream: { kind: 'starred' },
			sidebar: 30
		});
	});

	it('writes at once when the page goes away', () => {
		openView('fiona');
		noteView({ open: '9' });
		window.dispatchEvent(new Event('pagehide'));
		expect(parseView(localStorage.getItem(VIEW_KEY), 'fiona')?.open).toBe('9');
	});

	it('gives the list and the pane their places once, and only for what they were', () => {
		localStorage.setItem(VIEW_KEY, stored());
		expect(openView('fiona')).toEqual(left);
		expect(currentView()).toEqual(left);
		expect(takeListSpot({ kind: 'all' })).toBeNull();
		expect(takeListSpot(left.stream)).toBeNull();

		openView('fiona');
		expect(takeListSpot(left.stream)).toEqual(left.list);
		expect(takeListSpot(left.stream)).toBeNull();
		expect(takePaneSpot('345')).toBe(640);
		expect(takePaneSpot('345')).toBeNull();

		openView('fiona');
		expect(takePaneSpot('346')).toBeNull();
		expect(takePaneSpot('345')).toBeNull();
	});

	it('does not go back to the start page, and not into the view of another account', () => {
		localStorage.setItem(VIEW_KEY, stored({ stream: { kind: 'home' } }));
		openView('fiona');
		expect(takeListSpot({ kind: 'home' })).toBeNull();

		localStorage.setItem(VIEW_KEY, stored());
		expect(openView('ada')).toBeNull();
		expect(currentView().stream).toEqual({ kind: 'home' });
		expect(takeListSpot(left.stream)).toBeNull();
	});

	it('writes nothing once the page is closed, and forgets it all on logging out', () => {
		vi.useFakeTimers();
		openView('fiona');
		noteView({ open: '9' });
		closeView();
		vi.advanceTimersByTime(1000);
		expect(localStorage.getItem(VIEW_KEY)).toBeNull();

		openView('fiona');
		noteView({ open: '9' });
		vi.advanceTimersByTime(1000);
		expect(localStorage.getItem(VIEW_KEY)).not.toBeNull();
		forgetView();
		expect(localStorage.getItem(VIEW_KEY)).toBeNull();
		noteView({ open: '10' });
		vi.advanceTimersByTime(1000);
		expect(localStorage.getItem(VIEW_KEY)).toBeNull();
	});

	it('lives without storage', () => {
		const failing = vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
			throw new Error('denied');
		});
		const writing = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
			throw new Error('denied');
		});
		expect(openView('fiona')).toBeNull();
		noteView({ open: '9' });
		window.dispatchEvent(new Event('pagehide'));
		expect(currentView().open).toBe('9');
		failing.mockRestore();
		writing.mockRestore();
	});
});
