<script lang="ts">
	// A window over the page: the settings, the keyboard shortcuts, the versions of an
	// article. Escape, the close button and a click beside it close it.
	import { onMount, type Snippet } from 'svelte';
	import { m } from '#lib/i18n/index.ts';
	import Icon from './Icon.svelte';

	let {
		title,
		wide = false,
		onclose,
		children
	}: { title: string; wide?: boolean; onclose: () => void; children: Snippet } = $props();

	let panel: HTMLElement | undefined = $state();

	// Focus goes into the window, and back to where it was when the window closes.
	// Where it was is noted before anything in the window takes the focus.
	const before = document.activeElement as HTMLElement | null;

	onMount(() => {
		if (!panel?.contains(document.activeElement)) panel?.focus();
		return () => before?.focus?.();
	});

	function keydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.stopPropagation();
			onclose();
		}
	}
</script>

<div
	class="fixed inset-0 z-40 flex items-start justify-center overflow-y-auto bg-black/40 p-3 sm:p-8"
	role="presentation"
	onclick={(event) => event.target === event.currentTarget && onclose()}
	onkeydown={keydown}
>
	<div
		bind:this={panel}
		role="dialog"
		aria-modal="true"
		aria-label={title}
		tabindex="-1"
		class={[
			'w-full rounded-lg border border-line bg-paper shadow-xl outline-none',
			wide ? 'max-w-3xl' : 'max-w-lg'
		]}
	>
		<header class="flex items-center gap-3 border-b border-line px-4 py-3">
			<h2 class="text-lg font-semibold">{title}</h2>
			<button
				type="button"
				class="plain ml-auto size-8"
				aria-label={m.common.close}
				title={m.common.close}
				onclick={onclose}
			>
				<Icon name="close" class="size-5" />
			</button>
		</header>
		<div class="p-4">
			{@render children()}
		</div>
	</div>
</div>
