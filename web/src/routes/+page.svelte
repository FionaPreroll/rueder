<script lang="ts">
	// The page of rueder with its server: the reader for whoever is logged in, the
	// login for everybody else. The builds without a server have a page of their own
	// (src/single).
	import { onMount } from 'svelte';
	import { Session } from '#lib/api/session.svelte.ts';
	import Login from '#lib/components/Login.svelte';
	import ReaderPage from '#lib/components/ReaderPage.svelte';
	import { m } from '#lib/i18n/index.ts';

	const session = new Session();

	onMount(() => {
		void session.check();
	});
</script>

{#if session.reader}
	<!-- Another account gets a page of its own, with nothing left of the one before. -->
	{#key session.reader}
		<ReaderPage reader={session.reader} />
	{/key}
{:else if session.state === 'checking'}
	<p class="p-10 text-center text-soft" role="status">{m.login.checking}</p>
{:else}
	<Login {session} />
{/if}
