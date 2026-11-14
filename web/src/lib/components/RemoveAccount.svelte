<script lang="ts">
	// Deleting the own account, for good: what goes and what stays, and the
	// password, or the name of an account without one, to show it is meant. Where
	// the account has a second factor, the server asks for its code as well. A
	// passkey of the account stands for all of that.
	import type { PasskeyProof as Proof, Reader, RemoveRefusal } from '#lib/reader.ts';
	import { m } from '#lib/i18n/index.ts';
	import PasskeyProof from './PasskeyProof.svelte';

	let { reader }: { reader: Reader } = $props();

	const name = $derived(reader.account?.name ?? '');
	const byName = $derived(reader.account?.password === false);

	let proof = $state('');
	let code = $state('');
	/** The server asked for the code of the second factor. */
	let asksCode = $state(false);
	let busy = $state(false);
	let refusal = $state<RemoveRefusal | null>(null);

	function say(refusal: RemoveRefusal) {
		switch (refusal.problem) {
			case 'wrong':
				return m.removeAccount.wrong;
			case 'code':
				return m.removeAccount.codeNeeded;
			case 'busy':
				return m.removeAccount.busy(refusal.minutes);
			case 'unreachable':
				return m.removeAccount.unreachable;
		}
	}

	async function withPasskey(passkey: Proof) {
		if (busy) return;
		busy = true;
		refusal = null;
		try {
			// Gone, the page is logged out and this is no longer there.
			refusal = await reader.removeAccount('', undefined, passkey);
		} finally {
			busy = false;
		}
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !proof || (asksCode && !code.trim())) return;
		busy = true;
		refusal = null;
		try {
			// Gone, the page is logged out and this is no longer there.
			refusal = await reader.removeAccount(proof, asksCode ? code : undefined);
			if (refusal?.problem === 'code') asksCode = true;
			else proof = '';
			code = '';
		} finally {
			busy = false;
		}
	}
</script>

<form class="space-y-3 text-sm" onsubmit={submit}>
	<h3 class="font-semibold">{m.removeAccount.title}</h3>
	<p>{m.removeAccount.intro}</p>
	<p class="text-soft">{m.removeAccount.goes}</p>
	<p class="text-soft">{m.removeAccount.stays}</p>
	<label class="block">
		<span class="font-medium">{byName ? m.removeAccount.name(name) : m.removeAccount.password}</span
		>
		{#if byName}
			<input
				class="mt-1.5 block w-full max-w-xs rounded py-1.5 text-sm"
				name="confirm-name"
				autocomplete="off"
				autocapitalize="none"
				spellcheck="false"
				bind:value={proof}
			/>
		{:else}
			<input
				class="mt-1.5 block w-full max-w-xs rounded py-1.5 text-sm"
				type="password"
				name="password"
				autocomplete="current-password"
				bind:value={proof}
			/>
		{/if}
	</label>
	{#if asksCode}
		<label class="block">
			<span class="font-medium">{m.removeAccount.code}</span>
			<input
				class="mt-1.5 block w-full max-w-xs rounded py-1.5 text-sm"
				name="one-time-code"
				autocomplete="one-time-code"
				autocapitalize="none"
				spellcheck="false"
				bind:value={code}
			/>
		</label>
	{/if}
	{#if refusal}
		<p class="text-warn" role="alert">{say(refusal)}</p>
	{/if}
	<button type="submit" class="danger" disabled={busy || !proof || (asksCode && !code.trim())}>
		{m.removeAccount.submit}
	</button>
	<PasskeyProof {reader} disabled={busy} onproof={withPasskey} />
</form>
