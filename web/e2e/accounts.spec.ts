// One account sees nothing of another, though both read the same feed.

import { expect, test } from '@playwright/test';
import { articles, logIn, node, sidebar, subscribe } from './harness/helpers.ts';

test('what one account subscribes to, reads and sorts is its own', async ({ browser }) => {
	const mine = await (await browser.newContext()).newPage();
	await logIn(mine, 'other');
	await subscribe(mine, 'harbour', 'Harbour Herald');
	await mine.getByRole('button', { name: 'New folder' }).click();
	await mine.getByLabel('Name of the folder').fill('Mine alone');
	await mine.keyboard.press('Enter');
	await expect(node(mine, 'Mine alone')).toBeVisible();
	await node(mine, 'Harbour Herald').click();
	await expect(articles(mine)).toHaveCount(12);
	const tree = await (await mine.request.get('/api/tree')).json();
	const feed = tree.feeds.find((feed: { title: string }) => feed.title === 'Harbour Herald');

	// Another reader, in a browser of their own, has an empty rueder.
	const theirs = await (await browser.newContext()).newPage();
	await logIn(theirs, 'login');
	await expect(sidebar(theirs).locator('[data-node]')).toHaveCount(0);
	await expect(theirs.getByText('You have read everything.')).toBeVisible();
	// Asking for the feed and the folder by their numbers gets "not found", as for what is not there.
	expect((await theirs.request.get(`/api/feeds/${feed.id}/articles`)).status()).toBe(404);
	expect((await theirs.request.get(`/api/feeds/${feed.id}/articles/1`)).status()).toBe(404);
	expect((await theirs.request.get(`/api/folders/${tree.folders[0].id}/articles`)).status()).toBe(
		404
	);
	expect((await (await theirs.request.get('/api/articles')).json()).articles).toEqual([]);
});
