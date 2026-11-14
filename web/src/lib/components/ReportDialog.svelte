<script lang="ts">
	// Reporting a feed or an article to the operator of the server: a reason from a
	// fixed list and a short note. The operator sees what is reported and why, never
	// who reported it.
	import { m } from '#lib/i18n/index.ts';
	import {
		MAX_REPORT_NOTE,
		reportReasons,
		type Reader,
		type ReportOutcome,
		type ReportReason
	} from '#lib/reader.ts';
	import Choice from './Choice.svelte';
	import Dialog from './Dialog.svelte';

	let {
		reader,
		feedId,
		seq,
		title,
		onclose
	}: {
		reader: Reader;
		feedId: string;
		/** The article's number in its feed; none for the feed itself. */
		seq?: number;
		/** What is reported, as the reader knows it. */
		title: string;
		onclose: () => void;
	} = $props();

	let reason = $state<ReportReason>('spam');
	let note = $state('');
	let sending = $state(false);
	let outcome = $state<ReportOutcome | null>(null);

	const options = reportReasons.map((value) => ({ value, label: m.report.reasons[value] }));

	async function send(event: SubmitEvent) {
		event.preventDefault();
		if (sending) return;
		sending = true;
		outcome = await reader.reportContent({ feedId, seq }, reason, note);
		sending = false;
	}
</script>

<Dialog title={seq === undefined ? m.report.feedTitle : m.report.articleTitle} {onclose}>
	{#if outcome === 'sent'}
		<p class="text-sm" role="status">{m.report.sent}</p>
		<div class="mt-4">
			<button type="button" class="tool" onclick={onclose}>{m.common.close}</button>
		</div>
	{:else}
		<form class="space-y-4" onsubmit={send}>
			<p class="text-sm text-soft">{m.report.text(title)}</p>
			<Choice label={m.report.reason} {options} bind:value={reason} />
			<label class="block text-sm font-medium">
				{m.report.note}
				<textarea
					class="mt-1.5 block w-full rounded text-sm font-normal"
					rows="4"
					maxlength={MAX_REPORT_NOTE}
					bind:value={note}></textarea>
				<span class="mt-1 block text-xs font-normal text-soft">
					{m.report.left(MAX_REPORT_NOTE - note.length)}
				</span>
			</label>
			{#if outcome === 'tooMany' || outcome === 'unreachable'}
				<p class="text-sm text-warn" role="alert">{m.report[outcome]}</p>
			{/if}
			<div class="flex gap-2">
				<button type="submit" class="primary" disabled={sending}>
					{sending ? m.report.sending : m.report.send}
				</button>
				<button type="button" class="tool" onclick={onclose}>{m.common.cancel}</button>
			</div>
		</form>
	{/if}
</Dialog>
