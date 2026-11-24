<script lang="ts">
	// An image of an article over the whole window, for comics and pictures that are
	// too small in the text. Two fingers, the wheel, a double tap, the buttons and
	// + and - zoom it; a finger, the mouse and the arrow keys move it once it is
	// larger than the window (`#lib/zoom.ts`). Under it stand its title and its
	// alternative text, which on webcomics like xkcd hold the joke. Back closes
	// the image rather than the article (`#lib/layers.ts`); so do the close
	// button, Escape and a tap beside the image.
	import { onMount } from 'svelte';
	import { m } from '#lib/i18n/index.ts';
	import { backCloses } from '#lib/layers.ts';
	import { fitted, moveBy, STEP, zoomAt, type Point, type View } from '#lib/zoom.ts';
	import Icon, { type IconName } from './Icon.svelte';

	let {
		src,
		alt,
		title,
		onclose
	}: { src: string; alt: string; title: string; onclose: () => void } = $props();

	let panel: HTMLElement | undefined = $state();
	let stage: HTMLElement | undefined = $state();
	let image: HTMLImageElement | undefined = $state();
	let view = $state<View>(fitted);
	/** While fingers or the mouse move the image, it follows them without easing. */
	let dragging = $state(false);

	// Each text once: where a feed gave no title, the server took the alternative text for it.
	const texts = $derived([...new Set([title, alt].map((text) => text.trim()).filter(Boolean))]);

	const before = document.activeElement as HTMLElement | null;

	onMount(() => {
		panel?.focus();
		const leave = backCloses(() => onclose());
		// Not passive, so that the wheel zooms the image instead of scrolling the page under it.
		stage?.addEventListener('wheel', wheel, { passive: false });
		return () => {
			leave();
			before?.focus?.();
		};
	});

	function sizes() {
		return {
			image: { width: image?.offsetWidth ?? 0, height: image?.offsetHeight ?? 0 },
			stage: { width: stage?.clientWidth ?? 0, height: stage?.clientHeight ?? 0 }
		};
	}

	/** A point of the window, counted from the middle of the stage. */
	function onStage(x: number, y: number): Point {
		const box = stage?.getBoundingClientRect();
		return box ? { x: x - box.left - box.width / 2, y: y - box.top - box.height / 2 } : { x, y };
	}

	function zoom(factor: number, point: Point = { x: 0, y: 0 }, from: View = view) {
		const { image, stage } = sizes();
		view = zoomAt(from, factor, point, image, stage);
	}

	function move(dx: number, dy: number) {
		const { image, stage } = sizes();
		view = moveBy(view, dx, dy, image, stage);
	}

	function wheel(event: WheelEvent) {
		event.preventDefault();
		// A pinch on a touchpad comes as the wheel with Ctrl, in small steps.
		const speed = event.ctrlKey
			? 0.01
			: event.deltaMode === WheelEvent.DOM_DELTA_LINE
				? 0.05
				: 0.002;
		zoom(Math.exp(-event.deltaY * speed), onStage(event.clientX, event.clientY));
	}

	/** The pointers on the stage, each where it was last. */
	const pointers = new Map<number, Point>();
	/** Where the first pointer came down, and whether they have moved since, which makes it no tap. */
	let start: Point = { x: 0, y: 0 };
	let moved = false;
	let lastTap = { at: -Infinity, x: 0, y: 0 };

	const middle = (a: Point, b: Point) => ({ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 });
	const distance = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);

	function down(event: PointerEvent) {
		if (event.button !== 0) return;
		stage?.setPointerCapture?.(event.pointerId);
		const at = { x: event.clientX, y: event.clientY };
		if (pointers.size === 0) {
			start = at;
			moved = false;
		} else {
			moved = true;
		}
		pointers.set(event.pointerId, at);
		dragging = true;
	}

	function pointermove(event: PointerEvent) {
		const last = pointers.get(event.pointerId);
		if (!last) return;
		const [a, b] = pointers.values();
		pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
		const [c, d] = pointers.values();
		if (distance(c, start) > 8) moved = true;
		if (!b || !d) {
			move(c.x - a.x, c.y - a.y);
			return;
		}
		// Two fingers: the image follows their middle and grows as they part.
		const [from, to] = [middle(a, b), middle(c, d)];
		const factor = distance(a, b) > 0 ? distance(c, d) / distance(a, b) : 1;
		zoom(factor, onStage(to.x, to.y), {
			...view,
			x: view.x + to.x - from.x,
			y: view.y + to.y - from.y
		});
	}

	function up(event: PointerEvent) {
		if (!pointers.delete(event.pointerId) || pointers.size > 0) return;
		dragging = false;
		if (moved || event.type === 'pointercancel') return;
		const box = image?.getBoundingClientRect();
		const [x, y] = [event.clientX, event.clientY];
		const onImage = box && x >= box.left && x <= box.right && y >= box.top && y <= box.bottom;
		if (!onImage) {
			onclose();
		} else if (event.timeStamp - lastTap.at < 300 && distance(lastTap, { x, y }) < 30) {
			lastTap.at = -Infinity;
			if (view.scale > 1) view = fitted;
			else zoom(STEP, onStage(x, y));
		} else {
			lastTap = { at: event.timeStamp, x, y };
		}
	}

	const keys: Record<string, () => void> = {
		Escape: () => onclose(),
		'+': () => zoom(STEP),
		'=': () => zoom(STEP),
		'-': () => zoom(1 / STEP),
		'0': () => (view = fitted),
		ArrowLeft: () => move(48, 0),
		ArrowRight: () => move(-48, 0),
		ArrowUp: () => move(0, 48),
		ArrowDown: () => move(0, -48)
	};

	function keydown(event: KeyboardEvent) {
		// The reader's shortcuts wait while the image is open.
		event.stopPropagation();
		const act = keys[event.key];
		if (!act || event.ctrlKey || event.metaKey || event.altKey) return;
		event.preventDefault();
		act();
	}
