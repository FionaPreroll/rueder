<script lang="ts">
	// The third pane of the view of three panes: the open article, beside the list.
	// It scrolls on its own, and every article starts at its top, but the one that
	// was open before the page was loaded anew, which starts where it was left.
	import { tick } from 'svelte';
	import { Hold } from '#lib/hold.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { Article } from '#lib/model.ts';
	import type { Reader } from '#lib/reader.ts';
	import { noteView, takePaneSpot } from '#lib/view.ts';
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

	let hold: Hold | null = null;

	/**
	 * Which article is open, by its id alone. The article itself is a new object
	 * whenever the list is read anew or its text arrives, and the pane must not
	 * scroll back to the top for that: only another article does that.
	 */
	const openId = $derived(reader.open);

	// Another article starts at its top, not where the last one was left.
	$effect(() => {
		const id = openId;
		void tick().then(() => {
			hold?.stop();
			hold = null;
			const top = id ? takePaneSpot(id) : null;
			if (!scroller || !top) {
				scroller?.scrollTo?.({ top: 0 });
				return;
			}
			// Its text may still be on its way; the place holds until it is there.
			const hasText = () => reader.visible.find((listed) => listed.id === id)?.html != null;
			hold = new Hold(scroller, () => ({ top, ready: hasText() }));
		});
	});
	$effect(() => () => hold?.stop());

	function scrolled() {
		if (!scroller || !article || hold?.active) return;
		noteView({ pane: { id: article.id, top: scroller.scrollTop } });
	}
</script>

<section
	class="min-h-0 min-w-0 flex-1 overflow-y-auto bg-paper"
	aria-label={article?.title ?? m.list.pick}
	bind:this={scroller}
	onscroll={scrolled}
>
	{#if article}
		<div class="mx-auto max-w-3xl" data-pane-article={article.id}>
			<ArticleBody {reader} {article} {now} {onversions} />
		</div>
	{:else}
		<p class="p-10 text-center text-soft">{m.list.pick}</p>
	{/if}
</section>
