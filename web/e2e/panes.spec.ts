// The view of three panes: how wide the panes are, and where the open article was
// scrolled to, both across a reload.

import { expect, test, type Locator, type Page } from '@playwright/test';
import { articles, logIn, node, sidebar, subscribe } from './harness/helpers.ts';

test.describe.configure({ mode: 'serial' });
// Room for three panes, even once the sidebar has been pulled wider: below 880
// pixels beside the sidebar the list takes the whole pane by itself.
test.use({ viewport: { width: 1600, height: 900 } });

const pane = (page: Page) => page.locator('[data-pane="list"]');
/** What the open article scrolls in, beside the list. */
const reading = (page: Page) => page.locator('section:has([data-pane-article])');

const widthOf = (where: Locator) =>
	where.evaluate((element) => element.getBoundingClientRect().width);

/** Drags a bar this many pixels to the right. */
async function pull(page: Page, bar: Locator, by: number) {
	const box = (await bar.boundingBox())!;
	const [x, y] = [box.x + box.width / 2, box.y + box.height / 2];
	await page.mouse.move(x, y);
	await page.mouse.down();
	await page.mouse.move(x + by, y, { steps: 8 });
	await page.mouse.up();
}

/** Opens the feed in the view of three panes, with an article open. */
async function open(page: Page) {
	await node(page, 'Long Story').click();
	await expect(articles(page)).toHaveCount(4);
	await page.keyboard.press('3');
	// The view belongs to the account and reaches the server after a moment. A
	// reload before that would find the old one and go back to the plain list.
	await expect
		.poll(async () => (await (await page.request.get('/api/settings')).json()).view)
		.toBe('split');
	await articles(page)
		.first()
		.getByRole('button', { name: /Story news 4/ })
		.click();
	await expect(reading(page).getByText('Paragraph 1 of story 4.')).toBeVisible();
}

test('keeps where the open article was scrolled to over a reload', async ({ page }) => {
	await logIn(page, 'panes');
	await subscribe(page, 'long-panes', 'Long Story');
	await open(page);

	// Deep into the article, which is longer than the pane.
	await reading(page).evaluate((element) => element.scrollTo({ top: 1500 }));
	const top = await reading(page).evaluate((element) => element.scrollTop);
	expect(top).toBeGreaterThan(1000);

	await page.reload();
	await expect(reading(page).getByText('Paragraph 1 of story 4.')).toBeVisible();
	await expect
		.poll(() => reading(page).evaluate((element) => element.scrollTop))
		.toBeGreaterThan(top - 100);
});

test('starts another article at its top, and the one before it where it was', async ({ page }) => {
	await logIn(page, 'panes');
	await open(page);
	await reading(page).evaluate((element) => element.scrollTo({ top: 1200 }));
	await expect.poll(() => reading(page).evaluate((element) => element.scrollTop)).toBe(1200);

	// Another article begins at its top, not where the last one was left.
	await articles(page)
		.nth(1)
		.getByRole('button', { name: /Story news 3/ })
		.click();
	await expect(reading(page).getByText('Paragraph 1 of story 3.')).toBeVisible();
	expect(await reading(page).evaluate((element) => element.scrollTop)).toBe(0);
});

test('keeps how wide the sidebar and the list are, on this device', async ({ page }) => {
	await logIn(page, 'panes');
	await open(page);
	const bars = page.getByRole('separator');
	const [sidebarBar, listBar] = [bars.first(), bars.last()];
	const wasSidebar = await widthOf(sidebar(page));
	const wasList = await widthOf(pane(page));

	// Dragged, the panes follow the pointer.
	await pull(page, sidebarBar, 120);
	expect(await widthOf(sidebar(page))).toBe(wasSidebar + 120);
	await pull(page, listBar, 90);
	expect(await widthOf(pane(page))).toBe(wasList + 90);

	// The arrow keys move the bar too, and a reload finds both as they were left.
	await sidebarBar.press('ArrowLeft');
	await expect.poll(() => widthOf(sidebar(page))).toBe(wasSidebar + 104);
	await page.reload();
	await expect(sidebar(page)).toBeVisible();
	expect(await widthOf(sidebar(page))).toBe(wasSidebar + 104);
	expect(await widthOf(pane(page))).toBe(wasList + 90);

	// A double click puts a pane back to the width it started at.
	await sidebarBar.dblclick();
	await expect.poll(() => widthOf(sidebar(page))).toBe(wasSidebar);
});

test('moves the list at once in a window narrower than when it was pulled wide', async ({
	page
}) => {
	await logIn(page, 'panes');
	await open(page);
	const listBar = page.getByRole('separator').last();
	await listBar.press('End');
	await expect.poll(() => widthOf(pane(page))).toBe(720);

	// Narrower, the list takes what is left beside the article, not the 720 it was left at.
	await page.setViewportSize({ width: 1300, height: 900 });
	const room = await page.locator('main').evaluate((element) => element.clientWidth);
	const seen = Math.floor(room * 0.6);
	await expect.poll(() => widthOf(pane(page))).toBeLessThan(720);
	expect(Math.abs((await widthOf(pane(page))) - seen)).toBeLessThan(1);

	// The bar starts from what is seen, so pulling it and its keys move the list at once.
	await pull(page, listBar, -100);
	await expect.poll(() => widthOf(pane(page))).toBe(seen - 100);
	await listBar.press('ArrowLeft');
	await expect.poll(() => widthOf(pane(page))).toBe(seen - 116);
	await expect(listBar).toHaveAttribute('aria-valuemax', String(seen));
});
