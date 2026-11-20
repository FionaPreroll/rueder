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
	rueder,
	site,
	SITE,
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
	await expect(
		article(page, 'Wire news 3').getByRole('img', { name: /^Changed since it was first fetched/ })
	).toHaveCount(0);

	await site('revise', 'live', 'Correction: it was the fourth.');
	fetchSoon();
	await expect(
		article(page, 'Wire news 3').getByRole('img', { name: /^Changed since it was first fetched/ })
	).toBeVisible({
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

test('a headline that was rewritten says so, and the text is left alone', async ({ page }) => {
	await logIn(page, 'live');
	await node(page, 'Live Wire').click();
	await site('rename', 'live', 'Wire news 3, corrected');
	fetchSoon();
	const row = article(page, 'Wire news 3, corrected');
	await expect(row.getByRole('img', { name: /^Its headline was rewritten/ })).toBeVisible({
		timeout: AFTER_FETCH
	});
	await expect(row.getByRole('img', { name: /^Changed since/ })).toHaveCount(0);

	await row.getByRole('button', { name: /Wire news 3, corrected/ }).click();
	await page.getByRole('button', { name: 'Compare versions' }).click();
	const versions = page.getByRole('dialog', { name: 'Versions of this article' });
	// The first `del` is the old headline; the one after it is the legend.
	await expect(versions.locator('del').first()).toHaveText('Wire news 3');
	await expect(versions).toContainText('The text of this version is the same as the current one.');
});

test('a swapped picture shows in the compare, in the article info and in its source', async ({
	page
}) => {
	await logIn(page, 'live');
	await node(page, 'Live Wire').click();
	await site('picture', 'live', 'The ferry in port');
	fetchSoon();
	const row = article(page, 'Wire news 3, corrected');
	await expect(row.getByRole('img', { name: /^Changed since/ })).toBeVisible({
		timeout: AFTER_FETCH
	});

	await row.getByRole('button', { name: /Wire news 3, corrected/ }).click();
	await page.getByRole('button', { name: 'Compare versions' }).click();
	const versions = page.getByRole('dialog', { name: 'Versions of this article' });
	await expect(versions).toContainText('The words are the same; pictures or links changed.');
	const pictures = versions.getByRole('figure');
	await expect(pictures).toHaveCount(2);
	await expect(pictures.first()).toContainText('The ferry');
	await expect(pictures.last()).toContainText('The ferry in port');
	await expect(pictures.last().getByRole('img')).toHaveAttribute('src', /^\/img\//);
	// Where each came from, as the feed named it, with what changed marked.
	await expect(pictures.first()).toContainText(`${SITE}/pic.png`);
	await expect(pictures.last()).toContainText(`${SITE}/pic.png?in-port`);
	await expect(pictures.last().locator('mark')).toHaveText('?in-port');
	await versions.getByRole('button', { name: 'Close' }).last().click();

	await page.getByRole('button', { name: 'More actions for the article' }).click();
	await page.getByRole('menuitem', { name: 'Article info …' }).click();
	const info = page.getByRole('dialog', { name: 'Article info' });
	const line = (label: string) =>
		info.locator('dt', { hasText: new RegExp(`^${label}$`) }).locator('+ dd');
	await expect(line('Id in the feed')).toHaveText('live-3');
	await expect(line('Number in the feed')).toHaveText('3');
	await expect(line('Version')).toHaveText('4');
	await expect(line('Last changed')).toContainText('(the text)');

	// The operator sees the same, and every version as the feed sent it.
	const feed = (await line('Feed number').textContent()) ?? '';
	const shown = rueder('article', 'show', feed, '3', '--versions');
	expect(shown).toContain('Id:            live-3');
	expect(shown).toContain('Version:       4, 4 kept');
	expect(shown).toContain('alt="The ferry in port"');
	expect(shown).toMatch(/^Version 1, fetched .*: Wire news 3$/m);

	// Its source: as kept, with the picture's own address and without what the cleaning took out;
	// as the feed sent it, with all of that.
	await info.getByRole('button', { name: 'Close' }).last().click();
	await page.getByRole('button', { name: 'More actions for the article' }).click();
	await page.getByRole('menuitem', { name: 'Source …' }).click();
	const source = page.getByRole('dialog', { name: 'Source of the article' }).locator('pre');
	await expect(source).toContainText('alt="The ferry in port"');
	await expect(source).not.toContainText('/img/');
	await expect(source).not.toContainText('utm_source');
	await expect(source).not.toContainText('<script>');
	await page.locator('label', { hasText: 'As the feed sent it' }).click();
	await expect(source).toContainText('utm_source=feed');
	await expect(source).toContainText('<script>');
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
