<script lang="ts">
	// Says that a new version of rueder is ready, and reloads into it when the reader
	// chooses. Later hides it until the page is opened again.
	import { m } from '#lib/i18n/index.ts';
	import { update } from '#lib/updates.svelte.ts';

	let later = $state(false);
</script>

{#if update.apply && !later}
	{@const apply = update.apply}
	<div
		class="fixed inset-x-0 bottom-4 z-50 flex justify-center px-3"
		role="status"
		aria-live="polite"
	>
		<div
			class="flex flex-wrap items-center gap-3 rounded-lg border border-line bg-paper px-4 py-2.5 text-sm shadow-lg"
		>
			<span>{m.update.ready}</span>
			<button
				type="button"
				class="cursor-pointer rounded bg-accent px-3 py-1 font-semibold text-on-accent hover:brightness-110"
				onclick={apply}
			>
				{m.update.reload}
			</button>
			<button type="button" class="tool" onclick={() => (later = true)}>{m.update.later}</button>
		</div>
	</div>
{/if}
