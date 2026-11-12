// The whole reader on one page: the toolbar, the start page and the keyboard.

import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useLanguage } from '#lib/i18n/locale.svelte.ts';
import { DemoReader } from '#lib/demo/reader.svelte.ts';
import { defaults, settings } from '#lib/settings.svelte.ts';
import { VIEW_KEY } from '#lib/view.ts';
import Layout from './+layout.svelte';
// The page with made-up data: the one of the builds that run without a server.
import Page from '../single/+page.svelte';
import LayoutHarness from './test/LayoutHarness.svelte';

beforeEach(() => {
	vi.useFakeTimers({ now: new Date('2026-11-10T14:30:00Z'), toFake: ['Date'] });
});

afterEach(() => {
	vi.useRealTimers();
	vi.unstubAllGlobals();
	Object.assign(settings, defaults);
	useLanguage(null);
	localStorage.clear();
});

const press = (keys: string) => userEvent.keyboard(keys);
const heading = () => screen.getByRole('heading', { level: 1 });
const openArticle = () =>
	document.querySelector<HTMLElement>('[aria-expanded="true"][class*="py-"]');
const allItems = () => screen.getByRole('button', { name: /^All items \(/ });
const unreadInTitle = () => Number(/^\((\d+)\) rueder$/.exec(document.title)?.[1]);

describe('the start page', () => {
	it('shows what is new and counts the unread articles in the title', () => {
		render(Page);
		expect(heading()).toHaveTextContent('New in your subscriptions');
		expect(unreadInTitle()).toBeGreaterThan(0);
		expect(allItems()).toHaveTextContent(`(${unreadInTitle()})`);
		expect(screen.getByRole('heading', { name: 'Feeds with problems' })).toBeInTheDocument();
		expect(screen.getByRole('heading', { name: 'Recently starred' })).toBeInTheDocument();
	});

	it('goes to a feed and to an article', async () => {
		render(Page);
		const main = screen.getByRole('main');
		await userEvent.click(within(main).getByRole('button', { name: 'Tagesbote' }));
		expect(heading()).toHaveTextContent('Tagesbote');
		await userEvent.click(screen.getByRole('button', { name: /rueder/ }));
		expect(heading()).toHaveTextContent('New in your subscriptions');
		const unread = unreadInTitle();
		const first = within(main).getAllByRole('listitem')[0];
		await userEvent.click(within(first).getByRole('button'));
		expect(openArticle()).not.toBeNull();
		expect(unreadInTitle()).toBe(unread - 1);
	});
});

describe('the search', () => {
	it('from the start page goes through all articles', async () => {
		render(Page);
		const main = screen.getByRole('main');
		const first = within(main).getAllByRole('listitem')[0];
		const title = within(first).getAllByRole('button')[0].textContent?.trim() ?? '';
		const word = title.split(/\s+/).find((w) => w.length > 4) ?? title;
		await userEvent.type(screen.getByRole('searchbox'), word);
		expect(heading()).toHaveTextContent('All items');
		expect(screen.getByRole('searchbox')).toHaveValue(word);
		const rows = within(screen.getByRole('main')).getAllByRole('listitem');
		expect(rows.length).toBeGreaterThan(0);
		for (const row of rows) expect(row.textContent?.toLowerCase()).toContain(word.toLowerCase());
	});
});

describe('an article', () => {
	it('shows the picture beside its text and plays its recording only when asked', async () => {
		render(Page);
		const sidebar = screen.getByRole('navigation');
		await userEvent.click(within(sidebar).getByRole('button', { name: /^Lab Notes Weekly/ }));
		expect(heading()).toHaveTextContent('Lab Notes Weekly');
		const main = screen.getByRole('main');
		const first = within(main).getAllByRole('listitem')[0];
		await userEvent.click(within(first).getAllByRole('button')[1]);
		expect(openArticle()).not.toBeNull();
		const title = within(main).getByRole('heading', { level: 2 }).textContent!.trim();
		expect(within(main).getByRole('img', { name: title })).toBeInTheDocument();
		expect(within(main).getByRole('heading', { name: 'Attachments' })).toBeInTheDocument();
		expect(main.querySelector('audio')).toBeNull();
		await userEvent.click(within(main).getByRole('button', { name: /^Play/ }));
		expect(main.querySelector('audio')?.getAttribute('src')).toMatch(/^data:audio\/wav;base64,/);
	});
});

describe('the toolbar', () => {
	it('marks all as read, or only what is older', async () => {
		render(Page);
		await press('ga');
		expect(heading()).toHaveTextContent('All items');
		const unread = unreadInTitle();
		await userEvent.click(screen.getByRole('button', { name: 'More ways to mark as read' }));
		await userEvent.click(screen.getByRole('menuitem', { name: 'Items older than a week' }));
		const left = unreadInTitle();
		expect(left).toBeGreaterThan(0);
		expect(left).toBeLessThan(unread);
		await userEvent.click(screen.getByRole('button', { name: 'More ways to mark as read' }));
		await userEvent.click(screen.getByRole('menuitem', { name: 'Items older than a day' }));
		expect(unreadInTitle()).toBeLessThanOrEqual(left);
		await userEvent.click(screen.getByRole('button', { name: 'Mark all as read' }));
		expect(document.title).toBe('rueder');
		const sidebar = screen.getByRole('navigation');
		expect(within(sidebar).getByRole('button', { name: /^All items$/ })).toBeInTheDocument();
	});

	it('shows unread only, the oldest first, in either view', async () => {
		render(Page);
		await press('ga');
		const rows = () => within(screen.getByRole('main')).getAllByRole('listitem').length;
		expect(rows()).toBe(40);
		const toolbar = screen.getByRole('main');
		await userEvent.click(within(toolbar).getByRole('button', { name: /^All items$/ }));
		await userEvent.click(screen.getByRole('menuitemradio', { name: 'Unread only' }));
		expect(rows()).toBe(Math.min(40, unreadInTitle()));
		await userEvent.click(screen.getByRole('button', { name: 'Unread only' }));
		expect(screen.getByRole('menuitemradio', { name: 'Unread only' })).toBeChecked();
		await userEvent.click(screen.getByRole('menuitemradio', { name: 'Oldest first' }));
		const times = [...document.querySelectorAll('main time')].map((time) =>
			time.getAttribute('datetime')
		);
		expect(times).toEqual([...times].sort());

		await userEvent.click(screen.getByRole('button', { name: 'Expanded view' }));
		expect(settings.view).toBe('expanded');
		expect(screen.getAllByRole('article').length).toBeGreaterThan(0);
		await userEvent.click(screen.getByRole('button', { name: 'List view' }));
		expect(screen.queryByRole('article')).toBeNull();
	});

	it('steps with its arrows and refreshes', async () => {
		render(Page);
		await press('ga');
		await userEvent.click(screen.getByRole('button', { name: 'Next item' }));
		await userEvent.click(screen.getByRole('button', { name: 'Next item' }));
		const second = openArticle();
		await userEvent.click(screen.getByRole('button', { name: 'Previous item' }));
		expect(openArticle()).not.toBe(second);
		await userEvent.click(screen.getByRole('button', { name: 'Refresh' }));
		expect(openArticle()).not.toBeNull();
	});
});

describe('the bar on top', () => {
	it('searches the list', async () => {
		render(Page);
		await press('ga/');
		const search = screen.getByRole('searchbox', { name: 'Search' });
		expect(search).toHaveFocus();
		// Keys typed into a field are text, not shortcuts.
		await userEvent.type(search, 'jasu?');
		expect(search).toHaveValue('jasu?');
		expect(screen.getByText('No article matches the search.')).toBeInTheDocument();
		await press('{Escape}');
		expect(search).not.toHaveFocus();
	});

	it('pauses live updates, hides the sidebar, opens the settings and the help', async () => {
		render(Page);
		const live = screen.getByRole('button', { name: 'Live' });
		expect(live).toHaveAttribute('aria-pressed', 'true');
		await userEvent.click(live);
		expect(screen.getByRole('button', { name: 'Paused' })).toHaveAttribute('aria-pressed', 'false');

		// The first is the switch in the bar; the second lies beside the sidebar on a phone.
		await userEvent.click(screen.getAllByRole('button', { name: 'Hide the sidebar' })[0]);
		expect(screen.queryByRole('navigation')).toBeNull();
		await userEvent.click(screen.getByRole('button', { name: 'Show the sidebar' }));
		expect(screen.getByRole('navigation')).toBeInTheDocument();

		await userEvent.click(screen.getByRole('button', { name: 'Settings' }));
		expect(screen.getByRole('tab', { name: 'Appearance' })).toHaveAttribute(
			'aria-selected',
			'true'
		);
		await userEvent.click(screen.getByRole('button', { name: 'Close' }));
		await userEvent.click(screen.getByRole('button', { name: 'Keyboard shortcuts' }));
		expect(screen.getByRole('tab', { name: 'Keyboard' })).toHaveAttribute('aria-selected', 'true');
	});

	it('lets an article arrive and changes the open one, to try the versions', async () => {
		render(Page);
		await press('ga');
		await userEvent.click(screen.getByRole('button', { name: 'Draft' }));
		expect(screen.getByRole('menuitem', { name: 'Change the open article' })).toBeDisabled();
		await userEvent.click(screen.getByRole('menuitem', { name: 'Let an article arrive' }));
		expect(screen.getByRole('button', { name: /^\d new articles?$/ })).toBeInTheDocument();

		await press('j');
		await userEvent.click(screen.getByRole('button', { name: 'Draft' }));
		await userEvent.click(screen.getByRole('menuitem', { name: 'Change the open article' }));
		await userEvent.click(screen.getByRole('button', { name: 'Compare versions' }));
		const dialog = screen.getByRole('dialog', { name: 'Versions of this article' });
		expect(dialog.querySelector('ins.bg-added:not(.px-1)')).toHaveTextContent(
			'Update: this paragraph was added to try out the versions.'
		);
		// While a window is open the shortcuts rest.
		await press('ga');
		expect(screen.getByRole('dialog')).toBeInTheDocument();
		await press('{Escape}');
		expect(screen.queryByRole('dialog')).toBeNull();
	});

	it('subscribes from the sidebar', async () => {
		render(Page);
		await userEvent.click(screen.getByRole('button', { name: 'Subscribe' }));
		const dialog = screen.getByRole('dialog', { name: 'Subscribe to a feed' });
		await userEvent.type(within(dialog).getByRole('textbox'), 'blog.example{Enter}');
		await userEvent.click(
			await within(dialog).findByRole('button', { name: 'Subscribe: blog.example' })
		);
		expect(screen.queryByRole('dialog')).toBeNull();
		expect(heading()).toHaveTextContent('blog.example');
	});
});

describe('the keyboard', () => {
	it('goes to the lists with g and a second key', async () => {
		render(Page);
		await press('ga');
		expect(heading()).toHaveTextContent('All items');
		await press('gs');
		expect(heading()).toHaveTextContent('Starred items');
		await press('gh');
		expect(heading()).toHaveTextContent('New in your subscriptions');
		// Another key after g does what it does alone: here, nothing.
		await press('gx');
		expect(heading()).toHaveTextContent('New in your subscriptions');
	});

	it('forgets the g after a moment', async () => {
		vi.useFakeTimers({ now: new Date('2026-11-10T14:30:00Z') });
		const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
		render(Page);
		await user.keyboard('g');
		vi.advanceTimersByTime(2000);
		await user.keyboard('a');
		expect(heading()).toHaveTextContent('New in your subscriptions');
	});

	it('steps through the articles: j and k open, n and p do not', async () => {
		render(Page);
		// From the start page the first step goes into all items.
		await press('j');
		expect(heading()).toHaveTextContent('All items');
		const first = openArticle();
		expect(first).not.toBeNull();
		await press('j');
		expect(openArticle()).not.toBe(first);
		await press('k');
		expect(openArticle()).toBe(first);
		await press('n');
		expect(openArticle()).toBeNull();
		await press('p');
		expect(openArticle()).toBeNull();
		await press('o');
		expect(openArticle()).toBe(first);
		await press('{Enter}');
		expect(openArticle()).toBeNull();
		await press('{Enter}');
		expect(openArticle()).toBe(first);
		await press('{Escape}');
		expect(openArticle()).toBeNull();
	});

	it("takes the keys the reader chose instead of rueder's", async () => {
		settings.keys = { next: 'l', goAll: 'g x' };
		render(Page);
		await press('j');
		expect(heading()).toHaveTextContent('New in your subscriptions');
		await press('gx');
		expect(heading()).toHaveTextContent('All items');
		await press('l');
		expect(openArticle()).not.toBeNull();
	});

	it('shows the open article in a pane of its own beside the list in three panes', async () => {
		render(Page);
		await press('3');
		expect(settings.view).toBe('split');
		await press('ga');
		const pane = () => document.querySelector<HTMLElement>('section[aria-label]')!;
		expect(pane()).toHaveTextContent('Pick an article to read it here.');
		await press('j');
		const shown = () =>
			document.querySelector('[data-pane-article]')?.getAttribute('data-pane-article');
		const first = shown();
		const title = within(pane()).getByRole('heading', { level: 2 }).textContent;
		// The row stays a row: the text is in the pane, not under it.
		const list = document.querySelector<HTMLElement>('[data-pane="list"]')!;
		expect(within(list).queryByRole('heading', { level: 2 })).toBeNull();
		expect(within(list).getByRole('button', { current: true })).toHaveTextContent(title!);

		// A click on another row shows that one; Escape closes it.
		const rows = within(list).getAllByRole('button', { current: false });
		const other = rows.find((row) => !row.getAttribute('aria-label'))!;
		await userEvent.click(other);
		expect(shown()).not.toBe(first);
		expect(other).toHaveAttribute('aria-current', 'true');
		await press('{Escape}');
		expect(pane()).toHaveTextContent('Pick an article to read it here.');

		// The toolbar switches back to the list.
		await userEvent.click(screen.getByRole('button', { name: 'List view' }));
		expect(document.querySelector('[data-pane="list"]')).toBeNull();
	});

	it('shows a magazine: one article large, the others as tiles, read over them', async () => {
		render(Page);
		await press('4');
		expect(settings.view).toBe('magazine');
		await press('ga');
		const tiles = document.querySelectorAll('li[data-article]');
		const lead = document.querySelector<HTMLElement>('article[data-article]')!;
		expect(tiles.length).toBeGreaterThan(10);
		// The large place has a picture: the first of the newest articles that has one.
		expect(lead.querySelector('img')).not.toBeNull();
		expect(document.querySelectorAll('[data-article]')).toHaveLength(tiles.length + 1);

		// A tile opens the article over the tiles; the arrows and j step on, Escape closes.
		const tile = tiles[0] as HTMLElement;
		const title = tile.querySelector('button span span + span')!.textContent!.trim();
		await userEvent.click(within(tile).getAllByRole('button')[0]);
		const sheet = screen.getByRole('dialog');
		expect(within(sheet).getByRole('heading', { level: 2 })).toHaveTextContent(title);
		await userEvent.click(within(sheet).getByRole('button', { name: 'Next item' }));
		expect(
			within(screen.getByRole('dialog')).getByRole('heading', { level: 2 })
		).not.toHaveTextContent(title);
		await press('{Escape}');
		expect(screen.queryByRole('dialog')).toBeNull();
		await press('j');
		await userEvent.click(
			within(screen.getByRole('dialog')).getByRole('button', { name: 'Close the article' })
		);
		expect(screen.queryByRole('dialog')).toBeNull();
	});

	it('puts labels on the open article and takes them off', async () => {
		render(Page);
		await press('gaj');
		const article = openArticle()!;
		const body = article.parentElement!.parentElement!;
		const menu = () => within(body).getByRole('button', { name: 'Labels' });
		await userEvent.click(menu());
		await userEvent.click(screen.getByRole('menuitemcheckbox', { name: 'Arbeit' }));
		// The row shows it as a dot, the article as a chip.
		expect(within(body).getAllByLabelText('Label: Arbeit')).toHaveLength(2);
		await userEvent.click(menu());
		expect(screen.getByRole('menuitemcheckbox', { name: 'Arbeit' })).toHaveAttribute(
			'aria-checked',
			'true'
		);
		await userEvent.click(screen.getByRole('menuitemcheckbox', { name: 'Arbeit' }));
		expect(within(body).queryAllByLabelText('Label: Arbeit')).toHaveLength(0);

		// A new label is made from the same menu and goes on the article at once.
		await userEvent.click(menu());
		await userEvent.click(screen.getByRole('menuitem', { name: 'New label' }));
		await userEvent.type(within(body).getByLabelText('Name of the label'), 'Urlaub{Enter}');
		expect(within(body).queryByLabelText('Name of the label')).toBeNull();
		expect(within(body).getAllByLabelText('Label: Urlaub')).toHaveLength(2);
		await userEvent.click(menu());
		expect(screen.getByRole('menuitemcheckbox', { name: 'Urlaub' })).toHaveAttribute(
			'aria-checked',
			'true'
		);
	});

	it('opens the first article with k from the start page too', async () => {
		render(Page);
		await press('k');
		expect(heading()).toHaveTextContent('All items');
		expect(openArticle()).not.toBeNull();
	});

	it('stars, marks and opens the original of the article under the cursor', async () => {
		const open = vi.fn();
		vi.stubGlobal('open', open);
		render(Page);
		// Without an article these keys do nothing.
		await press('ga');
		await press('smvo{Enter}');
		expect(open).not.toHaveBeenCalled();

		const unread = unreadInTitle();
		await press('n');
		expect(unreadInTitle()).toBe(unread);
		await press('m');
		expect(unreadInTitle()).toBe(unread - 1);
		await press('m');
		expect(unreadInTitle()).toBe(unread);
		const starred = () => screen.queryAllByRole('button', { name: 'Remove star' }).length;
		const before = starred();
		await press('s');
		expect(starred()).toBe(before + 1);
		await press('s');
		expect(starred()).toBe(before);
		await press('v');
		expect(open).toHaveBeenCalledWith(
			expect.stringMatching(/^https:\/\/[\w.-]+\.example\//),
			'_blank',
			'noopener,noreferrer'
		);
		expect(unreadInTitle()).toBe(unread - 1);
	});

	it('marks all, filters, switches the view and the sidebar', async () => {
		render(Page);
		// On the start page there is no list to mark.
		const unread = unreadInTitle();
		await press('{Shift>}A{/Shift}');
		expect(unreadInTitle()).toBe(unread);

		await press('ga');
		await press('{Shift>}U{/Shift}');
		expect(screen.getByRole('button', { name: 'Unread only' })).toBeInTheDocument();
		await press('1');
		expect(settings.view).toBe('expanded');
		await press('2');
		expect(settings.view).toBe('list');
		await press('3');
		expect(settings.view).toBe('split');
		await press('4');
		expect(settings.view).toBe('magazine');
		await press('2');
		await press('u');
		expect(screen.queryByRole('navigation')).toBeNull();
		await press('u');
		await press('{Shift>}A{/Shift}');
		expect(document.title).toBe('rueder');
		await press('r');
		expect(screen.getByText('Nothing unread here.')).toBeInTheDocument();
	});

	it('opens the overview with ?, and leaves keys with Ctrl to the browser', async () => {
		render(Page);
		await press('{Control>}j{/Control}');
		expect(heading()).toHaveTextContent('New in your subscriptions');
		await press('?');
		expect(screen.getByRole('tab', { name: 'Keyboard' })).toHaveAttribute('aria-selected', 'true');
	});

	it('leaves Enter to the button it is pressed on', async () => {
		render(Page);
		await press('gan');
		screen.getByRole('button', { name: 'Refresh' }).focus();
		await press('{Enter}');
		expect(openArticle()).toBeNull();
	});
});

describe('on a phone', () => {
	it('starts with the sidebar closed and closes it again after a pick', async () => {
		vi.stubGlobal('matchMedia', (query: string) => ({
			matches: query === '(max-width: 767px)',
			addEventListener: () => {},
			removeEventListener: () => {}
		}));
		render(Page);
		expect(screen.queryByRole('navigation')).toBeNull();
		await userEvent.click(screen.getByRole('button', { name: 'Show the sidebar' }));
		await userEvent.click(allItems());
		expect(heading()).toHaveTextContent('All items');
		expect(screen.queryByRole('navigation')).toBeNull();
		// A tap beside the open sidebar closes it too.
		await userEvent.click(screen.getByRole('button', { name: 'Show the sidebar' }));
		await userEvent.click(screen.getAllByRole('button', { name: 'Hide the sidebar' })[1]);
		expect(screen.queryByRole('navigation')).toBeNull();
	});
});

describe('after a reload', () => {
	const tagesbote = new DemoReader().articles['feed-tagesbote'];
	const keep = (view: Record<string, unknown>) =>
		localStorage.setItem(
			VIEW_KEY,
			JSON.stringify({
				form: 1,
				account: '',
				stream: { kind: 'feed', id: 'feed-tagesbote' },
				unreadOnly: false,
				newestFirst: true,
				cursor: null,
				open: null,
				sidebar: 0,
				list: { ids: [], offset: 0, top: 0 },
				pane: null,
				...view
			})
		);
	const scrollerOf = (part: HTMLElement) =>
		part.querySelector<HTMLElement>('.overflow-y-auto') ?? part;

	it('shows the list and the article it was left at', async () => {
		const first = render(Page);
		await userEvent.click(
			within(screen.getByRole('main')).getByRole('button', { name: 'Tagesbote' })
		);
		const row = within(screen.getByRole('main')).getAllByRole('listitem')[2];
		await userEvent.click(within(row).getAllByRole('button')[0]);
		const title = openArticle()?.textContent?.trim();
		await userEvent.click(
			within(screen.getByRole('main')).getByRole('button', { name: /^All items$/ })
		);
		await userEvent.click(screen.getByRole('menuitemradio', { name: 'Unread only' }));
		await userEvent.click(screen.getByRole('button', { name: 'Unread only' }));
		await userEvent.click(screen.getByRole('menuitemradio', { name: 'Oldest first' }));
		// The page goes away, as on a reload.
		window.dispatchEvent(new Event('pagehide'));
		first.unmount();

		render(Page);
		expect(heading()).toHaveTextContent('Tagesbote');
		expect(openArticle()?.textContent?.trim()).toBe(title);
		await userEvent.click(screen.getByRole('button', { name: 'Unread only' }));
		expect(screen.getByRole('menuitemradio', { name: 'Unread only' })).toBeChecked();
		expect(screen.getByRole('menuitemradio', { name: 'Oldest first' })).toBeChecked();
	});

	it('scrolls the sidebar and the list back to where they were', () => {
		const open = tagesbote.at(-3)!;
		keep({ open: open.id, sidebar: 140, list: { ids: [open.id], offset: 25, top: 900 } });
		render(Page);
		expect(scrollerOf(screen.getByRole('navigation')).scrollTop).toBe(140);
		// Before the page is laid out the rows count with the guess of 56 pixels each.
		expect(scrollerOf(screen.getByRole('main')).scrollTop).toBe(2 * 56 + 25);
	});

	it('takes the next article still there, or the bare distance, where the top one is gone', () => {
		keep({ list: { ids: ['gone', tagesbote.at(-2)!.id], offset: 25, top: 900 } });
		const first = render(Page);
		expect(scrollerOf(screen.getByRole('main')).scrollTop).toBe(56);
		first.unmount();

		keep({ list: { ids: ['gone'], offset: 25, top: 900 } });
		render(Page);
		expect(scrollerOf(screen.getByRole('main')).scrollTop).toBe(900);
	});

	it('scrolls the open article in its pane back to where it was', async () => {
		settings.view = 'split';
		const open = tagesbote.at(-1)!;
		keep({ open: open.id, cursor: open.id, pane: { id: open.id, top: 333 } });
		render(Page);
		const pane = await screen.findByRole('region', { name: open.title });
		await vi.waitFor(() => expect(pane.scrollTop).toBe(333));
	});

	it('starts on the start page where what it was left at is gone', () => {
		keep({ stream: { kind: 'feed', id: 'feed-gone' } });
		render(Page);
		expect(heading()).toHaveTextContent('New in your subscriptions');
	});
});

describe('the layout', () => {
	it('puts the settings on the page and follows them', async () => {
		settings.theme = 'dark';
		render(LayoutHarness);
		expect(screen.getByText('Inside')).toBeInTheDocument();
		const root = document.documentElement;
		expect(root.dataset.theme).toBe('dark');
		settings.accent = 'purple';
		settings.language = 'de';
		await vi.waitFor(() => expect(root.dataset.accent).toBe('purple'));
		expect(root.lang).toBe('de');
		expect(JSON.parse(localStorage.getItem('rueder.settings') ?? '{}')).toMatchObject({
			theme: 'dark',
			accent: 'purple'
		});
		expect(Layout).toBeTypeOf('function');
	});

	it('follows the system between light and dark', async () => {
		let dark = false;
		const listeners = new Set<() => void>();
		vi.stubGlobal('matchMedia', () => ({
			get matches() {
				return dark;
			},
			addEventListener: (_: string, listener: () => void) => listeners.add(listener),
			removeEventListener: (_: string, listener: () => void) => listeners.delete(listener)
		}));
		const { unmount } = render(LayoutHarness);
		const root = document.documentElement;
		expect(root.dataset.theme).toBe('light');
		dark = true;
		listeners.forEach((listener) => listener());
		expect(root.dataset.theme).toBe('dark');
		unmount();
		expect(listeners.size).toBe(0);
	});
});
