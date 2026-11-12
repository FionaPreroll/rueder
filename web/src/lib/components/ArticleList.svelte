<script lang="ts">
	// The articles of what is picked in the sidebar, in one of two views: a dense list
	// whose rows open in place, or every article in full, one card after the other.
	// A long list draws only what is near the view; the rest is space of its height.
	import { tick, untrack } from 'svelte';
	import type { Reader } from '#lib/reader.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { Article } from '#lib/model.ts';
	import { settings } from '#lib/settings.svelte.ts';
	import ArticleBody from './ArticleBody.svelte';
	import ArticleRow from './ArticleRow.svelte';
	import Icon from './Icon.svelte';
	import { offsets, rowsInSight } from '#lib/virtual.ts';

	/** Up to this many rows, or this many cards with their whole text, all are drawn. */
	const FEW_ROWS = 60;
	const FEW_CARDS = 12;
	/** How far beyond the view rows are drawn, in pixels. */
	const MARGIN = 1200;
	/** The height of a row or a card, as a guess before any was drawn. */
	const GUESS_ROW = 56;
	const GUESS_CARD = 480;

	let {
		reader,
		now,
		onversions
	}: { reader: Reader; now: number; onversions: (article: Article) => void } = $props();

	let scroller: HTMLElement | undefined = $state();
	/** Where the rows begin within what scrolls. */
	let list: HTMLElement | undefined = $state();
	let scrollTop = $state(0);
	let viewport = $state(0);

	/** The heights of the rows as they were drawn, by article. */
	let heights = $state.raw(new Map<string, number>());
	const measured = new Map<string, number>();
	let measuring = 0;

	/** Notes the height of a row, and makes the list count with it once per frame. */
	function note(id: string, height: number) {
		if (measured.get(id) === height) return;
		measured.set(id, height);
		measuring ||= requestAnimationFrame(() => {
			measuring = 0;
			heights = new Map(measured);
		});
	}

	function measure(node: HTMLElement, id: string) {
		const take = () => note(id, node.offsetHeight);
		take();
		const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(take);
		observer?.observe(node);
		return {
			update(next: string) {
				id = next;
				take();
			},
			destroy: () => observer?.disconnect()
		};
	}

	const expanded = $derived(settings.view === 'expanded');
	const showFeed = $derived(reader.stream.kind !== 'feed');
	const shownFeed = $derived(
		reader.stream.kind === 'feed' ? reader.feeds[reader.stream.id] : undefined
	);
	const feedIsEmpty = $derived(shownFeed?.lastSeq === 0);

	const rowOf = (id: string) => scroller?.querySelector<HTMLElement>(`[data-article="${id}"]`);

	const ids = $derived(reader.visible.map((article) => article.id));
	// What was not drawn yet is guessed to be as high as what was, on average.
	const guess = $derived.by(() => {
		let sum = 0;
		for (const height of heights.values()) sum += height;
		return heights.size > 0 ? sum / heights.size : expanded ? GUESS_CARD : GUESS_ROW;
	});
	const tops = $derived(offsets(ids, heights, guess));
	// Without a view to measure, as before the page is laid out, everything is drawn.
	const range = $derived(
		ids.length <= (expanded ? FEW_CARDS : FEW_ROWS) || viewport === 0
			? { start: 0, end: ids.length }
			: rowsInSight(tops, scrollTop - (list?.offsetTop ?? 0), viewport, MARGIN)
	);
	const shown = $derived(reader.visible.slice(range.start, range.end));
	const above = $derived(tops[range.start]);
	const below = $derived(tops[ids.length] - tops[range.end]);

	/** Scrolls to where an article that is not drawn would be, so that it is. */
	async function reach(id: string) {
		const at = ids.indexOf(id);
		if (!scroller || at === -1 || rowOf(id)) return;
		scroller.scrollTop = (list?.offsetTop ?? 0) + tops[at];
		scrollTop = scroller.scrollTop;
		await tick();
	}

	// A step with the keyboard or the arrows brings the article to the top of the pane.
	$effect(() => {
		if (reader.moved === 0) return;
		untrack(async () => {
			const id = reader.cursor;
			if (!id) return;
			await tick();
			await reach(id);
			rowOf(id)?.scrollIntoView({
				block: expanded || reader.open === id ? 'start' : 'nearest'
			});
		});
	});

	// In the list, an article that opens or closes starts in sight. Without this,
	// closing a long article, or opening the next one, which closes it, leaves the
	// view far below the row that was clicked, as the text above it is gone.
	let lastOpen: string | null = null;
	$effect(() => {
		const opened = reader.open;
		const id = opened ?? lastOpen;
		lastOpen = opened;
		if (!id || expanded) return;
		untrack(async () => {
			await tick();
			const row = rowOf(id);
			if (!row || !scroller) return;
			if (row.getBoundingClientRect().top < scroller.getBoundingClientRect().top) {
				row.scrollIntoView({ block: 'start' });
			}
		});
	});

	/** Once the reader is a screen down the list, a button leads back to its top. */
	const farDown = $derived(viewport > 0 && scrollTop > viewport);

	function toTop() {
		scroller?.scrollTo?.({ top: 0, behavior: 'smooth' });
	}

	// Another stream starts at its top, and another view measures its rows anew.
	let first = true;
	$effect(() => {
		void reader.stream;
		void expanded;
		untrack(() => {
			scroller?.scrollTo?.({ top: 0 });
			scrollTop = 0;
			// The rows drawn first were measured already.
			if (first) {
				first = false;
				return;
			}
			measured.clear();
			heights = new Map();
		});
	});

	// Once a frame: what is in sight, and in the expanded view, what scrolled out at
	// the top has been read.
	let frame = 0;
	function scrolled() {
		if (frame || !scroller) return;
		frame = requestAnimationFrame(() => {
			frame = 0;
			if (!scroller) return;
			scrollTop = scroller.scrollTop;
			if (!expanded || !settings.markOnScroll) return;
			const start = list?.offsetTop ?? 0;
			const articles = reader.visible;
			for (let i = 0; i < articles.length; i++) {
				if (start + tops[i + 1] > scrollTop) break;
				if (!reader.isRead(articles[i])) reader.scrolledPast(articles[i]);
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

<div class="relative flex min-h-0 flex-1 flex-col">
	<div
		class="relative min-h-0 flex-1 overflow-y-auto"
		bind:this={scroller}
		bind:clientHeight={viewport}
		onscroll={scrolled}
	>
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
				{:else if feedIsEmpty && shownFeed?.waiting}
					<p role="status">{m.list.waitingFeed}</p>
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
			<div class="mx-auto max-w-4xl px-3 pb-3" bind:this={list}>
				<div style:height="{above}px"></div>
				{#each shown as article (article.id)}
					<div class="pt-3" use:measure={article.id}>
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
					</div>
				{/each}
				<div style:height="{below}px"></div>
			</div>
		{:else}
			<div bind:this={list}>
				<div style:height="{above}px"></div>
				<ul class="divide-y divide-line border-b border-line">
					{#each shown as article (article.id)}
						<li data-article={article.id} use:measure={article.id}>
							<ArticleRow {reader} {article} {now} {showFeed} />
							{#if reader.open === article.id}
								<div class="border-t border-l-3 border-line border-l-accent bg-paper">
									<ArticleBody {reader} {article} {now} {onversions} />
								</div>
							{/if}
						</li>
					{/each}
				</ul>
				<div style:height="{below}px"></div>
			</div>
		{/if}

		{#if reader.hasMore}
			<div class="flex justify-center p-4">
				<button type="button" class="tool" use:whenSeen onclick={() => reader.showMore()}>
					{m.list.more}
				</button>
			</div>
		{/if}
	</div>

	{#if farDown}
		<button
			type="button"
			class="absolute right-4 bottom-[max(1rem,env(safe-area-inset-bottom))] z-20 flex size-12 cursor-pointer items-center justify-center rounded-full bg-accent text-on-accent shadow-lg hover:brightness-110"
			aria-label={m.list.toTop}
			title={m.list.toTop}
			onclick={toTop}
		>
			<Icon name="up" class="size-6" />
		</button>
	{/if}
</div>
