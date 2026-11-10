<script lang="ts" module>
	import type { DropPosition } from '#lib/tree.ts';

	/** What is being dragged in the sidebar, and where it would land if dropped now. */
	export interface DragState {
		id: string | null;
		over: string | null;
		position: DropPosition | null;
	}
</script>

<script lang="ts">
	// A folder or a feed in the sidebar, with the folders and feeds inside it. Rows are
	// sorted by dragging them (before, after or into a folder), or with Alt and the
	// arrow keys.
	import { tick } from 'svelte';
	import type { Reader } from '#lib/demo/reader.svelte.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { TreeNode } from '#lib/model.ts';
	import Counter from './Counter.svelte';
	import FeedIcon from './FeedIcon.svelte';
	import Icon from './Icon.svelte';
	import Menu from './Menu.svelte';
	import TreeItem from './TreeItem.svelte';

	let {
		reader,
		node,
		depth,
		drag,
		onpick
	}: {
		reader: Reader;
		node: TreeNode;
		depth: number;
		drag: DragState;
		onpick?: () => void;
	} = $props();

	const feed = $derived(node.kind === 'feed' ? reader.feeds[node.id] : null);
	const label = $derived(node.kind === 'folder' ? node.title : (feed?.title ?? ''));
	const closed = $derived(node.kind === 'folder' && reader.isClosed(node.id));
	const chosen = $derived(
		reader.stream.kind === node.kind && 'id' in reader.stream && reader.stream.id === node.id
	);
	const count = $derived(reader.unreadIn(node));
	const landing = $derived(drag.over === node.id ? drag.position : null);

	let renaming = $state(false);
	let newTitle = $state('');

	function pick() {
		reader.show({ kind: node.kind, id: node.id });
		onpick?.();
	}

	function dragstart(event: DragEvent) {
		drag.id = node.id;
		if (event.dataTransfer) {
			event.dataTransfer.effectAllowed = 'move';
			// Firefox starts a drag only when it carries data.
			event.dataTransfer.setData('text/plain', label);
		}
	}

	/** Upper part of a row: before it. Lower part: after it. The middle of a folder: into it. */
	function positionAt(event: DragEvent): DropPosition {
		const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
		const at = (event.clientY - box.top) / box.height;
		if (node.kind === 'feed') return at < 0.5 ? 'before' : 'after';
		if (at < 0.3) return 'before';
		// Right under an open folder is its first row, so "after" would land inside anyway.
		if (at > 0.7 && (closed || node.children.length === 0)) return 'after';
		return 'inside';
	}

	function dragover(event: DragEvent) {
		if (!drag.id) return;
		const position = positionAt(event);
		if (reader.canMove(drag.id, node.id, position)) {
			event.preventDefault();
			drag.over = node.id;
			drag.position = position;
		} else if (drag.over === node.id) {
			drag.over = null;
			drag.position = null;
		}
	}

	function drop(event: DragEvent) {
		event.preventDefault();
		if (drag.id && drag.over === node.id && drag.position) {
			reader.move(drag.id, node.id, drag.position);
		}
		dragend();
	}

	function dragend() {
		drag.id = null;
		drag.over = null;
		drag.position = null;
	}

	const steps = { ArrowUp: 'up', ArrowDown: 'down', ArrowRight: 'in', ArrowLeft: 'out' } as const;

	async function keydown(event: KeyboardEvent) {
		if (!event.altKey || !(event.key in steps)) return;
		event.preventDefault();
		if (reader.nudge(node.id, steps[event.key as keyof typeof steps])) {
			// The row may have been drawn again in another folder.
			await tick();
			document.querySelector<HTMLElement>(`[data-node="${node.id}"]`)?.focus();
		}
	}

	function rename(event: SubmitEvent) {
		event.preventDefault();
		if (newTitle.trim()) reader.renameFolder(node.id, newTitle.trim());
		renaming = false;
	}

	const focus = (field: HTMLInputElement) => {
		field.focus();
		field.select();
	};
</script>

<li>
	<div
		class={[
			'group relative flex items-center gap-0.5 pr-1 select-none',
			chosen ? 'bg-chosen' : 'hover:bg-hover',
			landing === 'inside' && 'ring-2 ring-accent ring-inset',
			drag.id === node.id && 'opacity-40'
		]}
		style:padding-left="{0.4 + depth * 0.85}rem"
		role="presentation"
		draggable={!renaming}
		ondragstart={dragstart}
		ondragover={dragover}
		ondrop={drop}
		ondragend={dragend}
	>
		{#if landing === 'before'}
			<span class="pointer-events-none absolute inset-x-1 -top-px h-0.5 rounded bg-accent"></span>
		{:else if landing === 'after'}
			<span class="pointer-events-none absolute inset-x-1 -bottom-px h-0.5 rounded bg-accent"
			></span>
		{/if}

		{#if node.kind === 'folder'}
			<button
				type="button"
				class="plain size-6"
				aria-expanded={!closed}
				aria-label={closed ? m.nav.openFolder : m.nav.closeFolder}
				onclick={() => reader.setClosed(node.id, !closed)}
			>
				<Icon name={closed ? 'right' : 'down'} class="size-3.5" />
			</button>
		{:else if feed}
			<span class="flex size-6 items-center justify-center"><FeedIcon {feed} /></span>
		{/if}

		{#if renaming}
			<form class="min-w-0 flex-1 py-0.5" onsubmit={rename}>
				<input
					class="w-full rounded px-1.5 py-0.5 text-sm"
					aria-label={m.nav.folderName}
					bind:value={newTitle}
					use:focus
					onblur={() => (renaming = false)}
					onkeydown={(event) => event.key === 'Escape' && (renaming = false)}
				/>
			</form>
		{:else}
			<button
				type="button"
				data-node={node.id}
				class="flex min-w-0 flex-1 cursor-pointer items-center gap-1.5 py-1 text-left text-sm"
				aria-current={chosen ? 'page' : undefined}
				title={m.nav.moveHint}
				onclick={pick}
				onkeydown={keydown}
			>
				{#if node.kind === 'folder'}
					<Icon name="folder" class="size-4 text-soft" />
				{/if}
				<span class={['truncate', count > 0 && 'font-semibold']}>{label}</span>
				{#if feed?.error}
					<span class="text-warn" title={m.nav.fetchFailed(feed.error)}>
						<Icon name="warning" class="size-3.5" />
						<span class="sr-only">{m.nav.fetchFailed(feed.error)}</span>
					</span>
				{/if}
				<span class="ml-auto pl-1"><Counter {count} /></span>
			</button>
		{/if}

		{#if node.kind === 'folder' && !renaming}
			<div class="opacity-0 group-focus-within:opacity-100 group-hover:opacity-100">
				<Menu class="plain size-6" align="right" label={m.nav.folderActions(node.title)}>
					{#snippet button()}
						<Icon name="more" />
					{/snippet}
					<button
						type="button"
						role="menuitem"
						class="menu-item"
						onclick={() => {
							newTitle = node.title;
							renaming = true;
						}}
					>
						{m.nav.renameFolder}
					</button>
					<button
						type="button"
						role="menuitem"
						class="menu-item"
						onclick={() => reader.removeFolder(node.id)}
					>
						{m.nav.removeFolder}
					</button>
				</Menu>
			</div>
		{/if}
	</div>

	{#if node.kind === 'folder' && !closed && node.children.length > 0}
		<ul>
			{#each node.children as child (child.id)}
				<TreeItem {reader} node={child} depth={depth + 1} {drag} {onpick} />
			{/each}
		</ul>
	{/if}
</li>
