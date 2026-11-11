<script lang="ts">
	// Subscribing to a feed by its address.
	import type { Reader } from '#lib/reader.ts';
	import { m } from '#lib/i18n/index.ts';
	import Dialog from './Dialog.svelte';

	let { reader, onclose }: { reader: Reader; onclose: () => void } = $props();

	let address = $state('');

	function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!address.trim()) return;
		reader.subscribe(address);
		onclose();
	}

	const focus = (field: HTMLInputElement) => field.focus();
</script>

<Dialog title={m.subscribe.title} {onclose}>
	<form onsubmit={submit}>
		<label class="block text-sm">
			<span class="font-medium">{m.subscribe.address}</span>
			<input
				class="mt-1.5 block w-full rounded py-1.5 text-sm"
				placeholder="https://example.org/feed.xml"
				bind:value={address}
				use:focus
			/>
		</label>
		<p class="mt-2 text-xs text-soft">{m.subscribe.note}</p>
		<div class="mt-4 flex gap-2">
			<button
				type="submit"
				class="cursor-pointer rounded bg-accent px-3 py-1 text-sm font-semibold text-on-accent hover:brightness-110 disabled:cursor-default disabled:opacity-50"
				disabled={!address.trim()}
			>
				{m.subscribe.add}
			</button>
			<button type="button" class="tool" onclick={onclose}>{m.common.cancel}</button>
		</div>
	</form>
</Dialog>
