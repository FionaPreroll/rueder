<script lang="ts">
	// Asks an account without an address for one, where every account on the
	// server needs one. It cannot be closed past: giving an address and opening
	// its link, or logging out, are the ways on. Once the link is opened, in this
	// tab or another, the account has its address and this goes away.
	import { onMount } from 'svelte';
	import { m } from '#lib/i18n/index.ts';
	import type { Reader } from '#lib/reader.ts';
	import EmailAddress from './EmailAddress.svelte';

	let { reader }: { reader: Reader } = $props();

	let panel: HTMLElement | undefined = $state();

	onMount(() => panel?.focus());
</script>

<div
	class="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto bg-black/40 p-3 sm:p-8"
>
	<div
		bind:this={panel}
		role="alertdialog"
		aria-modal="true"
		aria-labelledby="email-ask-title"
		aria-describedby="email-ask-intro"
		tabindex="-1"
		class="w-full max-w-md rounded-lg border border-line bg-paper p-5 shadow-xl outline-none"
	>
		<h2 id="email-ask-title" class="text-lg font-semibold">{m.email.askTitle}</h2>
		<p id="email-ask-intro" class="mt-2 text-sm">{m.email.askIntro}</p>
		<div class="mt-4">
			<EmailAddress {reader} asking />
		</div>
		{#if reader.account?.emailPending}
			<p class="mt-3 text-xs text-soft">{m.email.askWaiting}</p>
		{/if}
		<div class="mt-5 flex justify-end">
			<button type="button" class="tool" onclick={() => reader.signOut()}>
				{m.email.logOut}
			</button>
		</div>
	</div>
</div>
