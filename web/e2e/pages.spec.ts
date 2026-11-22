// A website without a feed: the search offers it with a hint that only titles
// and summaries come, and once subscribed it reads like any feed. Its feed comes
// from the made-up RSSifier on the made-up site.

import { expect, test } from '@playwright/test';
import { article, logIn, node, SITE } from './harness/helpers.ts';

test('a website without a feed is offered with a hint and read like a feed', async ({ page }) => {
	await logIn(page, 'pages');
	await page.getByRole('button', { name: 'Subscribe', exact: true }).click();
	const dialog = page.getByRole('dialog', { name: 'Subscribe to a feed' });
	await dialog.getByLabel('Address of the feed or the website').fill(`${SITE}/club-pages/`);
	await dialog.getByRole('button', { name: 'Find feeds' }).click();
	await expect(dialog.getByRole('status')).toHaveText('There is one feed:');
	await expect(dialog).toContainText(
		'A website without a feed: rueder makes one of it, with titles and summaries only.'
	);
	await dialog.getByRole('button', { name: 'Subscribe: The Club' }).click();
	await expect(node(page, 'The Club')).toBeVisible();
	await expect(article(page, 'General meeting in November')).toBeVisible();
	await expect(article(page, 'Summer party at the harbour')).toBeVisible();
});
