<script lang="ts">
	// The bar between two panes that says how wide the one before it is, or the one
	// after it (`after`), in the reading direction. Drag it with the pointer, or give
	// it the focus and press the arrow keys; a double click puts the width back to
	// what it was at the start. Right or left, the pane grows towards the bar: in a
	// page written from right to left the pane before it lies on the right. On a
	// phone there is nothing to pull apart, so whoever uses it hides it there.

	let {
		width,
		min,
		max,
		fallback,
		label,
		after = false,
		onwidth
	}: {
		width: number;
		min: number;
		max: number;
		/** The width a double click goes back to. */
		fallback: number;
		label: string;
		/** The pane comes after the bar in the reading direction, as the sidebar at the end does. */
		after?: boolean;
		onwidth: (width: number) => void;
	} = $props();

	/** What an arrow key moves, and what it moves with Shift. */
	const STEP = 16;
	const FAR = 64;

	let dragging = $state(false);
	/** Where the pointer went down and how wide the pane was then. */
	let from = 0;
	let was = 0;

	const fits = (pixels: number) => Math.min(max, Math.max(min, Math.round(pixels)));

	/** 1 where the pane lies on the left and grows as the bar moves right, else -1. */
	const towards = (bar: Element) =>
		(bar.closest('[dir]')?.getAttribute('dir') === 'rtl') === after ? 1 : -1;
	let sign = 1;

	function down(event: PointerEvent) {
		// Only the main button, and never a second pointer in the middle of a drag.
		if (event.button !== 0 || dragging) return;
		event.preventDefault();
		dragging = true;
		from = event.clientX;
		was = width;
		sign = towards(event.currentTarget as Element);
		(event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
	}

	function move(event: PointerEvent) {
		if (dragging) onwidth(fits(was + sign * (event.clientX - from)));
	}

	function up(event: PointerEvent) {
		if (!dragging) return;
		dragging = false;
		(event.currentTarget as HTMLElement).releasePointerCapture?.(event.pointerId);
	}

	function keydown(event: KeyboardEvent) {
		const step = (event.shiftKey ? FAR : STEP) * towards(event.currentTarget as Element);
		const to = {
			ArrowLeft: width - step,
			ArrowRight: width + step,
			Home: min,
			End: max
		}[event.key];
		if (to === undefined) return;
		event.preventDefault();
		onwidth(fits(to));
	}
</script>

<!-- The line itself is the pane's border; this is the handle over it. A separator
	that can be moved is a widget of its own in ARIA, and takes the focus and the
	keys; Svelte's check knows the role only as a line to look at. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
	role="separator"
	aria-orientation="vertical"
	aria-label={label}
	title={label}
	aria-valuenow={width}
	aria-valuemin={min}
	aria-valuemax={max}
	tabindex="0"
	class={[
		'relative z-10 hidden w-2 shrink-0 cursor-col-resize touch-none transition-colors md:block',
		after ? '-me-1' : '-ms-1',
		dragging ? 'bg-accent/60' : 'hover:bg-accent/40 focus-visible:bg-accent/60'
	]}
	onpointerdown={down}
	onpointermove={move}
	onpointerup={up}
	onpointercancel={up}
	onkeydown={keydown}
	ondblclick={() => onwidth(fallback)}
></div>
