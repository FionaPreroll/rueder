// The arithmetic of the long list, held against drawing every row and asking each
// whether it is in sight.

import { describe, expect, it } from 'vitest';
import { offsets, rowsInSight } from './virtual.ts';

/** A simple pseudo-random sequence: the same every run. */
function random(seed: number) {
	return () => {
		seed = (seed * 1_103_515_245 + 12_345) % 2 ** 31;
		return seed / 2 ** 31;
	};
}

describe('offsets', () => {
	it('adds up the heights, with a guess for rows never drawn', () => {
		const heights = new Map([
			['a', 30],
			['c', 100]
		]);
		expect(offsets(['a', 'b', 'c'], heights, 50)).toEqual([0, 30, 80, 180]);
		expect(offsets([], heights, 50)).toEqual([0]);
	});
});

describe('rowsInSight', () => {
	it('draws every row in sight, and at most one more at either end', () => {
		const next = random(7);
		for (let round = 0; round < 300; round++) {
			const count = Math.floor(next() * 400);
			const ids = Array.from({ length: count }, (_, i) => `row-${i}`);
			const heights = new Map(ids.map((id) => [id, 1 + Math.floor(next() * 300)]));
			const tops = offsets(ids, heights, 0);
			const total = tops[count];
			const top = Math.floor(next() * (total + 500)) - 100;
			const height = Math.floor(next() * 900);
			const margin = Math.floor(next() * 600);
			const { start, end } = rowsInSight(tops, top, height, margin);

			// The model: a row is needed if any of it lies within the view and its margin.
			const from = Math.max(0, top - margin);
			const to = top + height + margin;
			const needed = ids.map((_, i) => i).filter((i) => tops[i + 1] > from && tops[i] < to);
			const why = `round ${round}: ${start}..${end} for ${needed[0]}..${needed.at(-1)}`;
			expect(start, why).toBeGreaterThanOrEqual(0);
			expect(end, why).toBeLessThanOrEqual(count);
			expect(start, why).toBeLessThanOrEqual(end);
			for (const i of needed) {
				expect(i >= start && i < end, why).toBe(true);
			}
			if (needed.length > 0) {
				expect(needed[0] - start, why).toBeLessThanOrEqual(1);
				expect(end - 1 - needed[needed.length - 1], why).toBeLessThanOrEqual(1);
			} else {
				expect(end - start, why).toBeLessThanOrEqual(1);
			}
		}
	});

	it('draws nothing of an empty list', () => {
		expect(rowsInSight([0], 0, 500, 100)).toEqual({ start: 0, end: 0 });
	});
});
