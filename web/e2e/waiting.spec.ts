// Changes made while the server cannot be reached wait in the tab and reach the
// server once it can, and from there the other tabs.

import { expect, test } from '@playwright/test';
import {
	article,
	articles,
	logIn,
	node,
	sidebar,
	subscribe,
	unreadOnServer
} from './harness/helpers.ts';

test('what is read and starred without a network arrives once it is back', async ({
	page,
	context,
	browser
}) => {
	await logIn(page, 'waiter');
	await subscribe(page, 'harbour-waiter', 'Harbour Herald');
	await node(page, 'Harbour Herald').click();
	await expect(articles(page)).toHaveCount(12);

	await context.setOffline(true);
	await expect(page.getByRole('button', { name: /^No network since/ })).toBeVisible();
	await article(page, 'Harbour news 12')
		.getByRole('button', { name: /Harbour news 12/ })
		.click();
	await page.keyboard.press('s');
	await expect(
		page.getByRole('button', { name: /^No network since .+\. 2 changes are waiting\./ })
	).toBeVisible();
	await expect(node(page, 'Harbour Herald')).toContainText('(11)');
	await expect(page.getByRole('alert')).toHaveCount(0);

	await context.setOffline(false);
	await expect(page.getByRole('button', { name: /waiting|No network/ })).toHaveCount(0);
	await expect.poll(() => unreadOnServer(page, 'Harbour Herald')).toBe(11);
	await expect(node(page, 'Harbour Herald')).toContainText('(11)');

	// Another tab has them from the server.
	const elsewhere = await browser.newContext();
	const other = await elsewhere.newPage();
	await logIn(other, 'waiter');
	await expect(node(other, 'Harbour Herald')).toContainText('(11)');
	await sidebar(other)
		.getByRole('button', { name: /^Starred items/ })
		.click();
	await expect(articles(other)).toHaveCount(1);
	await expect(articles(other).first()).toContainText('Harbour news 12');
	await elsewhere.close();
});
