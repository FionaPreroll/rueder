<script lang="ts">
	// A small sign beside an article, in one shape for all of them: a label with its color,
	// a category the feed gave, or a hint that leads on, like the one to earlier versions.
	import type { Snippet } from 'svelte';

	let {
		kind,
		color,
		title,
		label,
		onclick,
		children
	}: {
		kind: 'label' | 'tag' | 'hint';
		/** The color of a label, as a dot in front of its name. */
		color?: string;
		title?: string;
		/** The name for screen readers, where the text alone does not say enough. */
		label?: string;
		/** Makes a hint a button. */
		onclick?: () => void;
		children: Snippet;
	} = $props();

	const look = $derived([
		'inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs',
		kind === 'tag' ? 'bg-panel text-soft' : 'border border-line',
		kind === 'label' && 'text-ink',
		kind === 'hint' && 'text-soft',
		onclick && 'reach cursor-pointer hover:bg-hover hover:text-ink'
	]);
</script>

{#snippet inside()}
	{#if color}<span class="size-2 rounded-full" style:background-color={color}></span>{/if}
	{@render children()}
{/snippet}

{#if onclick}
	<button type="button" class={look} {title} aria-label={label} {onclick}>
		{@render inside()}
	</button>
{:else}
	<span class={look} {title} aria-label={label}>{@render inside()}</span>
{/if}
