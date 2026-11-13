<script lang="ts">
	// The reader: the bar on top, the sidebar with the tree, and the list or the start
	// page beside it; in the view of three panes the list and, beside it, the open
	// article, where there is room for both. The keyboard shortcuts are handled here, for the whole page.
	// Where the feeds and articles come from is the business of the `reader` it is
	// given.
	import { onMount, tick, untrack } from 'svelte';
	import type { AccountTab } from '#lib/account.ts';
	import { m } from '#lib/i18n/index.ts';
	import { formatNumber } from '#lib/i18n/intl.ts';
	import { sameStream, type Article, type ArticleVersion, type Stream } from '#lib/model.ts';
	import type { Reader } from '#lib/reader.ts';
	import { keyMap, type Action } from '#lib/keys.ts';
	import { listFits, settings, widths } from '#lib/settings.svelte.ts';
	import { closeView, dropSpots, noteView, openView } from '#lib/view.ts';
	import { Journey } from '#lib/history.ts';
	import AccountDialog from './AccountDialog.svelte';
	import ArticleList from './ArticleList.svelte';
	import ArticlePane from './ArticlePane.svelte';
	import HomeOverview from './HomeOverview.svelte';
	import Icon from './Icon.svelte';
	import Resizer from './Resizer.svelte';
	import SettingsDialog from './SettingsDialog.svelte';
	import StatusDialog from './StatusDialog.svelte';
	import TermsDialog from './TermsDialog.svelte';
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
	let dialog: 'settings' | 'account' | 'subscribe' | 'status' | null = $state(null);
	let settingsTab: 'look' | 'reading' | 'rules' | 'keys' = $state('look');
	/** Which of the account's settings is open; it stays as it was left. */
	let accountTab: AccountTab = $state('password');
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
			case 'unread':
				return m.nav.unreadItems;
			case 'starred':
				return m.nav.starred;
			case 'hidden':
				return m.nav.hidden;
			default:
				return reader.title(stream) ?? '';
		}
	});

	const narrow = () => window.matchMedia?.('(max-width: 767px)').matches ?? false;

	/**
	 * How wide the list and the article must be together for three panes, in pixels.
	 * Narrower, as on a phone or beside an open sidebar on a small screen, the view
	 * of three panes is the list view, with the article under its row.
	 */
	const SPLIT_AT = 880;
	let mainWidth = $state(0);
	// Before the page is laid out, as in tests, the width is not known yet.
	const split = $derived(settings.view === 'split' && (mainWidth === 0 || mainWidth >= SPLIT_AT));
	// The list as wide as it was left, unless the window has become too narrow for it.
	const list = $derived(listFits(settings.listWidth, mainWidth));

	// Whatever changes in the settings is kept with the account, where there is one.
	$effect(() => reader.saveSettings({ ...settings }));

	// A page loaded anew starts where it was left: the same list with the same article
	// open. The parts that scroll go back to their places once they are drawn.
	// The page is made anew for another reader, so the first one is the one.
	const kept = untrack(() => {
		const view = openView(reader.account?.name ?? '');
		if (view && view.stream.kind !== 'home') reader.resume({ ...view, wanted: view.list.ids });
		return view;
	});

	$effect(() => {
		const now = {
			stream: reader.stream,
			unreadOnly: reader.unreadOnly,
			newestFirst: reader.newestFirst,
			cursor: reader.cursor,
			open: reader.open
		};
		// Until the resumed list is there, what was left is still what is kept.
		if (reader.resuming) return;
		if (kept && !sameStream(now.stream, kept.stream)) dropSpots();
		noteView(now);
		// What the reader did, not what back or forward brought, makes an entry.
		if (!returning) journey.note({ stream: now.stream, open: now.open });
	});

	// The browser's back and forward, and the back button of a phone, go through
	// the lists and articles the reader went through (`#lib/history.ts`).
	const journey = new Journey();
	let returning = false;
	/** The article of a list that back or forward went to, opened once the list has it. */
	let waitingOpen = $state<string | null>(null);

	function popped(event: PopStateEvent) {
		const place = journey.returned(event.state);
		if (!place) return;
		returning = true;
		if (!sameStream(place.stream, reader.stream)) {
			reader.show(place.stream);
			waitingOpen = place.open;
		} else if (place.open === null) {
			waitingOpen = null;
			reader.open = null;
		} else if (place.open !== reader.open) {
			waitingOpen = place.open;
		}
		void tick().then(() => (returning = false));
	}

	$effect(() => {
		const id = waitingOpen;
		if (!id) return;
		const article = reader.visible.find((listed) => listed.id === id);
		if (!article) return;
		waitingOpen = null;
		untrack(() => reader.select(article));
	});

	onMount(() => {
		// On a phone the sidebar lies over the list, so it starts closed.
		if (narrow()) sidebarOpen = false;
		const stopLive = reader.start();
		const clock = setInterval(() => (now = Date.now()), 30_000);
		return () => {
			stopLive();
			clearInterval(clock);
			closeView();
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
			case 'splitView':
				settings.view = 'split';
				return true;
			case 'magazineView':
				settings.view = 'magazine';
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
		if (dialog || comparing || reader.account?.termsToAccept || typing(event.target)) return;

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
						: action === 'goUnread'
							? { kind: 'unread' }
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

<svelte:window onkeydown={keydown} onpopstate={popped} />

<div class="flex h-dvh flex-col bg-paper text-ink">
	<TopBar
		{reader}
		{sidebarOpen}
		ontogglesidebar={() => (sidebarOpen = !sidebarOpen)}
		onsettings={() => openSettings('look')}
		onaccount={() => (dialog = 'account')}
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
				class="w-[var(--sidebar-width)] shrink-0 border-r border-line max-md:absolute max-md:inset-y-0 max-md:left-0 max-md:z-30 max-md:w-[min(20rem,85vw)] max-md:shadow-xl"
				style:--sidebar-width="{settings.sidebarWidth}px"
			>
				<Sidebar
					{reader}
					onsubscribe={() => (dialog = 'subscribe')}
					onpick={() => narrow() && (sidebarOpen = false)}
				/>
			</aside>
			<Resizer
				width={settings.sidebarWidth}
				min={widths.sidebar.min}
				max={widths.sidebar.max}
				fallback={widths.sidebar.default}
				label={m.nav.resizeSidebar}
				onwidth={(width) => (settings.sidebarWidth = width)}
			/>
		{/if}

		<main class="flex min-w-0 flex-1 flex-col" bind:clientWidth={mainWidth}>
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
				{#if split}
					<div class="flex min-h-0 flex-1">
						<!-- 60% is LIST_SHARE; it holds before the page is laid out and the
							width beside the sidebar is known. -->
						<div
							class="flex w-[var(--list-width)] max-w-[60%] shrink-0 flex-col border-r border-line"
							style:--list-width="{list.width}px"
							data-pane="list"
						>
							<ArticleList {reader} {now} onversions={compare} pane />
						</div>
						<Resizer
							width={list.width}
							min={widths.list.min}
							max={list.max}
							fallback={widths.list.default}
							label={m.list.resizeList}
							onwidth={(width) => (settings.listWidth = width)}
						/>
						<ArticlePane {reader} {now} onversions={compare} />
					</div>
				{:else}
					<ArticleList {reader} {now} onversions={compare} />
				{/if}
			{/if}
		</main>
	</div>
</div>

<!-- In the magazine an article opens over the tiles; j, k and Escape work as in a list. -->
{#if settings.view === 'magazine' && reader.stream.kind !== 'home' && reader.open}
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-40 flex justify-center bg-black/40 md:p-6"
		onclick={(event) => event.target === event.currentTarget && (reader.open = null)}
	>
		<div
			role="dialog"
			aria-modal="true"
			aria-label={reader.current?.title ?? m.list.pick}
			class="flex max-h-full w-full max-w-3xl flex-col overflow-hidden bg-paper shadow-xl md:rounded-md"
		>
			<div class="flex items-center justify-end gap-1 border-b border-line px-2 py-1">
				<button
					type="button"
					class="plain size-8"
					aria-label={m.toolbar.previous}
					title={m.toolbar.previous}
					onclick={() => reader.step(-1)}
				>
					<Icon name="up" />
				</button>
				<button
					type="button"
					class="plain size-8"
					aria-label={m.toolbar.next}
					title={m.toolbar.next}
					onclick={() => reader.step(1)}
				>
					<Icon name="down" />
				</button>
				<button
					type="button"
					class="plain size-8"
					aria-label={m.list.closeArticle}
					title={m.list.closeArticle}
					onclick={() => (reader.open = null)}
				>
					<Icon name="close" />
				</button>
			</div>
			<ArticlePane {reader} {now} onversions={compare} />
		</div>
	</div>
{/if}

{#if reader.account?.termsToAccept}
	<TermsDialog {reader} />
{/if}

{#if dialog === 'settings'}
	<SettingsDialog {reader} bind:tab={settingsTab} onclose={() => (dialog = null)} />
{:else if dialog === 'account'}
	<AccountDialog {reader} bind:tab={accountTab} onclose={() => (dialog = null)} />
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
