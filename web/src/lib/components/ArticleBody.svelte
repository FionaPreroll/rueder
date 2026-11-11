<script lang="ts">
	// An open article: title to the original, where it is from, the text, and the row
	// of actions under it. Used inside the list view and as a card of the expanded view.
	import type { Reader } from '#lib/reader.ts';
	import { formatAgo, formatFull } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { Article } from '#lib/model.ts';
	import { ruleText } from '#lib/rule-text.ts';
	import Enclosures from './Enclosures.svelte';
	import Icon from './Icon.svelte';

	let {
		reader,
		article,
		now,
		onversions
	}: {
		reader: Reader;
		article: Article;
		now: number;
		onversions: (article: Article) => void;
	} = $props();

	const read = $derived(reader.isRead(article));
	const starred = $derived(reader.isStarred(article));
	const feed = $derived(reader.feeds[article.feedId]);
	const rule = $derived(reader.isHidden(article) ? reader.filters.hidingRule(article) : null);

	// The picture a feed gives beside the text is shown where the text has none of its own.
	const lead = $derived(
		article.image && article.html !== null && !/<img\b/i.test(article.html) ? article.image : null
	);

	// A list comes without the texts; the text is fetched when the article is shown.
	$effect(() => {
		if (article.html === null) reader.load(article);
	});
</script>

<div class="px-4 pt-3 pb-4 sm:px-6">
	<header>
		<h2 class="text-xl leading-snug font-semibold">
			{#if article.link}
				<a
					class="text-link hover:underline"
					href={article.link}
					target="_blank"
					rel="noopener noreferrer"
				>
					{article.title}
				</a>
			{:else}
				{article.title}
			{/if}
		</h2>
		<p class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-soft">
			<span>
				{m.list.from}
				<button
					type="button"
					class="cursor-pointer text-link hover:underline"
					onclick={() => reader.show({ kind: 'feed', id: article.feedId })}
				>
					{feed?.title}
				</button>
				{#if article.author}{m.list.by(article.author)}{/if}
			</span>
			<span aria-hidden="true">·</span>
			<time datetime={new Date(article.publishedAt).toISOString()}>
				{formatFull(article.publishedAt)} ({formatAgo(article.publishedAt, now)})
			</time>
			{#if article.changes > 0}
				<button
					type="button"
					class="inline-flex cursor-pointer items-center gap-1 rounded border border-line px-1.5 text-xs hover:bg-hover hover:text-ink"
					title={m.list.updatedTitle(article.changes)}
					onclick={() => onversions(article)}
				>
					<Icon name="versions" class="size-3.5" />
					{m.list.updated}
				</button>
			{/if}
		</p>
		{#if rule}
			<p class="mt-2 inline-flex items-center gap-1.5 rounded bg-panel px-2 py-1 text-sm">
				<Icon name="hidden" />
				{m.list.hiddenBy(ruleText(rule))}
			</p>
		{/if}
	</header>

	{#if article.html === null}
		<p class="mt-3 text-sm text-soft" role="status">{m.list.loading}</p>
	{:else}
		<!-- The text comes cleaned from the server; with made-up data it is made up in the browser. -->
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{#if lead}
			<img
				class="mt-3 max-h-96 max-w-full rounded-md"
				src={lead.url}
				alt={lead.title}
				title={lead.title || undefined}
				loading="lazy"
			/>
		{/if}
		<div class="article-body mt-3">{@html article.html}</div>
		{#if article.enclosures.length > 0}
			<Enclosures enclosures={article.enclosures} />
		{/if}
	{/if}

	<footer
		class="mt-4 flex flex-wrap items-center gap-x-4 gap-y-2 border-t border-line pt-3 text-sm text-soft"
	>
		<button
			type="button"
			class={[
				'inline-flex cursor-pointer items-center gap-1 hover:text-ink',
				starred && 'text-star'
			]}
			aria-pressed={starred}
			onclick={() => reader.toggleStar(article)}
		>
			<Icon name="star" filled={starred} />
			<span class="text-soft">{starred ? m.list.unstar : m.list.star}</span>
		</button>
		<label class="inline-flex cursor-pointer items-center gap-1.5 hover:text-ink">
			<input
				type="checkbox"
				class="rounded text-accent"
				checked={!read}
				onchange={(event) => reader.setRead(article, !event.currentTarget.checked)}
			/>
			{m.list.keepUnread}
		</label>
		{#if article.changes > 0}
			<button
				type="button"
				class="inline-flex cursor-pointer items-center gap-1 hover:text-ink"
				onclick={() => onversions(article)}
			>
				<Icon name="versions" />
				{m.list.compare}
			</button>
		{/if}
		{#if article.link}
			<a
				class="inline-flex items-center gap-1 hover:text-ink"
				href={article.link}
				target="_blank"
				rel="noopener noreferrer"
			>
				<Icon name="external" />
				{m.list.openOriginal}
			</a>
		{/if}
		<span class="ml-auto flex gap-1">
			{#each article.tags as tag (tag)}
				<span class="rounded bg-panel px-1.5 py-0.5 text-xs">{tag}</span>
			{/each}
		</span>
	</footer>
</div>
