<script lang="ts">
	// The login: name and password, and where the server has it, the way through an
	// OpenID Connect provider. Where the server lets anybody register, or the page
	// was opened with an invitation, it is also where an account is made.
	import type { LoginFailure, RegisterFailure, Session } from '#lib/api/session.svelte.ts';
	import { m } from '#lib/i18n/index.ts';
	import Icon from './Icon.svelte';

	let { session }: { session: Session } = $props();

	let name = $state('');
	let password = $state('');
	let busy = $state(false);
	let failure: LoginFailure | null = $state(null);
	/** The account has a second factor: the login takes its code too. */
	let needsCode = $state(false);
	let code = $state('');

	/** Making an account instead of logging in to one. */
	let registering = $state(false);
	let again = $state('');
	let refusal: RegisterFailure | null = $state(null);

	const invitation = $derived(session.invitation);
	const mayRegister = $derived(session.ways.registration || invitation?.valid === true);
	// Whoever comes with an invitation that can be used is here to make an account.
	let greeted = false;
	$effect(() => {
		if (invitation?.valid && !greeted) {
			greeted = true;
			registering = true;
		}
	});

	/** The shortest password the server takes. */
	const MIN_PASSWORD = 8;
	const mismatch = $derived(again !== '' && again !== password);
	const ready = $derived(
		name.trim() !== '' && password.length >= MIN_PASSWORD && again === password
	);

	function refused(refusal: RegisterFailure) {
		switch (refusal.kind) {
			case 'closed':
				return m.register.closed;
			case 'invite':
				return m.register.inviteGone;
			case 'full':
				return m.register.full;
			case 'today':
				return m.register.today;
			case 'taken':
				return m.register.taken;
			case 'invalid':
				return m.register.invalid(refusal.reason);
			case 'unreachable':
				return m.login.unreachable;
		}
	}

	/** How the way through the provider ended, which the server says in the address. */
	const outcomes = {
		failed: () => m.login.oidcFailed,
		denied: () => m.login.oidcDenied,
		unknown: () => m.login.oidcUnknown,
		disabled: () => m.login.disabled
	};
	const came = new URLSearchParams(location.search).get('oidc') ?? '';
	const outcome = came in outcomes ? outcomes[came as keyof typeof outcomes] : null;
	// Said once: a reload of the page does not say it again.
	if (came) history.replaceState(history.state, '', location.pathname);

	const problem = $derived.by(() => {
		if (session.state === 'unreachable') return m.login.unreachable;
		if (registering) return refusal ? refused(refusal) : null;
		// An invitation that cannot be used is said once, where the login is.
		if (invitation && !invitation.valid && !failure) return m.register.inviteGone;
		switch (failure?.kind) {
			case 'wrong':
				return m.login.wrong;
			case 'codeNeeded':
				return m.login.codeNeeded;
			case 'wrongCode':
				return m.login.wrongCode;
			case 'tooMany':
				return m.login.tooMany(failure.minutes);
			case 'disabled':
				return m.login.disabled;
			case 'unreachable':
				return m.login.unreachable;
			default:
				return outcome?.() ?? null;
		}
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !name.trim() || !password) return;
		busy = true;
		failure = null;
		try {
			failure = await session.login(name, password, needsCode ? code : undefined);
			if (failure?.kind === 'codeNeeded' || failure?.kind === 'wrongCode') {
				needsCode = true;
				code = '';
			} else if (failure) {
				password = '';
			}
		} finally {
			busy = false;
		}
	}

	async function register(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !ready) return;
		busy = true;
		refusal = null;
		try {
			refusal = await session.register(name, password);
			// The invitation did its work; the address no longer needs to carry it.
			if (!refusal) history.replaceState(history.state, '', location.pathname);
		} finally {
			busy = false;
		}
	}

	function turn(to: boolean) {
		registering = to;
		failure = null;
		refusal = null;
		password = '';
		again = '';
		needsCode = false;
		code = '';
	}

	const focus = (field: HTMLInputElement) => field.focus();
</script>

<svelte:head>
	<title>rueder</title>
</svelte:head>

