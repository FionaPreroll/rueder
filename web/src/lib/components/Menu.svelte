<script lang="ts">
	// A button that opens a small menu under it. A click on an entry, a click
	// elsewhere and Escape close it.
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

	let open = $state(false);
	let root: HTMLElement | undefined = $state();

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

<div class="relative" bind:this={root} onkeydown={keydown} role="presentation">
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
			class={[
				'absolute z-30 mt-1 min-w-52 rounded-md border border-line bg-paper py-1 shadow-lg',
				align === 'right' ? 'right-0' : 'left-0'
			]}
			onclick={() => (open = false)}
			onkeydown={keydown}
		>
			{@render children()}
		</div>
	{/if}
</div>
