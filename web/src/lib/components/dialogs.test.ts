import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ServerReader } from '#lib/api/reader.svelte.ts';
import { FakeServer } from '#lib/api/test/fake-server.ts';
import { DemoReader } from '#lib/demo/reader.svelte.ts';
import { demoStatus } from '#lib/demo/status.ts';
import { useLanguage } from '#lib/i18n/locale.svelte.ts';
import { MAX_OPML } from '#lib/reader.ts';
import { defaults, settings } from '#lib/settings.svelte.ts';
import { ancestorIds } from '#lib/tree.ts';
import TopBar from './TopBar.svelte';
import SettingsDialog from './SettingsDialog.svelte';
import StatusDialog from './StatusDialog.svelte';
import SubscribeDialog from './SubscribeDialog.svelte';
import VersionCompare from './VersionCompare.svelte';

const now = Date.parse('2026-11-10T14:30:00Z');

afterEach(() => {
	Object.assign(settings, defaults);
	useLanguage(null);
});

describe('SettingsDialog', () => {
	it('changes how the reader looks', async () => {
		render(SettingsDialog, { reader: new DemoReader(() => now), onclose: () => {} });
		expect(screen.getByRole('dialog', { name: 'Settings' })).toBeInTheDocument();
		expect(screen.getByRole('tab', { name: 'Appearance' })).toHaveAttribute(
			'aria-selected',
			'true'
		);
		await userEvent.click(screen.getByRole('radio', { name: 'Dark' }));
		await userEvent.click(screen.getByRole('radio', { name: 'Green' }));
		await userEvent.click(screen.getByRole('radio', { name: 'Comfortable' }));
		await userEvent.click(screen.getByRole('radio', { name: 'As badges' }));
		await userEvent.click(screen.getByRole('radio', { name: 'Large' }));
		await userEvent.click(screen.getByRole('radio', { name: 'Serif' }));
		expect(settings).toMatchObject({
			theme: 'dark',
			accent: 'green',
			density: 'comfortable',
			counters: 'badge',
			textSize: 'large',
			readingFont: 'serif'
		});
		await userEvent.selectOptions(screen.getByLabelText('Language'), 'de');
		expect(settings.language).toBe('de');
	});

	it('has a tab for reading, for the rules and for the keys', async () => {
		render(SettingsDialog, { reader: new DemoReader(() => now), onclose: () => {} });
		await userEvent.click(screen.getByRole('tab', { name: 'Reading' }));
		await userEvent.click(screen.getByRole('radio', { name: 'Expanded' }));
		await userEvent.click(screen.getByRole('checkbox', { name: /while scrolling past/ }));
		await userEvent.click(screen.getByRole('checkbox', { name: /first words/ }));
		expect(settings).toMatchObject({ view: 'expanded', markOnScroll: false, teasers: false });

		await userEvent.click(screen.getByRole('tab', { name: 'Filter rules' }));
		expect(screen.getByRole('button', { name: 'Add a rule' })).toBeInTheDocument();
		expect(screen.queryByText(/stay in this browser/)).toBeNull();

		await userEvent.click(screen.getByRole('tab', { name: 'Keyboard' }));
		expect(screen.getByText('Next article, opened')).toBeInTheDocument();
	});

	it('opens on the tab it is given, and closes', async () => {
		const onclose = vi.fn();
		render(SettingsDialog, { reader: new DemoReader(() => now), tab: 'keys', onclose });
		expect(screen.getByRole('tab', { name: 'Keyboard' })).toHaveAttribute('aria-selected', 'true');
		await userEvent.keyboard('{Escape}');
		expect(onclose).toHaveBeenCalledTimes(1);
		await userEvent.click(screen.getByRole('button', { name: 'Close' }));
		expect(onclose).toHaveBeenCalledTimes(2);
		// A click beside the window closes it too, a click in it does not.
		await userEvent.click(screen.getByRole('dialog'));
		expect(onclose).toHaveBeenCalledTimes(2);
		await userEvent.click(screen.getByRole('dialog').parentElement!);
		expect(onclose).toHaveBeenCalledTimes(3);
	});
});

