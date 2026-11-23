// Zooming and moving an image in the large view of it (`ImageViewer.svelte`). The
// image lies in the middle of its stage, fitted into it; a view scales it about
// its middle and moves it by `x` and `y`. Points are counted from the middle of
// the stage. An image smaller than the stage stays in its middle, a larger one
// can be moved until its edge reaches the edge of the stage, not beyond.

export interface View {
	scale: number;
	x: number;
	y: number;
}

export interface Size {
	width: number;
	height: number;
}

export interface Point {
	x: number;
	y: number;
}

export const MIN_SCALE = 1;
export const MAX_SCALE = 8;

/** How far a double tap or the zoom buttons go. */
export const STEP = 2;

export const fitted: View = { scale: 1, x: 0, y: 0 };

const within = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));

/** The view moved back as far as needed for the image to stay on the stage. */
export function keep(view: View, image: Size, stage: Size): View {
	const room = (size: number, space: number) => Math.max(0, (size * view.scale - space) / 2);
	const width = room(image.width, stage.width);
	const height = room(image.height, stage.height);
	// `0 - width` rather than `-width`, so that the middle is 0 and not -0.
	return {
		scale: view.scale,
		x: within(view.x, 0 - width, width),
		y: within(view.y, 0 - height, height)
	};
}

/** The view zoomed by `factor`, with what is under `point` staying there. */
export function zoomAt(view: View, factor: number, point: Point, image: Size, stage: Size): View {
	const scale = within(view.scale * factor, MIN_SCALE, MAX_SCALE);
	const by = scale / view.scale;
	return keep(
		{ scale, x: point.x - (point.x - view.x) * by, y: point.y - (point.y - view.y) * by },
		image,
		stage
	);
}

/** The view moved by `dx` and `dy`. */
export function moveBy(view: View, dx: number, dy: number, image: Size, stage: Size): View {
	return keep({ scale: view.scale, x: view.x + dx, y: view.y + dy }, image, stage);
}
