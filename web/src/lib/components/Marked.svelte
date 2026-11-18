<script lang="ts">
	// A title with the words of the search marked, as the list found them. It runs
	// in its own direction, whichever way the page is written (bdi).
	import { marks } from '#lib/marks.ts';

	let { text, query }: { text: string; query: string } = $props();

	const pieces = $derived(marks(text, query));
</script>

<bdi
	>{#each pieces as piece, i (i)}{#if piece.marked}<mark class="rounded-sm bg-mark text-inherit"
				>{piece.text}</mark
			>{:else}{piece.text}{/if}{/each}</bdi
>
