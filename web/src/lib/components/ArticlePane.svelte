<script lang="ts">
	// The third pane of the view of three panes: the open article, beside the list.
	// It scrolls on its own, and every article starts at its top.
	import { tick } from 'svelte';
	import { m } from '#lib/i18n/index.ts';
	import type { Article } from '#lib/model.ts';
	import type { Reader } from '#lib/reader.ts';
	import ArticleBody from './ArticleBody.svelte';

	let {
		reader,
		now,
		onversions
	}: { reader: Reader; now: number; onversions: (article: Article) => void } = $props();

	let scroller: HTMLElement | undefined = $state();

	const article = $derived.by(() => {
		const id = reader.open;
		return id ? (reader.visible.find((listed) => listed.id === id) ?? null) : null;
	});

	// Another article starts at its top, not where the last one was left.
	$effect(() => {
		void article?.id;
		void tick().then(() => scroller?.scrollTo?.({ top: 0 }));
	});
</script>

<section
	class="min-h-0 min-w-0 flex-1 overflow-y-auto bg-paper"
	aria-label={article?.title ?? m.list.pick}
	bind:this={scroller}
>
	{#if article}
		<div class="mx-auto max-w-3xl" data-pane-article={article.id}>
			<ArticleBody {reader} {article} {now} {onversions} />
		</div>
	{:else}
		<p class="p-10 text-center text-soft">{m.list.pick}</p>
	{/if}
</section>
