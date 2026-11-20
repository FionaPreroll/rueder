<script lang="ts">
	// What the server knows of an article and its versions, for a reader who asks
	// for help with it: to read, and to copy into a message to the operator, with
	// the times as the server gives them.
	import { onMount } from 'svelte';
	import { formatFull } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import { formatNumber } from '#lib/i18n/intl.ts';
	import type { Article } from '#lib/model.ts';
	import type { ArticleInfo, Reader } from '#lib/reader.ts';
	import Dialog from './Dialog.svelte';

	let { reader, article, onclose }: { reader: Reader; article: Article; onclose: () => void } =
		$props();

	let info = $state.raw<ArticleInfo | null>(null);
	let failed = $state(false);
	let copied = $state(false);

	onMount(async () => {
		info = await reader.articleInfo(article);
		failed = info === null;
	});

	/** Label and value of each line; `time` writes the times. */
	function lines(info: ArticleInfo, time: (at: string) => string) {
		const texts = m.articleInfo;
		const changed =
			info.updated_at === null
				? texts.never
				: info.updated_what === null
					? time(info.updated_at)
					: texts.changedAt(time(info.updated_at), texts.what[info.updated_what]);
		return [
			[texts.feed, info.feed_title || String(info.feed_id)],
			[texts.feedNumber, String(info.feed_id)],
			[texts.number, String(info.seq)],
			[texts.name, info.title],
			[texts.id, info.guid],
			[texts.link, info.url ?? texts.noLink],
			[texts.published, time(info.published_at)],
			[texts.fetched, time(info.first_fetched_at)],
			[texts.changed, changed],
			[texts.version, formatNumber(info.version)],
			[texts.kept, formatNumber(info.versions_kept)],
			...(info.text_dropped ? [[texts.textLabel, texts.textDropped]] : [])
		];
	}

	const shown = $derived(info ? lines(info, (at) => formatFull(Date.parse(at))) : []);

	async function copy() {
		if (!info) return;
		const text = lines(info, (at) => at)
			.map(([label, value]) => `${label}: ${value}`)
			.join('\n');
		try {
			await navigator.clipboard.writeText(`${m.articleInfo.title}\n${text}\n`);
			copied = true;
		} catch {
			// Without the clipboard the lines are there to be marked and copied by hand.
		}
	}
</script>

<Dialog title={m.articleInfo.title} {onclose}>
	{#if info}
		<p class="text-sm text-soft">{m.articleInfo.text(article.title)}</p>
		<dl class="mt-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-sm">
			{#each shown as [label, value] (label)}
				<dt class="text-soft">{label}</dt>
				<!-- Titles, names and addresses from the feed run in their own direction. -->
				<dd class="min-w-0 break-words select-all" dir="auto">{value}</dd>
			{/each}
		</dl>
		<div class="mt-4 flex gap-2">
			<button type="button" class="primary" onclick={copy}>
				{copied ? m.articleInfo.copied : m.articleInfo.copy}
			</button>
			<button type="button" class="tool" onclick={onclose}>{m.common.close}</button>
		</div>
	{:else if failed}
		<p class="text-sm text-warn" role="alert">{m.articleInfo.unreachable}</p>
	{:else}
		<p class="text-sm text-soft">{m.articleInfo.loading}</p>
	{/if}
</Dialog>
