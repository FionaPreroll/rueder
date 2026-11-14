<script lang="ts">
	// What the link from a mail did, said once at the top of the page, whether
	// anybody is logged in here or not. The reader closes it.
	import type { Session } from '#lib/api/session.svelte.ts';
	import { m } from '#lib/i18n/index.ts';
	import Icon from './Icon.svelte';

	let { session }: { session: Session } = $props();

	const said = $derived.by(() => {
		const done = session.confirmation;
		switch (done?.kind) {
			case 'confirmed':
				return {
					good: true,
					text: m.email.confirmed(done.email),
					more: done.activated && !session.reader ? m.email.confirmedLogIn : null
				};
			case 'gone':
				return { good: false, text: m.email.gone, more: null };
			case 'taken':
				return { good: false, text: m.email.taken, more: null };
			case 'unreachable':
				return { good: false, text: m.email.confirmUnreachable, more: null };
			default:
				return null;
		}
	});
</script>

{#if said}
	<div class="fixed inset-x-0 top-0 z-[60] flex justify-center p-3">
		<div
			class="flex max-w-md items-start gap-2 rounded-lg border bg-paper px-4 py-3 text-sm shadow-lg"
			class:border-line={said.good}
			class:border-warn={!said.good}
			role={said.good ? 'status' : 'alert'}
		>
			<span class="mt-0.5" class:text-soft={said.good} class:text-warn={!said.good}>
				<Icon name={said.good ? 'info' : 'warning'} />
			</span>
			<span>
				{said.text}
				{#if said.more}<br />{said.more}{/if}
			</span>
			<button
				type="button"
				class="tool -my-1 ml-2 shrink-0"
				onclick={() => (session.confirmation = null)}
			>
				{m.email.close}
			</button>
		</div>
	</div>
{/if}
