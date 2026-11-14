<script lang="ts">
	// A count in the sidebar: "(30)" as in Google Reader, or a badge, as the
	// settings say. Nothing for zero. An unread count stands out; a quiet one, like
	// what the filters hid, has the same form in the color of the other text.
	import { m } from '#lib/i18n/index.ts';
	import { formatNumber } from '#lib/i18n/intl.ts';
	import { settings } from '#lib/settings.svelte.ts';

	let { count, quiet = false }: { count: number; quiet?: boolean } = $props();

	const shown = $derived(count > 999 ? '1000+' : formatNumber(count));
	const title = $derived(quiet ? m.nav.hiddenCount(count) : m.nav.unread(count));
</script>

{#if count > 0}
	{#if settings.counters === 'badge'}
		<span
			class={[
				'rounded-full px-1.5 text-[0.6875rem] leading-5 font-semibold',
				quiet ? 'bg-line text-ink' : 'bg-accent text-on-accent'
			]}
			{title}
		>
			{shown}
		</span>
	{:else}
		<span class={['text-sm', quiet ? 'text-soft' : 'font-bold']} {title}>({shown})</span>
	{/if}
{/if}
