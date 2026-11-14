<script lang="ts">
	// Asks the reader to agree to a new version of the operator's terms of use. It
	// cannot be closed past: agreeing or logging out are the ways on.
	import { onMount } from 'svelte';
	import { m } from '#lib/i18n/index.ts';
	import type { Reader } from '#lib/reader.ts';

	let { reader }: { reader: Reader } = $props();

	let busy = $state(false);
	let failed = $state(false);
	let panel: HTMLElement | undefined = $state();

	onMount(() => panel?.focus());

	async function agree() {
		busy = true;
		failed = false;
		try {
			failed = !(await reader.acceptTerms());
		} finally {
			busy = false;
		}
	}
</script>

<div class="fixed inset-0 z-50 flex items-start justify-center bg-black/40 p-3 sm:p-8">
	<div
		bind:this={panel}
		role="alertdialog"
		aria-modal="true"
		aria-labelledby="terms-title"
		aria-describedby="terms-intro"
		tabindex="-1"
		class="w-full max-w-md rounded-lg border border-line bg-paper p-5 shadow-xl outline-none"
	>
		<h2 id="terms-title" class="text-lg font-semibold">{m.legal.changed}</h2>
		<p id="terms-intro" class="mt-2 text-sm text-soft">{m.legal.changedIntro}</p>
		{#if reader.legal?.terms}
			<p class="mt-3 text-sm">
				<a
					class="text-link hover:underline"
					href={reader.legal.terms}
					target="_blank"
					rel="noopener noreferrer"
				>
					{m.legal.read}
				</a>
			</p>
		{/if}
		{#if failed}
			<p class="mt-3 text-sm text-warn" role="alert">{m.legal.notSaved}</p>
		{/if}
		<div class="mt-5 flex justify-end gap-2">
			<button type="button" class="tool" onclick={() => reader.signOut()}>
				{m.legal.decline}
			</button>
			<button type="button" class="primary" disabled={busy} onclick={agree}>
				{m.legal.accept}
			</button>
		</div>
	</div>
</div>