<main class="flex min-h-dvh items-center justify-center bg-panel p-4 text-ink">
	<div class="w-full max-w-sm rounded-lg border border-line bg-paper p-6 shadow-sm">
		<h1 class="flex items-center gap-2 text-lg font-semibold">
			<svg class="size-6" viewBox="0 0 24 24" aria-hidden="true">
				<rect width="24" height="24" rx="5" fill="var(--color-accent)" />
				<g fill="none" stroke="var(--color-on-accent)" stroke-width="2.4" stroke-linecap="round">
					<path d="M7 11a6 6 0 0 1 6 6" />
					<path d="M7 6a11 11 0 0 1 11 11" />
				</g>
				<circle cx="7.6" cy="16.6" r="1.6" fill="var(--color-on-accent)" />
			</svg>
			{registering ? m.register.title : m.login.title}
		</h1>

		{#if session.farewell && !problem}
			<p class="mt-4 rounded bg-panel px-3 py-2 text-sm" role="status">{m.login.removed}</p>
		{/if}

		{#if problem}
			<p class="mt-4 flex gap-2 rounded bg-panel px-3 py-2 text-sm" role="alert">
				<span class="mt-0.5 text-warn"><Icon name="warning" /></span>
				<span>{problem}</span>
			</p>
		{/if}

		{#if session.state === 'unreachable'}
			<button type="button" class="tool mt-4" onclick={() => session.check()}>
				{m.login.retry}
			</button>
		{:else if registering}
			{#if invitation?.valid}
				<p class="mt-3 text-sm text-soft">{m.register.invited}</p>
			{/if}
			<form class="mt-4 space-y-3" onsubmit={register}>
				<label class="block text-sm">
					<span class="font-medium">{m.login.name}</span>
					<input
						class="mt-1.5 block w-full rounded py-1.5 text-sm"
						name="username"
						autocomplete="username"
						autocapitalize="none"
						spellcheck="false"
						maxlength="64"
						required
						bind:value={name}
						use:focus
					/>
				</label>
				<label class="block text-sm">
					<span class="font-medium">{m.login.password}</span>
					<input
						class="mt-1.5 block w-full rounded py-1.5 text-sm"
						type="password"
						name="new-password"
						autocomplete="new-password"
						minlength={MIN_PASSWORD}
						required
						aria-describedby="password-rule"
						bind:value={password}
					/>
				</label>
				<p id="password-rule" class="-mt-2 text-xs text-soft">
					{m.register.passwordRule(MIN_PASSWORD)}
				</p>
				<label class="block text-sm">
					<span class="font-medium">{m.register.again}</span>
					<input
						class="mt-1.5 block w-full rounded py-1.5 text-sm"
						type="password"
						name="new-password-again"
						autocomplete="new-password"
						required
						aria-invalid={mismatch}
						aria-describedby={mismatch ? 'password-mismatch' : undefined}
						bind:value={again}
					/>
				</label>
				{#if mismatch}
					<p id="password-mismatch" class="-mt-2 text-xs text-warn">{m.register.mismatch}</p>
				{/if}
				<button
					type="submit"
					class="w-full cursor-pointer rounded bg-accent px-3 py-1.5 text-sm font-semibold text-on-accent hover:brightness-110 disabled:cursor-default disabled:opacity-50"
					disabled={busy || !ready}
				>
					{m.register.submit}
				</button>
			</form>
			<p class="mt-4 text-sm">
				<button
					type="button"
					class="cursor-pointer text-link hover:underline"
					onclick={() => turn(false)}
				>
					{m.register.toLogin}
				</button>
			</p>
		{:else}
			<form class="mt-4 space-y-3" onsubmit={submit}>
				<label class="block text-sm">
					<span class="font-medium">{m.login.name}</span>
					<input
						class="mt-1.5 block w-full rounded py-1.5 text-sm"
						name="username"
						autocomplete="username"
						autocapitalize="none"
						spellcheck="false"
						required
						bind:value={name}
						use:focus
					/>
				</label>
				<label class="block text-sm">
					<span class="font-medium">{m.login.password}</span>
					<input
						class="mt-1.5 block w-full rounded py-1.5 text-sm"
						type="password"
						name="password"
						autocomplete="current-password"
						required
						bind:value={password}
					/>
				</label>
				{#if needsCode}
					<label class="block text-sm">
						<span class="font-medium">{m.login.code}</span>
						<input
							class="mt-1.5 block w-full rounded py-1.5 text-sm tracking-widest"
							name="code"
							autocomplete="one-time-code"
							autocapitalize="characters"
							spellcheck="false"
							maxlength="32"
							required
							aria-describedby="code-note"
							bind:value={code}
							use:focus
						/>
					</label>
					<p id="code-note" class="-mt-2 text-xs text-soft">{m.login.codeNote}</p>
				{/if}
				<button
					type="submit"
					class="w-full cursor-pointer rounded bg-accent px-3 py-1.5 text-sm font-semibold text-on-accent hover:brightness-110 disabled:cursor-default disabled:opacity-50"
					disabled={busy || !name.trim() || !password || (needsCode && !code.trim())}
				>
					{m.login.submit}
				</button>
			</form>

			{#if session.ways.oidc}
				<p class="my-3 text-center text-xs text-soft">{m.login.or}</p>
				<!-- The server sends the browser on to the provider and takes it back in. -->
				<a class="tool w-full justify-center" href="/api/oidc/start" rel="external">
					{m.login.oidc}
				</a>
			{/if}

			{#if mayRegister}
				<p class="mt-4 text-sm">
					<button
						type="button"
						class="cursor-pointer text-link hover:underline"
						onclick={() => turn(true)}
					>
						{m.register.toRegister}
					</button>
				</p>
			{:else}
				<p class="mt-4 text-xs text-soft">{m.login.note}</p>
			{/if}
		{/if}
	</div>
</main>
