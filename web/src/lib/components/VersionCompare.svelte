<script lang="ts">
	// What changed in an article since an earlier fetch: the words taken out and the
	// words put in, between the picked earlier version and the current one.
	import { untrack } from 'svelte';
	import { diffWords } from '#lib/diff.ts';
	import { textOf } from '#lib/filters.ts';
	import { formatFull } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { Article } from '#lib/model.ts';
	import Dialog from './Dialog.svelte';

	let { article, onclose }: { article: Article; onclose: () => void } = $props();

	// The newest of the earlier versions first: that is the latest change.
	let rev = $state(untrack(() => article.versions.at(-1)?.rev ?? 0));

	const earlier = $derived(article.versions.find((version) => version.rev === rev));
	const parts = $derived(earlier ? diffWords(textOf(earlier.html), textOf(article.html)) : []);
	const changed = $derived(parts.some((part) => part.kind !== 'same'));
</script>

<Dialog title={m.versions.title} wide {onclose}>
	<h3 class="text-lg font-semibold">{article.title}</h3>

	<label class="mt-3 block text-sm">
		<span class="font-medium">{m.versions.compareWith}</span>
		<select class="mt-1 block rounded py-1 text-sm" bind:value={rev}>
			{#each article.versions.toReversed() as version (version.rev)}
				<option value={version.rev}>
					{m.versions.version(version.rev)}, {m.versions.fetched(formatFull(version.fetchedAt))}
				</option>
			{/each}
		</select>
	</label>

	{#if earlier}
		{#if earlier.title !== article.title}
			<p class="mt-3 text-sm">
				{m.versions.titleWas}
				<del class="rounded bg-removed px-1">{earlier.title}</del>
			</p>
		{/if}

		<p class="mt-3 flex gap-3 text-xs text-soft">
			<span
				><ins class="rounded bg-added px-1 text-ink no-underline">{m.versions.legendAdded}</ins
				></span
			>
			<span><del class="rounded bg-removed px-1 text-ink">{m.versions.legendRemoved}</del></span>
		</p>

		{#if changed}
			<p class="article-body mt-2 rounded-md border border-line p-3">
				{#each parts as part, index (index)}
					{#if part.kind === 'added'}
						<ins class="rounded bg-added no-underline">{part.text}</ins>
					{:else if part.kind === 'removed'}
						<del class="rounded bg-removed">{part.text}</del>
					{:else}
						{part.text}
					{/if}
				{/each}
			</p>
		{:else}
			<p class="mt-2 text-sm text-soft">{m.versions.same}</p>
		{/if}
	{/if}
</Dialog>