</script>

{#snippet tool(icon: IconName, label: string, onclick: () => void)}
	<!-- A press on a button is no drag of the image. -->
	<button
		type="button"
		class="plain size-10 bg-paper/80"
		aria-label={label}
		title={label}
		onpointerdown={(event) => event.stopPropagation()}
		{onclick}
	>
		<Icon name={icon} class="size-5" />
	</button>
{/snippet}

<svelte:window onresize={() => move(0, 0)} />

<div
	bind:this={panel}
	class="fixed inset-0 z-50 flex flex-col bg-paper text-ink outline-none"
	role="dialog"
	aria-modal="true"
	aria-label={texts[0] || m.media.image}
	tabindex="-1"
	onkeydown={keydown}
>
	<div
		bind:this={stage}
		class="relative flex min-h-0 flex-1 touch-none items-center justify-center overflow-hidden select-none"
		role="presentation"
		onpointerdown={down}
		onpointermove={pointermove}
		onpointerup={up}
		onpointercancel={up}
	>
		<img
			bind:this={image}
			{src}
			{alt}
			draggable="false"
			class={[
				'max-h-full max-w-full object-contain',
				view.scale > 1 ? 'cursor-grab' : 'cursor-zoom-in',
				!dragging && 'transition-transform duration-150 motion-reduce:transition-none'
			]}
			style:transform="translate({view.x}px, {view.y}px) scale({view.scale})"
		/>
		<div class="absolute inset-e-2 top-2 flex gap-1">
			{@render tool('zoomOut', m.media.zoomOut, () => zoom(1 / STEP))}
			{@render tool('zoomIn', m.media.zoomIn, () => zoom(STEP))}
			{@render tool('close', m.common.close, onclose)}
		</div>
	</div>
	{#if texts.length > 0}
		<div class="max-h-[35%] shrink-0 overflow-y-auto border-t border-line px-4 py-3">
			<div class="mx-auto max-w-prose space-y-1">
				{#each texts as text, index (index)}
					<p class={[index > 0 && 'text-sm text-soft']} dir="auto">{text}</p>
				{/each}
			</div>
		</div>
	{/if}
</div>