describe('VersionCompare', () => {
	const reader = new DemoReader(() => now);
	const original = reader.articles['feed-ferris'].find((article) => article.versions.length === 0)!;

	it('marks what was added since the earlier version', () => {
		const article = reader.revise(original, 'Nachtrag mit neuen Zahlen.');
		const { container } = render(VersionCompare, {
			article,
			versions: article.versions,
			onclose: () => {}
		});
		expect(screen.getByRole('heading', { name: article.title })).toBeInTheDocument();
		const added = [...container.querySelectorAll('.article-body ins')].map((el) => el.textContent);
		expect(added.join('').trim()).toBe('Nachtrag mit neuen Zahlen.');
		expect(container.querySelector('.article-body del')).toBeNull();
		expect(screen.getByLabelText('Compare the current version with')).toHaveValue('1');
	});

	it('compares with any earlier version, and shows an earlier title', async () => {
		const once = reader.revise(original, 'First addition.');
		const twice = { ...reader.revise(once, 'Second addition.'), title: 'A new title' };
		const { container } = render(VersionCompare, {
			article: twice,
			versions: twice.versions,
			onclose: () => {}
		});
		const added = () =>
			[...container.querySelectorAll('.article-body ins')]
				.map((el) => el.textContent)
				.join('')
				.trim();
		// The latest change first.
		expect(added()).toBe('Second addition.');
		expect(screen.getByText('The title was:')).toHaveTextContent(original.title);
		const options = screen.getAllByRole('option').map((option) => option.textContent?.trim());
		expect(options[0]).toMatch(/^Version 2, fetched /);
		expect(options[1]).toMatch(/^Version 1, fetched /);
		await userEvent.selectOptions(screen.getByRole('combobox'), '1');
		expect(added()).toBe('First addition. Second addition.');
	});

	it('says when the text is the same', () => {
		const versions = [{ rev: 1, fetchedAt: now, title: original.title, html: original.html }];
		render(VersionCompare, { article: original, versions, onclose: () => {} });
		expect(
			screen.getByText('The text of this version is the same as the current one.')
		).toBeInTheDocument();
		expect(screen.queryByText('The title was:')).toBeNull();
	});
});

const address = () => screen.getByLabelText('Address of the feed or the website');

describe('SubscribeDialog', () => {
	it('finds the feed at an address and subscribes to it in a folder', async () => {
		const reader = new DemoReader(() => now);
		const onclose = vi.fn();
		render(SubscribeDialog, { reader, onclose });
		expect(address()).toHaveFocus();
		const find = screen.getByRole('button', { name: 'Find feeds' });
		expect(find).toBeDisabled();
		expect(screen.getByText(/The draft cannot fetch anything/)).toBeInTheDocument();
		await userEvent.type(address(), 'blog.example/feed.xml{Enter}');
		expect(await screen.findByRole('status')).toHaveTextContent('There is one feed:');
		expect(screen.getByText('https://blog.example/feed.xml')).toBeInTheDocument();
		await userEvent.selectOptions(screen.getByLabelText('Into the folder'), 'folder-news');
		await userEvent.click(screen.getByRole('button', { name: 'Subscribe: blog.example' }));
		expect(onclose).toHaveBeenCalledOnce();
		expect(reader.title()).toBe('blog.example');
		expect(ancestorIds(reader.tree, (reader.stream as { id: string }).id)).toEqual(['folder-news']);
		// The draft has no OPML.
		expect(screen.queryByText('Import and export')).toBeNull();
	});

	it('says when there is nothing to subscribe to', async () => {
		const reader = new DemoReader(() => now);
		render(SubscribeDialog, { reader, onclose: () => {} });
		await userEvent.type(address(), 'not an address{Enter}');
		expect(await screen.findByRole('alert')).toHaveTextContent('That did not work.');
		expect(screen.queryByText(/The server says/)).toBeNull();
	});

	it('can be cancelled', async () => {
		const reader = new DemoReader(() => now);
		const feeds = Object.keys(reader.feeds).length;
		const onclose = vi.fn();
		render(SubscribeDialog, { reader, onclose });
		await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
		expect(onclose).toHaveBeenCalledOnce();
		expect(Object.keys(reader.feeds)).toHaveLength(feeds);
	});
});

