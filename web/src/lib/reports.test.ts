// What leaves the browser in a report of an error, and when.

import { afterEach, describe, expect, it, vi } from 'vitest';
import {
	MAX_MESSAGE,
	MAX_WAITING,
	Reporter,
	SEND_EVERY,
	browserOf,
	ownFrames,
	scrub
} from './reports.ts';

describe('scrub', () => {
	it('takes out addresses and numbers that name something', () => {
		expect(scrub('Failed to fetch https://example.org/feed?id=12 for ada@example.org')).toBe(
			'Failed to fetch <address> for <address>'
		);
		expect(scrub('GET /feeds/12/articles/3456: 502')).toBe('GET /feeds/<n>/articles/<n>: 502');
		expect(scrub('at 1731340800, line\n two\ttabs')).toBe('at <n>, line two tabs');
		expect([...scrub('ä'.repeat(900))]).toHaveLength(MAX_MESSAGE);
	});
});

describe('ownFrames', () => {
	it('keeps the lines of the own files, from Chrome and from Firefox', () => {
		const chrome = [
			'TypeError: x is undefined',
			'    at open (https://rueder.example/_app/immutable/chunks/list.js:12:34)',
			'    at https://rueder.example/_app/immutable/nodes/2.js:1:99',
			'    at chrome-extension://abc/_app/immutable/x.js:1:1',
			'    at https://cdn.example/_app/immutable/y.js:5:6'
		].join('\n');
		expect(ownFrames(chrome, 'rueder.example')).toBe(
			'_app/immutable/chunks/list.js:12:34\n_app/immutable/nodes/2.js:1:99'
		);
		const firefox =
			'open@https://rueder.example/_app/immutable/chunks/list.js:12:34\n@debugger eval code:1:1';
		expect(ownFrames(firefox, 'rueder.example')).toBe('_app/immutable/chunks/list.js:12:34');
		expect(ownFrames('/_app/a.js:1:1\n'.repeat(30), 'rueder.example').split('\n')).toHaveLength(10);
	});
});

describe('browserOf', () => {
	it('names the browser and its major version, and nothing else', () => {
		const firefox = 'Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0';
		const chrome =
			'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36';
		const edge = `${chrome} Edg/141.0.0.0`;
		const safari =
			'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.4 Safari/605.1.15';
		expect(browserOf(firefox)).toBe('Firefox 140');
		expect(browserOf(chrome)).toBe('Chrome 141');
		expect(browserOf(edge)).toBe('Edge 141');
		expect(browserOf(safari)).toBe('Safari 18');
		expect(browserOf('curl/8.0')).toBe('Other');
	});
});

describe('Reporter', () => {
	afterEach(() => vi.useRealTimers());

	function reporter() {
		const sent: unknown[][] = [];
		const beacon = vi.fn(() => true);
		const reports = new Reporter({
			send: async (reports) => sent.push(reports),
			beacon,
			version: '1731340800000',
			browser: 'Firefox 140',
			host: 'rueder.example'
		});
		return { reports, sent, beacon };
	}

	it('gathers errors of the page and sends them now and then', async () => {
		vi.useFakeTimers();
		const { reports, sent } = reporter();
		const stop = reports.start(window);
		const error = new TypeError('x is undefined at https://example.org/a');
		error.stack = 'at f (https://rueder.example/_app/immutable/chunks/a.js:1:2)';
		window.dispatchEvent(new ErrorEvent('error', { error, message: error.message }));
		window.dispatchEvent(new ErrorEvent('error', { error, message: error.message }));
		const rejection = new Event('unhandledrejection') as PromiseRejectionEvent;
		Object.assign(rejection, { reason: 'no feed 12345' });
		window.dispatchEvent(rejection);
		// The same error waits once.
		expect(reports.pending).toBe(2);
		expect(sent).toEqual([]);
		await vi.advanceTimersByTimeAsync(SEND_EVERY);
		expect(sent).toEqual([
			[
				{
					kind: 'error',
					message: 'TypeError: x is undefined at <address>',
					stack: '_app/immutable/chunks/a.js:1:2',
					version: '1731340800000',
					browser: 'Firefox 140'
				},
				{
					kind: 'rejection',
					message: 'no feed <n>',
					stack: '',
					version: '1731340800000',
					browser: 'Firefox 140'
				}
			]
		]);
		stop();
		// After stopping, nothing is heard.
		window.dispatchEvent(new ErrorEvent('error', { error }));
		expect(reports.pending).toBe(0);
	});

	it('keeps at most twenty, and sends them when the page is left', () => {
		const { reports, sent, beacon } = reporter();
		const stop = reports.start(window);
		for (let n = 0; n < MAX_WAITING + 5; n++) reports.note('error', new Error(`e${'x'.repeat(n)}`));
		expect(reports.pending).toBe(MAX_WAITING);
		window.dispatchEvent(new Event('pagehide'));
		expect(beacon).toHaveBeenCalledTimes(1);
		expect(beacon.mock.calls[0]).toHaveLength(1);
		expect(reports.pending).toBe(0);
		expect(sent).toEqual([]);
		stop();
	});
});
