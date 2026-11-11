// The page of rueder with its server, against a made-up one: logging in, reading
// and logging out, and what the page leaves out because the server cannot do it yet.

import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { FakeServer } from '#lib/api/test/fake-server.ts';
import { defaults, settings } from '#lib/settings.svelte.ts';
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

		await userEvent.click(screen.getByRole('button', { name: 'Logged in as fiona' }));
		await userEvent.click(screen.getByRole('menuitem', { name: 'Log out' }));
		expect(await screen.findByRole('heading', { name: 'Log in to rueder' })).toBeInTheDocument();
		expect(server.signedIn).toBeNull();
		expect(document.title).toBe('rueder');
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

	it('leaves out what the server cannot do yet', async () => {
		render(Page);
		expect(await screen.findByText('New in your subscriptions')).toBeInTheDocument();
		// No subscribing, and none of the switches of the made-up reader.
		expect(screen.queryByRole('button', { name: 'Subscribe' })).toBeNull();
		expect(screen.queryByRole('button', { name: 'Draft' })).toBeNull();
		expect(screen.queryByRole('button', { name: 'Live' })).toBeNull();

		await userEvent.keyboard('ga');
		await userEvent.click(screen.getByRole('button', { name: /^All items/, expanded: false }));
		expect(screen.getByRole('menuitemradio', { name: 'Unread only' })).toBeInTheDocument();
		expect(screen.queryByRole('menuitemradio', { name: 'Oldest first' })).toBeNull();
		expect(screen.queryByRole('button', { name: 'More ways to mark as read' })).toBeNull();
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
