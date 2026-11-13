<script lang="ts">
	// Apps of other makers, in the settings: the app passwords there are, and
	// setting up an app in three steps: which app, an app password for it, and
	// what to enter in the app, until the app logs in. While a step is under
	// way, `busy` says so, so that the window around it is not closed by
	// mistake.
	import { onMount } from 'svelte';
	import { knownApps, serverAddress, shot, type KnownApp } from '#lib/apps.ts';
	import { formatAgo, formatFull } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { AppPasswordRefusal, AppPasswords, NewAppPassword, Reader } from '#lib/reader.ts';

	let { reader, busy: settingUp = $bindable(false) }: { reader: Reader; busy?: boolean } = $props();

	const texts = m.settings.apps;

	/** The app being set up; `other` for one that is not in the list. */
	type Choice = KnownApp | 'other';

	let apps = $state.raw<AppPasswords | null>(null);
	let step = $state<'list' | 'choose' | 'password' | 'enter'>('list');
	let chosen = $state.raw<Choice | null>(null);
	let name = $state('');
	let password = $state('');
	let code = $state('');
	let needsCode = $state(false);
	let refusal = $state<AppPasswordRefusal | null>(null);
	let sending = $state(false);
	let made = $state.raw<NewAppPassword | null>(null);
	let copied = $state<'server' | 'user' | 'secret' | null>(null);

	// The time "ago" counts from: when the list was loaded last.
	const now = $derived.by(() => {
		void apps;
		return Date.now();
	});
	const hasPassword = $derived(reader.account?.password !== false);
	const user = $derived(reader.account?.name ?? '');
	const address = $derived(
		serverAddress(location.origin, chosen !== null && chosen !== 'other' && chosen.slash)
	);
	const connected = $derived(
		made !== null && apps?.list.some((app) => app.id === made?.id && app.usedAt !== null)
	);

	$effect(() => {
		settingUp = step === 'password' || step === 'enter';
	});

	async function load() {
		apps = await reader.appPasswords();
	}

	// Again whenever app passwords change elsewhere, or an app logs in.
	$effect(() => {
		void reader.appsVersion;
		void load();
	});

	onMount(() => {
		if (reader.can.secondFactor) {
			void reader.secondFactor().then((state) => (needsCode = state?.on ?? false));
		}
	});

	function choose(app: Choice) {
		chosen = app;
		name = app === 'other' ? '' : app.name;
		password = '';
		code = '';
		refusal = null;
		step = 'password';
	}

	async function create(event: SubmitEvent) {
		event.preventDefault();
		if (sending) return;
		sending = true;
		refusal = null;
		const answer = await reader.createAppPassword(name, password, code);
		sending = false;
		password = '';
		code = '';
		if (typeof answer === 'string') {
			refusal = answer;
			if (answer === 'codeNeeded') needsCode = true;
			return;
		}
		made = answer;
		copied = null;
		step = 'enter';
		await load();
	}

	async function copy(what: 'server' | 'user' | 'secret', text: string) {
		try {
			await navigator.clipboard.writeText(text);
			copied = what;
		} catch {
			// Without the clipboard the text is there to be marked and copied by hand.
		}
	}

	function finish() {
		made = null;
		chosen = null;
		step = 'list';
	}

	async function revoke(id: number) {
		if (await reader.revokeAppPassword(id)) await load();
	}

	const appName = (app: Choice | null) =>
		app === null || app === 'other' ? texts.other : app.name;
	const how = (app: Choice | null) =>
		app === null || app === 'other' ? texts.how.other : texts.how[app.id];
</script>

