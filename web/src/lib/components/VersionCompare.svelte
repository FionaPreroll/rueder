<script lang="ts">
	// What changed in an article since an earlier fetch: the words taken out and the
	// words put in, between the picked earlier version and the current one, and the
	// pictures and links, which the words do not show.
	import { untrack } from 'svelte';
	import { diffMedia, diffWords, splitOff } from '#lib/diff.ts';
	import { textOf } from '#lib/filters.ts';
	import { formatFull } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { Article, ArticleVersion } from '#lib/model.ts';
	import Dialog from './Dialog.svelte';

	let {
		article,
		versions,
		onclose
	}: {
		article: Article;
		/** The article's earlier versions, oldest first. */
		versions: ArticleVersion[];
		onclose: () => void;
	} = $props();

	// The newest of the earlier versions first: that is the latest change.
	let rev = $state(untrack(() => versions.at(-1)?.rev ?? 0));

	const earlier = $derived(versions.find((version) => version.rev === rev));
	const parts = $derived(
		earlier ? diffWords(textOf(earlier.html), textOf(article.html ?? '')) : []
	);
	const changed = $derived(parts.some((part) => part.kind !== 'same'));
	const media = $derived(earlier ? diffMedia(earlier.html, article.html ?? '') : null);
	const pictures = $derived(
		media && media.pictures.removed.length + media.pictures.added.length > 0 ? media.pictures : null
	);
	const links = $derived(
		media && media.links.removed.length + media.links.added.length > 0 ? media.links : null
	);
	// The pictures only the earlier version has beside those only the current one has,
	// each against the one in its place on the other side, for what changed in its address.
	const sides = $derived(
		pictures && earlier
			? [
					{
						label: m.versions.version(earlier.rev),
						shown: pictures.removed,
						others: pictures.added,
						tone: 'bg-removed'
					},
					{
						label: m.versions.current,
						shown: pictures.added,
						others: pictures.removed,
						tone: 'bg-added'
					}
				]
			: []
	);
</script>

<Dialog title={m.versions.title} wide {onclose}>
	<h3 class="text-lg font-semibold">{article.title}</h3>

	<label class="mt-3 block text-sm">
		<span class="font-medium">{m.versions.compareWith}</span>
		<select class="mt-1 block w-full max-w-full rounded py-1 text-sm sm:w-auto" bind:value={rev}>
			{#each versions.toReversed() as version (version.rev)}
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
		{:else if pictures || links}
			<p class="mt-2 text-sm text-soft">{m.versions.sameWords}</p>
		{:else if earlier.html !== article.html}
			<p class="mt-2 text-sm text-soft">{m.versions.markup}</p>
		{:else}
			<p class="mt-2 text-sm text-soft">{m.versions.same}</p>
		{/if}

		{#if pictures}
			<h4 class="mt-4 text-sm font-medium">{m.versions.pictures}</h4>
			<div class="mt-2 grid grid-cols-2 gap-3 text-xs">
				{#each sides as side (side.tone)}
					<div class="min-w-0">
						<p class="text-soft">{side.label}</p>
						{#each side.shown as picture, index (index)}
							{@const [same, own, end] = splitOff(
								picture.source,
								side.others[index]?.source ?? picture.source
							)}
							<figure class={['mt-1 rounded p-1', side.tone]}>
								<img class="max-h-48 w-full object-contain" src={picture.url} alt={picture.title} />
								<figcaption class="text-ink">
									{#if picture.title}
										<p class="mt-1 break-words" dir="auto">{picture.title}</p>
									{/if}
									<!-- Where it came from, with what differs from the picture in its place on the other side marked. -->
									<p class="mt-1 font-mono wrap-anywhere" dir="ltr">
										<span class="sr-only">{m.versions.address}</span>
										{same}{#if own}<mark class="rounded bg-paper px-0.5 font-semibold text-ink"
												>{own}</mark
											>{/if}{end}
									</p>
								</figcaption>
							</figure>
						{:else}
							<p class="mt-1 text-soft">{m.versions.noPicture}</p>
						{/each}
					</div>
				{/each}
			</div>
		{/if}

		{#if links}
			<h4 class="mt-4 text-sm font-medium">{m.versions.links}</h4>
			<ul class="mt-1 space-y-1 text-sm break-all">
				{#each links.removed as link, index ('removed' + index)}
					<li dir="auto"><del class="rounded bg-removed px-1">{link}</del></li>
				{/each}
				{#each links.added as link, index ('added' + index)}
					<li dir="auto"><ins class="rounded bg-added px-1 no-underline">{link}</ins></li>
				{/each}
			</ul>
		{/if}
	{/if}
</Dialog>
