<script lang="ts">
	// An unread count in the sidebar: "(30)" as in Google Reader, or a badge, as the
	// settings say. Nothing for zero.
	import { m } from '#lib/i18n/index.ts';
	import { formatNumber } from '#lib/i18n/intl.ts';
	import { settings } from '#lib/settings.svelte.ts';

	let { count }: { count: number } = $props();

	const shown = $derived(count > 999 ? '1000+' : formatNumber(count));
</script>

{#if count > 0}
	{#if settings.counters === 'badge'}
		<span
			class="rounded-full bg-accent px-1.5 text-[0.6875rem] leading-5 font-semibold text-on-accent"
			title={m.nav.unread(count)}
		>
			{shown}
		</span>
	{:else}
		<span class="text-sm font-bold" title={m.nav.unread(count)}>({shown})</span>
	{/if}
{/if}
