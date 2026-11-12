<script lang="ts">
	// The second factor of the account, in the settings: whether it is on, setting
	// it up with the password, a QR code and a code of the app, the recovery codes
	// shown once, and switching it off with the password.
	import { m } from '#lib/i18n/index.ts';
	import type {
		Reader,
		SecondFactorRefusal,
		SecondFactorSetup,
		SecondFactorState
	} from '#lib/reader.ts';

	let { reader }: { reader: Reader } = $props();

	const texts = m.settings.secondFactor;

	let current = $state.raw<SecondFactorState | null>(null);
	let setup = $state.raw<SecondFactorSetup | null>(null);
	let codes = $state.raw<string[] | null>(null);
	let password = $state('');
	let code = $state('');
	let refusal = $state<SecondFactorRefusal | null>(null);
	let busy = $state(false);

	async function load() {
		current = await reader.secondFactor();
	}
	$effect(() => {
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

	function start(event: SubmitEvent) {
		event.preventDefault();
		void step(async () => {
			const answer = await reader.setUpSecondFactor(password);
			if (isRefusal(answer)) return answer;
			setup = answer;
			return null;
		});
	}

	function confirm(event: SubmitEvent) {
		event.preventDefault();
		void step(async () => {
			const answer = await reader.confirmSecondFactor(code);
			code = '';
			if (isRefusal(answer)) return answer;
			setup = null;
			codes = answer;
			return null;
		});
	}

	function switchOff(event: SubmitEvent) {
		event.preventDefault();
		void step(async () => {
			const answer = await reader.switchOffSecondFactor(password);
			if (answer) return answer;
			await load();
			return null;
		});
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
			<ul class="grid grid-cols-2 gap-1 font-mono">
				{#each codes as one (one)}
					<li>{one}</li>
				{/each}
			</ul>
			<button type="button" class="tool" onclick={kept}>{texts.codesDone}</button>
		</div>
	{:else if setup}
		<div class="space-y-2 rounded border border-line p-3">
			<p>{texts.scan}</p>
			<img
				class="size-48 bg-white"
				src="data:image/svg+xml,{encodeURIComponent(setup.qrSvg)}"
				alt={texts.qr}
			/>
			<p>
				<span class="text-soft">{texts.key}:</span>
				<code class="font-mono select-all">{grouped(setup.secret)}</code>
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
			{#if current.on}
				<strong>{texts.on}</strong>
				{texts.left(current.recoveryCodesLeft)}
			{:else}
				<strong>{texts.off}</strong>
			{/if}
		</p>
		<form class="flex flex-wrap items-end gap-2" onsubmit={current.on ? switchOff : start}>
			<label class="block">
				<span class="block text-soft">{texts.password}</span>
				<input
					class="mt-1 w-56 rounded py-1"
					type="password"
					autocomplete="current-password"
					bind:value={password}
				/>
			</label>
			<button type="submit" class="tool" disabled={busy}>
				{current.on ? texts.switchOff : texts.start}
			</button>
		</form>
		{#if current.on}
			<p class="text-xs text-soft">{texts.switchOffNote}</p>
		{/if}
	{/if}

	{#if refusal}
		<p class="text-warn" role="alert">{texts.refusals[refusal]}</p>
	{/if}
</section>
