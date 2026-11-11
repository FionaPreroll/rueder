<script lang="ts">
	// Subscribing: an address is looked at for feeds, which may be the feed itself or
	// the feeds a website names, and one of them is subscribed to, into a folder if
	// the reader wants. Below, the subscriptions go in and out as an OPML file.
	import {
		MAX_OPML,
		type FeedCandidate,
		type Imported,
		type Reader,
		type Refusal
	} from '#lib/reader.ts';
	import { m } from '#lib/i18n/index.ts';
	import { folderList } from '#lib/tree.ts';
	import Dialog from './Dialog.svelte';

	let { reader, onclose }: { reader: Reader; onclose: () => void } = $props();

	let address = $state('');
	/** The feeds at the address that was looked at last; null before. */
	let found = $state<FeedCandidate[] | null>(null);
	let folder = $state('');
	let finding = $state(false);
	/** The feed being subscribed to. */
	let adding = $state<string | null>(null);
	let problem = $state<Refusal | null>(null);

	let importing = $state(false);
	let imported = $state<Imported | null>(null);
	let exporting = $state(false);
	let exportFailed = $state(false);

	const folders = $derived(folderList(reader.tree));
	const busy = $derived(finding || adding !== null);
	// A button that is pressed stays usable while its work goes on, and says that it
	// waits with aria-disabled: one that is disabled loses the focus, and with it
	// Escape and the place of whoever uses the keyboard.
	const waits = 'aria-disabled:cursor-default aria-disabled:opacity-50';

	async function find(event: SubmitEvent) {
		event.preventDefault();
		if (!address.trim() || busy) return;
		finding = true;
		problem = null;
		found = null;
		const result = await reader.findFeeds(address);
		finding = false;
		if ('feeds' in result) found = result.feeds;
		else problem = result;
	}

	async function subscribe(url: string) {
		if (busy) return;
		adding = url;
		problem = null;
		const result = await reader.subscribe(url, folder || null);
		adding = null;
		if ('feedId' in result) onclose();
		else problem = result;
	}

	/** What a refusal means, in the reader's words. */
	function say(refusal: Refusal) {
		switch (refusal.problem) {
			case 'refused':
				return m.subscribe.refused;
			case 'busy':
				return m.subscribe.busy(refusal.retryAfter);
			case 'noFetching':
				return m.subscribe.noFetching;
			case 'tooLarge':
				return m.subscribe.tooLarge(MAX_OPML / 1024 / 1024);
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
		imported = null;
		imported = await reader.importOpml(file);
		importing = false;
	}

	async function exportFile() {
		if (exporting) return;
		exporting = true;
		exportFailed = false;
		const file = await reader.exportOpml();
		exporting = false;
		if (file === null) {
			exportFailed = true;
			return;
		}
		const url = URL.createObjectURL(new Blob([file], { type: 'text/x-opml' }));
		const link = document.createElement('a');
		link.href = url;
		link.download = `rueder-${new Date().toISOString().slice(0, 10)}.opml`;
		link.click();
		// The download has started by then.
		setTimeout(() => URL.revokeObjectURL(url), 10_000);
	}

	const focus = (field: HTMLInputElement) => field.focus();
</script>

<Dialog title={m.subscribe.title} {onclose}>
	<form onsubmit={find}>
		<label class="block text-sm">
			<span class="font-medium">{m.subscribe.address}</span>
			<input
				class="mt-1.5 block w-full rounded py-1.5 text-sm"
				placeholder="https://example.org"
				bind:value={address}
				use:focus
			/>
		</label>
		{#if reader.demo}
			<p class="mt-2 text-xs text-soft">{m.subscribe.note}</p>
		{/if}
		<div class="mt-3 flex gap-2">
			<button type="submit" class={['tool', waits]} disabled={!address.trim()} aria-disabled={busy}>
				{finding ? m.subscribe.finding : m.subscribe.find}
			</button>
			<button type="button" class="tool" onclick={onclose}>{m.common.cancel}</button>
		</div>
	</form>

	{#if found !== null}
		<div class="mt-4">
			{#if found.length === 0}
				<p class="text-sm" role="status">{m.subscribe.none}</p>
			{:else}
				<p class="text-sm" role="status">{m.subscribe.found(found.length)}</p>
				{#if folders.length > 0}
					<label class="mt-2 flex items-center gap-2 text-sm">
						<span>{m.subscribe.folder}</span>
						<select class="rounded py-1 text-sm" bind:value={folder} disabled={busy}>
							<option value="">{m.subscribe.topLevel}</option>
							{#each folders as listed (listed.id)}
								<option value={listed.id}
									>{'\u00a0\u00a0'.repeat(listed.depth)}{listed.title}</option
								>
							{/each}
						</select>
					</label>
				{/if}
				<ul class="mt-2 divide-y divide-line rounded border border-line">
					{#each found as feed (feed.url)}
						<li class="flex items-center gap-3 px-3 py-2">
							<div class="min-w-0 flex-1">
								<p class="truncate text-sm font-medium">{feed.title || m.subscribe.untitled}</p>
								<p class="truncate text-xs text-soft">{feed.url}</p>
							</div>
							<button
								type="button"
								class={[
									'cursor-pointer rounded bg-accent px-3 py-1 text-sm font-semibold text-on-accent hover:brightness-110',
									waits
								]}
								aria-label={`${m.subscribe.add}: ${feed.title || feed.url}`}
								aria-disabled={busy}
								onclick={() => subscribe(feed.url)}
							>
								{adding === feed.url ? m.subscribe.adding : m.subscribe.add}
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}

	{#if problem}
		<div class="mt-4 text-sm text-warn" role="alert">
			<p>{say(problem)}</p>
			{#if problem.problem === 'refused' && problem.reason}
				<p class="mt-1 text-xs text-soft">{m.subscribe.serverSays(problem.reason)}</p>
			{/if}
		</div>
	{/if}

	{#if reader.can.opml}
		<section class="mt-5 border-t border-line pt-4">
			<h3 class="text-sm font-medium">{m.opml.title}</h3>
			<p class="mt-1 text-xs text-soft">{m.opml.intro}</p>
			<div class="mt-3 flex flex-wrap gap-2">
				<label
					class={[
						'tool has-focus-visible:outline-2 has-focus-visible:outline-link',
						importing && 'opacity-50'
					]}
				>
					{importing ? m.opml.importing : m.opml.import}
					<input
						type="file"
						class="sr-only"
						accept=".opml,.xml,text/x-opml,text/xml,application/xml"
						aria-disabled={importing}
						onchange={importFile}
					/>
				</label>
				<button
					type="button"
					class={['tool', waits]}
					aria-disabled={exporting}
					onclick={exportFile}
				>
					{m.opml.export}
				</button>
			</div>
			{#if imported && 'feeds' in imported}
				<div class="mt-3 text-sm" role="status">
					<p>{m.opml.imported(imported.feeds, imported.folders)}</p>
					{#if imported.known > 0}<p>{m.opml.known(imported.known)}</p>{/if}
					{#if imported.skipped > 0}<p>{m.opml.skipped(imported.skipped)}</p>{/if}
					{#if imported.feeds > 0}<p class="text-xs text-soft">{m.opml.later}</p>{/if}
				</div>
			{:else if imported}
				<div class="mt-3 text-sm text-warn" role="alert">
					<p>{say(imported)}</p>
					{#if imported.problem === 'refused' && imported.reason}
						<p class="mt-1 text-xs text-soft">{m.subscribe.serverSays(imported.reason)}</p>
					{/if}
				</div>
			{/if}
			{#if exportFailed}
				<p class="mt-3 text-sm text-warn" role="alert">{m.subscribe.unreachable}</p>
			{/if}
		</section>
	{/if}
</Dialog>
