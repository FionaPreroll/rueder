<script lang="ts">
	// The magazine view: the first article large, with its picture over the whole
	// width, and the others as tiles in a grid, each with its picture, its title and
	// the first words of its text. A tile without a picture gives its room to more
	// of the text. A click opens the article over the grid.
	import type { Reader } from '#lib/reader.ts';
	import { formatFull, formatWhen } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { Article } from '#lib/model.ts';
	import Icon from './Icon.svelte';

	let { reader, now, articles }: { reader: Reader; now: number; articles: Article[] } = $props();

	/** How far down the list the large place looks for an article with a picture. */
	const LEAD_FROM = 6;

	// The large place goes to the first of the newest articles that has a picture,
	// else to the first; the tiles keep the order of the list.
	const lead = $derived(
		articles.slice(0, LEAD_FROM).find((article) => article.thumbnail) ?? articles[0]
	);
	const tiles = $derived(articles.filter((article) => article !== lead));

	// The list's picture, as the proxy makes it for lists: up to 800 pixels wide.
	const leadPicture = $derived(lead ? (lead.thumbnail ?? lead.image?.url ?? null) : null);
</script>

{#snippet star(article: Article)}
	{@const starred = reader.isStarred(article)}
	<button
		type="button"
		class={[
			'plain size-8 shrink-0 rounded-full bg-paper/80',
			starred && 'text-star hover:text-star'
		]}
		aria-pressed={starred}
		aria-label={starred ? m.list.unstar : m.list.star}
		title={starred ? m.list.unstar : m.list.star}
		onclick={() => reader.toggleStar(article)}
	>
		<Icon name="star" filled={starred} />
	</button>
{/snippet}

{#snippet byline(article: Article)}
	<span class="flex items-baseline gap-2 text-xs text-soft">
		<span class="min-w-0 truncate">{reader.feeds[article.feedId]?.title}</span>
		<time
			class="shrink-0 tabular-nums"
			datetime={new Date(article.publishedAt).toISOString()}
			title={formatFull(article.publishedAt)}
		>
			{formatWhen(article.publishedAt, now)}
		</time>
	</span>
{/snippet}

{#snippet picture(url: string | null | undefined, big: boolean)}
	{#if url}
		<img
			src={url}
			alt=""
			loading={big ? 'eager' : 'lazy'}
			class={['dimmed w-full object-cover', big ? 'aspect-[21/9] max-h-[26rem]' : 'aspect-video']}
		/>
	{/if}
{/snippet}

<div class="mx-auto max-w-6xl p-3 sm:p-4">
	{#if lead}
		{@const read = reader.isRead(lead)}
		<article
			data-article={lead.id}
			class={[
				'relative mb-4 overflow-hidden rounded-md border bg-paper',
				reader.cursor === lead.id ? 'border-accent-line shadow-sm' : 'border-line',
				read && 'opacity-75'
			]}
		>
			<button
				type="button"
				class="block w-full cursor-pointer text-left"
				onclick={() => reader.select(lead)}
			>
				{#if leadPicture}{@render picture(leadPicture, true)}{/if}
				<!-- Room for the star, which sits over the text where there is no picture. -->
				<span class="block p-4 pr-12">
					{@render byline(lead)}
					<span
						class={[
							'mt-1 block text-2xl leading-snug sm:text-3xl',
							read ? 'text-soft' : 'font-semibold text-ink'
						]}
					>
						{lead.title}
					</span>
					{#if lead.teaser}
						<span class="mt-2 line-clamp-3 block text-soft">{lead.teaser}</span>
					{/if}
				</span>
			</button>
			<span class="absolute top-2 right-2">{@render star(lead)}</span>
		</article>
	{/if}

	<ul class="grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-4">
		{#each tiles as article (article.id)}
			{@const read = reader.isRead(article)}
			<li
				data-article={article.id}
				class={[
					'relative overflow-hidden rounded-md border bg-paper',
					reader.cursor === article.id ? 'border-accent-line shadow-sm' : 'border-line',
					read && 'opacity-75'
				]}
			>
				<button
					type="button"
					class="flex h-full w-full cursor-pointer flex-col justify-start text-left"
					onclick={() => reader.select(article)}
				>
					{@render picture(article.thumbnail, false)}
					<!-- Without a picture the star sits over the text, so the text keeps clear of it. -->
					<span class={['block p-3', !article.thumbnail && 'pr-12']}>
						{@render byline(article)}
						<span
							class={[
								'mt-1 line-clamp-3 block leading-snug',
								read ? 'text-soft' : 'font-semibold text-ink'
							]}
						>
							{article.title}
						</span>
						{#if article.teaser}
							<span
								class={[
									'mt-1 block text-sm text-soft',
									article.thumbnail ? 'line-clamp-2' : 'line-clamp-6'
								]}
							>
								{article.teaser}
							</span>
						{/if}
					</span>
				</button>
				<span class="absolute top-2 right-2">{@render star(article)}</span>
			</li>
		{/each}
	</ul>
</div>
