// The page of rueder with its server, against a made-up one: logging in, reading
// and logging out, and what the page leaves out because the server cannot do it yet.

import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FakeServer } from '#lib/api/test/fake-server.ts';
import { defaults, settings } from '#lib/settings.svelte.ts';
import { VIEW_KEY } from '#lib/view.ts';
import Page from './+page.svelte';

let server: FakeServer;
let tech: number;

beforeEach(() => {
	server = new FakeServer();
	const news = server.addFolder('News');
	tech = server.addFeed('Tech', news);
	server.addArticles(tech, 3);
	server.addArticles(server.addFeed('Lone', null, { error: 'timed out' }), 1);
	vi.stubGlobal('fetch', server.fetch);
	vi.stubGlobal('EventSource', server.EventSource);
});

afterEach(() => {
	vi.unstubAllGlobals();
	Object.assign(settings, defaults);
	localStorage.clear();
});

const heading = () => screen.getByRole('heading', { level: 1 });
const sidebar = () => screen.getByRole('navigation', { name: 'Subscriptions' });
const unreadInTitle = () => Number(/^\((\d+)\) rueder$/.exec(document.title)?.[1]);

/** The page of somebody who is logged in, once the tree is there. */
async function reading() {
	render(Page);
	await within(await screen.findByRole('navigation', { name: 'Subscriptions' })).findByRole(
		'button',
		{ name: /^Tech/ }
	);
}

