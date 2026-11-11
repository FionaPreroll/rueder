<script lang="ts" module>
	import type { DropPosition } from '#lib/tree.ts';

	/**
	 * Where a dragged row lands: before, after or into a folder, or, dropped on the
	 * middle of another feed, together with it in a new folder.
	 */
	export type Landing = DropPosition | 'group';

	/** What is being dragged in the sidebar, and where it would land if dropped now. */
	export interface DragState {
		id: string | null;
		over: string | null;
		position: Landing | null;
	}
</script>

<script lang="ts">
	// A folder or a feed in the sidebar, with the folders and feeds inside it. Rows are
	// sorted by dragging them (before, after or into a folder), or with Alt and the
	// arrow keys. A feed dropped on the middle of another makes a new folder of both.
	// The drag state belongs to the sidebar and is bound through every row.
	import { tick } from 'svelte';
	import type { Reader } from '#lib/reader.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { TreeNode } from '#lib/model.ts';
	import Counter from './Counter.svelte';
	import Dialog from './Dialog.svelte';
	import FeedIcon from './FeedIcon.svelte';
	import Icon from './Icon.svelte';
	import Menu from './Menu.svelte';
	import TreeItem from './TreeItem.svelte';

	let {
		reader,
		node,
		depth,
		drag = $bindable(),
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
	let unsubscribing = $state(false);

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

	/**
	 * Upper part of a row: before it. Lower part: after it. The middle of a folder:
	 * into it. The middle of a feed, for a feed: both into a new folder.
	 */
	function positionAt(event: DragEvent): Landing {
		const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
		const at = (event.clientY - box.top) / box.height;
		if (node.kind === 'feed') {
			if (drag.id && reader.canGroup(drag.id, node.id) && at >= 0.3 && at <= 0.7) return 'group';
			return at < 0.5 ? 'before' : 'after';
		}
		if (at < 0.3) return 'before';
		// Right under an open folder is its first row, so "after" would land inside anyway.
		if (at > 0.7 && (closed || node.children.length === 0)) return 'after';
		return 'inside';
	}

	function dragover(event: DragEvent) {
		if (!drag.id) return;
		const position = positionAt(event);
		const allowed =
			position === 'group'
				? reader.canGroup(drag.id, node.id)
				: reader.canMove(drag.id, node.id, position);
		if (allowed) {
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
		const { id, position } = drag;
		if (id && drag.over === node.id && position) {
			if (position === 'group') reader.group(id, node.id, m.nav.groupedFolder);
			else reader.move(id, node.id, position);
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

	/** A folder keeps its name if the new one is empty; a feed gets its own name back. */
	function rename(event: SubmitEvent) {
		event.preventDefault();
		const title = newTitle.trim();
		if (node.kind === 'feed') reader.renameFeed(node.id, title);
		else if (title) reader.renameFolder(node.id, title);
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
			(landing === 'inside' || landing === 'group') && 'ring-2 ring-accent ring-inset',
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
					aria-label={node.kind === 'folder' ? m.nav.folderName : m.nav.feedName}
					placeholder={node.kind === 'feed' ? m.nav.feedNameHint : undefined}
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
				{#if feed?.waiting}
					<span class="text-soft" title={m.nav.waiting}>
						<Icon name="clock" class="size-3.5" />
						<span class="sr-only">{m.nav.waiting}</span>
					</span>
				{/if}
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
		{:else if node.kind === 'feed' && feed && !renaming}
			<div class="opacity-0 group-focus-within:opacity-100 group-hover:opacity-100">
				<Menu class="plain size-6" align="right" label={m.nav.feedActions(feed.title)}>
					{#snippet button()}
						<Icon name="more" />
					{/snippet}
					<button
						type="button"
						role="menuitem"
						class="menu-item"
						onclick={() => {
							newTitle = feed.title;
							renaming = true;
						}}
					>
						{m.nav.renameFeed}
					</button>
					<button
						type="button"
						role="menuitem"
						class="menu-item"
						onclick={() => (unsubscribing = true)}
					>
						{m.nav.unsubscribe}
					</button>
				</Menu>
			</div>
		{/if}
	</div>

	{#if unsubscribing && feed}
		<Dialog title={m.nav.unsubscribeTitle} onclose={() => (unsubscribing = false)}>
			<p class="text-sm">{m.nav.unsubscribeText(feed.title)}</p>
			<div class="mt-4 flex gap-2">
				<button
					type="button"
					class="cursor-pointer rounded bg-accent px-3 py-1 text-sm font-semibold text-on-accent hover:brightness-110"
					onclick={() => {
						unsubscribing = false;
						reader.unsubscribe(node.id);
					}}
				>
					{m.nav.unsubscribe}
				</button>
				<button type="button" class="tool" onclick={() => (unsubscribing = false)}>
					{m.common.cancel}
				</button>
			</div>
		</Dialog>
	{/if}

	{#if node.kind === 'folder' && !closed && node.children.length > 0}
		<ul>
			{#each node.children as child (child.id)}
				<TreeItem {reader} node={child} depth={depth + 1} bind:drag {onpick} />
			{/each}
		</ul>
	{/if}
</li>