describe('SubscribeDialog with a server', () => {
	let server: FakeServer;
	const session = { signOut: () => {}, expired: () => {} };

	beforeEach(() => {
		server = new FakeServer();
		server.addFolder('News');
		server.addFeed('Tech');
		vi.stubGlobal('fetch', server.fetch);
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		vi.restoreAllMocks();
	});

	async function open() {
		const reader = new ServerReader({ name: 'fiona', admin: false }, session);
		reader.start();
		const onclose = vi.fn();
		render(SubscribeDialog, { reader, onclose });
		await vi.waitFor(() =>
			expect(reader.tree.length).toBe(server.folders.length + server.feeds.length)
		);
		return { reader, onclose };
	}

	it('lists the feeds a website names, and subscribes to the one picked', async () => {
		server.addSite(
			'https://site.example',
			{ url: 'https://site.example/feed.xml', title: 'Articles' },
			{ url: 'https://site.example/comments.xml', title: '' }
		);
		const { reader, onclose } = await open();
		expect(screen.queryByText(/The draft cannot fetch anything/)).toBeNull();
		await userEvent.type(address(), 'site.example{Enter}');
		expect(await screen.findByRole('status')).toHaveTextContent('There are 2 feeds:');
		expect(screen.getByText('Without a name')).toBeInTheDocument();
		await userEvent.click(
			screen.getByRole('button', { name: 'Subscribe: https://site.example/comments.xml' })
		);
		await vi.waitFor(() => expect(onclose).toHaveBeenCalledOnce());
		expect(reader.stream.kind).toBe('feed');
		expect(server.feeds.at(-1)?.url).toBe('https://site.example/comments.xml');
	});

	it('shows a feed that is subscribed already instead of subscribing again', async () => {
		server.addSite('https://site.example', { url: 'https://site.example/feed.xml', title: 'Old' });
		server.addFeed('Old', null, { url: 'https://site.example/feed.xml' });
		const { reader, onclose } = await open();
		await userEvent.type(address(), 'site.example{Enter}');
		expect(await screen.findByText('Subscribed')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Subscribe: Old' })).toBeNull();
		await userEvent.click(screen.getByRole('button', { name: 'Show: Old' }));
		expect(onclose).toHaveBeenCalledOnce();
		expect(reader.stream).toEqual({ kind: 'feed', id: `feed-${server.feeds.at(-1)?.id}` });
		expect(server.writes.filter((write) => write === 'POST /feeds')).toEqual([]);
	});

	it('says where a site sent the request when there is no feed', async () => {
		server.addSite('https://login.example');
		server.moves.set('https://walled.example/r/news', 'https://login.example');
		await open();
		await userEvent.type(address(), 'walled.example/r/news{Enter}');
		const status = await screen.findByRole('status');
		expect(status).toHaveTextContent('There is no feed at this address.');
		expect(status).toHaveTextContent('The site sent the request on to https://login.example.');
		expect(status).toHaveTextContent(
			'Some sites send whoever is not logged in to their login page.'
		);
	});

	it('says what the server would not do, in its own words too', async () => {
		const { onclose } = await open();
		await userEvent.type(address(), 'nowhere.example{Enter}');
		const alert = await screen.findByRole('alert');
		expect(alert).toHaveTextContent('That did not work.');
		expect(alert).toHaveTextContent('The server says: the address cannot be fetched');

		server.refuse('POST', /^\/feeds\/find$/, 429, 1, 'too many', { 'Retry-After': '30' });
		await userEvent.click(screen.getByRole('button', { name: 'Find feeds' }));
		await vi.waitFor(() =>
			expect(screen.getByRole('alert')).toHaveTextContent(
				'Too many lookups at once. Try again in 30 seconds.'
			)
		);
		server.addSite('https://nowhere.example', { url: 'https://nowhere.example/f', title: 'F' });
		await userEvent.click(screen.getByRole('button', { name: 'Find feeds' }));
		await screen.findByRole('button', { name: 'Subscribe: F' });
		server.refuse('POST', /^\/feeds$/, 503);
		await userEvent.click(screen.getByRole('button', { name: 'Subscribe: F' }));
		await vi.waitFor(() =>
			expect(screen.getByRole('alert')).toHaveTextContent('This server does not fetch feeds.')
		);
		expect(onclose).not.toHaveBeenCalled();
	});

	it('imports an OPML file and says what it did', async () => {
		await open();
		expect(screen.getByRole('heading', { name: 'Import and export' })).toBeInTheDocument();
		const file = new File(
			[
				'<opml version="2.0"><body>' +
					'<outline text="New" xmlUrl="https://new.example/feed.xml"/>' +
					`<outline text="Tech" xmlUrl="https://feed-${server.feeds[0].id}.example/feed.xml"/>` +
					'<outline text="Bad" xmlUrl="mailto:someone"/>' +
					'</body></opml>'
			],
			'subscriptions.opml',
			{ type: 'text/x-opml' }
		);
		await userEvent.upload(screen.getByLabelText('Import an OPML file'), file);
		const status = await screen.findByRole('status');
		expect(status).toHaveTextContent('1 new feed and 0 new folders.');
		expect(status).toHaveTextContent('1 feed was subscribed already.');
		expect(status).toHaveTextContent('1 entry was left out');
		expect(status).toHaveTextContent('Their articles arrive with the next round of fetching.');
	});

	it('says why a file was not taken', async () => {
		await open();
		const input = screen.getByLabelText('Import an OPML file');
		await userEvent.upload(input, new File(['<html></html>'], 'page.opml'));
		expect(await screen.findByRole('alert')).toHaveTextContent(
			'The server says: this is not an OPML file'
		);
		await userEvent.upload(input, new File(['x'.repeat(MAX_OPML + 1)], 'large.opml'));
		await vi.waitFor(() =>
			expect(screen.getByRole('alert')).toHaveTextContent('The file is larger than 1 MB.')
		);
	});

	it('exports the subscriptions as a file to download', async () => {
		await open();
		const made = vi.fn((_: Blob) => 'blob:rueder/opml');
		vi.stubGlobal('URL', Object.assign(URL, { createObjectURL: made, revokeObjectURL: vi.fn() }));
		const clicked: HTMLAnchorElement[] = [];
		vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (
			this: HTMLAnchorElement
		) {
			clicked.push(this);
		});
		await userEvent.click(screen.getByRole('button', { name: 'Export as OPML' }));
		await vi.waitFor(() => expect(clicked).toHaveLength(1));
		expect(clicked[0].href).toBe('blob:rueder/opml');
		expect(clicked[0].download).toMatch(/^rueder-\d{4}-\d{2}-\d{2}\.opml$/);
		expect(await made.mock.calls[0][0].text()).toContain('<opml version="2.0">');

		server.down = true;
		await userEvent.click(screen.getByRole('button', { name: 'Export as OPML' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('The server does not answer.');
		expect(clicked).toHaveLength(1);
	});
});

describe('StatusDialog', () => {
	it('says what wants looking at and how the parts of the server fare', async () => {
		render(StatusDialog, { reader: new DemoReader(() => now), onclose: () => {} });
		expect(screen.getByRole('dialog', { name: 'Server status' })).toBeInTheDocument();
		expect(screen.getByText(/no server here/)).toBeInTheDocument();
		expect(
			await screen.findByText(/POST \/api\/feeds\/find is slow: one request in twenty/)
		).toBeInTheDocument();
		expect(screen.getByText('7 of 31 feeds fail.')).toBeInTheDocument();
		expect(screen.getByText('The server logged 1 error in the last hour.')).toBeInTheDocument();

		// Each part has its facts.
		for (const title of ['Server', 'Database', 'Fetching feeds', 'Feed search', 'Image proxy']) {
			expect(screen.getByRole('heading', { name: title })).toBeInTheDocument();
		}
		expect(screen.getByText('31 subscribed, 34 in all')).toBeInTheDocument();
		expect(screen.getByText('4 open, 3 idle, at most 10')).toBeInTheDocument();
		expect(screen.getByText('The site answers 403')).toBeInTheDocument();
		expect(screen.getByText('3 feeds')).toBeInTheDocument();
		expect(screen.getByText('191.9 MB of 1.1 GB')).toBeInTheDocument();

		// The routes, the slowest first, and the largest tables.
		const rows = screen.getAllByRole('row').map((row) => row.textContent ?? '');
		const find = rows.findIndex((row) => row.includes('POST /api/feeds/find'));
		const tree = rows.findIndex((row) => row.includes('GET /api/tree'));
		expect(find).toBeGreaterThan(0);
		expect(tree).toBeGreaterThan(find);
		expect(rows.some((row) => row.includes('article_texts'))).toBe(true);

		// And what the log said.
		expect(screen.getByText(/canceling statement due to statement timeout/)).toBeInTheDocument();
		expect(screen.getByText('rueder::oidc')).toBeInTheDocument();
		expect(screen.getByText(/^2 times, last/)).toBeInTheDocument();
	});

	it('asks again when told to, and says so when the server does not answer', async () => {
		const reader = new DemoReader(() => now);
		const asked = vi.spyOn(reader, 'status');
		render(StatusDialog, { reader, onclose: () => {} });
		await screen.findByText('Worth a look');
		asked.mockResolvedValueOnce(null);
		await userEvent.click(screen.getByRole('button', { name: 'Refresh' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('The server did not answer.');
		expect(asked).toHaveBeenCalledTimes(2);
		// What it knew stays on the page.
		expect(screen.getByText('7 of 31 feeds fail.')).toBeInTheDocument();
		vi.restoreAllMocks();
	});

	it('says that nothing stands out, in German too', async () => {
		useLanguage('de');
		const reader = new DemoReader(() => now);
		vi.spyOn(reader, 'status').mockResolvedValue({ ...demoStatus(now), hints: [] });
		render(StatusDialog, { reader, onclose: () => {} });
		expect(await screen.findByText('Nichts fällt auf.')).toBeInTheDocument();
		expect(screen.getByRole('heading', { name: 'Abruf der Feeds' })).toBeInTheDocument();
		expect(screen.getByText('Die Seite antwortet mit 403')).toBeInTheDocument();
		vi.restoreAllMocks();
	});
});

describe('The status with a server', () => {
	let server: FakeServer;
	const session = { signOut: () => {}, expired: () => {} };

	beforeEach(() => {
		server = new FakeServer();
		vi.stubGlobal('fetch', server.fetch);
	});

	afterEach(() => {
		vi.unstubAllGlobals();
	});

	function bar(reader: ServerReader) {
		render(TopBar, {
			reader,
			sidebarOpen: true,
			ontogglesidebar: () => {},
			onsettings: () => {},
			onhelp: () => {},
			onstatus: () => {}
		});
	}

	it('is there for an admin, from the menu of the account', async () => {
		const reader = new ServerReader({ name: 'fiona', admin: true }, session);
		expect(reader.can.status).toBe(true);
		bar(reader);
		await userEvent.click(screen.getByRole('button', { name: 'Logged in as fiona' }));
		expect(screen.getByRole('menuitem', { name: 'Server status' })).toBeInTheDocument();
		const status = await reader.status();
		expect(status?.counts.subscribed_feeds).toBe(31);
	});

	it('is not there for a reader, and the server would not tell', async () => {
		server.admin = false;
		const reader = new ServerReader({ name: 'fiona', admin: false }, session);
		expect(reader.can.status).toBe(false);
		bar(reader);
		await userEvent.click(screen.getByRole('button', { name: 'Logged in as fiona' }));
		expect(screen.queryByRole('menuitem', { name: 'Server status' })).toBeNull();
		expect(await reader.status()).toBeNull();
	});
});

describe('Privacy in the settings', () => {
	let server: FakeServer;
	const session = { signOut: () => {}, expired: () => {} };

	beforeEach(() => {
		server = new FakeServer();
		vi.stubGlobal('fetch', server.fetch);
	});

	afterEach(() => {
		vi.unstubAllGlobals();
	});

	async function opened() {
		const reader = new ServerReader({ name: 'fiona', admin: false }, session);
		const stop = reader.start();
		await vi.waitFor(() => expect(reader.reportsConsent).not.toBeNull());
		render(SettingsDialog, { reader, onclose: () => {} });
		return { reader, stop };
	}

	it('lets the reader agree to send reports of errors, and take it back', async () => {
		const { stop } = await opened();
		await userEvent.click(screen.getByRole('tab', { name: 'Privacy' }));
		const box = screen.getByRole('checkbox', { name: /Send reports of errors/ });
		expect(box).not.toBeChecked();
		expect(screen.getByText(/Never what you read/)).toBeInTheDocument();
		await userEvent.click(box);
		await vi.waitFor(() => expect(server.sendsReports).toBe(true));
		expect(box).toBeChecked();
		await userEvent.click(box);
		await vi.waitFor(() => expect(server.sendsReports).toBe(false));
		stop();
	});

	it('is not there where the server takes no reports, nor with made-up data', async () => {
		server.reportsAvailable = false;
		const { stop } = await opened();
		expect(screen.queryByRole('tab', { name: 'Privacy' })).toBeNull();
		stop();
		cleanup();
		render(SettingsDialog, { reader: new DemoReader(() => now), onclose: () => {} });
		expect(screen.queryByRole('tab', { name: 'Privacy' })).toBeNull();
	});
});

describe('Errors in browsers on the status page', () => {
	it('says how often and in how many accounts, never which', async () => {
		render(StatusDialog, { reader: new DemoReader(() => now), onclose: () => {} });
		expect(await screen.findByRole('heading', { name: 'Errors in browsers' })).toBeInTheDocument();
		expect(
			screen.getByText('2 accounts send reports. Errors stay 30 days after they last happened.')
		).toBeInTheDocument();
		expect(screen.getByText(/reading 'feedId'/)).toBeInTheDocument();
		expect(screen.getByText(/^3 times in 2 accounts, last/)).toBeInTheDocument();
		expect(screen.getByText('Version 1762899600000 · Chrome 141, Firefox 140')).toBeInTheDocument();
		expect(screen.getByText('Browsers reported 1 error in the last day.')).toBeInTheDocument();
	});
});
