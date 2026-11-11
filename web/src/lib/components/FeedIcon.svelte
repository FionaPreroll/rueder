<script lang="ts">
	// A feed's icon, through the server's image proxy. Where a feed has none, or it
	// does not load, a stand-in shows: the feed's first letter on its color.
	import type { Feed } from '#lib/model.ts';

	let { feed }: { feed: Feed } = $props();

	/** The address that failed to load, so a new one gets its try. */
	let broken: string | null = $state(null);
</script>

{#if feed.icon && feed.icon !== broken}
	<img
		class="size-4 shrink-0 rounded-sm object-contain"
		src={feed.icon}
		alt=""
		loading="lazy"
		onerror={() => (broken = feed.icon ?? null)}
	/>
{:else}
	<span
		class="inline-flex size-4 shrink-0 items-center justify-center rounded-sm text-[0.625rem] leading-none font-bold text-white"
		style:background-color="hsl({feed.hue} 55% 40%)"
		aria-hidden="true"
	>
		{feed.title.slice(0, 1).toUpperCase()}
	</span>
{/if}
