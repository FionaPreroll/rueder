<script lang="ts">
	// Another password for the account, with the one so far to show that whoever
	// asks is the reader. An account that logs in through single sign-on alone has
	// no password yet and sets a first one here, which lets it log in with its
	// name as well. The server ends every other session of the account and gives
	// this browser a new one, so the page stays as it is.
	import { m } from '#lib/i18n/index.ts';
	import type { PasswordRefusal, Reader } from '#lib/reader.ts';

	let { reader }: { reader: Reader } = $props();

	/** As short as the server lets a password be (`accounts::MIN_PASSWORD`). */
	const MIN = 8;

	const texts = $derived(m.account.password);
	/** The account has no password yet: this one is its first. */
	const first = $derived(reader.account?.password === false);

	let current = $state('');
	let fresh = $state('');
	let again = $state('');
	let busy = $state(false);
	let refusal = $state<PasswordRefusal | null>(null);
	/** It worked: what was done, as the account had a password or had none. */
	let done = $state<'first' | 'again' | null>(null);

	const mismatch = $derived(again.length > 0 && fresh !== again);
	const ready = $derived(
		!busy && (first || current.length > 0) && fresh.length >= MIN && fresh === again
	);

	function say(refusal: PasswordRefusal) {
		switch (refusal.problem) {
			case 'wrong':
				return texts.wrong;
			case 'invalid':
				return texts.invalid(MIN);
			case 'busy':
				return texts.busy(refusal.minutes);
			case 'unreachable':
				return texts.unreachable;
		}
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!ready) return;
		busy = true;
		refusal = null;
		done = null;
		// Once it worked the account has a password, so what it was then is kept.
		const wasFirst = first;
		try {
			// An account without one so far sends nothing for it.
			refusal = await reader.changePassword(wasFirst ? '' : current, fresh);
			if (!refusal) {
				done = wasFirst ? 'first' : 'again';
				current = '';
				fresh = '';
				again = '';
			}
		} finally {
			busy = false;
		}
	}
</script>

<form class="space-y-3 text-sm" onsubmit={submit}>
	<h3 class="font-semibold">{first ? texts.titleFirst : texts.title}</h3>
	<p class="text-soft">{first ? texts.introFirst : texts.intro}</p>
	{#if !first}
		<label class="block">
			<span class="font-medium">{texts.current}</span>
			<input
				class="mt-1.5 block w-full max-w-xs rounded py-1.5 text-sm"
				type="password"
				name="current-password"
				autocomplete="current-password"
				bind:value={current}
			/>
		</label>
	{/if}
	<label class="block">
		<span class="font-medium">{texts.fresh}</span>
		<input
			class="mt-1.5 block w-full max-w-xs rounded py-1.5 text-sm"
			type="password"
			name="new-password"
			autocomplete="new-password"
			minlength={MIN}
			aria-describedby="password-rule"
			bind:value={fresh}
		/>
	</label>
	<p id="password-rule" class="-mt-2 text-xs text-soft">{m.register.passwordRule(MIN)}</p>
	<label class="block">
		<span class="font-medium">{texts.again}</span>
		<input
			class="mt-1.5 block w-full max-w-xs rounded py-1.5 text-sm"
			type="password"
			name="new-password-again"
			autocomplete="new-password"
			bind:value={again}
		/>
	</label>
	{#if mismatch}
		<p class="text-warn">{texts.mismatch}</p>
	{/if}
	{#if refusal}
		<p class="text-warn" role="alert">{say(refusal)}</p>
	{/if}
	{#if done}
		<p class="text-soft" role="status">{done === 'first' ? texts.doneFirst : texts.done}</p>
	{/if}
	<button
		type="submit"
		class="tool font-semibold disabled:cursor-default disabled:opacity-50"
		disabled={!ready}
	>
		{first ? texts.submitFirst : texts.submit}
	</button>
</form>
