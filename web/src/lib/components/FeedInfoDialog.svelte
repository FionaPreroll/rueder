<script lang="ts">
	// What the server knows of a feed and how it fetches it, for a reader who asks
	// for help with it: to read, and to copy into a message to the operator, with
	// the times as the server gives them. An admin says here as what and at most
	// how often the feed is fetched, for all who read it.
	import { onMount } from 'svelte';
	import { formatDuration, formatFull } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import { formatNumber } from '#lib/i18n/intl.ts';
	import type { FeedInfo, FetchAgent, Reader } from '#lib/reader.ts';
	import Choice from './Choice.svelte';
	import Dialog from './Dialog.svelte';

	let {
		reader,
		feedId,
		title,
		onclose
	}: {
		reader: Reader;
		feedId: string;
		/** The feed's name, as the reader knows it. */
		title: string;
		onclose: () => void;
	} = $props();

	let info = $state.raw<FeedInfo | null>(null);
	let failed = $state(false);
	let notSwitched = $state(false);
	let copied = $state(false);

	async function load() {
		const answer = await reader.feedInfo(feedId);
		failed = answer === null;
		if (answer) info = answer;
	}

	onMount(() => void load());

	async function fetchAs(agent: FetchAgent) {
		if (!info || agent === info.fetch_agent) return;
		info = { ...info, fetch_agent: agent };
		notSwitched = !(await reader.setFetchAgent(feedId, agent));
		await load();
	}

	/** The limits an admin can choose from, in seconds. */
	const limits = [3600, 2 * 3600, 4 * 3600, 6 * 3600, 12 * 3600];

	async function limitTo(value: string) {
		const seconds = value === 'none' ? null : Number(value);
		if (!info || seconds === info.min_interval_s) return;
		info = { ...info, min_interval_s: seconds };
		notSwitched = !(await reader.setMinInterval(feedId, seconds));
		await load();
	}

	/** Label and value of each line; `time` writes the times. */
	function lines(info: FeedInfo, time: (at: string) => string) {
		const texts = m.feedInfo;
		const when = (at: string | null) => (at === null ? texts.never : time(at));
		const failed = when(info.last_error_at);
		return [
			[texts.number, String(info.id)],
			[texts.name, info.title],
			[texts.address, info.feed_url],
			[texts.site, info.site_url ?? texts.noSite],
			[texts.agent, texts.agents[info.fetch_agent]],
			[texts.every, formatDuration(info.fetch_interval_s * 1000)],
			[
				texts.atMost,
				info.min_interval_s === null ? texts.noLimit : formatDuration(info.min_interval_s * 1000)
			],
			[texts.fetched, when(info.fetched_at)],
			[texts.succeeded, when(info.last_success_at)],
			[texts.failed, info.error ? `${failed}: ${info.error}` : failed],
			[texts.next, info.blocked ? texts.blocked : when(info.next_fetch_at)],
			[texts.articles, formatNumber(info.last_seq)]
		];
	}

	const shown = $derived(info ? lines(info, (at) => formatFull(Date.parse(at))) : []);
	const agents = $derived(
		(['rueder', 'browser'] as const).map((value) => ({
			value,
			label: m.feedInfo.agents[value]
		}))
	);
	const intervals = $derived([
		{ value: 'none', label: m.feedInfo.noLimit },
		...limits.map((seconds) => ({ value: String(seconds), label: formatDuration(seconds * 1000) }))
	]);

	async function copy() {
		if (!info) return;
		const text = lines(info, (at) => at)
			.map(([label, value]) => `${label}: ${value}`)
			.join('\n');
		try {
			await navigator.clipboard.writeText(`${m.feedInfo.title}\n${text}\n`);
			copied = true;
		} catch {
			// Without the clipboard the lines are there to be marked and copied by hand.
		}
	}
</script>

<Dialog title={m.feedInfo.title} {onclose}>
	{#if info}
		<p class="text-sm text-soft">{m.feedInfo.text(title)}</p>
		<dl class="mt-4 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-sm">
			{#each shown as [label, value] (label)}
				<dt class="text-soft">{label}</dt>
				<dd class="min-w-0 break-words select-all">{value}</dd>
			{/each}
		</dl>
		{#if reader.can.fetching}
			<div class="mt-4">
				<Choice
					label={m.feedInfo.fetchAs}
					options={agents}
					bind:value={() => info?.fetch_agent ?? 'rueder', fetchAs}
				/>
				<p class="mt-1.5 text-xs text-soft">{m.feedInfo.fetchAsHint}</p>
			</div>
			<div class="mt-4">
				<Choice
					label={m.feedInfo.atMost}
					options={intervals}
					bind:value={() => String(info?.min_interval_s ?? 'none'), limitTo}
				/>
				<p class="mt-1.5 text-xs text-soft">{m.feedInfo.atMostHint}</p>
				{#if notSwitched}
					<p class="mt-1.5 text-sm text-warn" role="alert">{m.feedInfo.notSwitched}</p>
				{/if}
			</div>
		{/if}
		<div class="mt-4 flex gap-2">
			<button type="button" class="primary" onclick={copy}>
				{copied ? m.feedInfo.copied : m.feedInfo.copy}
			</button>
			<button type="button" class="tool" onclick={onclose}>{m.common.close}</button>
		</div>
	{:else if failed}
		<p class="text-sm text-warn" role="alert">{m.feedInfo.unreachable}</p>
	{:else}
		<p class="text-sm text-soft">{m.feedInfo.loading}</p>
	{/if}
</Dialog>
