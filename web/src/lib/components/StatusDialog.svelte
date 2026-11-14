<script lang="ts">
	// The status of the server, for its admins: a few numbers at a glance and what
	// wants looking at, then, each to be opened, the reported feeds, the server, its
	// database, fetching, the feed search, the image proxy, the requests by route
	// and what the log said, counted over the last hour. It asks again every few
	// seconds while it is open.
	import { onMount } from 'svelte';
	import type { Durations, StatusHint } from '#lib/api/types.ts';
	import {
		formatAgo,
		formatBytes,
		formatDuration,
		formatFull,
		formatPercent
	} from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import { formatNumber } from '#lib/i18n/intl.ts';
	import type { Reader, ServerStatus } from '#lib/reader.ts';
	import Dialog from './Dialog.svelte';
	import Icon from './Icon.svelte';

	let { reader, onclose }: { reader: Reader; onclose: () => void } = $props();

	/** How often it asks again, in milliseconds. */
	const EVERY = 15_000;

	let status = $state.raw<ServerStatus | null>(null);
	let failed = $state(false);
	let asking = $state(false);
	/** When the status was asked for, for "as of" and "ago". */
	let at = $state(Date.now());

	async function ask() {
		if (asking) return;
		asking = true;
		const answer = await reader.status();
		asking = false;
		at = Date.now();
		failed = answer === null;
		if (answer) status = answer;
	}

	onMount(() => {
		void ask();
		const every = setInterval(() => void ask(), EVERY);
		return () => clearInterval(every);
	});

	const empty = m.status.empty;
	const ms = (value: number | null | undefined) => (value == null ? empty : formatDuration(value));
	const took = (durations: Durations) =>
		durations.count === 0 ? empty : m.status.took(ms(durations.p50_ms), ms(durations.p95_ms));
	const ago = (time: string) => formatAgo(Date.parse(time), at);

	function hint(h: StatusHint) {
		const hints = m.status.hints;
		switch (h.kind) {
			case 'slow_route':
				return hints.slowRoute(h.route, formatDuration(h.p95_ms));
			case 'server_errors':
				return hints.serverErrors(h.route, h.count);
			case 'pool_busy':
				return hints.poolBusy(h.size);
			case 'fetch_backlog':
				return hints.fetchBacklog(h.overdue, formatDuration(h.most_overdue_s * 1000));
			case 'feeds_failing':
				return hints.feedsFailing(h.failing, h.subscribed);
			case 'not_fetching':
				return hints.notFetching;
			case 'fetch_pace_full':
				return hints.fetchPaceFull(h.per_hour);
			case 'cache_misses':
				return hints.cacheMisses(formatPercent(h.hit_ratio));
			case 'long_query':
				return hints.longQuery(formatDuration(h.seconds * 1000));
			case 'image_cache_full':
				return hints.imageCacheFull(formatPercent(h.percent / 100));
			case 'errors':
				return hints.errors(h.count);
			case 'limited':
				return hints.limited(h.count);
			case 'browser_errors':
				return hints.browserErrors(h.errors);
			case 'reported_feeds':
				return hints.reportedFeeds(h.feeds);
		}
	}

	/** What this build is, as rows for the card: what is not known is left out. */
	function built(build: ServerStatus['build']): [string, string, string?][] {
		const b = m.status.build;
		const short = build.commit.slice(0, 12);
		const rows: [string, string, string?][] = [
			[b.commit, build.commit ? (build.dirty ? b.dirty(short) : short) : empty]
		];
		if (build.branch) rows.push([b.branch, build.branch]);
		if (build.tag) rows.push([b.tag, build.tag]);
		rows.push(
			[b.built, build.built_at ? formatFull(Date.parse(build.built_at)) : empty],
			[b.where, build.container ? b.container : b.host],
			[b.target, build.target || empty],
			[b.profile, b.profileValue(build.profile, build.opt_level)],
			[b.ci, build.ci_run ? b.ciValue : b.byHand, build.ci_run || undefined]
		);
		return rows;
	}

	/** A text cut where it names a command of `rueder`, so the command shows as code. */
	const commands = (text: string) =>
		text.split(/[“„](rueder [^”“]+)[”“]/).map((part, i) => ({ part, code: i % 2 === 1 }));

	const failure = (failing: ServerStatus['fetcher']['failing'][number]) => {
		const failures = m.status.fetcher.failures;
		return failing.failure === 'status'
			? failures.status(failing.status)
			: failures[failing.failure];
	};
