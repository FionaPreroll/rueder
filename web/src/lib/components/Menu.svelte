<script lang="ts">
	// A button that opens a small menu under it. A click on an entry, a click
	// elsewhere and Escape close it. Where the menu would leave the window, as on a
	// narrow phone, it moves sideways into it, and above its button if there is no
	// room below.
	import type { Snippet } from 'svelte';

	let {
		label,
		title,
		class: look = 'tool',
		align = 'left',
		button,
		children
	}: {
		/** For screen readers, when the button shows only an icon. */
		label?: string;
		title?: string;
		class?: string;
		align?: 'left' | 'right';
		button: Snippet;
		children: Snippet;
	} = $props();

	/** How far from the edge of the window a menu stays, in pixels. */
	const EDGE = 8;

	let open = $state(false);
	let root: HTMLElement | undefined = $state();
	let menu: HTMLElement | undefined = $state();
	/** How far the menu moved sideways to stay in the window, in pixels. */
	let shift = $state(0);
	let above = $state(false);

	// Measured once the menu is drawn where its alignment puts it.
	$effect(() => {
		if (!open || !menu) return;
		const place = fit(menu.getBoundingClientRect(), root?.getBoundingClientRect());
		shift = place.shift;
		above = place.above;
		return () => {
			shift = 0;
			above = false;
		};
	});

	/** Where a menu drawn at `rect` under its button at `button` fits into the window. */
	function fit(rect: DOMRect, button: DOMRect | undefined) {
		const width = document.documentElement.clientWidth || window.innerWidth;
		const height = window.innerHeight;
		let shift = 0;
		if (rect.right > width - EDGE) shift = width - EDGE - rect.right;
		if (rect.left + shift < EDGE) shift = EDGE - rect.left;
		const roomAbove = (button?.top ?? 0) - EDGE;
		const above = rect.bottom > height - EDGE && roomAbove > height - EDGE - rect.top;
		return { shift, above };
	}

	function outside(event: MouseEvent) {
		if (open && !root?.contains(event.target as Node)) open = false;
	}

	function keydown(event: KeyboardEvent) {
		if (open && event.key === 'Escape') {
			event.stopPropagation();
			open = false;
			root?.querySelector('button')?.focus();
		}
	}
</script>

<svelte:window onclick={outside} />

<div class="relative flex" bind:this={root} onkeydown={keydown} role="presentation">
	<button
		type="button"
		class={look}
		aria-haspopup="menu"
		aria-expanded={open}
		aria-label={label}
		{title}
		onclick={() => (open = !open)}
	>
		{@render button()}
	</button>
	{#if open}
		<div
			role="menu"
			tabindex="-1"
			bind:this={menu}
			class={[
				'absolute z-30 max-w-[calc(100vw-1rem)] min-w-52 rounded-md border border-line bg-paper py-1 shadow-lg',
				align === 'right' ? 'right-0' : 'left-0',
				above ? 'bottom-full mb-1' : 'mt-1'
			]}
			style:translate={shift ? `${shift}px 0` : undefined}
			onclick={() => (open = false)}
			onkeydown={keydown}
		>
			{@render children()}
		</div>
	{/if}
</div>
