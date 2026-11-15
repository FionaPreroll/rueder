// The unread items against the real server: only what is unread, and an
// article that was read goes once the reader leaves it, by opening another or
// by closing it, the last one too.

import { expect, test } from '@playwright/test';
import { article, articles, logIn, sidebar, subscribe, unreadOnServer } from './harness/helpers.ts';

test('the unread items lose what was read once the reader leaves it', async ({ page }) => {
	await logIn(page, 'unreader');
	await subscribe(page, 'harbour-unread', 'Harbour Herald');
	// One read before: it is not among the unread items.
	await sidebar(page).getByText('Harbour Herald').click();
	await article(page, 'Harbour news 12')
		.getByRole('button', { name: /Harbour news 12/ })
		.click();
	await expect.poll(() => unreadOnServer(page, 'Harbour Herald')).toBe(11);

	await sidebar(page)
		.getByRole('button', { name: /^Unread items/ })
		.click();
	await expect(page.getByRole('heading', { level: 1 })).toHaveText('Unread items');
	await expect(sidebar(page).getByRole('button', { name: /^Unread items/ })).toContainText('(11)');
	await expect(articles(page)).toHaveCount(11);
	await expect(articles(page).first()).toContainText('Harbour news 11');

	// Opened, it stays while it is read.
	await page.keyboard.press('j');
	await expect(page.locator('.article-body')).toContainText('trip 11.');
	await expect(articles(page)).toHaveCount(11);
	// Opening the next leaves it.
	await page.keyboard.press('j');
	await expect(page.locator('.article-body')).toContainText('trip 10.');
	await expect(articles(page)).toHaveCount(10);
	await expect(articles(page).first()).toContainText('Harbour news 10');
	// Closing leaves it too; the next step opens the one that took its place.
	await page.keyboard.press('o');
	await expect(articles(page)).toHaveCount(9);
	await page.keyboard.press('j');
	await expect(page.locator('.article-body')).toContainText('trip 9.');
	await expect.poll(() => unreadOnServer(page, 'Harbour Herald')).toBe(8);

	// Loaded anew, the list is what the server has unread, and the open one,
	// still open, until the reader leaves it.
	await page.reload();
	await expect(page.getByRole('heading', { level: 1 })).toHaveText('Unread items');
	await expect(articles(page)).toHaveCount(9);
	await expect(page.locator('.article-body')).toContainText('trip 9.');
	await page.keyboard.press('j');
	await expect(page.locator('.article-body')).toContainText('trip 8.');
	await expect(articles(page)).toHaveCount(8);

	// Read on to the last: the button under it closes it, and the list is empty.
	for (let left = 7; left >= 1; left--) {
		await page.keyboard.press('j');
		await expect(articles(page)).toHaveCount(left);
	}
	await expect(page.locator('.article-body')).toContainText('trip 1.');
	await articles(page).getByRole('button', { name: 'Close', exact: true }).click();
	await expect(articles(page)).toHaveCount(0);
	await expect(page.getByText('Nothing unread here.')).toBeVisible();
	await expect.poll(() => unreadOnServer(page, 'Harbour Herald')).toBe(0);
});
