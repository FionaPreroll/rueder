<script lang="ts">
	// The reader: the bar on top, the sidebar with the tree, and the list or the start
	// page beside it. The keyboard shortcuts are handled here, for the whole page.
	// Where the feeds and articles come from is the business of the `reader` it is
	// given.
	import { onMount } from 'svelte';
	import { m } from '#lib/i18n/index.ts';
	import { formatNumber } from '#lib/i18n/intl.ts';
	import type { Article, ArticleVersion, Stream } from '#lib/model.ts';
	import type { Reader } from '#lib/reader.ts';
	import { keyMap, type Action } from '#lib/keys.ts';
	import { settings } from '#lib/settings.svelte.ts';
	import ArticleList from './ArticleList.svelte';
	import HomeOverview from './HomeOverview.svelte';
	import Icon from './Icon.svelte';
	import SettingsDialog from './SettingsDialog.svelte';
	import StatusDialog from './StatusDialog.svelte';
	import Sidebar from './Sidebar.svelte';
	import SubscribeDialog from './SubscribeDialog.svelte';
	import Toolbar from './Toolbar.svelte';
	import TopBar from './TopBar.svelte';
	import VersionCompare from './VersionCompare.svelte';

	/** Where the feeds and articles come from: the server, or made-up data. */
	let { reader }: { reader: Reader } = $props();

	/** The time the page counts from; it ticks so "5 minutes ago" stays true. */
	let now = $state(Date.now());

	let sidebarOpen = $state(true);
	let dialog: 'settings' | 'subscribe' | 'status' | null = $state(null);
	let settingsTab: 'look' | 'reading' | 'rules' | 'keys' = $state('look');
	/** The article whose versions are compared, by id: it is replaced when it changes. */
	let comparing: { id: string; versions: ArticleVersion[] } | null = $state(null);

	const compared = $derived.by(() => {
		const id = comparing?.id;
		return id ? (reader.visible.find((article) => article.id === id) ?? null) : null;
	});

	/** The earlier versions are fetched when somebody asks for them. */
	async function compare(article: Article) {
		const versions = await reader.versions(article);
		if (versions.length > 0) comparing = { id: article.id, versions };
	}

	const title = $derived.by(() => {
		const stream = reader.stream;
		switch (stream.kind) {
			case 'home':
				return m.nav.home;
			case 'all':
				return m.nav.all;
			case 'starred':
				return m.nav.starred;
			case 'hidden':
				return m.nav.hidden;
			default:
				return reader.title(stream) ?? '';
		}
	});

	const narrow = () => window.matchMedia?.('(max-width: 767px)').matches ?? false;

	// Whatever changes in the settings is kept with the account, where there is one.
	$effect(() => reader.saveSettings({ ...settings }));

	onMount(() => {
		// On a phone the sidebar lies over the list, so it starts closed.
		if (narrow()) sidebarOpen = false;
		const stopLive = reader.start();
		const clock = setInterval(() => (now = Date.now()), 30_000);
		return () => {
			stopLive();
			clearInterval(clock);
		};
	});

	function openSettings(tab: typeof settingsTab) {
		settingsTab = tab;
		dialog = 'settings';
	}

	function go(stream: Stream) {
		reader.show(stream);
	}

	function openOriginal(article: Article) {
		// Not every article says where it is from.
		if (!article.link) return;
		reader.setRead(article, true);
		window.open(article.link, '_blank', 'noopener,noreferrer');
	}

	/** The first key of a two-key shortcut ("g h"), until the second one or a moment passes. */
	let lead: string | null = null;
	let leadTimer: ReturnType<typeof setTimeout> | undefined;

	const typing = (target: EventTarget | null) =>
		target instanceof HTMLElement &&
		(target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName));

	const pressable = (target: EventTarget | null) =>
		target instanceof HTMLElement && target.closest('button, a, summary, [role="menuitem"]');

	/** Does what a shortcut says; false for a key that is none. */
	function act(action: Action | undefined, article: Article | null, inList: boolean) {
		switch (action) {
			case 'next':
			case 'previous':
			case 'nextCursor':
			case 'previousCursor': {
				// From the start page the first step goes into all items.
				if (!inList) go({ kind: 'all' });
				const forward = action === 'next' || action === 'nextCursor';
				reader.step(forward ? 1 : -1, action === 'next' || action === 'previous');
				return true;
			}
			case 'toggle':
				if (article) reader.toggleOpen(article);
				return true;
			case 'star':
				if (article) reader.toggleStar(article);
				return true;
			case 'read':
				if (article) reader.setRead(article, !reader.isRead(article));
				return true;
			case 'original':
				if (article) openOriginal(article);
				return true;
			case 'markAll':
				if (inList) reader.markAllRead();
				return true;
			case 'refresh':
				reader.refresh();
				return true;
			case 'unreadOnly':
				reader.unreadOnly = !reader.unreadOnly;
				return true;
			case 'expandedView':
				settings.view = 'expanded';
				return true;
			case 'listView':
				settings.view = 'list';
				return true;
			case 'search':
				document.querySelector<HTMLInputElement>('input[type="search"]')?.focus();
				return true;
			case 'sidebar':
				sidebarOpen = !sidebarOpen;
				return true;
			case 'help':
				openSettings('keys');
				return true;
			default:
				return false;
		}
	}

	function keydown(event: KeyboardEvent) {
		if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey) return;
		if (dialog || comparing || typing(event.target)) return;

		const key = event.key;
		const article = reader.current;
		const inList = reader.stream.kind !== 'home';
		const keys = keyMap(settings.keys);

		if (lead === 'g') {
			lead = null;
			clearTimeout(leadTimer);
			const action = keys.afterG.get(key);
			const target: Stream | null =
				action === 'goHome'
					? { kind: 'home' }
					: action === 'goAll'
						? { kind: 'all' }
						: action === 'goStarred'
							? { kind: 'starred' }
							: null;
			if (target) {
				event.preventDefault();
				go(target);
				return;
			}
		}

		let handled = true;
		switch (key) {
			case 'g':
				lead = 'g';
				clearTimeout(leadTimer);
				leadTimer = setTimeout(() => (lead = null), 1500);
				break;
			case 'Enter':
				// On a button or a link, Enter presses that.
				if (pressable(event.target) || !article) handled = false;
				else reader.toggleOpen(article);
				break;
			case 'Escape':
				if (reader.open) reader.open = null;
				else handled = false;
				break;
			default:
				handled = act(keys.single.get(key), article, inList);
		}
		if (handled) event.preventDefault();
	}
