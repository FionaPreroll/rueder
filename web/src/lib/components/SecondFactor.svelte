<script lang="ts">
	// The second factor of the account, in the settings: whether it is on, through
	// the authenticator app or passkeys, setting the app up with the password, a QR
	// code and a code of the app, the recovery codes shown once, and switching the
	// app off with the password and a code. A passkey can stand in for password and
	// code. While the app is being set up, `busy` says so, so that the window around
	// it is not closed by mistake.
	import { m } from '#lib/i18n/index.ts';
	import type {
		PasskeyProof as Proof,
		Reader,
		SecondFactorRefusal,
		SecondFactorSetup,
		SecondFactorState
	} from '#lib/reader.ts';
	import PasskeyProof from './PasskeyProof.svelte';

	let {
		reader,
		busy: settingUp = $bindable(false),
		version = 0,
		onchange
	}: {
		reader: Reader;
		busy?: boolean;
		/** Counts up when something else changed the second factor. */
		version?: number;
		onchange?: () => void;
	} = $props();

	const texts = m.settings.secondFactor;

	let current = $state.raw<SecondFactorState | null>(null);
	let setup = $state.raw<SecondFactorSetup | null>(null);
	let codes = $state.raw<string[] | null>(null);
	let password = $state('');
	let code = $state('');
	let refusal = $state<SecondFactorRefusal | null>(null);
	let busy = $state(false);
	/** The QR code shown large, and what was copied last. */
	let large = $state(false);
	let copied = $state<'key' | 'codes' | null>(null);

	$effect(() => {
		settingUp = setup !== null || codes !== null;
	});

	async function copy(what: 'key' | 'codes', text: string) {
		try {
			await navigator.clipboard.writeText(text);
			copied = what;
		} catch {
			// Without the clipboard the text is there to be marked and copied by hand.
		}
	}

	async function load() {
		current = await reader.secondFactor();
	}
	$effect(() => {
		void version;
		void load();
	});

	/** Runs a step, says why it did not work, and takes the password away either way. */
	async function step(work: () => Promise<SecondFactorRefusal | null>) {
		if (busy) return;
		busy = true;
		refusal = null;
		try {
			refusal = await work();
		} finally {
			busy = false;
			password = '';
		}
	}

	const isRefusal = (value: unknown): value is SecondFactorRefusal => typeof value === 'string';

	function startWith(passkey?: Proof) {
		return step(async () => {
			const answer = await reader.setUpSecondFactor(password, code, passkey);
			code = '';
			if (isRefusal(answer)) return answer;
			setup = answer;
			large = false;
			copied = null;
			return null;
		});
	}

	function start(event: SubmitEvent) {
		event.preventDefault();
		void startWith();
	}

	function confirm(event: SubmitEvent) {
		event.preventDefault();
		void step(async () => {
			const answer = await reader.confirmSecondFactor(code);
			code = '';
			if (isRefusal(answer)) return answer;
			setup = null;
			codes = answer;
			copied = null;
			onchange?.();
			return null;
		});
	}

	function switchOffWith(passkey?: Proof) {
		return step(async () => {
			const answer = await reader.switchOffSecondFactor(password, code, passkey);
			code = '';
			if (answer) return answer;
			await load();
			onchange?.();
			return null;
		});
	}

	function switchOff(event: SubmitEvent) {
		event.preventDefault();
		void switchOffWith();
	}

	async function kept() {
		codes = null;
		await load();
	}

	// The key in groups of four, as apps show it and people type it.
	const grouped = (secret: string) => secret.match(/.{1,4}/g)?.join(' ') ?? secret;
</script>

