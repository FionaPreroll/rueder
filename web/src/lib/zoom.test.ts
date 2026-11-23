// Zooming and moving an image in its large view.

import { describe, expect, it } from 'vitest';
import { fitted, keep, MAX_SCALE, MIN_SCALE, moveBy, zoomAt, type View } from './zoom.ts';

const stage = { width: 400, height: 300 };
// A comic fitted to the width of the stage, lower than it.
const image = { width: 400, height: 200 };

/** Where a point of the image (from its middle, unscaled) is on the stage. */
const shown = (view: View, x: number, y: number) => ({
	x: view.x + x * view.scale,
	y: view.y + y * view.scale
});

describe('the large view of an image', () => {
	it('keeps the fitted image in the middle', () => {
		expect(moveBy(fitted, 50, -30, image, stage)).toEqual(fitted);
	});

	it('zooms around the point under the fingers', () => {
		const view = zoomAt(fitted, 2, { x: 100, y: 20 }, image, stage);
		expect(view.scale).toBe(2);
		// What was at (100, 20) of the image is still at (100, 20) of the stage.
		expect(shown(view, 100, 20)).toEqual({ x: 100, y: 20 });
		const again = zoomAt(view, 1.5, { x: -40, y: 10 }, image, stage);
		const point = { x: (-40 - view.x) / view.scale, y: (10 - view.y) / view.scale };
		const after = shown(again, point.x, point.y);
		expect(after.x).toBeCloseTo(-40);
		expect(after.y).toBeCloseTo(10);
	});

	it('moves a zoomed image only until its edge meets the edge of the stage', () => {
		const view: View = { scale: 2, x: 0, y: 0 };
		// 800 by 400 on 400 by 300: 200 to each side, 50 up and down.
		expect(moveBy(view, 1000, 1000, image, stage)).toEqual({ scale: 2, x: 200, y: 50 });
		expect(moveBy(view, -1000, -1000, image, stage)).toEqual({ scale: 2, x: -200, y: -50 });
		expect(moveBy(view, 30, -20, image, stage)).toEqual({ scale: 2, x: 30, y: -20 });
	});

	it('zooms no further than its bounds, and back out to the middle', () => {
		expect(zoomAt(fitted, 100, { x: 0, y: 0 }, image, stage).scale).toBe(MAX_SCALE);
		const zoomed = zoomAt(fitted, 4, { x: 150, y: 80 }, image, stage);
		expect(zoomAt(zoomed, 0.01, { x: 0, y: 0 }, image, stage)).toEqual({
			scale: MIN_SCALE,
			x: 0,
			y: 0
		});
	});

	it('stays on the stage whatever comes, as a simple model says', () => {
		let view = fitted;
		let seed = 7;
		const random = () => ((seed = (seed * 16807) % 2147483647) / 2147483647) * 2 - 1;
		for (let step = 0; step < 1000; step++) {
			view =
				step % 3 === 0
					? zoomAt(view, Math.exp(random()), { x: random() * 300, y: random() * 200 }, image, stage)
					: moveBy(view, random() * 400, random() * 400, image, stage);
			expect(view.scale).toBeGreaterThanOrEqual(MIN_SCALE);
			expect(view.scale).toBeLessThanOrEqual(MAX_SCALE);
			// The stage is covered where the image is larger, and the image centered where it is not.
			const half = { x: (image.width * view.scale) / 2, y: (image.height * view.scale) / 2 };
			for (const [axis, space] of [
				['x', stage.width],
				['y', stage.height]
			] as const) {
				if (half[axis] * 2 <= space) expect(view[axis]).toBe(0);
				else {
					expect(view[axis] - half[axis]).toBeLessThanOrEqual(-space / 2 + 1e-9);
					expect(view[axis] + half[axis]).toBeGreaterThanOrEqual(space / 2 - 1e-9);
				}
			}
			expect(keep(view, image, stage)).toEqual(view);
		}
	});
});
