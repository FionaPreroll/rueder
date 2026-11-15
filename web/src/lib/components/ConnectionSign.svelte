<script lang="ts">
	// The sign in the top bar that says when what is shown is not live: the
	// connection for live updates is being opened again, or the server does not
	// answer, and how many changes wait for it. While all is live and sent, it
	// shows nothing. On a wide screen it says so beside its icon; a tap or a
	// click opens what it says, on a phone too, where nothing shows a title,
	// with a way to try again at once.
	import { formatWhen } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { Reader } from '#lib/reader.ts';
	import Icon from './Icon.svelte';
	import Menu from './Menu.svelte';

	let { reader }: { reader: Reader } = $props();

	const connection = $derived(reader.connection);
	const state = $derived.by(() => {
		if (connection.state === 'reconnecting') return m.connection.reconnecting;
		if (connection.state !== 'unreachable') return null;
		const since = formatWhen(connection.since, Date.now());
		return connection.offline ? m.connection.offline(since) : m.connection.unreachable(since);
	});
	const unsent = $derived(
		reader.unsentChanges > 0 ? m.connection.unsent(reader.unsentChanges) : null
	);
	const text = $derived([state, unsent].filter(Boolean).join(' '));
</script>

<!-- Live, so that a screen reader says it when it comes and goes. -->
<div class="flex shrink-0 items-center" aria-live="polite">
	{#if text}
		<Menu class="plain h-8 gap-1.5 px-2 text-sm text-warn" label={text} title={text} align="right">
			{#snippet button()}
				{#if connection.state === 'unreachable'}
					<Icon name="warning" />
				{:else}
					<Icon name="refresh" class="size-4 motion-safe:animate-spin" />
				{/if}
				<span class="hidden lg:inline">{text}</span>
			{/snippet}
			<p class="max-w-72 px-3 py-2 text-sm">{text}</p>
			<button type="button" role="menuitem" class="menu-item" onclick={() => reader.reconnect()}>
				{m.connection.retry}
			</button>
		</Menu>
	{/if}
</div>