</script>

<!-- The line that opens a part; a note in it stays in sight while the part is closed. -->
{#snippet opener(title: string, id?: string, note?: string)}
	<summary
		class="flex cursor-pointer flex-wrap items-center gap-x-2 rounded px-1 py-1 hover:bg-hover [&::-webkit-details-marker]:hidden"
	>
		<Icon
			name="right"
			class="size-4 shrink-0 text-soft transition-transform group-open:rotate-90"
		/>
		<h3 {id} class="text-sm font-semibold">{title}</h3>
		{#if note}<span class="basis-full pl-6 text-sm text-soft">{note}</span>{/if}
	</summary>
{/snippet}

{#snippet said(text: string)}
	{#each commands(text) as { part, code }, i (i)}
		{#if code}<code class="rounded bg-panel px-1 font-mono text-xs">{part}</code>{:else}{part}{/if}
	{/each}
{/snippet}

{#snippet facts(title: string, rows: [string, string, string?][])}
	<details class="group rounded border border-line p-2">
		{@render opener(title)}
		<dl class="mt-2 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 px-1 text-sm">
			{#each rows as [label, value, href] (label)}
				<dt class="text-soft">{label}</dt>
				<dd class="min-w-0 break-words">
					{#if href}
						<a class="underline" {href} target="_blank" rel="noopener noreferrer">{value}</a>
					{:else}
						{value}
					{/if}
				</dd>
			{/each}
		</dl>
	</details>
{/snippet}

<Dialog title={m.status.title} wide {onclose}>
	<div class="flex flex-wrap items-center gap-3">
		<p class="text-xs text-soft" role="status">
			{#if status}{m.status.asOf(formatFull(at))}{:else if !failed}{m.status.loading}{/if}
		</p>
		<button
			type="button"
			class="tool ml-auto aria-disabled:opacity-50"
			aria-disabled={asking}
			onclick={() => void ask()}
		>
			<Icon name="refresh" />
			{m.status.refresh}
		</button>
	</div>
	{#if reader.demo}
		<p class="mt-2 text-xs text-soft">{m.status.demo}</p>
	{/if}
	{#if failed}
		<p class="mt-3 text-sm text-warn" role="alert">{m.status.failed}</p>
	{/if}

	{#if status}
		{@const s = status}
		{@const failingFeeds = s.fetcher.failing.reduce((sum, failing) => sum + failing.feeds, 0)}
		<dl class="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-4" aria-label={m.status.summary.label}>
			{#each [[m.status.summary.hints, s.hints.length], [m.status.summary.failing, failingFeeds], [m.status.summary.errors, s.problems.errors], [m.status.summary.reported, s.reported.length]] as const as [label, count] (label)}
				<div class="rounded border border-line px-3 py-2">
					<dt class="text-xs text-soft">{label}</dt>
					<dd class={['text-2xl font-semibold tabular-nums', count > 0 && 'text-warn']}>
						{formatNumber(count)}
					</dd>
				</div>
			{/each}
		</dl>

		<section class="mt-4" aria-labelledby="status-hints">
			<h3 id="status-hints" class="text-sm font-semibold">{m.status.hintsTitle}</h3>
			{#if s.hints.length === 0}
				<p class="mt-1 flex items-center gap-2 text-sm text-soft">
					<Icon name="check" />{m.status.allWell}
				</p>
			{:else}
				<ul class="mt-1 space-y-1 text-sm">
					{#each s.hints as h, i (i)}
						<li class="flex items-start gap-2">
							<Icon name="warning" class="mt-0.5 size-4 shrink-0 text-warn" />
							<span>{hint(h)}</span>
						</li>
					{/each}
				</ul>
			{/if}
		</section>

		<details
			class="group mt-2 rounded border border-line p-2"
			open={s.reported.length > 0}
			aria-labelledby="status-reported"
		>
			{@render opener(m.status.reported.title, 'status-reported')}
			<p class="mt-1 text-sm text-soft">{@render said(m.status.reported.text)}</p>
			{#if s.reported.length === 0}
				<p class="mt-2 text-sm text-soft">{m.status.reported.none}</p>
			{:else}
				<ul class="mt-2 divide-y divide-line rounded border border-line">
					{#each s.reported as feed (feed.feed_id)}
						<li class="px-3 py-2 text-sm">
							<p class="flex flex-wrap items-baseline gap-x-2">
								<span class="font-semibold">{feed.title}</span>
								{#if feed.blocked}
									<span class="text-xs font-semibold text-warn">{m.status.reported.blocked}</span>
								{/if}
								<span
									class="ml-auto text-xs text-soft"
									title={formatFull(Date.parse(feed.last_at))}
								>
									{m.status.reported.reports(feed.reports, ago(feed.last_at))}
								</span>
							</p>
							<p class="mt-1 font-mono text-xs break-all">{feed.url}</p>
							<p class="mt-1 text-xs text-soft">
								{feed.reasons
									.map((r) => `${m.report.reasons[r.reason]} ${formatNumber(r.reports)}`)
									.join(' · ')}
							</p>
						</li>
					{/each}
				</ul>
			{/if}
		</details>

		<div class="mt-4 space-y-2">
			{@render facts(m.status.server.title, [
				[m.status.server.version, s.version],
				[m.status.server.running, ago(s.started_at)],
				[m.status.server.memory, s.memory_bytes == null ? empty : formatBytes(s.memory_bytes)],
				[m.status.server.accounts, formatNumber(s.counts.accounts)],
				[
					m.status.server.feeds,
					m.status.server.feedsValue(s.counts.subscribed_feeds, s.counts.feeds)
				],
				[m.status.server.articles, m.status.server.articlesValue(s.counts.articles)],
				[m.status.server.live, formatNumber(s.counts.live_connections)]
			])}
			{@render facts(m.status.build.title, built(s.build))}
			{@render facts(m.status.database.title, [
				[
					m.status.database.pool,
					m.status.database.poolValue(
						s.database.pool_size,
						s.database.pool_idle,
						s.database.pool_max
					)
				],
				[m.status.database.connections, formatNumber(s.database.connections)],
				[m.status.database.size, formatBytes(s.database.size_bytes)],
				[
					m.status.database.hitRatio,
					s.database.cache_hit_ratio == null ? empty : formatPercent(s.database.cache_hit_ratio)
				],
				[m.status.database.deadlocks, formatNumber(s.database.deadlocks)],
				[
					m.status.database.longest,
					s.database.longest_query_s == null ? empty : ms(s.database.longest_query_s * 1000)
				]
			])}
			{@render facts(m.status.fetcher.title, [
				[m.status.fetcher.state, s.fetcher.running ? m.status.on : m.status.off],
				[
					m.status.fetcher.pace,
					s.fetcher.pace
						? m.status.fetcher.paceValue(
								s.fetcher.pace.round,
								ms(s.fetcher.pace.every_s * 1000),
								s.fetcher.pace.at_once,
								s.fetcher.pace.per_hour
							)
						: empty
				],
				[m.status.fetcher.perHour, formatNumber(s.fetcher.per_hour)],
				[
					m.status.fetcher.behind,
					s.fetcher.behind_s > 0
						? m.status.fetcher.behindValue(ms(s.fetcher.behind_s * 1000))
						: m.status.fetcher.notBehind
				],
				[
					m.status.fetcher.lastRound,
					s.fetcher.last_round_at
						? m.status.fetcher.lastRoundValue(
								ago(s.fetcher.last_round_at),
								s.fetcher.last_round_feeds ?? 0,
								ms((s.fetcher.last_round_s ?? 0) * 1000)
							)
						: empty
				],
				[
					m.status.fetcher.fetches,
					m.status.fetcher.fetchesValue(s.fetcher.fetched, s.fetcher.unchanged, s.fetcher.failed)
				],
				[m.status.fetcher.broken, formatNumber(s.fetcher.broken)],
				[m.status.fetcher.newArticles, formatNumber(s.fetcher.new_articles)],
				[m.status.fetcher.took, took(s.fetcher.took)],
				[
					m.status.fetcher.overdue,
					s.fetcher.overdue > 0
						? m.status.fetcher.overdueValue(
								s.fetcher.overdue,
								ms((s.fetcher.most_overdue_s ?? 0) * 1000)
							)
						: formatNumber(0)
				],
				[m.status.fetcher.waiting, formatNumber(s.fetcher.waiting)]
			])}
			<details class="group rounded border border-line p-2">
				{@render opener(m.status.fetcher.failing)}
				{#if s.fetcher.failing.length === 0}
					<p class="mt-2 px-1 text-sm text-soft">{m.status.fetcher.noneFailing}</p>
				{:else}
					<ul class="mt-2 space-y-1 px-1 text-sm">
						{#each s.fetcher.failing as failing (`${failing.failure}-${failing.status}`)}
							<li class="flex gap-3">
								<span class="min-w-0 flex-1">{failure(failing)}</span>
								<span class="text-soft">{m.status.fetcher.feeds(failing.feeds)}</span>
							</li>
						{/each}
					</ul>
				{/if}
			</details>
			{@render facts(m.status.finder.title, [
				[m.status.finder.state, s.finder.on ? m.status.on : m.status.off],
				[
					m.status.finder.lookups,
					m.status.finder.lookupsValue(s.finder.found, s.finder.none, s.finder.failed)
				],
				[m.status.finder.limited, formatNumber(s.finder.limited)],
				[m.status.finder.took, took(s.finder.took)]
			])}
			{@render facts(m.status.images.title, [
				[m.status.images.state, s.images.on ? m.status.on : m.status.off],
				[
					m.status.images.cache,
					m.status.images.cacheValue(
						formatBytes(s.images.cache_bytes),
						formatBytes(s.images.cache_max_bytes)
					)
				],
				[m.status.images.served, m.status.images.servedValue(s.images.cached, s.images.fetched)],
				[
					m.status.images.refused,
					m.status.images.refusedValue(
						s.images.source_failed,
						s.images.not_image,
						s.images.too_large,
						s.images.not_found
					)
				],
				[m.status.images.took, took(s.images.took)],
				[m.status.images.failingSources, formatNumber(s.images.failing_sources)]
			])}
		</div>

		<details class="group mt-2 rounded border border-line p-2" aria-labelledby="status-requests">
			{@render opener(m.status.requests.title, 'status-requests')}
			<dl class="mt-1 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
				<dt class="text-soft">{m.status.requests.all}</dt>
				<dd>{m.status.requests.allValue(s.requests.all.count, took(s.requests.all))}</dd>
				<dt class="text-soft">{m.status.requests.answers}</dt>
				<dd>
					{m.status.requests.answersValue(
						s.requests.answers[0] + s.requests.answers[1] + s.requests.answers[2],
						s.requests.answers[3],
						s.requests.answers[4]
					)}
				</dd>
			</dl>
			{#if s.requests.routes.length === 0}
				<p class="mt-2 text-sm text-soft">{m.status.requests.none}</p>
			{:else}
				<div class="mt-2 overflow-x-auto">
					<table class="w-full text-left text-sm">
						<thead class="text-xs text-soft">
							<tr>
								<th class="py-1 pr-3 font-medium">{m.status.requests.route}</th>
								<th class="py-1 pr-3 text-right font-medium">{m.status.requests.count}</th>
								<th class="py-1 pr-3 text-right font-medium">{m.status.requests.median}</th>
								<th class="py-1 pr-3 text-right font-medium">{m.status.requests.p95}</th>
								<th class="py-1 pr-3 text-right font-medium">{m.status.requests.max}</th>
								<th class="py-1 text-right font-medium">{m.status.requests.errors}</th>
							</tr>
						</thead>
						<tbody class="divide-y divide-line">
							{#each s.requests.routes as route (route.route)}
								<tr>
									<td class="py-1 pr-3 font-mono text-xs break-all">{route.route}</td>
									<td class="py-1 pr-3 text-right">{formatNumber(route.took.count)}</td>
									<td class="py-1 pr-3 text-right whitespace-nowrap">{ms(route.took.p50_ms)}</td>
									<td class="py-1 pr-3 text-right whitespace-nowrap">{ms(route.took.p95_ms)}</td>
									<td class="py-1 pr-3 text-right whitespace-nowrap">{ms(route.took.max_ms)}</td>
									<td class={['py-1 text-right', route.errors > 0 && 'text-warn']}>
										{formatNumber(route.errors)}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</details>

		<details class="group mt-2 rounded border border-line p-2" aria-labelledby="status-tables">
			{@render opener(m.status.database.tables, 'status-tables')}
			<div class="mt-2 overflow-x-auto">
				<table class="w-full text-left text-sm">
					<thead class="text-xs text-soft">
						<tr>
							<th class="py-1 pr-3 font-medium">{m.status.database.table}</th>
							<th class="py-1 pr-3 text-right font-medium">{m.status.database.bytes}</th>
							<th class="py-1 text-right font-medium">{m.status.database.rows}</th>
						</tr>
					</thead>
					<tbody class="divide-y divide-line">
						{#each s.database.tables as table (table.name)}
							<tr>
								<td class="py-1 pr-3 font-mono text-xs">{table.name}</td>
								<td class="py-1 pr-3 text-right">{formatBytes(table.bytes)}</td>
								<td class="py-1 text-right">{formatNumber(table.rows)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</details>

		<details
			class="group mt-2 rounded border border-line p-2"
			open={s.problems.errors > 0}
			aria-labelledby="status-problems"
		>
			{@render opener(
				m.status.problems.title,
				'status-problems',
				m.status.problems.counts(s.problems.errors, s.problems.warnings)
			)}
			{#if s.problems.recent.length === 0}
				<p class="mt-2 text-sm text-soft">{m.status.problems.none}</p>
			{:else}
				<ul class="mt-2 divide-y divide-line rounded border border-line">
					{#each s.problems.recent as problem, i (i)}
						<li class="px-3 py-2 text-sm">
							<p class="flex flex-wrap items-baseline gap-x-2">
								<span class={['text-xs font-semibold', problem.error ? 'text-warn' : 'text-soft']}>
									{problem.error ? m.status.problems.error : m.status.problems.warning}
								</span>
								<span class="font-mono text-xs text-soft">{problem.target}</span>
								<span
									class="ml-auto text-xs text-soft"
									title={formatFull(Date.parse(problem.last))}
								>
									{m.status.problems.times(problem.count, ago(problem.last))}
								</span>
							</p>
							<p class="mt-1 font-mono text-xs break-words">{problem.text}</p>
						</li>
					{/each}
				</ul>
			{/if}
		</details>

		<details class="group mt-2 rounded border border-line p-2" aria-labelledby="status-browsers">
			{@render opener(m.status.browsers.title, 'status-browsers')}
			{#if !s.browsers.available}
				<p class="mt-1 text-sm text-soft">{m.status.browsers.off}</p>
			{:else}
				<p class="mt-1 text-sm text-soft">{m.status.browsers.consenting(s.browsers.consenting)}</p>
				{#if s.browsers.errors.length === 0}
					<p class="mt-2 text-sm text-soft">{m.status.browsers.none}</p>
				{:else}
					<ul class="mt-2 divide-y divide-line rounded border border-line">
						{#each s.browsers.errors as error, i (i)}
							<li class="px-3 py-2 text-sm">
								<p class="flex flex-wrap items-baseline gap-x-2">
									<span class="text-xs font-semibold text-warn">
										{m.status.browsers.kinds[error.kind]}
									</span>
									<span
										class="ml-auto text-xs text-soft"
										title={formatFull(Date.parse(error.last))}
									>
										{m.status.browsers.hits(error.count, error.accounts, ago(error.last))}
									</span>
								</p>
								<p class="mt-1 font-mono text-xs break-words">{error.message}</p>
								{#if error.stack}
									<pre class="mt-1 overflow-x-auto font-mono text-xs text-soft">{error.stack}</pre>
								{/if}
								<p class="mt-1 text-xs text-soft">
									{m.status.browsers.where(error.versions.join(', '), error.browsers.join(', '))}
								</p>
							</li>
						{/each}
					</ul>
				{/if}
			{/if}
		</details>
	{/if}
</Dialog>
