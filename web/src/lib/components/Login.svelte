<script lang="ts">
	// The login: name and password, and where the server has it, the way through an
	// OpenID Connect provider. Where the server lets anybody register, or the page
	// was opened with an invitation, it is also where an account is made.
	import type {
		ForgotOutcome,
		LoginFailure,
		RegisterFailure,
		ResendOutcome,
		ResetFailure,
		Session
	} from '#lib/api/session.svelte.ts';
	import { m } from '#lib/i18n/index.ts';
	import { EMAIL_LINK_HOURS, RESET_LINK_MINUTES } from '#lib/reader.ts';
	import Icon from './Icon.svelte';

	let { session }: { session: Session } = $props();

	/** The shortest password the server takes. */
	const MIN_PASSWORD = 8;

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
	let email = $state('');
	let refusal: RegisterFailure | null = $state(null);
	/** Whether registration asks for an address, and whether it must be one. */
	const asksEmail = $derived(session.emailMode !== 'off');
	const needsEmail = $derived(session.emailMode === 'required');
	/** How asking for the link again went. */
	let resent: ResendOutcome | null = $state(null);

	/** Asking for a link that sets a new password, instead of logging in. */
	let forgetting = $state(false);
	let forgotten: ForgotOutcome | null = $state(null);
	/** The page came with such a link: the form for the new password. */
	const resetting = $derived(session.resetting !== null);
	let resetFailure: ResetFailure | null = $state(null);
	const resetReady = $derived(
		password.length >= MIN_PASSWORD && again === password && session.resetting !== null
	);
	/** The reader agreed to the terms of use, where the server names a version of them. */
	let agreed = $state(false);
	const legal = $derived(session.ways.legal);
	const needsAgreement = $derived(legal.terms_version !== null && legal.terms !== null);

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

	const mismatch = $derived(again !== '' && again !== password);
	const ready = $derived(
		name.trim() !== '' &&
			password.length >= MIN_PASSWORD &&
			again === password &&
			(!needsEmail || email.trim() !== '') &&
			(agreed || !needsAgreement)
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
			case 'terms':
				return m.register.terms;
			case 'taken':
				return m.register.taken;
			case 'invalid':
				return m.register.invalid(refusal.reason);
			case 'email':
				return m.register.emailInvalid;
			case 'unsent':
				return m.register.unsent;
			case 'tooMany':
				return m.register.tooMany(refusal.minutes);
			case 'unreachable':
				return m.login.unreachable;
		}
	}

	function resendSaid(outcome: ResendOutcome) {
		switch (outcome.kind) {
			case 'sent':
				return { hint: m.login.resent };
			case 'wrong':
				return m.login.resendWrong;
			case 'tooMany':
				return m.register.tooMany(outcome.minutes);
			case 'unsent':
				return m.email.unsent;
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

	function resetSaid(failure: ResetFailure) {
		switch (failure.kind) {
			case 'gone':
				return m.reset.gone;
			case 'invalid':
				return m.reset.invalid(failure.reason);
			case 'unreachable':
				return m.login.unreachable;
		}
	}

	function forgotSaid(outcome: ForgotOutcome) {
		switch (outcome.kind) {
			case 'asked':
				return { hint: m.forgot.asked(RESET_LINK_MINUTES) };
			case 'tooMany':
				return m.forgot.tooMany(outcome.minutes);
			case 'unreachable':
				return m.login.unreachable;
		}
	}

	/** What went wrong, or a hint of what to do next, which is not an error. */
	const said = $derived.by(() => {
		if (session.state === 'unreachable') return m.login.unreachable;
		// Said until the reader does something else, also once a link that is gone left.
		if (resetFailure) return resetSaid(resetFailure);
		if (resetting) return null;
		if (forgetting) return forgotten ? forgotSaid(forgotten) : null;
		if (registering) return refusal ? refused(refusal) : null;
		if (resent) return resendSaid(resent);
		// An invitation that cannot be used is said once, where the login is.
		if (invitation && !invitation.valid && !failure) return m.register.inviteGone;
		switch (failure?.kind) {
			case 'wrong':
				return m.login.wrong;
			case 'codeNeeded':
				return { hint: m.login.codeNeeded };
			case 'wrongCode':
				return m.login.wrongCode;
			case 'tooMany':
				return m.login.tooMany(failure.minutes);
			case 'disabled':
				return m.login.disabled;
			case 'unconfirmed':
				return { hint: m.login.unconfirmed };
			case 'passkeyCancelled':
				return { hint: m.login.passkeyCancelled };
			case 'passkeyFailed':
				return m.login.passkeyFailed;
			case 'passkeyWrong':
				return m.login.passkeyWrong;
			case 'unreachable':
				return m.login.unreachable;
			default:
				if (session.passwordSet) return { hint: m.reset.done };
				return outcome?.() ?? null;
		}
	});
	const problem = $derived(typeof said === 'string' ? said : null);
	const hint = $derived(said !== null && typeof said === 'object' ? said.hint : null);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !name.trim() || !password) return;
		busy = true;
		failure = null;
		resent = null;
		resetFailure = null;
		try {
			failure = await session.login(name, password, needsCode ? code : undefined);
			if (failure?.kind === 'codeNeeded' || failure?.kind === 'wrongCode') {
				needsCode = true;
				code = '';
			} else if (failure && failure.kind !== 'unconfirmed') {
				// The password of an account that waits is kept, to send its link again.
				password = '';
			}
		} finally {
			busy = false;
		}
	}

	/** Logs in with a passkey: no name, no password, the device says whose it is. */
	async function withPasskey() {
		if (busy) return;
		busy = true;
		failure = null;
		try {
			failure = await session.loginWithPasskey();
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
			refusal = await session.register(name, password, agreed, email);
			// The invitation did its work; the address no longer needs to carry it.
			if (!refusal) history.replaceState(history.state, '', location.pathname);
			// Made, the account waits for its link: the login says so and logs in once it is opened.
			if (!refusal && session.awaiting) {
				registering = false;
				again = '';
			}
		} finally {
			busy = false;
		}
	}

	/** Has the link mailed to an account that waits for its address sent again. */
	async function resend() {
		if (busy || !name.trim() || !password) return;
		busy = true;
		failure = null;
		try {
			resent = await session.resend(name, password);
		} finally {
			busy = false;
		}
	}

	async function askForLink(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !name.trim()) return;
		busy = true;
		forgotten = null;
		try {
			forgotten = await session.forgot(name);
		} finally {
			busy = false;
		}
	}

	async function setPassword(event: SubmitEvent) {
		event.preventDefault();
		if (busy || !resetReady) return;
		busy = true;
		resetFailure = null;
		try {
			resetFailure = await session.resetPassword(password);
			password = '';
			again = '';
			if (!resetFailure) {
				forgetting = false;
				registering = false;
			}
		} finally {
			busy = false;
		}
	}

	/** Leaves the form for the new password, and its link with it. */
	function cancelReset() {
		session.resetting = null;
		resetFailure = null;
		password = '';
		again = '';
	}

	/** Between the login and asking for a link to set a new password. */
	function forget(to: boolean) {
		forgetting = to;
		forgotten = null;
		resetFailure = null;
		failure = null;
		resent = null;
		password = '';
		needsCode = false;
		code = '';
	}

	/** The account waits for its link: said after registering, or after the right password. */
	const waiting = $derived.by(
		() => !registering && (session.awaiting !== null || failure?.kind === 'unconfirmed')
	);

	function turn(to: boolean) {
		registering = to;
		failure = null;
		resetFailure = null;
		refusal = null;
		resent = null;
		session.awaiting = null;
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

<main class="flex min-h-dvh flex-col items-center justify-center bg-panel p-4 pb-14 text-ink">
	<!-- Who it is before what to do: the logo, the name and what rueder does. -->
	<header class="mb-6 flex max-w-sm flex-col items-center text-center">
		<svg class="size-14" viewBox="0 0 24 24" aria-hidden="true">
			<rect width="24" height="24" rx="5" fill="var(--color-accent)" />
			<g fill="none" stroke="var(--color-on-accent)" stroke-width="2.4" stroke-linecap="round">
				<path d="M7 11a6 6 0 0 1 6 6" />
				<path d="M7 6a11 11 0 0 1 11 11" />
			</g>
			<circle cx="7.6" cy="16.6" r="1.6" fill="var(--color-on-accent)" />
		</svg>
		<p class="mt-2 text-3xl font-semibold tracking-tight">rueder</p>
		<p class="mt-1 text-soft">{m.login.about}</p>
	</header>
	<div class="w-full max-w-sm rounded-lg border border-line bg-paper p-6 shadow-sm">
		<h1 class="text-lg font-semibold">
			{resetting
				? m.reset.title
				: forgetting
					? m.forgot.title
					: registering
						? m.register.title
						: m.login.title}
		</h1>

		{#if session.farewell && !said}
			<p class="mt-4 rounded bg-panel px-3 py-2 text-sm" role="status">{m.login.removed}</p>
		{/if}

		{#if session.awaiting && !registering}
			<div class="mt-4 rounded bg-panel px-3 py-2 text-sm" role="status">
				<p class="font-medium">{m.register.awaitingTitle}</p>
				<p class="mt-1">{m.register.awaiting(session.awaiting.address, EMAIL_LINK_HOURS)}</p>
			</div>
		{/if}

		{#if problem}
			<p class="mt-4 flex gap-2 rounded border border-warn bg-panel px-3 py-2 text-sm" role="alert">
				<span class="mt-0.5 text-warn"><Icon name="warning" /></span>
				<span>{problem}</span>
			</p>
		{:else if hint}
			<p class="mt-4 flex gap-2 rounded bg-panel px-3 py-2 text-sm" role="status">
				<span class="mt-0.5 text-soft"><Icon name="info" /></span>
				<span>{hint}</span>
			</p>
		{/if}

		{#if session.state === 'unreachable'}
			<button type="button" class="tool mt-4" onclick={() => session.check()}>
				{m.login.retry}
			</button>
		{:else if resetting}
			<p class="mt-3 text-sm text-soft">{m.reset.intro}</p>
			<form class="mt-4 space-y-3" onsubmit={setPassword}>
				<label class="block text-sm">
					<span class="font-medium">{m.reset.fresh}</span>
					<input
						class="mt-1.5 block w-full rounded py-1.5 text-sm"
						type="password"
						name="new-password"
						autocomplete="new-password"
						minlength={MIN_PASSWORD}
						required
						aria-describedby="reset-rule"
						bind:value={password}
						use:focus
					/>
				</label>
				<p id="reset-rule" class="-mt-2 text-xs text-soft">
					{m.register.passwordRule(MIN_PASSWORD)}
				</p>
				<label class="block text-sm">
					<span class="font-medium">{m.reset.again}</span>
					<input
						class="mt-1.5 block w-full rounded py-1.5 text-sm"
						type="password"
						name="new-password-again"
						autocomplete="new-password"
						required
						aria-invalid={mismatch}
						bind:value={again}
					/>
				</label>
				{#if mismatch}
					<p class="-mt-2 text-xs text-warn">{m.reset.mismatch}</p>
				{/if}
				<button type="submit" class="primary h-9 w-full" disabled={busy || !resetReady}>
					{m.reset.submit}
				</button>
			</form>
			<p class="mt-4 text-sm">
				<button
					type="button"
					class="cursor-pointer text-link hover:underline"
					onclick={cancelReset}
				>
					{m.reset.cancel}
				</button>
			</p>
		{:else if forgetting}
			<p class="mt-3 text-sm text-soft">{m.forgot.intro}</p>
			<form class="mt-4 space-y-3" onsubmit={askForLink}>
				<label class="block text-sm">
					<span class="font-medium">{m.forgot.field}</span>
					<input
						class="mt-1.5 block w-full rounded py-1.5 text-sm"
						name="username"
						autocomplete="username"
						autocapitalize="none"
						spellcheck="false"
						maxlength="254"
						required
						bind:value={name}
						use:focus
					/>
				</label>
				<button type="submit" class="primary h-9 w-full" disabled={busy || !name.trim()}>
					{m.forgot.submit}
				</button>
			</form>
			<p class="mt-4 text-sm">
				<button
					type="button"
					class="cursor-pointer text-link hover:underline"
					onclick={() => forget(false)}
				>
					{m.forgot.back}
				</button>
			</p>
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
				{#if asksEmail}
					<label class="block text-sm">
						<span class="font-medium"
							>{needsEmail ? m.register.email : m.register.emailOptional}</span
						>
						<input
							class="mt-1.5 block w-full rounded py-1.5 text-sm"
							type="email"
							name="email"
							autocomplete="email"
							autocapitalize="none"
							spellcheck="false"
							maxlength="254"
							required={needsEmail}
							aria-describedby="email-note"
							bind:value={email}
						/>
					</label>
					<p id="email-note" class="-mt-2 text-xs text-soft">
						{needsEmail ? m.register.emailRequiredNote : m.register.emailNote}
					</p>
				{/if}
				{#if needsAgreement && legal.terms}
					<p class="flex items-start gap-2 text-sm pointer-coarse:items-center">
						<input
							id="agree"
							type="checkbox"
							class="mt-0.5 rounded text-accent"
							required
							bind:checked={agreed}
						/>
						<label
							for="agree"
							class="pointer-coarse:inline-flex pointer-coarse:min-h-11 pointer-coarse:items-center"
							><span
								>{m.legal.agree.before}<a
									class="reach text-link underline"
									href={legal.terms}
									target="_blank"
									rel="noopener noreferrer">{m.legal.agree.link}</a
								>{m.legal.agree.after}</span
							></label
						>
					</p>
				{/if}
				<button type="submit" class="primary h-9 w-full" disabled={busy || !ready}>
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
					class="primary h-9 w-full"
					disabled={busy || !name.trim() || !password || (needsCode && !code.trim())}
				>
					{m.login.submit}
				</button>
				{#if waiting}
					<button
						type="button"
						class="tool w-full justify-center"
						disabled={busy || !name.trim() || !password}
						onclick={resend}
					>
						{m.login.resend}
					</button>
				{/if}
			</form>

			{#if session.passkeys}
				<p class="my-3 text-center text-xs text-soft">{m.login.or}</p>
				<button
					type="button"
					class="tool w-full justify-center"
					disabled={busy}
					onclick={withPasskey}
				>
					{m.login.passkey}
				</button>
			{/if}

			{#if session.ways.oidc}
				<p class="my-3 text-center text-xs text-soft">{m.login.or}</p>
				<!-- The server sends the browser on to the provider and takes it back in. -->
				<a class="tool w-full justify-center" href="/api/oidc/start" rel="external">
					{m.login.oidc}
				</a>
			{/if}

			{#if session.ways.password_reset}
				<p class="mt-3 text-sm">
					<button
						type="button"
						class="cursor-pointer text-link hover:underline"
						onclick={() => forget(true)}
					>
						{m.login.forgot}
					</button>
				</p>
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
	{#if legal.imprint || legal.privacy || legal.terms}
		<!-- The operator's texts, as the server names them. -->
		<nav class="fixed inset-x-0 bottom-0 flex justify-center gap-4 p-3 text-xs text-soft">
			{#each [[legal.imprint, m.legal.imprint], [legal.privacy, m.legal.privacy], [legal.terms, m.legal.terms]] as [href, label] (label)}
				{#if href}
					<a class="reach hover:underline" {href} target="_blank" rel="noopener noreferrer"
						>{label}</a
					>
				{/if}
			{/each}
		</nav>
	{/if}
</main>
