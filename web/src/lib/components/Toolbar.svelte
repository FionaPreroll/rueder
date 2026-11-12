<script lang="ts">
	// The row of buttons over the list, in Google Reader's order: refresh, what to
	// show, mark all as read, the views, previous and next.
	import type { Reader } from '#lib/reader.ts';
	import { m } from '#lib/i18n/index.ts';
	import { settings } from '#lib/settings.svelte.ts';
	import Icon from './Icon.svelte';
	import Menu from './Menu.svelte';

	let { reader, title }: { reader: Reader; title: string } = $props();

	const DAY = 86_400_000;
</script>

{#snippet option(label: string, on: boolean, pick: () => void)}
	<button type="button" role="menuitemradio" aria-checked={on} class="menu-item" onclick={pick}>
		<span class="w-4">
			{#if on}<Icon name="check" />{/if}
		</span>
		{label}
	</button>
{/snippet}

<div class="flex flex-wrap items-center gap-x-2 gap-y-1.5 border-b border-line px-3 py-2">
	<h1 class="mr-auto min-w-0 truncate text-lg font-semibold">{title}</h1>

	<button
		type="button"
		class="tool"
		aria-label={m.toolbar.refresh}
		title={m.toolbar.refresh}
		onclick={() => reader.refresh()}
	>
		<Icon name="refresh" />
	</button>

	<Menu title={m.toolbar.show}>
		{#snippet button()}
			{reader.unreadOnly ? m.toolbar.showUnread : m.toolbar.showAll}
			<Icon name="down" class="size-3.5" />
		{/snippet}
		{@render option(m.toolbar.showAll, !reader.unreadOnly, () => (reader.unreadOnly = false))}
		{@render option(m.toolbar.showUnread, reader.unreadOnly, () => (reader.unreadOnly = true))}
		{#if reader.can.oldestFirst}
			<hr class="my-1 border-line" />
			{@render option(m.toolbar.newestFirst, reader.newestFirst, () => (reader.newestFirst = true))}
			{@render option(
				m.toolbar.oldestFirst,
				!reader.newestFirst,
				() => (reader.newestFirst = false)
			)}
		{/if}
	</Menu>

	<div class="flex">
		<button
			type="button"
			class={['tool', reader.can.markOlder && 'rounded-r-none']}
			title={m.toolbar.markAllRead}
			onclick={() => reader.markAllRead()}
		>
			<Icon name="check" />
			<span class="hidden lg:inline">{m.toolbar.markAllRead}</span>
		</button>
		{#if reader.can.markOlder}
			<Menu
				class="tool rounded-l-none border-l-0 px-1.5"
				align="right"
				label={m.toolbar.markOptions}
			>
				{#snippet button()}
					<Icon name="down" class="size-3.5" />
				{/snippet}
				<button
					type="button"
					role="menuitem"
					class="menu-item"
					onclick={() => reader.markAllRead()}
				>
					{m.toolbar.markEverything}
				</button>
				<button
					type="button"
					role="menuitem"
					class="menu-item"
					onclick={() => reader.markAllRead(DAY)}
				>
					{m.toolbar.markOlderDay}
				</button>
				<button
					type="button"
					role="menuitem"
					class="menu-item"
					onclick={() => reader.markAllRead(7 * DAY)}
				>
					{m.toolbar.markOlderWeek}
				</button>
			</Menu>
		{/if}
	</div>

	<div class="flex" role="group" aria-label={m.settings.view}>
		<button
			type="button"
			class="tool rounded-r-none"
			aria-pressed={settings.view === 'expanded'}
			aria-label={m.toolbar.expandedView}
			title={m.toolbar.expandedView}
			onclick={() => (settings.view = 'expanded')}
		>
			<Icon name="expanded" />
		</button>
		<button
			type="button"
			class="tool rounded-none border-l-0"
			aria-pressed={settings.view === 'list'}
			aria-label={m.toolbar.listView}
			title={m.toolbar.listView}
			onclick={() => (settings.view = 'list')}
		>
			<Icon name="list" />
		</button>
		<button
			type="button"
			class="tool rounded-l-none border-l-0"
			aria-pressed={settings.view === 'split'}
			aria-label={m.toolbar.splitView}
			title={m.toolbar.splitView}
			onclick={() => (settings.view = 'split')}
		>
			<Icon name="split" />
		</button>
	</div>

	<div class="flex">
		<button
			type="button"
			class="tool rounded-r-none"
			aria-label={m.toolbar.previous}
			title={m.toolbar.previous}
			onclick={() => reader.step(-1)}
		>
			<Icon name="up" />
		</button>
		<button
			type="button"
			class="tool rounded-l-none border-l-0"
			aria-label={m.toolbar.next}
			title={m.toolbar.next}
			onclick={() => reader.step(1)}
		>
			<Icon name="down" />
		</button>
	</div>
</div>
