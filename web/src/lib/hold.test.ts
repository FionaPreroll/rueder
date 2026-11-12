import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Hold, HOLD_AT_MOST, SETTLED_AFTER, type Aim } from '#lib/hold.ts';

let scroller: HTMLElement;
let height: number;

beforeEach(() => {
	vi.useFakeTimers({
		toFake: ['setTimeout', 'requestAnimationFrame', 'cancelAnimationFrame', 'performance']
	});
	scroller = document.createElement('div');
	height = 5000;
	Object.defineProperty(scroller, 'scrollHeight', { get: () => height });
	document.body.append(scroller);
});

afterEach(() => {
	scroller.remove();
	vi.useRealTimers();
});

describe('Hold', () => {
	it('scrolls to the place and keeps it there while what is above it moves', () => {
		let aim: Aim | null = { top: 400, ready: true };
		const hold = new Hold(scroller, () => aim);
		expect(scroller.scrollTop).toBe(400);

		// The rows above are measured: the place moves, and the list follows it.
		aim = { top: 520, ready: true };
		vi.advanceTimersByTime(100);
		expect(scroller.scrollTop).toBe(520);

		// Something else scrolls it away, as the browser does when it lays out anew.
		scroller.scrollTop = 0;
		vi.advanceTimersByTime(100);
		expect(scroller.scrollTop).toBe(520);
		expect(hold.active).toBe(true);
	});

	it('lets go once nothing moved for a moment', () => {
		const ended = vi.fn();
		const hold = new Hold(scroller, () => ({ top: 300, ready: true }), ended);
		vi.advanceTimersByTime(SETTLED_AFTER / 2);
		height += 200;
		vi.advanceTimersByTime(SETTLED_AFTER - 100);
		expect(hold.active).toBe(true);
		vi.advanceTimersByTime(200);
		expect(hold.active).toBe(false);
		expect(ended).toHaveBeenCalledOnce();

		scroller.scrollTop = 10;
		vi.advanceTimersByTime(100);
		expect(scroller.scrollTop).toBe(10);
	});

	it('waits for what decides the place, but not for ever', () => {
		let aim: Aim | null = null;
		const hold = new Hold(scroller, () => aim);
		vi.advanceTimersByTime(SETTLED_AFTER * 2);
		expect(scroller.scrollTop).toBe(0);
		expect(hold.active).toBe(true);

		aim = { top: 800, ready: false };
		vi.advanceTimersByTime(SETTLED_AFTER * 2);
		expect(scroller.scrollTop).toBe(800);
		expect(hold.active).toBe(true);

		vi.advanceTimersByTime(HOLD_AT_MOST);
		expect(hold.active).toBe(false);
	});

	it('lets go at once when the reader does something', () => {
		const first = new Hold(scroller, () => ({ top: 300, ready: true }));
		const second = new Hold(scroller, () => ({ top: 300, ready: true }));
		window.dispatchEvent(new Event('wheel'));
		expect(first.active).toBe(false);
		expect(second.active).toBe(false);

		const third = new Hold(scroller, () => ({ top: 300, ready: true }));
		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'j' }));
		expect(third.active).toBe(false);
	});
});