{#snippet value(label: string, what: 'server' | 'user' | 'secret', text: string)}
	<div class="flex flex-wrap items-center gap-2">
		<span class="w-36 text-soft">{label}</span>
		<code class="font-mono break-all select-all">{text}</code>
		<button type="button" class="tool" onclick={() => copy(what, text)}>
			{copied === what ? texts.copied : texts.copy(label)}
		</button>
	</div>
{/snippet}

<section class="space-y-3 text-sm">
	<p class="text-soft">{texts.intro}</p>

	{#if step === 'list'}
		{#if apps}
			{#if apps.list.length === 0}
				<p>{texts.none}</p>
			{:else}
				<ul class="divide-y divide-line rounded border border-line">
					{#each apps.list as app (app.id)}
						<li class="flex flex-wrap items-center gap-x-3 gap-y-1 px-3 py-2">
							<span class="font-medium">{app.name}</span>
							<span class="text-xs text-soft" title={formatFull(app.createdAt)}>
								{texts.made(formatAgo(app.createdAt, now))}
							</span>
							<span class="text-xs text-soft">
								{app.usedAt === null ? texts.unused : texts.used(formatAgo(app.usedAt, now))}
							</span>
							<button
								type="button"
								class="tool ml-auto"
								aria-label={texts.revokeName(app.name)}
								onclick={() => revoke(app.id)}
							>
								{texts.revoke}
							</button>
						</li>
					{/each}
				</ul>
			{/if}
			{#if apps.list.length >= apps.max}
				<p class="text-soft">{texts.full(apps.max)}</p>
			{:else}
				<button type="button" class="tool" onclick={() => (step = 'choose')}>{texts.add}</button>
			{/if}
		{:else}
			<p class="text-warn" role="alert">{texts.refusals.unreachable}</p>
		{/if}
	{:else if step === 'choose'}
		<h3 class="font-semibold">{texts.choose}</h3>
		<ul class="grid gap-2 sm:grid-cols-2">
			{#each knownApps as app (app.id)}
				<li class="flex flex-col gap-1 rounded border border-line p-3">
					<button
						type="button"
						class="cursor-pointer text-left font-semibold hover:underline"
						onclick={() => choose(app)}
					>
						{app.name}
					</button>
					<span class="text-xs text-soft">
						{app.platforms.map((platform) => texts.platforms[platform]).join(', ')}
					</span>
					<a
						class="text-xs text-link hover:underline"
						href={app.url}
						target="_blank"
						rel="noopener noreferrer"
						aria-label={texts.website(app.name)}
					>
						{app.url.replace(/^https:\/\//, '').replace(/\/$/, '')}
					</a>
				</li>
			{/each}
			<li class="flex flex-col gap-1 rounded border border-line p-3">
				<button
					type="button"
					class="cursor-pointer text-left font-semibold hover:underline"
					onclick={() => choose('other')}
				>
					{texts.other}
				</button>
				<span class="text-xs text-soft">{texts.otherHint}</span>
			</li>
		</ul>
		<button type="button" class="tool" onclick={() => (step = 'list')}>{texts.back}</button>
	{:else if step === 'password'}
		<h3 class="font-semibold">{appName(chosen)}</h3>
		<form class="space-y-2" onsubmit={create}>
			<label class="block">
				<span class="block text-soft">{texts.name}</span>
				<input class="mt-1 w-64 rounded py-1" required maxlength="100" bind:value={name} />
			</label>
			{#if hasPassword}
				<label class="block">
					<span class="block text-soft">{texts.password}</span>
					<input
						class="mt-1 w-64 rounded py-1"
						type="password"
						autocomplete="current-password"
						bind:value={password}
					/>
				</label>
			{/if}
			{#if needsCode}
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
				<button type="submit" class="tool" disabled={sending || name.trim() === ''}>
					{sending ? texts.creating : texts.create}
				</button>
				<button type="button" class="tool" onclick={() => (step = 'choose')}>{texts.back}</button>
			</div>
		</form>
	{:else if made}
		<h3 class="font-semibold">{appName(chosen)}</h3>
		<p>{how(chosen)}</p>
		{#if chosen !== null && chosen !== 'other' && chosen.shots > 0}
			{@const app = chosen}
			<details class="rounded border border-line">
				<summary class="cursor-pointer px-3 py-2 font-medium">{texts.shotsTitle}</summary>
				<ol class="flex gap-3 overflow-x-auto px-3 pb-3">
					{#each texts.shots[app.id] as caption, i (i)}
						<li class="w-40 shrink-0 text-xs">
							<img
								class="w-40 rounded border border-line"
								src={shot(app, i + 1)}
								alt={texts.shotAlt(app.name, i + 1)}
								width="360"
								height="718"
								loading="lazy"
							/>
							<p class="mt-1">{i + 1}. {caption}</p>
						</li>
					{/each}
				</ol>
			</details>
		{/if}
		<div class="space-y-2 rounded border border-line p-3">
			{@render value(texts.server, 'server', address)}
			{#if user}
				{@render value(texts.user, 'user', user)}
			{/if}
			{@render value(texts.secret, 'secret', made.secret)}
		</div>
		<p class="text-xs text-soft">{texts.once}</p>
		{#if connected}
			<p class="font-medium text-accent" role="status">{texts.connected}</p>
		{:else}
			<p class="text-soft" role="status">{texts.waiting}</p>
		{/if}
		<button type="button" class="tool" onclick={finish}>{texts.done}</button>
	{/if}

	{#if refusal}
		<p class="text-warn" role="alert">{texts.refusals[refusal]}</p>
	{/if}
</section>
