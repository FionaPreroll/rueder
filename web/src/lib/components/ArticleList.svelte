<script lang="ts">
	// The articles of what is picked in the sidebar, in one of two views: a dense list
	// whose rows open in place, or every article in full, one card after the other.
	import { tick, untrack } from 'svelte';
	import type { Reader } from '#lib/reader.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { Article } from '#lib/model.ts';
	import { settings } from '#lib/settings.svelte.ts';
	import ArticleBody from './ArticleBody.svelte';
	import ArticleRow from './ArticleRow.svelte';

	let {
		reader,
		now,
		onversions
	}: { reader: Reader; now: number; onversions: (article: Article) => void } = $props();

	let scroller: HTMLElement | undefined = $state();

	const expanded = $derived(settings.view === 'expanded');
	const showFeed = $derived(reader.stream.kind !== 'feed');
	const feedIsEmpty = $derived(
		reader.stream.kind === 'feed' && reader.feeds[reader.stream.id]?.lastSeq === 0
	);

	const rowOf = (id: string) => scroller?.querySelector<HTMLElement>(`[data-article="${id}"]`);

	// A step with the keyboard or the arrows brings the article to the top of the pane.
	$effect(() => {
		if (reader.moved === 0) return;
		untrack(async () => {
			const id = reader.cursor;
			if (!id) return;
			await tick();
			rowOf(id)?.scrollIntoView({
				block: expanded || reader.open === id ? 'start' : 'nearest'
			});
		});
	});

	// Another stream starts at its top.
	$effect(() => {
		void reader.stream;
		scroller?.scrollTo?.({ top: 0 });
	});

	// In the expanded view, what scrolls out at the top has been read.
	let frame = 0;
	function scrolled() {
		if (!expanded || !settings.markOnScroll || frame || !scroller) return;
		frame = requestAnimationFrame(() => {
			frame = 0;
			if (!scroller) return;
			const edge = scroller.getBoundingClientRect().top;
			for (const article of reader.visible) {
				const card = rowOf(article.id);
				if (!card || card.getBoundingClientRect().bottom > edge) break;
				if (!reader.isRead(article)) reader.scrolledPast(article);
			}
		});
	}

	// More of the list comes when its end scrolls into sight.
	function whenSeen(button: HTMLElement) {
		if (typeof IntersectionObserver === 'undefined') return;
		const observer = new IntersectionObserver(
			(entries) => entries.some((entry) => entry.isIntersecting) && reader.showMore(),
			{ root: scroller, rootMargin: '400px' }
		);
		observer.observe(button);
		return { destroy: () => observer.disconnect() };
	}

	function showNew() {
		reader.showNew();
		if (reader.newestFirst) scroller?.scrollTo?.({ top: 0, behavior: 'smooth' });
	}
</script>

<div class="relative min-h-0 flex-1 overflow-y-auto" bind:this={scroller} onscroll={scrolled}>
	<!-- New arrivals wait here, so the list does not jump under the reader. -->
	{#if reader.pending > 0}
		<div class="sticky top-2 z-10 flex h-0 justify-center">
			<button
				type="button"
				class="h-7 cursor-pointer rounded-full bg-accent px-4 text-sm font-semibold text-on-accent shadow-md hover:brightness-110"
				onclick={showNew}
			>
				{m.toolbar.newArticles(reader.pending)}
			</button>
		</div>
	{/if}

	{#if reader.visible.length === 0}
		<div class="p-10 text-center text-soft">
			{#if reader.loading}
				<p role="status">{m.list.loadingList}</p>
			{:else if reader.query.trim()}
				<p>{m.list.emptySearch}</p>
			{:else if feedIsEmpty}
				<p>{m.list.emptyFeed}</p>
			{:else if reader.unreadOnly && reader.stream.kind !== 'starred' && reader.stream.kind !== 'hidden'}
				<p>{m.list.emptyUnread}</p>
				<button type="button" class="tool mt-3" onclick={() => (reader.unreadOnly = false)}>
					{m.list.showAll}
				</button>
			{:else}
				<p>{m.list.empty}</p>
			{/if}
		</div>
	{:else if expanded}
		<div class="mx-auto max-w-4xl space-y-3 p-3">
			{#each reader.visible as article (article.id)}
				<!-- A click picks the card, as j and k do from the keyboard. -->
				<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
				<article
					data-article={article.id}
					class={[
						'scroll-mt-3 rounded-md border bg-paper',
						reader.cursor === article.id ? 'border-accent shadow-sm' : 'border-line'
					]}
					onclick={() => reader.select(article)}
				>
					<ArticleBody {reader} {article} {now} {onversions} />
				</article>
			{/each}
		</div>
	{:else}
		<ul class="divide-y divide-line border-b border-line">
			{#each reader.visible as article (article.id)}
				<li data-article={article.id}>
					<ArticleRow {reader} {article} {now} {showFeed} />
					{#if reader.open === article.id}
						<div class="border-t border-l-3 border-line border-l-accent bg-paper">
							<ArticleBody {reader} {article} {now} {onversions} />
						</div>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}

	{#if reader.hasMore}
		<div class="flex justify-center p-4">
			<button type="button" class="tool" use:whenSeen onclick={() => reader.showMore()}>
				{m.list.more}
			</button>
		</div>
	{/if}
</div>