</script>

<svelte:head>
	<title>{reader.totalUnread > 0 ? `(${formatNumber(reader.totalUnread)}) ` : ''}rueder</title>
</svelte:head>

<svelte:window onkeydown={keydown} />

<div class="flex h-dvh flex-col bg-paper text-ink">
	<TopBar
		{reader}
		{sidebarOpen}
		ontogglesidebar={() => (sidebarOpen = !sidebarOpen)}
		onsettings={() => openSettings('look')}
		onhelp={() => openSettings('keys')}
		onstatus={() => (dialog = 'status')}
	/>

	<div class="relative flex min-h-0 flex-1">
		{#if sidebarOpen}
			<!-- On a phone a tap beside the sidebar closes it. -->
			<button
				type="button"
				class="absolute inset-0 z-20 bg-black/30 md:hidden"
				aria-label={m.nav.closeSidebar}
				onclick={() => (sidebarOpen = false)}
			></button>
			<aside
				class="w-72 shrink-0 border-r border-line max-md:absolute max-md:inset-y-0 max-md:left-0 max-md:z-30 max-md:w-[min(20rem,85vw)] max-md:shadow-xl"
			>
				<Sidebar
					{reader}
					onsubscribe={() => (dialog = 'subscribe')}
					onpick={() => narrow() && (sidebarOpen = false)}
				/>
			</aside>
		{/if}

		<main class="flex min-w-0 flex-1 flex-col">
			{#if reader.notice}
				<p
					class="flex items-center gap-2 border-b border-line bg-panel px-3 py-1.5 text-sm"
					role="alert"
				>
					<span class="text-warn"><Icon name="warning" /></span>
					<span class="min-w-0 flex-1">{m.notice[reader.notice]}</span>
					<button
						type="button"
						class="plain size-6"
						aria-label={m.common.close}
						title={m.common.close}
						onclick={() => reader.dismissNotice()}
					>
						<Icon name="close" />
					</button>
				</p>
			{/if}
			{#if reader.stream.kind === 'home'}
				<HomeOverview {reader} {now} />
			{:else}
				<Toolbar {reader} {title} />
				<ArticleList {reader} {now} onversions={compare} />
			{/if}
		</main>
	</div>
</div>

{#if dialog === 'settings'}
	<SettingsDialog {reader} bind:tab={settingsTab} onclose={() => (dialog = null)} />
{:else if dialog === 'subscribe'}
	<SubscribeDialog {reader} onclose={() => (dialog = null)} />
{:else if dialog === 'status'}
	<StatusDialog {reader} onclose={() => (dialog = null)} />
{/if}

{#if compared && comparing}
	<VersionCompare
		article={compared}
		versions={comparing.versions}
		onclose={() => (comparing = null)}
	/>
{/if}
