<script lang="ts">
	// A title with the words of the search marked, as the list found them.
	import { marks } from '#lib/marks.ts';

	let { text, query }: { text: string; query: string } = $props();

	const pieces = $derived(marks(text, query));
</script>

{#each pieces as piece, i (i)}{#if piece.marked}<mark class="rounded-sm bg-mark text-inherit"
			>{piece.text}</mark
		>{:else}{piece.text}{/if}{/each}
