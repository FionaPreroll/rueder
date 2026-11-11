<script lang="ts">
	// The start page, after Google Reader's: what is new in each subscription, and
	// beside it the latest stars, feeds whose fetch fails, and a few tips.
	import type { Reader } from '#lib/reader.ts';
	import { formatAgo } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import { formatNumber } from '#lib/i18n/intl.ts';
	import type { Article } from '#lib/model.ts';
	import FeedIcon from './FeedIcon.svelte';
	import Icon from './Icon.svelte';

	let { reader, now }: { reader: Reader; now: number } = $props();

	const fresh = $derived(reader.fresh);
	const starred = $derived(reader.latestStarred);

	const broken = $derived(Object.values(reader.feeds).filter((feed) => feed.error));

	function open(article: Article) {
		reader.show({ kind: 'feed', id: article.feedId });
		reader.select(article);
	}
</script>

<div class="min-h-0 flex-1 overflow-y-auto">
	<div class="mx-auto grid max-w-6xl gap-8 p-4 sm:p-6 lg:grid-cols-[minmax(0,1fr)_18rem]">
		<section>
			<h1 class="text-sm font-semibold tracking-wide text-soft uppercase">{m.home.newIn}</h1>
			{#each fresh as { feed, count, top } (feed.id)}
				<div class="mt-5">
					<h2 class="flex items-center gap-2 text-lg font-semibold">
						<FeedIcon {feed} />
						<button
							type="button"
							class="cursor-pointer text-link hover:underline"
							title={m.home.showFeed(feed.title)}
							onclick={() => reader.show({ kind: 'feed', id: feed.id })}
						>
							{feed.title}
						</button>
						<span class="font-normal text-soft">({formatNumber(count)})</span>
					</h2>
					<ul class="mt-1 space-y-2">
						{#each top as article (article.id)}
							<li>
								<button
									type="button"
									class="cursor-pointer text-left text-sm font-semibold wrap-anywhere hover:underline"
									onclick={() => open(article)}
								>
									{article.title}
								</button>
								<p class="line-clamp-2 text-sm text-soft">{article.teaser}</p>
							</li>
						{/each}
					</ul>
				</div>
			{:else}
				<p class="mt-5 text-soft" role={reader.loading ? 'status' : undefined}>
					{reader.loading ? m.list.loadingList : m.home.allRead}
				</p>
			{/each}
		</section>

		<aside class="space-y-6 text-sm">
			<section>
				<h2 class="font-semibold">{m.home.recentlyStarred}</h2>
				<ul class="mt-2 space-y-2">
					{#each starred as article (article.id)}
						<li>
							<button
								type="button"
								class="cursor-pointer text-left wrap-anywhere text-link hover:underline"
								onclick={() => open(article)}
							>
								{article.title}
							</button>
							<p class="text-xs text-soft">
								{reader.feeds[article.feedId]?.title} · {formatAgo(article.publishedAt, now)}
							</p>
						</li>
					{:else}
						<li class="text-soft">{m.home.noStars}</li>
					{/each}
				</ul>
			</section>

			{#if broken.length > 0}
				<section>
					<h2 class="font-semibold">{m.home.problems}</h2>
					<ul class="mt-2 space-y-2">
						{#each broken as feed (feed.id)}
							<li class="flex gap-1.5">
								<span class="mt-0.5 text-warn"><Icon name="warning" /></span>
								<span>
									<button
										type="button"
										class="cursor-pointer text-link hover:underline"
										onclick={() => reader.show({ kind: 'feed', id: feed.id })}
									>
										{feed.title}
									</button>
									<span class="block text-xs text-soft">{m.nav.fetchFailed(feed.error ?? '')}</span>
								</span>
							</li>
						{/each}
					</ul>
				</section>
			{/if}

			<section class="rounded-md border border-line bg-panel p-3">
				<h2 class="font-semibold">{m.home.tips}</h2>
				<ul class="mt-2 list-disc space-y-1.5 pl-4 text-soft">
					<li>{m.home.tipKeys}</li>
					<li>{m.home.tipDrag}</li>
					<li>{m.home.tipFilters}</li>
				</ul>
			</section>
		</aside>
	</div>
</div>
