// The search bar asks the server: in a feed, and in all articles from the
// start page. That another account finds nothing is tested in the server's
// own tests (`server/tests/api_search.rs`).

import { expect, test } from '@playwright/test';
import { article, articles, logIn, node, sidebar, subscribe } from './harness/helpers.ts';

test.describe.configure({ mode: 'serial' });

const searchBar = (page: import('@playwright/test').Page) =>
	page.getByPlaceholder('Search these articles');

test('finds the words of the text in a feed and in all articles', async ({ page }) => {
	await logIn(page, 'search');
	await subscribe(page, 'harbour-search', 'Harbour Herald');
	await subscribe(page, 'market-search', 'Market Courier');

	// In a feed: a word of the text, not of the title, and a beginning of one.
	await node(page, 'Harbour Herald').click();
	await expect(articles(page)).toHaveCount(12);
	const asked = page.waitForRequest((request) => request.url().includes('/api/search?'));
	await searchBar(page).fill('ferr deern');
	expect(new URL((await asked).url()).searchParams.get('feed')).not.toBeNull();
	await expect(articles(page)).toHaveCount(12);
	await searchBar(page).fill('trip 7');
	await expect(articles(page)).toHaveCount(1);
	await expect(articles(page).first()).toContainText('Harbour news 7');

	// From the start page the search goes through all articles.
	await sidebar(page).getByText('Home', { exact: true }).click();
	await searchBar(page).fill('fish');
	await expect(articles(page)).toHaveCount(1);
	await expect(article(page, 'Market news 3')).toBeVisible();
	await searchBar(page).fill('zitronenfalter');
	await expect(page.getByText('No article matches the search.')).toBeVisible();
});