<section class="space-y-3 text-sm">
	<h3 class="font-semibold">{texts.title}</h3>
	<p class="text-soft">{texts.intro}</p>

	{#if reader.account?.adminNeedsSecondFactor}
		<p class="rounded border border-line bg-panel p-2" role="status">{texts.adminNeeds}</p>
	{/if}

	{#if codes}
		<div class="space-y-2 rounded border border-line p-3">
			<h4 class="font-semibold">{texts.codesTitle}</h4>
			<p class="text-soft">{texts.codesIntro}</p>
			<ul class="grid grid-cols-2 gap-1 font-mono select-all">
				{#each codes as one (one)}
					<li>{one}</li>
				{/each}
			</ul>
			<div class="flex flex-wrap gap-2">
				<button type="button" class="tool" onclick={() => copy('codes', codes!.join('\n'))}>
					{copied === 'codes' ? texts.copied : texts.copyCodes}
				</button>
				<button type="button" class="tool" onclick={kept}>{texts.codesDone}</button>
			</div>
		</div>
	{:else if setup}
		<div class="space-y-2 rounded border border-line p-3">
			<p>{texts.scan}</p>
			<button
				type="button"
				class={['block', large ? 'cursor-zoom-out' : 'cursor-zoom-in']}
				aria-pressed={large}
				aria-label={large ? texts.smaller : texts.larger}
				title={large ? texts.smaller : texts.larger}
				onclick={() => (large = !large)}
			>
				<img
					class={['aspect-square bg-white', large ? 'w-96 max-w-full' : 'w-48']}
					src="data:image/svg+xml,{encodeURIComponent(setup.qrSvg)}"
					alt={texts.qr}
				/>
			</button>
			<p class="flex flex-wrap items-center gap-2">
				<span class="text-soft">{texts.key}:</span>
				<code class="font-mono select-all">{grouped(setup.secret)}</code>
				<button type="button" class="tool" onclick={() => copy('key', setup!.secret)}>
					{copied === 'key' ? texts.copied : texts.copyKey}
				</button>
			</p>
			<form class="flex flex-wrap items-end gap-2" onsubmit={confirm}>
				<label class="block">
					<span class="block text-soft">{texts.code}</span>
					<input
						class="mt-1 w-32 rounded py-1 tracking-widest"
						autocomplete="one-time-code"
						inputmode="numeric"
						maxlength="6"
						required
						bind:value={code}
					/>
				</label>
				<button type="submit" class="tool" disabled={busy || code.trim().length !== 6}>
					{texts.confirm}
				</button>
				<button type="button" class="tool" onclick={() => (setup = null)}>{texts.cancel}</button>
			</form>
		</div>
	{:else if current}
		<p>
			<span class="text-soft">{texts.app}:</span>
			{#if current.app}
				<strong>{texts.on}</strong>
			{:else if current.passkeys > 0}
				{texts.byPasskeys}
			{:else}
				<strong>{texts.off}</strong>
			{/if}
			{#if current.on}
				{texts.left(current.recoveryCodesLeft)}
			{/if}
		</p>
		<form class="flex flex-wrap items-end gap-2" onsubmit={current.app ? switchOff : start}>
			<label class="block">
				<span class="block text-soft">{texts.password}</span>
				<input
					class="mt-1 w-56 rounded py-1"
					type="password"
					autocomplete="current-password"
					bind:value={password}
				/>
			</label>
			{#if current.on}
				<label class="block">
					<span class="block text-soft">{texts.codeOrRecovery}</span>
					<input
						class="mt-1 w-40 rounded py-1 tracking-widest"
						autocomplete="one-time-code"
						maxlength="32"
						required={current.app}
						bind:value={code}
					/>
				</label>
			{/if}
			<button type="submit" class="tool" disabled={busy}>
				{current.app ? texts.switchOff : texts.start}
			</button>
		</form>
		<PasskeyProof
			{reader}
			has={current.passkeys > 0}
			label={current.app ? texts.switchOffWithPasskey : texts.startWithPasskey}
			disabled={busy}
			onproof={(passkey) => (current?.app ? switchOffWith(passkey) : startWith(passkey))}
		/>
		{#if current.app}
			<p class="text-xs text-soft">
				{current.passkeys > 0 ? texts.switchOffNoteKeys : texts.switchOffNote}
			</p>
		{/if}
	{/if}

	{#if refusal}
		<p class="text-warn" role="alert">{texts.refusals[refusal]}</p>
	{/if}
</section>
