<script lang="ts">
	// The account's email address: the one it has, one that waits for its link,
	// and a new one, which counts once the link mailed to it is opened. The
	// password, and the code of the second factor where the account has one, show
	// that the owner asks, as a stolen session must not send the mail elsewhere;
	// a passkey stands for both. Where the address is optional it can go again.
	// In the window that asks an account for its address (EmailDialog.svelte)
	// only the giving part shows.
	import { m } from '#lib/i18n/index.ts';
	import {
		EMAIL_LINK_HOURS,
		type EmailRefusal,
		type PasskeyProof as Proof,
		type Reader
	} from '#lib/reader.ts';
	import PasskeyProof from './PasskeyProof.svelte';

	let { reader, asking = false }: { reader: Reader; asking?: boolean } = $props();

	const texts = $derived(m.email);
	const account = $derived(reader.account);
	/** An account that logs in through single sign-on alone has no password to give. */
	const withoutPassword = $derived(account?.password === false);
	const removable = $derived(!asking && account?.emailMode === 'optional' && !!account?.email);

	let address = $state('');
	let password = $state('');
	let code = $state('');
	/** The server asked for the code of the second factor. */
	let asksCode = $state(false);
	let busy = $state(false);
	let refusal = $state<EmailRefusal | null>(null);
	let done = $state<'sent' | 'removed' | null>(null);

	const proved = $derived(
		(withoutPassword || password !== '') && (!asksCode || code.trim() !== '')
	);
	const ready = $derived(!busy && address.trim() !== '' && proved);

	function say(refusal: EmailRefusal) {
		switch (refusal.problem) {
			case 'wrong':
				return texts.wrong;
			case 'code':
				return texts.codeNeeded;
			case 'invalid':
				return texts.invalid;
			case 'same':
				return texts.same;
			case 'required':
				return texts.required;
			case 'busy':
				return texts.busy(refusal.minutes);
			case 'unsent':
				return texts.unsent;
			case 'unreachable':
				return texts.unreachable;
		}
	}

	/** What a try came to: the code is asked for, or the fields start over. */
	function settle(outcome: EmailRefusal | null, what: 'sent' | 'removed') {
		refusal = outcome;
		if (outcome?.problem === 'code') {
			asksCode = true;
			return;
		}
		if (outcome) {
			if (outcome.problem === 'wrong') password = '';
			code = '';
			return;
		}
		done = what;
		address = '';
		password = '';
		code = '';
		asksCode = false;
	}

	async function run(what: 'sent' | 'removed', passkey?: Proof) {
		if (busy) return;
		busy = true;
		refusal = null;
		done = null;
		try {
			const typed = asksCode ? code : undefined;
			const outcome =
				what === 'sent'
					? await reader.changeEmail(address, password, typed, passkey)
					: await reader.removeEmail(password, typed, passkey);
			settle(outcome, what);
		} finally {
			busy = false;
		}
	}

	function submit(event: SubmitEvent) {
		event.preventDefault();
		if (ready) void run('sent');
	}
</script>

<form class="space-y-3 text-sm" onsubmit={submit}>
	{#if !asking}
		<h3 class="font-semibold">{texts.title}</h3>
	{/if}
	<p class="text-soft">{texts.intro}</p>
	{#if !asking}
		<p>
			<span class="font-medium">{texts.current}:</span>
			{#if account?.email}
				<span data-email>{account.email}</span>
			{:else}
				<span class="text-soft">{texts.none}</span>
			{/if}
		</p>
	{/if}
	{#if account?.emailPending}
		<p class="rounded bg-panel px-3 py-2" role="status">
			{texts.pending(account.emailPending, EMAIL_LINK_HOURS)}
		</p>
	{/if}
	<label class="block">
		<span class="font-medium">{texts.address}</span>
		<input
			class="mt-1.5 block w-full max-w-xs rounded py-1.5 text-sm"
			type="email"
			name="email"
			autocomplete="email"
			autocapitalize="none"
			spellcheck="false"
			maxlength="254"
			bind:value={address}
		/>
	</label>
	{#if !withoutPassword}
		<label class="block">
			<span class="font-medium">{texts.password}</span>
			<input
				class="mt-1.5 block w-full max-w-xs rounded py-1.5 text-sm"
				type="password"
				name="current-password"
				autocomplete="current-password"
				aria-describedby="email-password-note"
				bind:value={password}
			/>
		</label>
		<p id="email-password-note" class="-mt-2 text-xs text-soft">{texts.passwordNote}</p>
	{/if}
	{#if asksCode}
		<label class="block">
			<span class="font-medium">{texts.code}</span>
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
	{#if done}
		<p class="text-soft" role="status">{done === 'sent' ? texts.sent : texts.removed}</p>
	{/if}
	<div class="flex flex-wrap gap-2">
		<button type="submit" class="primary" disabled={!ready}>{texts.submit}</button>
	</div>
	<PasskeyProof
		{reader}
		disabled={busy || address.trim() === ''}
		onproof={(proof) => run('sent', proof)}
	/>
	{#if removable}
		<div class="space-y-2 border-t border-line pt-4">
			<p class="text-soft">{texts.removeIntro}</p>
			<button
				type="button"
				class="danger"
				disabled={busy || !proved}
				onclick={() => void run('removed')}
			>
				{texts.remove}
			</button>
		</div>
	{/if}
</form>
