// A long list that draws only the rows near what is in sight. The rows above and
// below are space of the height they had when they were drawn, or of a guess for
// those never drawn. Here is the arithmetic; the list itself is ArticleList.svelte.

/** Where each row starts, and after the last, where the list ends: n + 1 numbers. */
export function offsets(
	ids: readonly string[],
	heights: ReadonlyMap<string, number>,
	guess: number
) {
	const tops = new Array<number>(ids.length + 1);
	tops[0] = 0;
	for (let i = 0; i < ids.length; i++) tops[i + 1] = tops[i] + (heights.get(ids[i]) ?? guess);
	return tops;
}

/** The first row whose end lies below `y`: the row `y` falls in; n below the last. */
export function rowAt(tops: readonly number[], y: number) {
	let low = 0;
	let high = tops.length - 1;
	// The smallest i with tops[i + 1] > y.
	while (low < high) {
		const middle = (low + high) >> 1;
		if (tops[middle + 1] > y) high = middle;
		else low = middle + 1;
	}
	return low;
}

/**
 * The rows to draw, from `start` to before `end`, for a view of `height` that
 * begins `top` into the list, with `margin` more above and below.
 */
export function rowsInSight(
	tops: readonly number[],
	top: number,
	height: number,
	margin: number
): { start: number; end: number } {
	const count = tops.length - 1;
	if (count <= 0) return { start: 0, end: 0 };
	const start = rowAt(tops, Math.max(0, top - margin));
	const end = Math.min(count, rowAt(tops, top + height + margin) + 1);
	return { start: Math.min(start, end), end };
}
