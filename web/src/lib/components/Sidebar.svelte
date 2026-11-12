<script lang="ts">
	// The left column, as in Google Reader: Subscribe on top, then the fixed lists and
	// the tree of folders and feeds with their unread counts.
	import { Hold } from '#lib/hold.ts';
	import type { Reader } from '#lib/reader.ts';
	import { m } from '#lib/i18n/index.ts';
	import { formatNumber } from '#lib/i18n/intl.ts';
	import { sameStream, type Stream } from '#lib/model.ts';
	import Counter from './Counter.svelte';
	import Icon, { type IconName } from './Icon.svelte';
	import LabelList from './LabelList.svelte';
	import TreeItem, { type DragState } from './TreeItem.svelte';
	import { currentView, noteView } from '#lib/view.ts';

	let {
		reader,
		onsubscribe,
		onpick
	}: {
		reader: Reader;
		onsubscribe: () => void;
		/** Something was picked; on a phone the sidebar closes then. */
		onpick?: () => void;
	} = $props();

	let drag: DragState = $state({ id: null, over: null, position: null });

	let naming = $state(false);
	let folderName = $state('');

	function pick(stream: Stream) {
		reader.show(stream);
		onpick?.();
	}

	function createFolder(event: SubmitEvent) {
		event.preventDefault();
		if (folderName.trim()) reader.addFolder(folderName.trim());
		naming = false;
		folderName = '';
	}

	const focus = (field: HTMLInputElement) => field.focus();

	// The tree is scrolled as far as it was, after a reload and when the sidebar opens
	// again on a phone, once it is there.
	let scroller: HTMLElement | undefined = $state();
	let hold: Hold | null = null;
	let placed = false;
	$effect(() => {
		if (placed || !scroller || reader.tree.length === 0) return;
		placed = true;
		const top = currentView().sidebar;
		if (top > 0) hold = new Hold(scroller, () => ({ top, ready: true }));
	});
	$effect(() => () => hold?.stop());

	function scrolled() {
		if (scroller && !hold?.active) noteView({ sidebar: scroller.scrollTop });
	}
</script>

{#snippet entry(stream: Stream, icon: IconName, label: string, unread: number, quiet = 0)}
	{@const chosen = sameStream(reader.stream, stream)}
	<button
		type="button"
		class={[
			'flex w-full cursor-pointer items-center gap-2 px-3 py-1 text-left text-sm',
			chosen ? 'bg-chosen' : 'hover:bg-hover'
		]}
		aria-current={chosen ? 'page' : undefined}
		onclick={() => pick(stream)}
	>
		<Icon name={icon} class="size-4 text-soft" />
		<span class={['truncate', unread > 0 && 'font-semibold']}>{label}</span>
		<span class="ml-auto pl-1">
			<Counter count={unread} />
			{#if quiet > 0}
				<span class="text-xs text-soft">{formatNumber(quiet)}</span>
			{/if}
		</span>
	</button>
{/snippet}

<nav class="flex h-full flex-col bg-panel" aria-label={m.nav.subscriptions}>
	{#if reader.can.subscribe}
		<div class="p-3">
			<button
				type="button"
				class="cursor-pointer rounded bg-accent px-4 py-1.5 text-xs font-bold tracking-wider text-on-accent uppercase shadow-sm hover:brightness-110"
				onclick={onsubscribe}
			>
				{m.nav.subscribe}
			</button>
		</div>
	{:else}
		<div class="pt-3"></div>
	{/if}

	<div class="min-h-0 flex-1 overflow-y-auto pb-6" bind:this={scroller} onscroll={scrolled}>
		{@render entry({ kind: 'home' }, 'home', m.nav.home, 0)}
		{@render entry({ kind: 'all' }, 'list', m.nav.all, reader.totalUnread)}
		{@render entry({ kind: 'starred' }, 'star', m.nav.starred, 0)}
		{@render entry({ kind: 'hidden' }, 'hidden', m.nav.hidden, 0, reader.hiddenCount)}

		<LabelList {reader} {onpick} />

		<div class="mt-4 flex items-center px-3 pb-1">
			<h2 class="text-xs font-semibold tracking-wide text-soft uppercase">
				{m.nav.subscriptions}
			</h2>
			<button
				type="button"
				class="plain ml-auto size-6"
				aria-label={m.nav.newFolder}
				title={m.nav.newFolder}
				onclick={() => (naming = true)}
			>
				<Icon name="plus" />
			</button>
		</div>

		{#if naming}
			<form class="px-3 pb-1" onsubmit={createFolder}>
				<input
					class="w-full rounded px-2 py-1 text-sm"
					placeholder={m.nav.folderName}
					aria-label={m.nav.folderName}
					bind:value={folderName}
					use:focus
					onblur={() => (naming = false)}
					onkeydown={(event) => event.key === 'Escape' && (naming = false)}
				/>
			</form>
		{/if}

		<ul>
			{#each reader.tree as node (node.id)}
				<TreeItem {reader} {node} depth={0} bind:drag {onpick} />
			{/each}
		</ul>
	</div>
</nav>
