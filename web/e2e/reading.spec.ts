// The way of a reader: subscribe to a feed, read, star, mark as read.

import { expect, test } from '@playwright/test';
import {
	article,
	articles,
	foreignRequests,
	logIn,
	node,
	sidebar,
	SITE,
	subscribe,
	unreadInTitle,
	unreadOnServer
} from './harness/helpers.ts';

// One account for the whole file: each test goes on where the one before left off.
test.describe.configure({ mode: 'serial' });

test('subscribes to a feed found on a website', async ({ page }) => {
	await logIn(page, 'reading');
	await expect(page.getByText('You have read everything.')).toBeVisible();
	await page.getByRole('button', { name: 'Subscribe', exact: true }).click();
	const dialog = page.getByRole('dialog', { name: 'Subscribe to a feed' });
	// The address of the site, not of its feed: the server looks for the feed.
	await dialog.getByLabel('Address of the feed or the website').fill(`${SITE}/harbour/`);
	await dialog.getByRole('button', { name: 'Find feeds' }).click();
	await expect(dialog.getByRole('status')).toHaveText('There is one feed:');
	await dialog.getByRole('button', { name: 'Subscribe: Harbour Herald' }).click();

	// A feed nobody read before brings what its first fetch found, all of it unread.
	await expect(node(page, 'Harbour Herald')).toContainText('(12)');
	await expect.poll(() => unreadInTitle(page)).toBe(12);
});

test('reads an article: cleaned text, its image through the proxy, nothing from elsewhere', async ({
	page
}) => {
	const foreign = foreignRequests(page);
	await logIn(page, 'reading');
	await node(page, 'Harbour Herald').click();
	await expect(page.getByRole('heading', { level: 1 })).toHaveText('Harbour Herald');
	await expect(articles(page)).toHaveCount(12);
	await expect(articles(page).first()).toContainText('Harbour news 12');

	await article(page, 'Harbour news 12')
		.getByRole('button', { name: /Harbour news 12/ })
		.click();
	const body = page.locator('.article-body');
	await expect(body).toContainText('The ferry Seute Deern runs again, trip 12.');

	// The picture comes from rueder, in the size the article shows.
	const image = body.locator('img');
	await expect(image).toHaveCount(1);
	await expect(image).toHaveAttribute('src', /^\/img\/[\w-]+\/image\//);
	await expect
		.poll(() => image.evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth))
		.toBe(320);
	// The counting pixel and the script are gone, the link lost its tracking and its handler.
	await expect(body.locator('script')).toHaveCount(0);
	expect(await page.evaluate(() => 'tracked' in window)).toBe(false);
	const link = body.getByRole('link', { name: 'Read more' });
	await expect(link).toHaveAttribute('href', `${SITE}/harbour/more?id=12`);
	await expect(link).toHaveAttribute('rel', /noopener/);
	await expect(link).not.toHaveAttribute('onclick');

	// Opening it read it.
	await expect(node(page, 'Harbour Herald')).toContainText('(11)');
	expect(foreign).toEqual([]);
});

test('stars, keeps unread and finds both again after a reload', async ({ page }) => {
	await logIn(page, 'reading');
	await node(page, 'Harbour Herald').click();
	await expect(articles(page)).toHaveCount(12);

	// With the keyboard: down to the second article, star it, and unread again.
	await page.keyboard.press('j');
	await page.keyboard.press('j');
	await expect(page.locator('.article-body')).toContainText('trip 11.');
	await expect(node(page, 'Harbour Herald')).toContainText('(10)');
	await page.keyboard.press('s');
	await page.keyboard.press('m');
	await expect(node(page, 'Harbour Herald')).toContainText('(11)');

	await expect.poll(() => unreadOnServer(page, 'Harbour Herald')).toBe(11);
	await page.reload();
	await expect(node(page, 'Harbour Herald')).toContainText('(11)');
	await sidebar(page).getByRole('button', { name: 'Starred items' }).click();
	await expect(articles(page)).toHaveCount(1);
	await expect(articles(page).first()).toContainText('Harbour news 11');
});

test('shows only what is unread, and marks all as read', async ({ page }) => {
	await logIn(page, 'reading');
	await node(page, 'Harbour Herald').click();
	await expect(articles(page)).toHaveCount(12);
	await page.keyboard.press('U');
	await expect(articles(page)).toHaveCount(11);
	await expect(article(page, 'Harbour news 12')).toHaveCount(0);

	await page.getByRole('button', { name: 'Mark all as read' }).first().click();
	await expect(node(page, 'Harbour Herald')).not.toContainText('(');
	await expect.poll(() => unreadInTitle(page)).toBe(0);

	await expect.poll(() => unreadOnServer(page, 'Harbour Herald')).toBe(0);
	await page.reload();
	await expect(node(page, 'Harbour Herald')).not.toContainText('(');
	await node(page, 'Harbour Herald').click();
	await expect(articles(page)).toHaveCount(12);
	await page.keyboard.press('U');
	await expect(page.getByText('Nothing unread here.')).toBeVisible();
});
