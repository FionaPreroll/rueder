<script lang="ts">
	// The page of rueder with its server: the reader for whoever is logged in, the
	// login for everybody else. The builds without a server have a page of their own
	// (src/single). Only this one has a service worker, which keeps the app for a
	// quick start and tells of new versions, and so only this one installs as an app.
	import { onMount } from 'svelte';
	import { dev } from '$app/env';
	import UpdateNotice from '#lib/components/UpdateNotice.svelte';
	import { registerServiceWorker } from '#lib/updates.svelte.ts';
	import { watchInstall } from '#lib/install.svelte.ts';
	import { Session } from '#lib/api/session.svelte.ts';
	import Login from '#lib/components/Login.svelte';
	import EmailConfirmation from '#lib/components/EmailConfirmation.svelte';
	import ReaderPage from '#lib/components/ReaderPage.svelte';
	import { m } from '#lib/i18n/index.ts';

	const session = new Session();

	onMount(() => {
		void session.check();
		// The page may have been opened with the link from a mail.
		void session.confirmFromAddress();
		// The dev server has no worker, and one would keep its files from changing.
		if (dev) return;
		const stopping = registerServiceWorker();
		// Only a page with a worker can be installed as an app.
		const unwatch = watchInstall();
		return () => {
			unwatch();
			void stopping.then((stop) => stop());
		};
	});
</script>

{#if session.reader}
	<!-- Another account gets a page of its own, with nothing left of the one before. -->
	{#key session.reader}
		<!-- A part of the page that fails to draw is reported, if the account agreed, and the page offers to start again. -->
		<svelte:boundary onerror={(error) => session.reader?.noteError('component', error)}>
			<ReaderPage reader={session.reader} />
			{#snippet failed()}
				<div class="p-10 text-center" role="alert">
					<p class="font-semibold">{m.crash.title}</p>
					<p class="mt-2 text-sm text-soft">{m.crash.text}</p>
					<button type="button" class="tool mt-4" onclick={() => location.reload()}>
						{m.crash.reload}
					</button>
				</div>
			{/snippet}
		</svelte:boundary>
	{/key}
{:else if session.state === 'checking'}
	<p class="p-10 text-center text-soft" role="status">{m.login.checking}</p>
{:else}
	<Login {session} />
{/if}

<EmailConfirmation {session} />
<UpdateNotice />
