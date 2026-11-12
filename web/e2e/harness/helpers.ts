// What the specs share: logging in, the made-up site's switches, and making the
// server fetch now instead of when a feed is due.

import { execFileSync } from 'node:child_process';
import { expect, type Page } from '@playwright/test';
import { PASSWORD, SITE, database } from './ports.js';

export { PASSWORD, SITE };

/** Logs in through the login page and waits for the reader. */
export async function logIn(page: Page, name: string) {
	await page.goto('/');
	await page.getByLabel('Name').fill(name);
	await page.getByLabel('Password').fill(PASSWORD);
	await page.getByRole('button', { name: 'Log in' }).click();
	await expect(sidebar(page)).toBeVisible();
}

export const sidebar = (page: Page) => page.getByRole('navigation', { name: 'Subscriptions' });

/** The row of a feed or a folder in the sidebar. */
export const node = (page: Page, title: string) =>
	sidebar(page).locator('[data-node]', { hasText: title });

/** The rows of the list, each an article. */
export const articles = (page: Page) => page.locator('[data-article]');

/** The row of the article with this title. */
export const article = (page: Page, title: string) =>
	page.locator('[data-article]', { hasText: title });

/** The unread count the page's title carries: "(3) rueder". */
export async function unreadInTitle(page: Page) {
	const count = /^\(([\d,.]+)\)/.exec(await page.title())?.[1] ?? '0';
	return Number(count.replace(/\D/g, ''));
}

/**
 * How many articles of a feed the server counts as unread. A change shows in
 * the page before the server has it; a test that reloads asks here first.
 */
export async function unreadOnServer(page: Page, title: string) {
	const tree = await (await page.request.get('/api/tree')).json();
	const feed = tree.feeds.find((feed: { title: string }) => feed.title === title);
	return feed?.unread as number | undefined;
}

/** Subscribes to a feed of the made-up site through the dialog, by the address of its page. */
export async function subscribe(page: Page, path: string, title: string) {
	await page.getByRole('button', { name: 'Subscribe', exact: true }).click();
	const dialog = page.getByRole('dialog', { name: 'Subscribe to a feed' });
	await dialog.getByLabel('Address of the feed or the website').fill(`${SITE}/${path}/`);
	await dialog.getByRole('button', { name: 'Find feeds' }).click();
	await dialog.getByRole('button', { name: `Subscribe: ${title}` }).click();
	await expect(node(page, title)).toBeVisible();
	if (await dialog.isVisible()) await dialog.getByRole('button', { name: 'Close' }).click();
}

/** Changes what the made-up site serves: `publish` a new article, `revise` the newest, `reset`. */
export async function site(change: 'publish' | 'revise' | 'reset', feed = '', text = '') {
	const query = new URLSearchParams({ feed, text });
	const answer = await fetch(`${SITE}/_control/${change}?${query}`, { method: 'POST' });
	if (!answer.ok) throw new Error(`the made-up site answered ${answer.status} to ${change}`);
}

/**
 * Makes every feed due, so the server fetches it on its next round, which
 * comes every 30 seconds. There is no request for that: a reader cannot make
 * the server fetch, so the tests say it to the database.
 */
export function fetchSoon() {
	execFileSync('psql', [database(), '--quiet', '-c', 'UPDATE feeds SET next_fetch_at = now()'], {
		stdio: ['ignore', 'ignore', 'inherit']
	});
}

/** How long a test waits for what a fetch brings: a round of the server and some. */
export const AFTER_FETCH = 45_000;

/** Everything the page asked for that is not rueder's own. Empty is the point. */
export function foreignRequests(page: Page) {
	const own = new URL(page.url() === 'about:blank' ? 'http://localhost:3210' : page.url()).origin;
	const foreign: string[] = [];
	page.on('request', (request) => {
		const url = new URL(request.url());
		if (url.origin !== own && !['data:', 'blob:'].includes(url.protocol))
			foreign.push(request.url());
	});
	return foreign;
}
