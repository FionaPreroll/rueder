<script lang="ts">
	// Moving the account to another rueder: everything it has goes out as one file,
	// and a file of this rueder or another comes in. What came in is said in numbers.
	import { MAX_ACCOUNT_FILE, type Moved, type Reader, type Refusal } from '#lib/reader.ts';
	import { m } from '#lib/i18n/index.ts';

	let { reader }: { reader: Reader } = $props();

	let importing = $state(false);
	let moved = $state<Moved | null>(null);
	let exporting = $state(false);
	let exportFailed = $state(false);

	// A button that is pressed stays usable while its work goes on, and says that it
	// waits with aria-disabled: one that is disabled loses the focus.
	const waits = 'aria-disabled:cursor-default aria-disabled:opacity-50';

	function say(refusal: Refusal) {
		switch (refusal.problem) {
			case 'refused':
				return m.subscribe.refused;
			case 'busy':
				return m.move.busy(refusal.retryAfter);
			case 'tooLarge':
				return m.subscribe.tooLarge(MAX_ACCOUNT_FILE / 1024 / 1024);
			case 'noFetching':
			case 'unreachable':
				return m.subscribe.unreachable;
		}
	}

	async function importFile(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		// The same file can be chosen again.
		input.value = '';
		if (!file || importing) return;
		importing = true;
		moved = null;
		moved = await reader.importAccount(file);
		importing = false;
	}

	async function exportFile() {
		if (exporting) return;
		exporting = true;
		exportFailed = false;
		const file = await reader.exportAccount();
		exporting = false;
		if (file === null) {
			exportFailed = true;
			return;
		}
		const url = URL.createObjectURL(new Blob([file], { type: 'application/json' }));
		const link = document.createElement('a');
		link.href = url;
		link.download = `rueder-${new Date().toISOString().slice(0, 10)}.json`;
		link.click();
		// The download has started by then.
		setTimeout(() => URL.revokeObjectURL(url), 10_000);
	}
</script>

<div class="space-y-3 text-sm">
	<p>{m.move.intro}</p>
	<p class="text-soft">{m.move.howItWorks}</p>
	<p class="text-xs text-soft">{m.move.notInIt}</p>
	<div class="flex flex-wrap gap-2 pt-1">
		<button type="button" class={['tool', waits]} aria-disabled={exporting} onclick={exportFile}>
			{m.move.export}
		</button>
		<label
			class={[
				'tool has-focus-visible:outline-2 has-focus-visible:outline-link',
				importing && 'opacity-50'
			]}
		>
			{importing ? m.move.importing : m.move.import}
			<input
				type="file"
				class="sr-only"
				accept=".json,application/json"
				aria-disabled={importing}
				onchange={importFile}
			/>
		</label>
	</div>
	{#if moved && 'feeds' in moved}
		<div role="status">
			<p>{m.move.feeds(moved.feeds, moved.known)}</p>
			{#if moved.folders > 0}<p>{m.move.folders(moved.folders)}</p>{/if}
			<p>{m.move.articles(moved.articles)}</p>
			{#if moved.missing > 0}<p>{m.move.missing(moved.missing)}</p>{/if}
			{#if moved.labels > 0 || moved.rules > 0}<p>{m.move.marks(moved.labels, moved.rules)}</p>{/if}
			{#if moved.skipped > 0}<p>{m.move.skipped(moved.skipped)}</p>{/if}
			{#if moved.settings}<p>{m.move.settings}</p>{/if}
			{#if moved.feeds > 0}<p class="mt-1 text-xs text-soft">{m.move.later}</p>{/if}
		</div>
	{:else if moved}
		<div class="text-warn" role="alert">
			<p>{say(moved)}</p>
			{#if moved.problem === 'refused' && moved.reason}
				<p class="mt-1 text-xs text-soft">{m.subscribe.serverSays(moved.reason)}</p>
			{/if}
		</div>
	{/if}
	{#if exportFailed}
		<p class="text-warn" role="alert">{m.subscribe.unreachable}</p>
	{/if}
</div>
