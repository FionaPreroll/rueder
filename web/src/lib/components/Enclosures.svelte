<script lang="ts">
	// The files that come with an article, like the episode of a podcast: each with a
	// link to it, and a player for sound and film. A player is only put on the page
	// when the reader asks for it, so that nothing is fetched from elsewhere before.
	import { formatBytes } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { Enclosure } from '#lib/model.ts';
	import Icon from './Icon.svelte';

	let { enclosures }: { enclosures: Enclosure[] } = $props();

	/** The addresses of the files that are being played. */
	let playing = $state<string[]>([]);

	const kindOf = (enclosure: Enclosure) => {
		const type = enclosure.type?.toLowerCase() ?? '';
		if (type.startsWith('audio/')) return 'audio';
		if (type.startsWith('video/')) return 'video';
		return 'file';
	};

	/** The name of the file as its address has it, else the address. */
	function nameOf(enclosure: Enclosure) {
		try {
			const { protocol, pathname, hostname } = new URL(enclosure.url);
			// An address that holds the file itself has no name in it.
			if (!/^https?:$/.test(protocol)) return enclosure.type ?? protocol;
			const last = pathname.split('/').filter(Boolean).at(-1);
			return last ? decodeURIComponent(last) : hostname;
		} catch {
			return enclosure.url.slice(0, 80);
		}
	}
</script>

<section class="mt-4 rounded-md border border-line bg-panel px-3 py-2 text-sm">
	<h3 class="font-medium">{m.media.title}</h3>
	<ul class="mt-1 space-y-2">
		{#each enclosures as enclosure (enclosure.url)}
			{@const kind = kindOf(enclosure)}
			{@const name = nameOf(enclosure)}
			<li>
				<div class="flex flex-wrap items-center gap-x-2 gap-y-1">
					{#if kind !== 'file' && !playing.includes(enclosure.url)}
						<button
							type="button"
							class="tool"
							aria-label="{m.media.play}: {name}"
							onclick={() => (playing = [...playing, enclosure.url])}
						>
							<Icon name="play" />
							{m.media.play}
						</button>
					{/if}
					<a
						class="min-w-0 truncate text-link hover:underline"
						href={enclosure.url}
						target="_blank"
						rel="noopener noreferrer"
					>
						{name}
					</a>
					<span class="text-xs text-soft">
						{[enclosure.type, enclosure.length ? formatBytes(enclosure.length) : null]
							.filter(Boolean)
							.join(' · ')}
					</span>
				</div>
				{#if playing.includes(enclosure.url)}
					{#if kind === 'audio'}
						<audio class="mt-2 w-full" controls autoplay src={enclosure.url}></audio>
					{:else}
						<!-- svelte-ignore a11y_media_has_caption -->
						<video class="mt-2 max-h-96 w-full rounded" controls autoplay src={enclosure.url}
						></video>
					{/if}
				{/if}
			</li>
		{/each}
	</ul>
</section>
