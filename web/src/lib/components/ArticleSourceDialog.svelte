<script lang="ts">
	// The source of an article for a reader who wants to look under it: as rueder
	// keeps it after cleaning it, or as the feed sent it. It is only ever text here,
	// with its addresses as they are, so a tracking parameter the cleaning left shows.
	import { onMount } from 'svelte';
	import { m } from '#lib/i18n/index.ts';
	import type { Article } from '#lib/model.ts';
	import type { ArticleSource, Reader } from '#lib/reader.ts';
	import Choice from './Choice.svelte';
	import Dialog from './Dialog.svelte';

	let { reader, article, onclose }: { reader: Reader; article: Article; onclose: () => void } =
		$props();

	type Kind = 'cleaned' | 'original';

	let source = $state.raw<ArticleSource | null>(null);
	let failed = $state(false);
	let shown = $state<Kind>('cleaned');
	let copied = $state<Kind | null>(null);

	onMount(async () => {
		source = await reader.articleSource(article);
		failed = source === null;
	});

	const text = $derived(source && (shown === 'cleaned' ? source.cleaned : source.original));

	async function copy() {
		if (!text) return;
		try {
			await navigator.clipboard.writeText(text);
			copied = shown;
		} catch {
			// Without the clipboard the text is there to be marked and copied by hand.
		}
	}
</script>

<Dialog title={m.articleSource.title} wide {onclose}>
	{#if source}
		<p class="text-sm text-soft">{m.articleSource.text}</p>
		<div class="mt-4">
			<Choice
				label={m.articleSource.show}
				options={(['cleaned', 'original'] as const).map((kind) => ({
					value: kind,
					label: m.articleSource.kinds[kind]
				}))}
				bind:value={shown}
			/>
		</div>
		{#if text}
			<!-- A line for each tag where one follows another; the copy is the text exactly as it is. -->
			<pre
				class="mt-3 max-h-[60vh] overflow-auto rounded-md border border-line p-3 font-mono text-xs wrap-anywhere whitespace-pre-wrap"
				dir="ltr">{text.replaceAll('><', '>\n<')}</pre>
		{:else}
			<p class="mt-3 text-sm text-soft">
				{shown === 'cleaned' ? m.articleSource.noText : m.articleSource.noOriginal}
			</p>
		{/if}
		<div class="mt-4 flex gap-2">
			{#if text}
				<button type="button" class="primary" onclick={copy}>
					{copied === shown ? m.articleSource.copied : m.articleSource.copy}
				</button>
			{/if}
			<button type="button" class="tool" onclick={onclose}>{m.common.close}</button>
		</div>
	{:else if failed}
		<p class="text-sm text-warn" role="alert">{m.articleSource.unreachable}</p>
	{:else}
		<p class="text-sm text-soft">{m.articleSource.loading}</p>
	{/if}
</Dialog>
