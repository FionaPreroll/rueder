// Windows over the page (the dialogs, an image opened large, the form of a filter
// rule) make an entry in the browser's history of their own while they are open,
// so the browser's back, and the back button of a phone, close the topmost of
// them rather than the article or the page under it. Each entry counts the
// windows open up to it (`layer`), so a step back closes the windows above the
// entry it comes to and no others; the rest of the entry is the one it was made
// over, so the reader's way (`#lib/history.ts`) stays where it is. A window closed
// some other way takes its entry with it, unless the reader went somewhere else
// meanwhile.

/** How many windows were open at an entry of the history. */
export function layerOf(state: unknown): number {
	const layer = state && typeof state === 'object' ? (state as { layer?: unknown }).layer : 0;
	return typeof layer === 'number' && Number.isInteger(layer) && layer > 0 ? layer : 0;
}

/**
 * Makes the entry of a window that just opened. `onback` closes it when back
 * leaves its entry; while `holds` says something would be lost, back leaves it
 * open. Returns what to call once the window is gone.
 */
export function backCloses(onback: () => void, holds: () => boolean = () => false): () => void {
	const depth = layerOf(history.state) + 1;
	const enter = () => history.pushState({ ...history.state, layer: depth }, '');
	enter();
	const popped = (event: PopStateEvent) => {
		if (layerOf(event.state) >= depth) return;
		if (holds()) enter();
		else onback();
	};
	window.addEventListener('popstate', popped);
	return () => {
		window.removeEventListener('popstate', popped);
		// Once what closed it is done: if that went to another list, that list's
		// entry lies on top now, and stepping back would undo it.
		setTimeout(() => {
			if (layerOf(history.state) >= depth) history.back();
		});
	};
}
