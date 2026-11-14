<script lang="ts">
	// Instead of the password and a code: a passkey of the account confirms that
	// its owner is there, for one change. Shown only where the account has a
	// passkey and the browser can use it; `has` says so where the caller knows,
	// else this asks the server.
	import { m } from '#lib/i18n/index.ts';
	import type { AccountSettings, PasskeyProof, PasskeyRefusal } from '#lib/account.ts';

	let {
		accountSettings,
		has,
		label,
		disabled = false,
		onproof
	}: {
		accountSettings: AccountSettings;
		has?: boolean;
		/** What the button says, where "Confirm with a passkey" says too little. */
		label?: string;
		disabled?: boolean;
		onproof: (proof: PasskeyProof) => void | Promise<void>;
	} = $props();

	const texts = m.settings.passkeys;

	let asked = $state<boolean | null>(null);
	let asking = $state(false);
	let refusal = $state<PasskeyRefusal | null>(null);

	$effect(() => {
		if (has !== undefined || !accountSettings.can.passkeys) return;
		void accountSettings.secondFactor().then((state) => (asked = (state?.passkeys ?? 0) > 0));
	});

	const shown = $derived(accountSettings.can.passkeys && (has ?? asked ?? false));

	async function confirm() {
		if (asking) return;
		asking = true;
		refusal = null;
		try {
			const answer = await accountSettings.provePasskey();
			if (typeof answer === 'string') refusal = answer;
			else await onproof(answer);
		} finally {
			asking = false;
		}
	}
</script>

{#if shown}
	<div class="flex flex-wrap items-center gap-2">
		<span class="text-soft">{texts.or}</span>
		<button type="button" class="tool" disabled={disabled || asking} onclick={confirm}>
			{asking ? texts.working : (label ?? texts.confirmWith)}
		</button>
	</div>
	{#if refusal}
		<p class="text-warn" role="alert">{texts.refusals[refusal]}</p>
	{/if}
{/if}
