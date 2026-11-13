<script lang="ts">
	// The sign in the top bar that says when what is shown is not live: the
	// connection for live updates is being opened again, or the server does not
	// answer. While all is live, it shows nothing. A click tries again at once.
	import { formatWhen } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { Reader } from '#lib/reader.ts';
	import Icon from './Icon.svelte';

	let { reader }: { reader: Reader } = $props();

	const connection = $derived(reader.connection);
	const text = $derived.by(() => {
		if (connection.state === 'reconnecting') return m.connection.reconnecting;
		if (connection.state !== 'unreachable') return null;
		const since = formatWhen(connection.since, Date.now());
		return connection.offline ? m.connection.offline(since) : m.connection.unreachable(since);
	});
</script>

<!-- Live, so that a screen reader says it when it comes and goes. -->
<div class="flex shrink-0 items-center" aria-live="polite">
	{#if text}
		<button
			type="button"
			class="plain h-8 gap-1.5 px-2 text-sm text-warn"
			aria-label={`${text}. ${m.connection.retry}`}
			title={m.connection.retry}
			onclick={() => reader.reconnect()}
		>
			{#if connection.state === 'reconnecting'}
				<Icon name="refresh" class="size-4 motion-safe:animate-spin" />
			{:else}
				<Icon name="warning" />
			{/if}
			<span class="hidden lg:inline">{text}</span>
			{#if connection.state === 'unreachable'}
				<span class="hidden underline lg:inline">{m.connection.retry}</span>
			{/if}
		</button>
	{/if}
</div>
