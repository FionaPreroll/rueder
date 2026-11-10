<script lang="ts">
	// A stand-in for a feed's icon: its first letter on its color. The real icons will
	// come through the server's image proxy.
	import type { Feed } from '#lib/model.ts';

	let { feed }: { feed: Feed } = $props();
</script>

<span
	class="inline-flex size-4 shrink-0 items-center justify-center rounded-sm text-[0.625rem] leading-none font-bold text-white"
	style:background-color="hsl({feed.hue} 55% 40%)"
	aria-hidden="true"
>
	{feed.title.slice(0, 1).toUpperCase()}
</span>
