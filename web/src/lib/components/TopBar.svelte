<script lang="ts">
	// The bar across the top: the sidebar switch, the name, the search, and on the
	// right whether updates are live, the keyboard help and the settings.
	import type { Reader } from '#lib/demo/reader.svelte.ts';
	import { m } from '#lib/i18n/index.ts';
	import Icon from './Icon.svelte';
	import Menu from './Menu.svelte';

	let {
		reader,
		sidebarOpen,
		ontogglesidebar,
		onsettings,
		onhelp
	}: {
		reader: Reader;
		sidebarOpen: boolean;
		ontogglesidebar: () => void;
		onsettings: () => void;
		onhelp: () => void;
	} = $props();

	/** The draft's own switch: change the open article, to try the versions. */
	function revise() {
		const article = reader.current;
		if (article) reader.revise(article, m.demo.revised);
	}
</script>

<header class="flex h-12 shrink-0 items-center gap-1.5 border-b border-line bg-panel px-2 sm:gap-2">
	<button
		type="button"
		class="plain size-9"
		aria-pressed={sidebarOpen}
		aria-label={sidebarOpen ? m.nav.closeSidebar : m.nav.openSidebar}
		title={sidebarOpen ? m.nav.closeSidebar : m.nav.openSidebar}
		onclick={ontogglesidebar}
	>
		<Icon name="list" class="size-5" />
	</button>

	<button
		type="button"
		class="flex cursor-pointer items-center gap-2 text-lg font-semibold tracking-tight"
		onclick={() => reader.show({ kind: 'home' })}
	>
		<svg class="size-6" viewBox="0 0 24 24" aria-hidden="true">
			<rect width="24" height="24" rx="5" fill="var(--color-accent)" />
			<g fill="none" stroke="var(--color-on-accent)" stroke-width="2.4" stroke-linecap="round">
				<path d="M7 11a6 6 0 0 1 6 6" />
				<path d="M7 6a11 11 0 0 1 11 11" />
			</g>
			<circle cx="7.6" cy="16.6" r="1.6" fill="var(--color-on-accent)" />
		</svg>
		<span class="hidden sm:inline">rueder</span>
	</button>

	<Menu
		class="cursor-pointer rounded-full border border-line px-2 py-0.5 text-xs font-medium text-soft hover:bg-hover"
		title={m.demo.note}
	>
		{#snippet button()}
			{m.demo.badge}
		{/snippet}
		<p class="max-w-64 px-3 py-1.5 text-xs text-soft">{m.demo.note}</p>
		<button type="button" role="menuitem" class="menu-item" onclick={() => reader.arrive()}>
			<Icon name="plus" />
			{m.demo.arrive}
		</button>
		<button
			type="button"
			role="menuitem"
			class="menu-item disabled:opacity-50"
			disabled={!reader.current}
			title={reader.current ? undefined : m.demo.reviseHint}
			onclick={revise}
		>
			<Icon name="versions" />
			{m.demo.revise}
		</button>
	</Menu>

	<label class="relative mx-auto w-full max-w-xl">
		<span class="sr-only">{m.nav.search}</span>
		<span class="pointer-events-none absolute inset-y-0 left-2.5 flex items-center text-soft">
			<Icon name="search" />
		</span>
		<input
			type="search"
			class="h-8 w-full rounded border-line py-0 pr-2 pl-8 text-sm"
			placeholder={m.nav.searchPlaceholder}
			bind:value={reader.query}
			onkeydown={(event) => event.key === 'Escape' && event.currentTarget.blur()}
		/>
	</label>

	<button
		type="button"
		class="plain h-8 gap-1.5 px-2 text-sm"
		aria-pressed={reader.live}
		title={reader.live ? m.nav.liveOn : m.nav.liveOff}
		onclick={() => (reader.live = !reader.live)}
	>
		<span class={['size-2 rounded-full', reader.live ? 'animate-pulse bg-live' : 'bg-soft']}></span>
		<span class="hidden md:inline">{reader.live ? m.nav.live : m.nav.paused}</span>
	</button>
	<button
		type="button"
		class="plain hidden size-9 sm:inline-flex"
		aria-label={m.nav.shortcuts}
		title={m.nav.shortcuts}
		onclick={onhelp}
	>
		<Icon name="keyboard" class="size-5" />
	</button>
	<button
		type="button"
		class="plain size-9"
		aria-label={m.nav.settings}
		title={m.nav.settings}
		onclick={onsettings}
	>
		<Icon name="settings" class="size-5" />
	</button>
</header>
