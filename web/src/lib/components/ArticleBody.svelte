<script lang="ts">
	// An open article: title to the original, where it is from, the text, and the row
	// of actions under it. Used inside the list view, in the pane beside the list,
	// over the magazine and as a card of the expanded view. Where it was opened, and
	// so can be closed, the row ends with a button that closes it (`closable`): read
	// to its end, the reader need not go back up to its line for that.
	import type { Reader } from '#lib/reader.ts';
	import { domainOf, formatAgo, formatFull } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import { nextLabelColor, type Article } from '#lib/model.ts';
	import { ruleText } from '#lib/rule-text.ts';
	import ArticleInfoDialog from './ArticleInfoDialog.svelte';
	import Badge from './Badge.svelte';
	import Enclosures from './Enclosures.svelte';
	import Icon from './Icon.svelte';
	import Marked from './Marked.svelte';
	import Menu from './Menu.svelte';
	import ReportDialog from './ReportDialog.svelte';

	let {
		reader,
		article,
		now,
		onversions,
		closable = false
	}: {
		reader: Reader;
		article: Article;
		now: number;
		onversions: (article: Article) => void;
		closable?: boolean;
	} = $props();

	const read = $derived(reader.isRead(article));
	const starred = $derived(reader.isStarred(article));
	const labelled = $derived(new Set(reader.labelsOf(article)));
	const feed = $derived(reader.feeds[article.feedId]);
	// Everything in the list of what the filters hid is hidden, whether its mark has come yet or not.
	const rule = $derived(
		reader.isHidden(article) || reader.stream.kind === 'hidden'
			? reader.filters.hidingRule(article)
			: null
	);

	// Where the title leads, so the reader knows the site before going there.
	const domain = $derived(article.link ? domainOf(article.link) : null);

	// The picture a feed gives beside the text is shown where the text has none of its own.
	const lead = $derived(
		article.image && article.html !== null && !/<img\b/i.test(article.html) ? article.image : null
	);

	// A new label, named in a field beside the menu, goes on this article at once.
	let naming = $state(false);
	let name = $state('');
	let reporting = $state(false);
	let informing = $state(false);

	function addLabel(event: SubmitEvent) {
		event.preventDefault();
		const title = name.trim();
		if (title) reader.addLabel(title, nextLabelColor(reader.labels.length), article);
		naming = false;
		name = '';
	}

	const focus = (field: HTMLInputElement) => field.focus();

	// As Escape does. In the unread items a read article goes from the list with it.
	function close() {
		if (reader.open === article.id) reader.open = null;
	}

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
					class="reach text-ink hover:underline"
					href={article.link}
					title={m.list.openOriginal}
					target="_blank"
					rel="noopener noreferrer"
				>
					<Marked text={article.title} query={reader.query} />
				</a>
				{#if domain}
					<span class="ms-1 text-sm font-normal text-soft"><bdi>{domain}</bdi></span>
				{/if}
			{:else}
				<Marked text={article.title} query={reader.query} />
			{/if}
		</h2>
		<p class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-soft">
			<span>
				{m.list.from}
				<button
					type="button"
					class="reach cursor-pointer text-link hover:underline"
					title={m.list.showFeed}
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
				{@const headline = article.changed === 'title'}
				<Badge
					kind="hint"
					title={headline
						? m.list.updatedHeadlineTitle(article.changes)
						: m.list.updatedTitle(article.changes)}
					onclick={() => onversions(article)}
				>
					<Icon name="versions" class="size-3.5" />
					{headline ? m.list.updatedHeadline : m.list.updated}
				</Badge>
			{/if}
		</p>
		{#if rule}
			<p class="mt-2">
				<Badge kind="hint">
					<Icon name="hidden" class="size-3.5" />
					{m.list.hiddenBy(ruleText(rule))}
				</Badge>
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
		{#if article.textDropped}
			<!-- The line stayed, the text went: the original may still have it. -->
			<p class="mt-3 text-sm text-soft">
				{m.list.textDropped}
				{#if article.link}
					<a
						class="text-link hover:underline"
						href={article.link}
						target="_blank"
						rel="noopener noreferrer">{m.list.textDroppedOriginal}</a
					>
				{/if}
			</p>
		{:else}
			<div class="article-body mt-3" dir="auto">{@html article.html}</div>
		{/if}
		{#if article.enclosures.length > 0}
			<Enclosures enclosures={article.enclosures} />
		{/if}
	{/if}

	<!-- What is done with the article wraps; its menu and the close stay together at the end of the first line. -->
	<footer class="mt-4 flex items-start gap-x-4 border-t border-line pt-3 text-sm text-soft">
		<div class="flex min-w-0 flex-1 flex-wrap items-center gap-x-4 gap-y-2">
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
			<Menu class="inline-flex cursor-pointer items-center gap-1 hover:text-ink">
				{#snippet button()}
					<Icon name="tag" />
					{m.labels.forArticle}
				{/snippet}
				{#each reader.labels as label (label.id)}
					<button
						type="button"
						role="menuitemcheckbox"
						aria-checked={labelled.has(label.id)}
						class="menu-item flex items-center gap-2"
						onclick={() => reader.setLabel(article, label.id, !labelled.has(label.id))}
					>
						<span class="size-2.5 rounded-full" style:background-color={label.color}></span>
						<span class="flex-1">{label.title}</span>
						{#if labelled.has(label.id)}<Icon name="check" class="size-4" />{/if}
					</button>
				{/each}
				{#if reader.labels.length > 0}<hr class="my-1 border-line" />{/if}
				<button
					type="button"
					role="menuitem"
					class="menu-item flex items-center gap-2"
					onclick={() => (naming = true)}
				>
					<Icon name="plus" class="size-4" />
					{m.labels.add}
				</button>
			</Menu>
			{#if naming}
				<form onsubmit={addLabel}>
					<input
						class="w-40 rounded px-2 py-0 text-sm"
						placeholder={m.labels.name}
						aria-label={m.labels.name}
						bind:value={name}
						use:focus
						onblur={() => (naming = false)}
						onkeydown={(event) => {
							if (event.key !== 'Escape') return;
							event.stopPropagation();
							naming = false;
						}}
					/>
				</form>
			{/if}
			{#each reader.labels.filter((label) => labelled.has(label.id)) as label (label.id)}
				<Badge kind="label" color={label.color} label={m.labels.set(label.title)}>
					{label.title}
				</Badge>
			{/each}
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
					class="inline-flex items-center gap-1 hover:text-ink pointer-coarse:min-h-11"
					href={article.link}
					target="_blank"
					rel="noopener noreferrer"
				>
					<Icon name="external" />
					{m.list.openOriginal}
				</a>
			{/if}
			<span class="ms-auto flex gap-1">
				{#each article.tags as tag (tag)}
					<Badge kind="tag">{tag}</Badge>
				{/each}
			</span>
		</div>
		<div class="flex shrink-0 items-center gap-x-4">
			<Menu
				class="inline-flex cursor-pointer items-center hover:text-ink"
				align="end"
				label={m.list.articleActions}
			>
				{#snippet button()}
					<Icon name="more" />
				{/snippet}
				<button type="button" role="menuitem" class="menu-item" onclick={() => (informing = true)}>
					{m.list.articleInfo}
				</button>
				{#if reader.can.reportContent}
					<button
						type="button"
						role="menuitem"
						class="menu-item"
						onclick={() => (reporting = true)}
					>
						{m.list.reportArticle}
					</button>
				{/if}
			</Menu>
			{#if closable}
				<button
					type="button"
					class="inline-flex cursor-pointer items-center gap-1 hover:text-ink"
					onclick={close}
				>
					<Icon name="close" />
					<!-- On a phone the cross alone, so that it fits beside the menu. -->
					<span class="max-sm:sr-only">{m.common.close}</span>
				</button>
			{/if}
		</div>
	</footer>
	{#if informing}
		<ArticleInfoDialog {reader} {article} onclose={() => (informing = false)} />
	{/if}
	{#if reporting}
		<ReportDialog
			{reader}
			feedId={article.feedId}
			seq={article.seq}
			title={article.title}
			onclose={() => (reporting = false)}
		/>
	{/if}
</div>
