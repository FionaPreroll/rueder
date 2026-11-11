<script lang="ts">
	// The login: name and password, and where the server has it, the way through an
	// OpenID Connect provider. There is no sign-up; accounts are made by whoever runs
	// the server.
	import type { LoginFailure, Session } from '#lib/api/session.svelte.ts';
	import { m } from '#lib/i18n/index.ts';
	import Icon from './Icon.svelte';

	let { session }: { session: Session } = $props();

	let name = $state('');
	let password = $state('');
	let busy = $state(false);
	let failure: LoginFailure | null = $state(null);

	/** How the way through the provider ended, which the server says in the address. */
	const outcomes = {
		failed: () => m.login.oidcFailed,
		denied: () => m.login.oidcDenied,
		unknown: () => m.login.oidcUnknown
	};
	const came = new URLSearchParams(location.search).get('oidc') ?? '';
	const outcome = came in outcomes ? outcomes[came as keyof typeof outcomes] : null;
	// Said once: a reload of the page does not say it again.
	if (came) history.replaceState(history.state, '', location.pathname);

	const problem = $derived.by(() => {
		if (session.state === 'unreachable') return m.login.unreachable;
		switch (failure?.kind) {
			case 'wrong':
				return m.login.wrong;
			case 'tooMany':
				return m.login.tooMany(failure.minutes);
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
			failure = await session.login(name, password);
			if (failure) password = '';
		} finally {
			busy = false;
		}
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
			{m.login.title}
		</h1>

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
				<button
					type="submit"
					class="w-full cursor-pointer rounded bg-accent px-3 py-1.5 text-sm font-semibold text-on-accent hover:brightness-110 disabled:cursor-default disabled:opacity-50"
					disabled={busy || !name.trim() || !password}
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

			<p class="mt-4 text-xs text-soft">{m.login.note}</p>
		{/if}
	</div>
</main>
