import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ServerReader } from '#lib/api/reader.svelte.ts';
import { fakeDevice, noDevice } from '#lib/api/test/fake-device.ts';
import { FakeServer } from '#lib/api/test/fake-server.ts';
import { DemoReader } from '#lib/demo/reader.svelte.ts';
import { demoStatus } from '#lib/demo/status.ts';
import { useLanguage } from '#lib/i18n/locale.svelte.ts';
import { MAX_OPML, type Reader } from '#lib/reader.ts';
import { defaults, settings } from '#lib/settings.svelte.ts';
import { ancestorIds } from '#lib/tree.ts';
import ReportDialog from './ReportDialog.svelte';
import TopBar from './TopBar.svelte';
import AccountDialog from './AccountDialog.svelte';
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

	it('keeps to the page and says where the account is settled', async () => {
		const session = { signOut: () => {}, expired: () => {} };
		vi.stubGlobal('fetch', new FakeServer().fetch);
		const reader = new ServerReader({ name: 'fiona', admin: true }, session);
		render(SettingsDialog, { reader, onclose: () => {} });
		expect(screen.getAllByRole('tab').map((tab) => tab.textContent?.trim())).toEqual([
			'Appearance',
			'Reading',
			'Filter rules',
			'Keyboard'
		]);
		expect(screen.getByText(/under Account settings, in the menu of your account/)).toBeVisible();
		vi.unstubAllGlobals();
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

	it('says what this build is and links the run that built it', async () => {
		const reader = new DemoReader(() => now);
		render(StatusDialog, { reader, onclose: () => {} });
		expect(await screen.findByRole('heading', { name: 'Build' })).toBeInTheDocument();
		expect(screen.getByText('9f1c2d4e5a6b')).toBeInTheDocument();
		expect(screen.getByText('main')).toBeInTheDocument();
		expect(screen.getByText('v0.1.0')).toBeInTheDocument();
		expect(screen.getByText('in a container')).toBeInTheDocument();
		expect(screen.getByText('x86_64-unknown-linux-gnu')).toBeInTheDocument();
		expect(screen.getByText('release, optimization 3')).toBeInTheDocument();
		expect(screen.getByRole('link', { name: 'a run of the CI' })).toHaveAttribute(
			'href',
			'https://github.com/FionaPreroll/rueder/actions/runs/1234567890'
		);
	});

	it('says when a build came from a working tree of its own, built by hand', async () => {
		const reader = new DemoReader(() => now);
		const built = demoStatus(now).build;
		vi.spyOn(reader, 'status').mockResolvedValue({
			...demoStatus(now),
			build: { ...built, dirty: true, tag: '', ci_run: '', container: false }
		});
		render(StatusDialog, { reader, onclose: () => {} });
		expect(await screen.findByText('9f1c2d4e5a6b, with changes of its own')).toBeInTheDocument();
		expect(screen.getByText('on the machine itself')).toBeInTheDocument();
		expect(screen.getByText('hand')).toBeInTheDocument();
		expect(screen.queryByRole('link', { name: 'a run of the CI' })).toBeNull();
		expect(screen.queryByText('v0.1.0')).toBeNull();
		vi.restoreAllMocks();
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
			onaccount: () => {},
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

	it('leads to the settings of the account, where there are any', async () => {
		const onaccount = vi.fn();
		const reader = new ServerReader({ name: 'fiona', admin: false }, session);
		render(TopBar, {
			reader,
			sidebarOpen: true,
			ontogglesidebar: () => {},
			onsettings: () => {},
			onaccount,
			onhelp: () => {},
			onstatus: () => {}
		});
		await userEvent.click(screen.getByRole('button', { name: 'Logged in as fiona' }));
		await userEvent.click(screen.getByRole('menuitem', { name: 'Account settings' }));
		expect(onaccount).toHaveBeenCalledOnce();
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

describe('Privacy in the account settings', () => {
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
		render(AccountDialog, { reader, onclose: () => {} });
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
		render(AccountDialog, { reader: new DemoReader(() => now), onclose: () => {} });
		expect(screen.queryByRole('tab', { name: 'Privacy' })).toBeNull();
	});
});

describe('Moving the account in the account settings', () => {
	let server: FakeServer;
	const session = { signOut: () => {}, expired: () => {} };

	beforeEach(() => {
		server = new FakeServer();
		vi.stubGlobal('fetch', server.fetch);
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		Object.assign(settings, defaults);
	});

	async function opened() {
		const reader = new ServerReader({ name: 'fiona', admin: false }, session);
		const stop = reader.start();
		await vi.waitFor(() => expect(reader.reportsConsent).not.toBeNull());
		render(AccountDialog, { reader, onclose: () => {} });
		await userEvent.click(screen.getByRole('tab', { name: 'Move' }));
		return { reader, stop };
	}

	it('imports the file of an account and says what came with it', async () => {
		const { stop } = await opened();
		const file = new File(
			[
				JSON.stringify({
					format: 'rueder-account',
					version: 1,
					settings: { theme: 'dark' },
					tree: [{ kind: 'feed', url: 'https://moved.example/feed.xml', title: 'Moved' }]
				})
			],
			'rueder-2026-11-12.json',
			{ type: 'application/json' }
		);
		await userEvent.upload(screen.getByLabelText('Import an account file'), file);
		const status = await screen.findByRole('status');
		expect(status).toHaveTextContent('1 new feed, 0 subscribed already.');
		expect(status).toHaveTextContent('Your settings are those of the file now.');
		expect(status).toHaveTextContent('with the next round of fetching');
		await vi.waitFor(() => expect(settings.theme).toBe('dark'));
		stop();
	});

	it('says why a file was not taken', async () => {
		const { stop } = await opened();
		const input = screen.getByLabelText('Import an account file');
		await userEvent.upload(input, new File(['{}'], 'other.json', { type: 'application/json' }));
		expect(await screen.findByRole('alert')).toHaveTextContent(
			'The server says: this is not a file of a rueder account'
		);
		server.refuse('POST', /^\/import$/, 429, 1, 'too many imports', { 'Retry-After': '3600' });
		await userEvent.upload(input, new File(['{}'], 'other.json', { type: 'application/json' }));
		await vi.waitFor(() =>
			expect(screen.getByRole('alert')).toHaveTextContent(
				'Too many imports. Try again in 60 minutes.'
			)
		);
		stop();
	});

	it('exports the account as a file to download', async () => {
		const { stop } = await opened();
		const made = vi.fn((_: Blob) => 'blob:rueder/account');
		vi.stubGlobal('URL', Object.assign(URL, { createObjectURL: made, revokeObjectURL: vi.fn() }));
		const clicked: HTMLAnchorElement[] = [];
		vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (
			this: HTMLAnchorElement
		) {
			clicked.push(this);
		});
		await userEvent.click(screen.getByRole('button', { name: 'Export the account' }));
		await vi.waitFor(() => expect(clicked).toHaveLength(1));
		expect(clicked[0].download).toMatch(/^rueder-\d{4}-\d{2}-\d{2}\.json$/);
		expect(JSON.parse(await made.mock.calls[0][0].text())).toMatchObject({
			format: 'rueder-account'
		});
		server.down = true;
		await userEvent.click(screen.getByRole('button', { name: 'Export the account' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('The server does not answer.');
		stop();
	});

	it('is not there with made-up data', () => {
		render(AccountDialog, { reader: new DemoReader(() => now), onclose: () => {} });
		expect(screen.queryByRole('tab', { name: 'Move' })).toBeNull();
	});
});

describe('The password in the account settings', () => {
	let server: FakeServer;
	const session = { signOut: () => {}, expired: () => {} };

	beforeEach(() => {
		server = new FakeServer();
		vi.stubGlobal('fetch', server.fetch);
	});

	afterEach(() => vi.unstubAllGlobals());

	function opened(password = true) {
		const reader = new ServerReader({ name: 'fiona', admin: false, password }, session);
		render(AccountDialog, { reader, onclose: () => {} });
		return reader;
	}

	it('is the first tab and takes the one so far with the new one', async () => {
		opened();
		expect(screen.getByRole('tab', { name: 'Password' })).toHaveAttribute('aria-selected', 'true');
		const button = screen.getByRole('button', { name: 'Change the password' });
		expect(button).toBeDisabled();

		await userEvent.type(screen.getByLabelText('Current password'), 'correct horse');
		await userEvent.type(screen.getByLabelText('New password'), 'a longer secret');
		// Not until both of the new ones are the same.
		expect(button).toBeDisabled();
		await userEvent.type(screen.getByLabelText('New password again'), 'a longer secre');
		expect(screen.getByText('The two do not match.')).toBeInTheDocument();
		expect(button).toBeDisabled();
		await userEvent.type(screen.getByLabelText('New password again'), 't');
		expect(button).toBeEnabled();

		await userEvent.click(button);
		expect(await screen.findByRole('status')).toHaveTextContent('The password is changed.');
		expect(server.accounts.get('fiona')).toBe('a longer secret');
		expect(server.requests.at(-1)?.body).toEqual({
			current: 'correct horse',
			new: 'a longer secret'
		});
		// The fields are empty again, so the next browser does not read them.
		expect(screen.getByLabelText('Current password')).toHaveValue('');
	});

	it('says when the one so far is wrong, and when the new one is too short', async () => {
		opened();
		await userEvent.type(screen.getByLabelText('Current password'), 'wrong');
		await userEvent.type(screen.getByLabelText('New password'), 'a longer secret');
		await userEvent.type(screen.getByLabelText('New password again'), 'a longer secret{Enter}');
		expect(await screen.findByRole('alert')).toHaveTextContent(
			'The current password is not right.'
		);
		expect(server.accounts.get('fiona')).toBe('correct horse');

		// Shorter than the server takes: the page does not even send it.
		await userEvent.clear(screen.getByLabelText('New password'));
		await userEvent.type(screen.getByLabelText('New password'), 'short');
		await userEvent.clear(screen.getByLabelText('New password again'));
		await userEvent.type(screen.getByLabelText('New password again'), 'short');
		expect(screen.getByRole('button', { name: 'Change the password' })).toBeDisabled();
	});

	it('sets a first one for an account that logs in through single sign-on', async () => {
		server.accounts.delete('fiona');
		const reader = opened(false);
		expect(screen.getByRole('tab', { name: 'Password' })).toBeInTheDocument();
		expect(screen.queryByLabelText('Current password')).toBeNull();
		expect(screen.getByText(/logs in through single sign-on/)).toBeInTheDocument();
		await userEvent.type(screen.getByLabelText('New password'), 'a longer secret');
		await userEvent.type(screen.getByLabelText('New password again'), 'a longer secret');
		await userEvent.click(screen.getByRole('button', { name: 'Set the password' }));
		expect(await screen.findByRole('status')).toHaveTextContent('The password is set.');
		expect(server.accounts.get('fiona')).toBe('a longer secret');
		// From now on the account has one, so the form asks for it.
		expect(reader.account.password).toBe(true);
		expect(screen.getByLabelText('Current password')).toBeInTheDocument();
	});

	it('is not there with made-up data, which has no account', () => {
		render(AccountDialog, { reader: new DemoReader(() => now), onclose: () => {} });
		expect(screen.queryByRole('tab', { name: 'Password' })).toBeNull();
	});
});

describe('Deleting the account in the account settings', () => {
	let server: FakeServer;

	beforeEach(() => {
		server = new FakeServer();
		vi.stubGlobal('fetch', server.fetch);
	});

	afterEach(() => {
		vi.unstubAllGlobals();
	});

	async function opened(password = true) {
		const removed = vi.fn();
		const session = { signOut: () => {}, expired: () => {}, removed };
		const reader = new ServerReader({ name: 'fiona', admin: false, password }, session);
		render(AccountDialog, { reader, onclose: () => {} });
		await userEvent.click(screen.getByRole('tab', { name: 'Delete account' }));
		return removed;
	}

	it('asks for the password and says what goes', async () => {
		const removed = await opened();
		expect(screen.getByText(/your subscriptions and folders/)).toBeInTheDocument();
		expect(screen.getByText(/Feeds stay for the others/)).toBeInTheDocument();
		const button = screen.getByRole('button', { name: 'Delete my account for good' });
		expect(button).toBeDisabled();
		await userEvent.type(screen.getByLabelText('Your password'), 'wrong{Enter}');
		expect(await screen.findByRole('alert')).toHaveTextContent('Nothing was deleted.');
		expect(server.accounts.has('fiona')).toBe(true);
		expect(removed).not.toHaveBeenCalled();

		await userEvent.type(screen.getByLabelText('Your password'), 'correct horse');
		await userEvent.click(button);
		await vi.waitFor(() => expect(removed).toHaveBeenCalledOnce());
		expect(server.accounts.has('fiona')).toBe(false);
	});

	it('asks for the code where the account has a second factor', async () => {
		server.secondFactor = { code: '123456', recovery: ['ABCD-EFGH'] };
		const removed = await opened();
		await userEvent.type(screen.getByLabelText('Your password'), 'correct horse{Enter}');
		expect(await screen.findByRole('alert')).toHaveTextContent('Your account has a second factor.');
		expect(server.accounts.has('fiona')).toBe(true);
		const code = screen.getByLabelText('Code from your app, or a recovery code');
		await userEvent.type(code, '654321{Enter}');
		expect(await screen.findByRole('alert')).toHaveTextContent('Nothing was deleted.');
		await userEvent.type(screen.getByLabelText('Your password'), 'correct horse');
		await userEvent.type(code, '123456{Enter}');
		await vi.waitFor(() => expect(removed).toHaveBeenCalledOnce());
		const asked = server.requests.findLast((request) => request.method === 'DELETE');
		expect(asked?.body).toEqual({ password: 'correct horse', code: '123456' });
	});

	it('asks for the name where the account has no password', async () => {
		server.accounts.delete('fiona');
		const removed = await opened(false);
		expect(screen.queryByLabelText('Your password')).toBeNull();
		await userEvent.type(screen.getByLabelText(/Type your name, fiona/), 'fiona{Enter}');
		await vi.waitFor(() => expect(removed).toHaveBeenCalledOnce());
		expect(server.requests.at(-1)?.body).toEqual({ name: 'fiona' });
	});

	it('is not there with made-up data', () => {
		render(AccountDialog, { reader: new DemoReader(() => now), onclose: () => {} });
		expect(screen.queryByRole('tab', { name: 'Delete account' })).toBeNull();
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

describe('passkeys in the account settings', () => {
	let server: FakeServer;
	const session = { signOut: () => {}, expired: () => {} };
	const onclose = vi.fn();

	beforeEach(() => {
		server = new FakeServer();
		server.passkeysOn = true;
		vi.stubGlobal('fetch', server.fetch);
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		noDevice();
	});

	function open() {
		onclose.mockClear();
		const reader = new ServerReader({ name: 'fiona', admin: false, passkeys: true }, session);
		render(AccountDialog, { reader, tab: 'security', onclose });
		return reader;
	}

	it('adds one with the password, shows the recovery codes once, and removes it with itself', async () => {
		const device = fakeDevice('a2V5MQ');
		open();
		expect(await screen.findByText('No passkey yet.')).toBeInTheDocument();
		await userEvent.click(screen.getByRole('button', { name: 'Add a passkey' }));
		expect(screen.getByLabelText('Name, such as the device')).toHaveValue('Passkey');
		const name = screen.getByLabelText('Name, such as the device');
		await userEvent.clear(name);
		await userEvent.type(name, 'Phone');
		// The second factor is off: the password is all it takes, and no code.
		const form = name.closest('form')!;
		expect(form.querySelector('[autocomplete="one-time-code"]')).toBeNull();
		const password = form.querySelector('input[type="password"]') as HTMLInputElement;
		await userEvent.type(password, 'wrong');
		await userEvent.click(screen.getByRole('button', { name: 'Create the passkey' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('The password is not right.');
		expect(password).toHaveValue('');
		expect(device.create).not.toHaveBeenCalled();

		await userEvent.type(password, 'correct horse');
		await userEvent.click(screen.getByRole('button', { name: 'Create the passkey' }));
		// The device was asked for a passkey it keeps, with its PIN or biometrics.
		const asked = device.create.mock.calls[0][0] as CredentialCreationOptions;
		expect(asked.publicKey!.authenticatorSelection).toMatchObject({
			residentKey: 'required',
			userVerification: 'required'
		});
		expect(await screen.findByText('Your recovery codes')).toBeInTheDocument();
		expect(screen.getByText('AAAAA-BBBBB')).toBeInTheDocument();
		// Neither a click beside the window nor Escape loses the codes.
		await userEvent.click(screen.getByRole('dialog', { name: 'Account settings' }).parentElement!);
		await userEvent.keyboard('{Escape}');
		expect(onclose).not.toHaveBeenCalled();
		await userEvent.click(screen.getByRole('button', { name: 'I have kept them' }));
		expect(await screen.findByText('Phone')).toBeInTheDocument();
		expect(screen.getByText('not used yet')).toBeInTheDocument();
		// The app is not set up; the passkey keeps the second factor on.
		expect(
			await screen.findByText(/Not set up\. Your passkeys keep the second factor on\./)
		).toBeInTheDocument();
		expect(server.passkeys.map((key) => key.name)).toEqual(['Phone']);

		// Removing it: the password alone is not enough any more, the passkey is.
		await userEvent.click(screen.getByRole('button', { name: 'Remove the passkey Phone' }));
		const removing = screen.getByRole('button', { name: 'Remove the passkey' }).closest('form')!;
		await userEvent.type(
			removing.querySelector('input[type="password"]') as HTMLInputElement,
			'correct horse'
		);
		await userEvent.click(screen.getByRole('button', { name: 'Remove the passkey' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('Your account has a second factor.');
		await userEvent.click(screen.getByRole('button', { name: 'Confirm with a passkey' }));
		expect(await screen.findByText('No passkey yet.')).toBeInTheDocument();
		expect(server.passkeys).toEqual([]);
		expect(await screen.findByText('Off.')).toBeInTheDocument();
	});

	it('adds another with a passkey instead of password and code', async () => {
		server.passkeys = [
			{
				id: 1,
				owner: 'fiona',
				key: 'a2V5MQ',
				name: 'Phone',
				created_at: '2026-11-01T10:00:00Z',
				used_at: null
			}
		];
		server.recoveryOfPasskeys = ['AAAAA-BBBBB'];
		const device = fakeDevice('a2V5MQ', 'a2V5Mg');
		open();
		await userEvent.click(await screen.findByRole('button', { name: 'Add a passkey' }));
		await userEvent.type(screen.getByLabelText('Name, such as the device'), ' 2');
		await userEvent.click(await screen.findByRole('button', { name: 'Confirm with a passkey' }));
		expect(await screen.findByText('Passkey 2')).toBeInTheDocument();
		expect(device.get).toHaveBeenCalledTimes(1);
		expect(device.create).toHaveBeenCalledTimes(1);
		// No new codes: the second factor was on already.
		expect(screen.queryByText('Your recovery codes')).toBeNull();
		expect(server.passkeys.map((key) => key.key)).toEqual(['a2V5MQ', 'a2V5Mg']);
	});

	it('says when the browser’s window was closed, or the device has one already', async () => {
		const device = fakeDevice('a2V5MQ');
		server.passkeys = [
			{
				id: 1,
				owner: 'fiona',
				key: 'a2V5MQ',
				name: 'Phone',
				created_at: '2026-11-01T10:00:00Z',
				used_at: null
			}
		];
		server.recoveryOfPasskeys = ['AAAAA-BBBBB'];
		open();
		await userEvent.click(await screen.findByRole('button', { name: 'Add a passkey' }));
		device.refuse('NotAllowedError');
		await userEvent.click(await screen.findByRole('button', { name: 'Confirm with a passkey' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('was closed before');
		device.refuse('InvalidStateError', 'create');
		await userEvent.click(screen.getByRole('button', { name: 'Confirm with a passkey' }));
		await vi.waitFor(() => expect(device.create).toHaveBeenCalled());
		expect(await screen.findByRole('alert')).toHaveTextContent(
			'This device has a passkey for your account already.'
		);
		expect(server.passkeys).toHaveLength(1);
	});

	it('is not there where the server or the browser has no passkeys', async () => {
		fakeDevice();
		server.passkeysOn = false;
		const reader = new ServerReader({ name: 'fiona', admin: false, passkeys: false }, session);
		render(AccountDialog, { reader, tab: 'security', onclose });
		expect(await screen.findByText('Off.')).toBeInTheDocument();
		expect(screen.queryByText('Passkeys')).toBeNull();
		cleanup();
		noDevice();
		open();
		expect(await screen.findByText('Off.')).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Add a passkey' })).toBeNull();
	});
});

describe('the second factor in the account settings', () => {
	let server: FakeServer;
	const session = { signOut: () => {}, expired: () => {} };

	beforeEach(() => {
		server = new FakeServer();
		vi.stubGlobal('fetch', server.fetch);
	});

	afterEach(() => vi.unstubAllGlobals());

	const onclose = vi.fn();

	function open(account = { name: 'fiona', admin: false }) {
		onclose.mockClear();
		const reader = new ServerReader(account, session);
		render(AccountDialog, { reader, tab: 'security', onclose });
		return reader;
	}

	/** A click beside the window, and Escape. */
	async function dismiss() {
		await userEvent.click(screen.getByRole('dialog', { name: 'Account settings' }).parentElement!);
		await userEvent.keyboard('{Escape}');
	}

	it('is set up with the password and a code, and shows the recovery codes once', async () => {
		open();
		expect(await screen.findByText('Off.')).toBeInTheDocument();
		const password = screen.getByLabelText('Your password');
		await userEvent.type(password, 'wrong');
		await userEvent.click(screen.getByRole('button', { name: 'Set up' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('The password is not right.');
		expect(password).toHaveValue('');

		await userEvent.type(password, 'correct horse');
		await userEvent.click(screen.getByRole('button', { name: 'Set up' }));
		const qr = await screen.findByRole('img', { name: 'QR code for the authenticator app' });
		expect(qr.getAttribute('src')).toMatch(/^data:image\/svg\+xml,/);
		expect(screen.getByText('JBSW Y3DP EHPK 3PXP JBSW Y3DP EHPK 3PXP')).toBeInTheDocument();

		// The QR code grows and shrinks with a click, and the key is copied as the app takes it.
		const larger = screen.getByRole('button', { name: 'Show the QR code larger' });
		expect(qr).toHaveClass('w-48');
		await userEvent.click(larger);
		expect(larger).toHaveAttribute('aria-pressed', 'true');
		expect(qr).toHaveClass('w-96');
		await userEvent.click(screen.getByRole('button', { name: 'Show the QR code smaller' }));
		expect(qr).toHaveClass('w-48');
		const writeText = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue();
		await userEvent.click(screen.getByRole('button', { name: 'Copy the key' }));
		expect(writeText).toHaveBeenLastCalledWith('JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP');
		expect(screen.getByRole('button', { name: 'Copied' })).toBeInTheDocument();

		// Neither a click beside the window nor Escape loses the setup.
		await dismiss();
		expect(onclose).not.toHaveBeenCalled();

		const code = screen.getByLabelText('The code the app shows now');
		await userEvent.type(code, '111111');
		await userEvent.click(screen.getByRole('button', { name: 'Confirm' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('That code is not right.');
		await userEvent.type(code, server.appCode);
		await userEvent.click(screen.getByRole('button', { name: 'Confirm' }));
		expect(await screen.findByText('AAAAA-BBBBB')).toBeInTheDocument();
		expect(screen.getByText('CCCCC-DDDDD')).toBeInTheDocument();
		await userEvent.click(screen.getByRole('button', { name: 'Copy the codes' }));
		expect(writeText).toHaveBeenLastCalledWith('AAAAA-BBBBB\nCCCCC-DDDDD');
		expect(screen.getByRole('button', { name: 'Copied' })).toBeInTheDocument();
		await dismiss();
		expect(onclose).not.toHaveBeenCalled();

		await userEvent.click(screen.getByRole('button', { name: 'I have kept them' }));
		expect(await screen.findByText('On.')).toBeInTheDocument();
		expect(screen.getByText('2 recovery codes left.')).toBeInTheDocument();
		expect(screen.queryByText('AAAAA-BBBBB')).toBeNull();
		// Once the codes are kept, the window closes as any other.
		await dismiss();
		expect(onclose).toHaveBeenCalled();
	});

	it('is switched off with the password and a code', async () => {
		server.secondFactor = { code: server.appCode, recovery: ['AAAAA-BBBBB'] };
		open();
		expect(await screen.findByText('On.')).toBeInTheDocument();
		const password = screen.getByLabelText('Your password');
		const code = screen.getByLabelText('Code of the app, or a recovery code');
		// The password alone is not enough, nor with a wrong code.
		await userEvent.type(password, 'correct horse');
		await userEvent.type(code, '111111');
		await userEvent.click(screen.getByRole('button', { name: 'Switch off' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('That code is not right.');
		expect([password, code].map((field) => (field as HTMLInputElement).value)).toEqual(['', '']);
		expect(server.secondFactor).not.toBeNull();

		await userEvent.type(password, 'wrong');
		await userEvent.type(code, 'AAAAA-BBBBB');
		await userEvent.click(screen.getByRole('button', { name: 'Switch off' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('The password is not right.');

		await userEvent.type(password, 'correct horse');
		await userEvent.type(code, 'AAAAA-BBBBB');
		await userEvent.click(screen.getByRole('button', { name: 'Switch off' }));
		expect(await screen.findByText('Off.')).toBeInTheDocument();
		expect(server.secondFactor).toBeNull();
	});

	it('tells an admin whose right waits for it', async () => {
		server.adminNeedsSecondFactor = true;
		open({ name: 'fiona', admin: false, adminNeedsSecondFactor: true } as never);
		expect(await screen.findByRole('status')).toHaveTextContent(
			'This server makes admins admins only with a second factor.'
		);
	});

	it('has no tab where the reader cannot have one', () => {
		render(AccountDialog, { reader: new DemoReader(() => now), onclose: () => {} });
		expect(screen.queryByRole('tab', { name: 'Security' })).toBeNull();
	});
});

describe('Reported feeds on the status page', () => {
	it('names the feed, its address and the reasons, never who reported', async () => {
		render(StatusDialog, { reader: new DemoReader(() => now), onclose: () => {} });
		expect(await screen.findByRole('heading', { name: 'Reported feeds' })).toBeInTheDocument();
		expect(screen.getByText('Readers reported 1 feed that is not blocked.')).toBeInTheDocument();
		expect(screen.getByText('Daily Deals')).toBeInTheDocument();
		expect(screen.getByText('https://deals.example/feed.xml')).toBeInTheDocument();
		expect(screen.getByText(/^3 reports, last/)).toBeInTheDocument();
		expect(screen.getByText('Spam 2 · Malware, phishing or fraud 1')).toBeInTheDocument();
	});
});

describe('ReportDialog', () => {
	function open(outcomes: ('sent' | 'tooMany' | 'unreachable')[], seq?: number) {
		const reportContent = vi.fn(async () => outcomes.shift() ?? 'sent');
		const reader = { reportContent } as unknown as Reader;
		const onclose = vi.fn();
		render(ReportDialog, { reader, feedId: 'feed-3', seq, title: 'Daily Deals', onclose });
		return { reportContent, onclose };
	}

	it('sends a reason and a note, and thanks', async () => {
		const { reportContent, onclose } = open(['sent'], 4);
		expect(screen.getByRole('dialog', { name: 'Report the article' })).toBeInTheDocument();
		expect(screen.getByText(/not who reported it/)).toBeInTheDocument();
		await userEvent.click(screen.getByRole('radio', { name: 'Somebody else’s work' }));
		await userEvent.type(screen.getByRole('textbox'), 'Copied from my blog');
		expect(screen.getByText('481 characters left')).toBeInTheDocument();
		await userEvent.click(screen.getByRole('button', { name: 'Send the report' }));
		expect(reportContent).toHaveBeenCalledWith(
			{ feedId: 'feed-3', seq: 4 },
			'copyright',
			'Copied from my blog'
		);
		expect(await screen.findByRole('status')).toHaveTextContent('Thank you.');
		// The window's own close button, and the one under the thanks.
		const closes = screen.getAllByRole('button', { name: 'Close' });
		expect(closes).toHaveLength(2);
		await userEvent.click(closes[1]!);
		expect(onclose).toHaveBeenCalled();
	});

	it('says when there were too many, and lets the reader try again', async () => {
		const { reportContent } = open(['tooMany', 'unreachable', 'sent']);
		expect(screen.getByRole('dialog', { name: 'Report the feed' })).toBeInTheDocument();
		expect(screen.getByRole('textbox')).toHaveAttribute('maxlength', '500');
		const send = screen.getByRole('button', { name: 'Send the report' });
		await userEvent.click(send);
		expect(await screen.findByRole('alert')).toHaveTextContent('many reports lately');
		await userEvent.click(send);
		expect(await screen.findByRole('alert')).toHaveTextContent('did not reach the server');
		await userEvent.click(send);
		expect(await screen.findByRole('status')).toHaveTextContent('Thank you.');
		expect(reportContent).toHaveBeenLastCalledWith(
			{ feedId: 'feed-3', seq: undefined },
			'spam',
			''
		);
	});
});

describe('Apps in the account settings', () => {
	let server: FakeServer;
	const session = { signOut: () => {}, expired: () => {} };
	const onclose = vi.fn();

	beforeEach(() => {
		server = new FakeServer();
		server.accounts.set('fiona', 'correct horse');
		server.signedIn = 'fiona';
		vi.stubGlobal('fetch', server.fetch);
		vi.stubGlobal('EventSource', server.EventSource);
		onclose.mockClear();
	});

	afterEach(() => {
		vi.unstubAllGlobals();
		vi.restoreAllMocks();
	});

	function open(account: Partial<{ appApi: boolean }> = { appApi: true }) {
		const reader = new ServerReader({ name: 'fiona', admin: false, ...account }, session);
		reader.start();
		render(AccountDialog, { reader, tab: 'apps', onclose });
		return reader;
	}

	it('sets up an app step by step, and says when it logged in', async () => {
		open();
		expect(await screen.findByText('No app is set up yet.')).toBeInTheDocument();
		await userEvent.click(screen.getByRole('button', { name: 'Set up an app' }));
		expect(screen.getByText('Which app do you want to use?')).toBeInTheDocument();
		const site = screen.getByRole('link', { name: 'Website of NetNewsWire' });
		expect(site).toHaveAttribute('href', 'https://netnewswire.com/');
		expect(site).toHaveAttribute('rel', 'noopener noreferrer');
		expect(screen.getByRole('link', { name: 'Website of Read You' })).toHaveAttribute(
			'href',
			'https://f-droid.org/packages/me.ash.reader/'
		);
		await userEvent.click(screen.getByRole('button', { name: 'Read You' }));

		expect(screen.getByLabelText('Name of the app password')).toHaveValue('Read You');
		await userEvent.type(screen.getByLabelText('Your password'), 'wrong');
		await userEvent.click(screen.getByRole('button', { name: 'Create the app password' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('The password is not right.');
		// A click beside the window does not lose the setup.
		await userEvent.click(screen.getByRole('dialog', { name: 'Account settings' }).parentElement!);
		expect(onclose).not.toHaveBeenCalled();
		await userEvent.type(screen.getByLabelText('Your password'), 'correct horse');
		await userEvent.click(screen.getByRole('button', { name: 'Create the app password' }));

		// What to enter, with Read You's address ending in a slash.
		expect(await screen.findByText(/In Read You, open Settings/)).toBeInTheDocument();
		expect(screen.getByText(`${location.origin}/greader/`)).toBeInTheDocument();
		expect(screen.getByText('fiona')).toBeInTheDocument();
		expect(screen.getByText('abcd-efgh-ijkl-mnop-qrst-uvwx')).toBeInTheDocument();
		// Read You loads nothing from the feeds' websites itself.
		expect(screen.queryByText(/^Privacy:/)).toBeNull();
		// The way in Read You, in pictures from the app itself.
		await userEvent.click(screen.getByText('Step by step, with pictures'));
		const pictures = screen.getAllByRole('img', { name: /^Read You, step / });
		expect(pictures.map((picture) => picture.getAttribute('src'))).toEqual(
			[1, 2, 3, 4, 5].map((n) => `/apps/readyou-${n}.webp`)
		);
		expect(screen.getByText(/^5\. Enter the server address as Server URL/)).toBeInTheDocument();
		const writeText = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue();
		await userEvent.click(screen.getByRole('button', { name: 'Copy App password' }));
		expect(writeText).toHaveBeenLastCalledWith('abcd-efgh-ijkl-mnop-qrst-uvwx');
		expect(screen.getByRole('status')).toHaveTextContent('Waiting for the app to log in');
		expect(server.appPasswords.map((app) => app.name)).toEqual(['Read You']);

		server.appLogsIn(1);
		expect(await screen.findByText('Connected: the app has logged in.')).toBeInTheDocument();
		await userEvent.click(screen.getByRole('button', { name: 'Done' }));
		expect(await screen.findByText('Read You')).toBeInTheDocument();
		expect(screen.getByText(/^last used/)).toBeInTheDocument();
		await userEvent.click(screen.getByRole('button', { name: 'Revoke the app password Read You' }));
		expect(await screen.findByText('No app is set up yet.')).toBeInTheDocument();
		expect(server.appPasswords).toEqual([]);
	});

	it('shows the way in NetNewsWire on the Mac in pictures, and what it loads itself', async () => {
		open();
		await userEvent.click(await screen.findByRole('button', { name: 'Set up an app' }));
		await userEvent.click(screen.getByRole('button', { name: 'NetNewsWire' }));
		await userEvent.type(screen.getByLabelText('Your password'), 'correct horse');
		await userEvent.click(screen.getByRole('button', { name: 'Create the app password' }));

		// Its fields by their names, and the address without a slash.
		expect(await screen.findByText(/^In NetNewsWire, open Settings/)).toBeInTheDocument();
		expect(screen.getByText(`${location.origin}/greader`)).toBeInTheDocument();
		// What it means that it loads the feeds' icons from their websites.
		expect(
			screen.getByText(/^Privacy: NetNewsWire loads the icons of your feeds itself/)
		).toHaveTextContent('which see your IP address');
		await userEvent.click(screen.getByText('Step by step, with pictures'));
		const pictures = screen.getAllByRole('img', { name: /^NetNewsWire, step / });
		const files = [1, 2, 3].map((n) => `/apps/netnewswire-${n}.webp`);
		expect(pictures.map((picture) => picture.getAttribute('src'))).toEqual(files);
		// Each opens large, and is shown at half its size in pixels.
		expect(pictures.map((picture) => picture.closest('a')?.getAttribute('href'))).toEqual(files);
		expect(pictures[0].closest('li')).toHaveStyle({ width: '320px' });
		expect(screen.getByText(/^3\. Enter the name as Login/)).toBeInTheDocument();
	});

	it('asks for a code where the account has a second factor', async () => {
		server.secondFactor = { code: server.appCode, recovery: [] };
		open();
		await userEvent.click(await screen.findByRole('button', { name: 'Set up an app' }));
		await userEvent.click(screen.getByRole('button', { name: 'Another app' }));
		await userEvent.type(screen.getByLabelText('Name of the app password'), 'Reeder');
		await userEvent.type(screen.getByLabelText('Your password'), 'correct horse');
		const code = await screen.findByLabelText('Code of your authenticator app, or a recovery code');
		await userEvent.type(code, '111111');
		await userEvent.click(screen.getByRole('button', { name: 'Create the app password' }));
		expect(await screen.findByRole('alert')).toHaveTextContent('That code is not right.');
		await userEvent.type(screen.getByLabelText('Your password'), 'correct horse');
		await userEvent.type(code, server.appCode);
		await userEvent.click(screen.getByRole('button', { name: 'Create the app password' }));
		expect(await screen.findByText(/“Google Reader API”/)).toBeInTheDocument();
		// Any other app gets the address without a slash.
		expect(screen.getByText(`${location.origin}/greader`)).toBeInTheDocument();
	});

	it('is not there where the server lets no app in', async () => {
		server.appApi = false;
		const reader = new ServerReader({ name: 'fiona', admin: false, appApi: false }, session);
		render(AccountDialog, { reader, onclose });
		expect(screen.queryByRole('tab', { name: 'Apps' })).toBeNull();
	});

	it('shows in the draft, with app passwords that open nothing', async () => {
		render(AccountDialog, { reader: new DemoReader(() => now), tab: 'apps', onclose });
		await userEvent.click(await screen.findByRole('button', { name: 'Set up an app' }));
		await userEvent.click(screen.getByRole('button', { name: 'NetNewsWire' }));
		await userEvent.click(screen.getByRole('button', { name: 'Create the app password' }));
		expect(await screen.findByText(/choose FreshRSS/)).toBeInTheDocument();
		expect(screen.getByText(/^[a-z2-7]{4}(-[a-z2-7]{4}){5}$/)).toBeInTheDocument();
	});
});
