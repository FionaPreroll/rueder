<script lang="ts">
	// The account's passkeys, in the account settings: the list, adding one on this
	// device, and removing one, each with the password and, where the second factor
	// is on, a code, or with a passkey instead. The first passkey brings the
	// recovery codes, shown once. While a step is under way or the codes are shown,
	// `busy` says so, so that the window around it is not closed by mistake.
	import { formatAgo, formatFull } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import type {
		PasskeyList,
		PasskeyProof as Proof,
		PasskeyRefusal,
		Reader,
		SecondFactorState
	} from '#lib/reader.ts';
	import PasskeyProof from './PasskeyProof.svelte';

	let {
		reader,
		busy: holding = $bindable(false),
		version = 0,
		onchange
	}: {
		reader: Reader;
		busy?: boolean;
		/** Counts up when something else changed the second factor. */
		version?: number;
		onchange?: () => void;
	} = $props();

	const texts = m.settings.passkeys;

	let keys = $state.raw<PasskeyList | null>(null);
	let factor = $state.raw<SecondFactorState | null>(null);
	let adding = $state(false);
	let removing = $state<number | null>(null);
	let name = $state(texts.defaultName);
	let password = $state('');
	let code = $state('');
	let working = $state(false);
	let refusal = $state<PasskeyRefusal | null>(null);
	let codes = $state.raw<string[] | null>(null);
	let copied = $state(false);

	const hasPassword = $derived(reader.account?.password !== false);
	const has = $derived((keys?.list.length ?? 0) > 0);
	// The time "ago" counts from: when the list was loaded last.
	const now = $derived.by(() => {
		void keys;
		return Date.now();
	});

	$effect(() => {
		holding = adding || removing !== null || codes !== null || working;
	});

	async function load() {
		[keys, factor] = await Promise.all([reader.passkeys(), reader.secondFactor()]);
	}
	$effect(() => {
		void version;
		void load();
	});

	/** Runs a step, says why it did not work, and takes password and code away either way. */
	async function run(work: () => Promise<PasskeyRefusal | null>) {
		if (working) return;
		working = true;
		refusal = null;
		try {
			refusal = await work();
		} finally {
			working = false;
			password = '';
			code = '';
		}
	}

	function add(passkey?: Proof) {
		return run(async () => {
			const answer = await reader.addPasskey(name, password, code, passkey);
			if (typeof answer === 'string') return answer;
			adding = false;
			name = texts.defaultName;
			codes = answer.recoveryCodes;
			copied = false;
			await load();
			onchange?.();
			return null;
		});
	}

	function remove(id: number, passkey?: Proof) {
		return run(async () => {
			const answer = await reader.removePasskey(id, password, code, passkey);
			if (answer) return answer;
			removing = null;
			await load();
			onchange?.();
			return null;
		});
	}

	function submitAdd(event: SubmitEvent) {
		event.preventDefault();
		void add();
	}

	function submitRemove(event: SubmitEvent) {
		event.preventDefault();
		if (removing !== null) void remove(removing);
	}

	function open(what: 'add' | number) {
		refusal = null;
		password = '';
		code = '';
		adding = what === 'add';
		removing = what === 'add' ? null : what;
	}

	function close() {
		adding = false;
		removing = null;
		refusal = null;
	}

	async function copy(text: string) {
		try {
			await navigator.clipboard.writeText(text);
			copied = true;
		} catch {
			// Without the clipboard the codes are there to be marked and copied by hand.
		}
	}
</script>

{#snippet proof(submit: string)}
	{#if hasPassword}
		<label class="block">
			<span class="block text-soft">{texts.password}</span>
			<input
				class="mt-1 w-56 rounded py-1"
				type="password"
				autocomplete="current-password"
				bind:value={password}
			/>
		</label>
	{/if}
	{#if factor?.on}
		<label class="block">
			<span class="block text-soft">{texts.code}</span>
			<input
				class="mt-1 w-40 rounded py-1 tracking-widest"
				autocomplete="one-time-code"
				maxlength="32"
				bind:value={code}
			/>
		</label>
	{/if}
	<div class="flex flex-wrap gap-2">
		<button type="submit" class="tool" disabled={working}>
			{working ? texts.working : submit}
		</button>
		<button type="button" class="tool" onclick={close}>{texts.cancel}</button>
	</div>
{/snippet}

<section class="space-y-3 text-sm">
	<h3 class="font-semibold">{texts.title}</h3>
	<p class="text-soft">{texts.intro}</p>
	<p class="text-soft">{texts.once}</p>

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
				<button type="button" class="tool" onclick={() => copy(codes!.join('\n'))}>
					{copied ? texts.copied : texts.copyCodes}
				</button>
				<button type="button" class="tool" onclick={() => (codes = null)}>
					{texts.codesDone}
				</button>
			</div>
		</div>
	{:else if keys}
		{#if keys.list.length === 0}
			<p>{texts.none}</p>
		{:else}
			<ul class="divide-y divide-line rounded border border-line">
				{#each keys.list as key (key.id)}
					<li class="space-y-2 px-3 py-2">
						<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
							<span class="font-medium">{key.name}</span>
							<span class="text-xs text-soft" title={formatFull(key.createdAt)}>
								{texts.made(formatAgo(key.createdAt, now))}
							</span>
							<span class="text-xs text-soft">
								{key.usedAt === null ? texts.unused : texts.used(formatAgo(key.usedAt, now))}
							</span>
							{#if removing !== key.id}
								<button
									type="button"
									class="tool ml-auto"
									aria-label={texts.removeName(key.name)}
									disabled={working}
									onclick={() => open(key.id)}
								>
									{texts.remove}
								</button>
							{/if}
						</div>
						{#if removing === key.id}
							<form class="space-y-2" onsubmit={submitRemove}>
								{@render proof(texts.confirmRemove)}
							</form>
							<PasskeyProof
								{reader}
								{has}
								disabled={working}
								onproof={(passkey) => remove(key.id, passkey)}
							/>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}

		{#if adding}
			<form class="space-y-2 rounded border border-line p-3" onsubmit={submitAdd}>
				<label class="block">
					<span class="block text-soft">{texts.name}</span>
					<input class="mt-1 w-64 rounded py-1" required maxlength="100" bind:value={name} />
				</label>
				{@render proof(texts.create)}
			</form>
			<PasskeyProof {reader} {has} disabled={working || !name.trim()} onproof={add} />
		{:else if keys.list.length >= keys.max}
			<p class="text-soft">{texts.full(keys.max)}</p>
		{:else if removing === null}
			<button type="button" class="tool" onclick={() => open('add')}>{texts.add}</button>
		{/if}
	{/if}

	{#if refusal}
		<p class="text-warn" role="alert">{texts.refusals[refusal]}</p>
	{/if}
</section>
