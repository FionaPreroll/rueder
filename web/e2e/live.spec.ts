// Live updates: what the server fetches reaches the open page by itself, and a
// change in one tab shows in another.

import { expect, test } from '@playwright/test';
import {
	AFTER_FETCH,
	article,
	articles,
	fetchSoon,
	logIn,
	node,
	site,
	subscribe
} from './harness/helpers.ts';

test.describe.configure({ mode: 'serial' });
// These wait for a round of the server's fetching, which comes every 30 seconds.
test.setTimeout(2 * AFTER_FETCH + 30_000);

test('a new article is announced over the list and comes in when asked for', async ({ page }) => {
	await logIn(page, 'live');
	await subscribe(page, 'live', 'Live Wire');
	await node(page, 'Live Wire').click();
	await expect(articles(page)).toHaveCount(2);
	await expect(node(page, 'Live Wire')).toContainText('(2)');

	await site('publish', 'live', 'The third report, just in.');
	fetchSoon();
	// The count moves and the list says so, but does not jump under the reader.
	const announcement = page.getByRole('button', { name: '1 new article' });
	await expect(announcement).toBeVisible({ timeout: AFTER_FETCH });
	await expect(node(page, 'Live Wire')).toContainText('(3)');
	await expect(articles(page)).toHaveCount(2);

	await announcement.click();
	await expect(articles(page)).toHaveCount(3);
	await expect(articles(page).first()).toContainText('Wire news 3');
	await expect(announcement).toHaveCount(0);
});

test('an article that changed says so, and its versions can be compared', async ({ page }) => {
	await logIn(page, 'live');
	await node(page, 'Live Wire').click();
	await expect(articles(page)).toHaveCount(3);
	await expect(article(page, 'Wire news 3').getByText('updated')).toHaveCount(0);

	await site('revise', 'live', 'Correction: it was the fourth.');
	fetchSoon();
	await expect(article(page, 'Wire news 3').getByText('updated')).toBeVisible({
		timeout: AFTER_FETCH
	});

	await article(page, 'Wire news 3')
		.getByRole('button', { name: /Wire news 3/ })
		.click();
	await expect(page.locator('.article-body')).toContainText('Correction: it was the fourth.');
	await page.getByRole('button', { name: 'Compare versions' }).click();
	const versions = page.getByRole('dialog', { name: 'Versions of this article' });
	await expect(versions.locator('ins')).toContainText(['Correction: it was the fourth.']);
});

test('what is read in one tab is read in the other', async ({ page, context }) => {
	await logIn(page, 'live');
	await expect(node(page, 'Live Wire')).toContainText('(2)');
	// A second tab of the same browser, so the same session.
	const second = await context.newPage();
	await second.goto('/');
	await expect(node(second, 'Live Wire')).toContainText('(2)');

	await node(page, 'Live Wire').click();
	await page.getByRole('button', { name: 'Mark all as read' }).first().click();
	await expect(node(page, 'Live Wire')).not.toContainText('(');
	await expect(node(second, 'Live Wire')).not.toContainText('(');
});