describe('the page with a server', () => {
	it('asks who is there, and shows the login to nobody', async () => {
		server.signedIn = null;
		const release = server.hold(/^\/me$/);
		render(Page);
		expect(screen.getByRole('status')).toHaveTextContent('One moment');
		release();
		expect(await screen.findByRole('heading', { name: 'Log in to rueder' })).toBeInTheDocument();
		// Nothing of the reader was asked for.
		expect(server.requests.map((r) => r.path)).toEqual(['/me', '/login/ways']);
	});

	it('logs in, reads and logs out', async () => {
		server.signedIn = null;
		render(Page);
		await userEvent.type(await screen.findByLabelText('Name'), 'fiona');
		await userEvent.type(screen.getByLabelText('Password'), 'correct horse{Enter}');

		// The start page, with the tree and what is new.
		expect(await screen.findByText('New in your subscriptions')).toBeInTheDocument();
		await vi.waitFor(() => expect(unreadInTitle()).toBe(4));
		expect(within(sidebar()).getByRole('button', { name: /^News/ })).toHaveTextContent('(3)');
		const main = screen.getByRole('main');
		expect(within(main).getByRole('heading', { name: 'Feeds with problems' })).toBeInTheDocument();
		expect(within(main).getByText('The last fetch failed: timed out')).toBeInTheDocument();

		// A feed, and an article of it: its text comes when it is opened.
		await userEvent.click(within(sidebar()).getByRole('button', { name: /^Tech/ }));
		expect(heading()).toHaveTextContent('Tech');
		await userEvent.click(await screen.findByRole('button', { name: /Article 3 of feed/ }));
		expect(await screen.findByText(`Text 3 of feed ${tech}.`)).toBeInTheDocument();
		expect(unreadInTitle()).toBe(3);
		await vi.waitFor(() => expect(server.article(tech, 3)!.read).toBe(true));

		await userEvent.keyboard('s');
		await vi.waitFor(() => expect(server.article(tech, 3)!.starred).toBe(true));
		await userEvent.keyboard('A');
		await vi.waitFor(() => expect(server.unread(tech)).toBe(0));
		expect(unreadInTitle()).toBe(1);
		// Where the reader was is kept for a reload, and goes with the session.
		window.dispatchEvent(new Event('pagehide'));
		expect(localStorage.getItem(VIEW_KEY)).toContain(`"open":"${server.article(tech, 3)!.id}"`);

		await userEvent.click(screen.getByRole('button', { name: 'Logged in as fiona' }));
		await userEvent.click(screen.getByRole('menuitem', { name: 'Log out' }));
		expect(await screen.findByRole('heading', { name: 'Log in to rueder' })).toBeInTheDocument();
		expect(server.signedIn).toBeNull();
		expect(document.title).toBe('rueder');
		expect(localStorage.getItem(VIEW_KEY)).toBeNull();
	});

	it('comes back to the article it was left at after a reload', async () => {
		const first = render(Page);
		await within(await screen.findByRole('navigation', { name: 'Subscriptions' })).findByRole(
			'button',
			{ name: /^Tech/ }
		);
		await userEvent.click(within(sidebar()).getByRole('button', { name: /^Tech/ }));
		await userEvent.click(await screen.findByRole('button', { name: /Article 2 of feed/ }));
		expect(await screen.findByText(`Text 2 of feed ${tech}.`)).toBeInTheDocument();
		window.dispatchEvent(new Event('pagehide'));
		first.unmount();
		const asked = server.requests.length;

		render(Page);
		expect(await screen.findByText(`Text 2 of feed ${tech}.`)).toBeInTheDocument();
		expect(heading()).toHaveTextContent('Tech');
		// The start page was not asked for.
		const after = server.requests.slice(asked);
		expect(after.filter((r) => r.path.includes('limit=60'))).toHaveLength(0);
	});

	it('goes back to the login when the session runs out', async () => {
		render(Page);
		expect(await screen.findByText('New in your subscriptions')).toBeInTheDocument();
		server.signedIn = null;
		await userEvent.keyboard('ga');
		expect(await screen.findByRole('heading', { name: 'Log in to rueder' })).toBeInTheDocument();
	});

	it('says when a change was not saved, until the line is closed', async () => {
		await reading();
		await userEvent.click(within(sidebar()).getByRole('button', { name: /^Tech/ }));
		server.refuse('PUT', /\/state$/, 500);
		await userEvent.click(await screen.findByRole('button', { name: /Article 3 of feed/ }));
		const notice = await screen.findByRole('alert');
		expect(notice).toHaveTextContent('The server did not take your last change.');
		expect(unreadInTitle()).toBe(4);
		await userEvent.click(within(notice).getByRole('button', { name: 'Close' }));
		expect(screen.queryByRole('alert')).toBeNull();
	});

	it('compares the versions of an article that changed', async () => {
		const article = server.article(tech, 3)!;
		article.version = 2;
		article.html = '<p>The text. With an addition.</p>';
		article.versions = [
			{ version: 1, html: '<p>The text.</p>' },
			{ version: 2, html: article.html }
		].map((version) => ({
			...version,
			fetched_at: '2026-11-02T09:00:00Z',
			title: article.title,
			authors: [],
			tags: [],
			url: null
		}));
		await reading();
		await userEvent.click(within(sidebar()).getByRole('button', { name: /^Tech/ }));
		await userEvent.click(await screen.findByRole('button', { name: /Article 3 of feed/ }));
		await userEvent.click(await screen.findByRole('button', { name: 'Compare versions' }));
		const dialog = await screen.findByRole('dialog', { name: 'Versions of this article' });
		const added = [...dialog.querySelectorAll('.article-body ins')].map((el) => el.textContent);
		expect(added.join('').trim()).toBe('With an addition.');
	});

	it('says what arrived while reading, and brings it in on request', async () => {
		await reading();
		await userEvent.click(within(sidebar()).getByRole('button', { name: /^Tech/ }));
		await screen.findByRole('button', { name: /Article 3 of feed/ });
		server.addArticles(tech, 2);
		server.emit({ event: 'articles', data: { feed_id: tech, last_seq: 5, new: 2 } });
		const arrived = await screen.findByRole('button', { name: /2 new articles/ });
		expect(screen.queryByRole('button', { name: /Article 5 of feed/ })).toBeNull();
		await userEvent.click(arrived);
		expect(await screen.findByRole('button', { name: /Article 5 of feed/ })).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /new articles/ })).toBeNull();
	});

	it('says when a feed waits for its first fetch', async () => {
		server.addFeed('', null, { url: 'https://later.example/feed.xml', fetched: false });
		await reading();
		const row = within(sidebar()).getByRole('button', { name: /Not fetched yet/ });
		await userEvent.click(row);
		expect(
			await screen.findByText(/This feed has not been fetched yet\. Its articles come/)
		).toBeInTheDocument();
	});

	it('opens the first article with j or k from the start page', async () => {
		for (const key of ['j', 'k']) {
			const { unmount } = render(Page);
			expect(await screen.findByText('New in your subscriptions')).toBeInTheDocument();
			await userEvent.keyboard(key);
			await vi.waitFor(() =>
				expect(document.querySelector('[aria-expanded="true"][class*="py-"]')).toHaveTextContent(
					'Lone Article 1 of feed'
				)
			);
			expect(heading()).toHaveTextContent('All items');
			unmount();
		}
	});

	it('leaves out what the server cannot do yet', async () => {
		render(Page);
		expect(await screen.findByText('New in your subscriptions')).toBeInTheDocument();
		// None of the switches of the made-up reader.
		expect(screen.queryByRole('button', { name: 'Draft' })).toBeNull();
		expect(screen.queryByRole('button', { name: 'Live' })).toBeNull();
	});

	it('turns the list around and marks as read by age', async () => {
		render(Page);
		await userEvent.keyboard('ga');
		await screen.findAllByText(/Article 1 of feed/);
		await userEvent.click(screen.getByRole('button', { name: /^All items/, expanded: false }));
		await userEvent.click(screen.getByRole('menuitemradio', { name: 'Oldest first' }));
		await vi.waitFor(() =>
			expect(server.requests.some((r) => /^\/articles\?.*after=0/.test(r.path))).toBe(true)
		);

		// Everything the made-up server has is older than a day.
		await userEvent.click(screen.getByRole('button', { name: 'More ways to mark as read' }));
		await userEvent.click(screen.getByRole('menuitem', { name: 'Items older than a day' }));
		await vi.waitFor(() =>
			expect(server.requests.find((r) => r.path === '/read')?.body).toMatchObject({
				older_than: 86_400
			})
		);
		await vi.waitFor(() => expect(server.unread(tech)).toBe(0));
	});

	it('subscribes to a feed, shows it, and ends the subscription again', async () => {
		server.addSite('https://site.example', { url: 'https://site.example/feed.xml', title: 'Site' });
		await reading();
		await userEvent.click(screen.getByRole('button', { name: 'Subscribe' }));
		const dialog = screen.getByRole('dialog', { name: 'Subscribe to a feed' });
		await userEvent.type(within(dialog).getByRole('textbox'), 'site.example{Enter}');
		await userEvent.selectOptions(await within(dialog).findByLabelText('Into the folder'), 'News');
		await userEvent.click(within(dialog).getByRole('button', { name: 'Subscribe: Site' }));
		await vi.waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
		expect(heading()).toHaveTextContent('Site');
		const row = within(sidebar()).getByRole('button', { name: /^Site/ });
		expect(row).toHaveAttribute('aria-current', 'page');

		await userEvent.click(
			within(sidebar()).getByRole('button', { name: 'Actions for the feed Site' })
		);
		await userEvent.click(screen.getByRole('menuitem', { name: 'Unsubscribe' }));
		const confirm = screen.getByRole('dialog', { name: 'Unsubscribe?' });
		await userEvent.click(within(confirm).getByRole('button', { name: 'Unsubscribe' }));
		expect(within(sidebar()).queryByRole('button', { name: /^Site/ })).toBeNull();
		expect(heading()).toHaveTextContent('All items');
		await vi.waitFor(() => expect(server.feeds.some((feed) => feed.title === 'Site')).toBe(false));
	});

	it('writes a filter rule with the server saying what it would catch', async () => {
		await reading();
		expect(unreadInTitle()).toBe(4);
		await userEvent.click(screen.getByRole('button', { name: 'Settings' }));
		await userEvent.click(screen.getByRole('tab', { name: 'Filter rules' }));
		expect(screen.getByText('No rules yet.')).toBeInTheDocument();
		await userEvent.click(screen.getByRole('button', { name: 'Add a rule' }));
		const save = screen.getByRole('button', { name: 'Save' });
		expect(screen.getByRole('status')).toHaveTextContent('Enter the text to look for.');

		// The server's regular expressions decide, so the server is asked.
		await userEvent.selectOptions(screen.getByLabelText('Match'), 'regex');
		await userEvent.type(screen.getByLabelText('Text to look for'), '(Article 3');
		expect(screen.getByRole('status')).toHaveTextContent('Looking what it would catch');
		expect(save).toBeDisabled();
		await vi.waitFor(() =>
			expect(screen.getByRole('status')).toHaveTextContent(
				'This is not a valid regular expression.'
			)
		);
		expect(save).toBeDisabled();

		await userEvent.type(screen.getByLabelText('Text to look for'), ' of)');
		await vi.waitFor(() =>
			expect(screen.getByRole('status')).toHaveTextContent('Applies to 1 of 4 articles here now.')
		);
		await userEvent.click(save);
		const row = await screen.findByText(
			'Title matches the regular expression “(Article 3 of)”: hide the article'
		);
		// The server does not count what a saved rule finds.
		expect(row.closest('li')).not.toHaveTextContent('hits');
		await vi.waitFor(() => expect(server.rules.length).toBe(1));
		await vi.waitFor(() => expect(unreadInTitle()).toBe(3));
	});

	it('shows a feed’s icon from the proxy, and the stand-in when it does not load', async () => {
		server.feeds[0].icon_url = '/img/signature/icon/address';
		await reading();
		const row = within(sidebar()).getByRole('button', { name: /^Tech/ });
		const icon = row.closest('div')!.querySelector('img')!;
		expect(icon).toHaveAttribute('src', '/img/signature/icon/address');
		icon.dispatchEvent(new Event('error'));
		await vi.waitFor(() => expect(row.closest('div')!.querySelector('img')).toBeNull());
		expect(row.closest('div')).toHaveTextContent('T');
	});
});

