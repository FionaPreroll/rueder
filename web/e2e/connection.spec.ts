// When the page cannot reach the server: the sign in the top bar says so, and
// once the network is back the page is live again by itself and loads what it
// could not meanwhile.

import { expect, test } from '@playwright/test';
import { articles, logIn, node, subscribe } from './harness/helpers.ts';

test('says when the server cannot be reached, and catches up once it can', async ({
	page,
	context
}) => {
	await logIn(page, 'cutoff');
	await subscribe(page, 'harbour-cutoff', 'Harbour Herald');
	await expect(node(page, 'Harbour Herald')).toContainText('(12)');

	await context.setOffline(true);
	const sign = page.getByRole('button', { name: /^No network since .+\. Try again now$/ });
	await expect(sign).toBeVisible();
	// What fails to load meanwhile is not a notice of its own: the sign says it.
	await node(page, 'Harbour Herald').click();
	await expect(page.getByRole('heading', { level: 1 })).toHaveText('Harbour Herald');
	await expect(articles(page)).toHaveCount(0);
	await expect(page.getByRole('alert')).toHaveCount(0);

	await context.setOffline(false);
	await expect(sign).toHaveCount(0);
	await expect(articles(page)).toHaveCount(12);
	await expect(page.getByRole('alert')).toHaveCount(0);
});
