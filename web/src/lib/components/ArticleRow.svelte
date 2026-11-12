<script lang="ts">
	// One line of the list view: star, feed, title with the first words of the text,
	// time. Unread is bold on paper, read is quiet on gray, as in Google Reader.
	// Stacked, for the narrow list beside the article in the view of three panes,
	// it puts feed and time over the title and the first words under it, and a
	// click opens the article in its pane instead of under the row.
	import type { Reader } from '#lib/reader.ts';
	import { formatFull, formatWhen } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { Article } from '#lib/model.ts';
	import { settings } from '#lib/settings.svelte.ts';
	import Icon from './Icon.svelte';

	let {
		reader,
		article,
		now,
		showFeed,
		stacked = false
	}: {
		reader: Reader;
		article: Article;
		now: number;
		showFeed: boolean;
		stacked?: boolean;
	} = $props();

	const read = $derived(reader.isRead(article));
	const starred = $derived(reader.isStarred(article));
	const labels = $derived(
		reader.labels.filter((label) => reader.labelsOf(article).includes(label.id))
	);
	const feed = $derived(reader.feeds[article.feedId]);
	const opened = $derived(reader.open === article.id);
</script>

{#snippet star()}
	<button
		type="button"
		class={['plain size-8 shrink-0', starred && 'text-star hover:text-star']}
		aria-pressed={starred}
		aria-label={starred ? m.list.unstar : m.list.star}
		title={starred ? m.list.unstar : m.list.star}
		onclick={() => reader.toggleStar(article)}
	>
		<Icon name="star" filled={starred} />
	</button>
{/snippet}

{#snippet dots()}
	{#each labels as label (label.id)}
		<span
			class="size-2 shrink-0 self-center rounded-full"
			style:background-color={label.color}
			title={label.title}
			aria-label={m.labels.set(label.title)}
			role="img"
		></span>
	{/each}
	{#if article.changes > 0}
		<span
			class="shrink-0 rounded border border-line px-1 text-xs text-soft"
			title={m.list.updatedTitle(article.changes)}
		>
			{m.list.updated}
		</span>
	{/if}
{/snippet}

{#snippet when()}
	<time
		class="shrink-0 text-xs text-soft tabular-nums"
		datetime={new Date(article.publishedAt).toISOString()}
		title={formatFull(article.publishedAt)}
	>
		{formatWhen(article.publishedAt, now)}
	</time>
{/snippet}

{#if stacked}
	<div
		class={[
			'flex items-start border-l-3 pr-3',
			reader.cursor === article.id ? 'border-l-accent' : 'border-l-transparent',
			opened ? 'bg-chosen' : read ? 'bg-panel' : 'bg-paper'
		]}
	>
		{@render star()}
		<button
			type="button"
			class="min-w-0 flex-1 cursor-pointer py-(--row-pad) text-left text-sm"
			aria-current={opened ? 'true' : undefined}
			onclick={() => reader.select(article)}
		>
			<span class="flex items-baseline gap-2">
				<span class={['min-w-0 flex-1 truncate text-xs', read ? 'text-soft' : 'text-ink']}>
					{showFeed ? feed?.title : article.author}
				</span>
				{@render dots()}
				{@render when()}
			</span>
			<span class={['mt-0.5 line-clamp-2', read ? 'text-soft' : 'font-semibold text-ink']}>
				{article.title}
			</span>
			{#if settings.teasers && article.teaser}
				<span class="mt-0.5 line-clamp-2 text-xs text-soft">{article.teaser}</span>
			{/if}
		</button>
	</div>
{:else}
	<div
		class={[
			'flex items-center border-l-3 pr-3',
			reader.cursor === article.id ? 'border-l-accent' : 'border-l-transparent',
			read ? 'bg-panel' : 'bg-paper'
		]}
	>
		{@render star()}
		<button
			type="button"
			class="flex min-w-0 flex-1 cursor-pointer items-baseline gap-3 py-(--row-pad) text-left text-sm"
			aria-expanded={reader.open === article.id}
			onclick={() => reader.toggleOpen(article)}
		>
			{#if showFeed}
				<span class={['hidden w-40 shrink-0 truncate md:block', read ? 'text-soft' : 'text-ink']}>
					{feed?.title}
				</span>
			{/if}
			<span class="min-w-0 flex-1 truncate">
				<span class={read ? 'text-soft' : 'font-semibold text-ink'}>{article.title}</span>
				{#if settings.teasers}
					<span class="text-soft"> – {article.teaser}</span>
				{/if}
			</span>
			{@render dots()}
			{@render when()}
		</button>
	</div>
{/if}