describe('the legal texts of the operator', () => {
	const texts = {
		imprint: 'https://operator.example/imprint',
		privacy: 'https://operator.example/privacy',
		terms: 'https://operator.example/terms'
	};

	it('are linked from the menu of the account, and new terms are asked for once', async () => {
		server.legal = { ...texts, terms_version: '2026-11' };
		render(Page);
		const dialog = await screen.findByRole('alertdialog', {
			name: 'The terms of use have changed'
		});
		expect(
			await within(dialog).findByRole('link', { name: 'Read the terms of use' })
		).toHaveAttribute('href', texts.terms);
		await userEvent.click(within(dialog).getByRole('button', { name: 'I agree' }));
		await vi.waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull());
		expect(server.agreedTerms).toBe('2026-11');

		await userEvent.click(screen.getByRole('button', { name: 'Logged in as fiona' }));
		for (const [name, href] of [
			['Imprint', texts.imprint],
			['Privacy policy', texts.privacy],
			['Terms of use', texts.terms]
		]) {
			const link = screen.getByRole('menuitem', { name });
			expect(link).toHaveAttribute('href', href);
			expect(link).toHaveAttribute('rel', 'noopener noreferrer');
		}
	});

	it('cannot be put aside: who does not agree logs out', async () => {
		server.legal = { ...texts, terms_version: '2026-11' };
		render(Page);
		const dialog = await screen.findByRole('alertdialog');
		await userEvent.keyboard('{Escape}');
		expect(screen.getByRole('alertdialog')).toBeInTheDocument();
		await userEvent.click(within(dialog).getByRole('button', { name: 'Log out' }));
		expect(await screen.findByRole('heading', { name: 'Log in to rueder' })).toBeInTheDocument();
		expect(server.agreedTerms).toBeNull();
	});

	it('are nowhere where the operator names none', async () => {
		await reading();
		expect(screen.queryByRole('alertdialog')).toBeNull();
		await userEvent.click(screen.getByRole('button', { name: 'Logged in as fiona' }));
		expect(screen.queryByRole('menuitem', { name: 'Imprint' })).toBeNull();
		expect(screen.queryByRole('menuitem', { name: 'Terms of use' })).toBeNull();
	});
});
