// An article whose feed tags it with many and long categories: the tags wrap
// below it in every view, on a wide screen and on a phone, and nothing scrolls
// sideways.

import { expect, test, type Page } from '@playwright/test';
import { articles, logIn, node, subscribe } from './harness/helpers.ts';

/** The views by their button, and whether they show the article without opening it. */
const views = [
	{ name: 'Expanded view', shown: true },
	{ name: 'List view', shown: false },
	{ name: 'Three panes: tree, list and article', shown: false },
	{ name: 'Magazine', shown: true }
];

/** What can be scrolled sideways, and by how far: the page and every pane of it. */
const sideways = (page: Page) =>
	page.evaluate(() =>
		[document.documentElement, ...document.querySelectorAll('*')]
			.filter(
				(element) =>
					element === document.documentElement ||
					['auto', 'scroll'].includes(getComputedStyle(element).overflowX)
			)
			.filter((element) => element.scrollWidth > element.clientWidth)
			.map((element) => `${element.tagName} by ${element.scrollWidth - element.clientWidth}px`)
	);

test('keeps a long list of tags within the article', async ({ page }) => {
	await page.setViewportSize({ width: 1280, height: 800 });
	await logIn(page, 'tags');
	await subscribe(page, 'tagged', 'Tagged Times');
	await node(page, 'Tagged Times').click();
	await expect(articles(page)).toHaveCount(1);
	const lastTag = page.getByText('Donaudampfschifffahrtsgesellschaftskapitänsmützenabzeichen');

	for (const { name: view, shown } of views) {
		await page.setViewportSize({ width: 1280, height: 800 });
		const button = page.getByRole('button', { name: view, exact: true });
		await button.click();
		await expect(button).toHaveAttribute('aria-pressed', 'true');
		if (!shown) {
			await articles(page)
				.first()
				.getByRole('button', { name: /Tagged news 1/ })
				.click();
		}
		await expect(lastTag, view).toBeVisible();
		expect(await sideways(page), view).toEqual([]);

		await page.setViewportSize({ width: 360, height: 800 });
		await expect(lastTag, `${view} on a phone`).toBeVisible();
		expect(await sideways(page), `${view} on a phone`).toEqual([]);
	}
});
