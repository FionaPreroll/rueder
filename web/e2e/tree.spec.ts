// Folders: making one, sorting feeds into it, and that the server keeps the order.

import { expect, test } from '@playwright/test';
import { logIn, node, sidebar, subscribe } from './harness/helpers.ts';

test.describe.configure({ mode: 'serial' });

/** The names of the account's folders as the server has them. */
const foldersOnServer = async (page: import('@playwright/test').Page) => {
	const tree = await (await page.request.get('/api/tree')).json();
	return tree.folders.map((folder: { title: string }) => folder.title);
};

/** The titles in the sidebar's tree, top to bottom. */
const order = async (page: import('@playwright/test').Page) =>
	(await sidebar(page).locator('[data-node]').allTextContents()).map((text) =>
		text.replace(/\(.*\)/, '').trim()
	);

test('makes a folder and moves feeds into it, by keyboard and by dragging', async ({ page }) => {
	await logIn(page, 'tree');
	await subscribe(page, 'harbour-tree', 'Harbour Herald');
	await subscribe(page, 'market-tree', 'Market Courier');

	await page.getByRole('button', { name: 'New folder' }).click();
	await page.getByLabel('Name of the folder').fill('Waterfront');
	await page.keyboard.press('Enter');
	await expect(node(page, 'Waterfront')).toBeVisible();
	expect(await order(page)).toEqual(['Harbour Herald', 'Market Courier', 'Waterfront']);

	// Alt and the arrows: the folder goes to the top, a feed into it.
	await node(page, 'Waterfront').focus();
	await page.keyboard.press('Alt+ArrowUp');
	await page.keyboard.press('Alt+ArrowUp');
	await expect.poll(() => order(page)).toEqual(['Waterfront', 'Harbour Herald', 'Market Courier']);
	await node(page, 'Harbour Herald').focus();
	await page.keyboard.press('Alt+ArrowRight');
	// The other one is dragged onto the folder.
	await node(page, 'Market Courier').dragTo(node(page, 'Waterfront'));
	await expect.poll(() => order(page)).toEqual(['Waterfront', 'Harbour Herald', 'Market Courier']);

	// The folder counts what is in it, and shows it as one list.
	await expect(node(page, 'Waterfront')).toContainText('(16)');
	await node(page, 'Waterfront').click();
	await expect(page.getByRole('heading', { level: 1 })).toHaveText('Waterfront');
	await expect(page.locator('[data-article]')).toHaveCount(16);
});

test('finds the tree as it was left, renames the folder and takes it out again', async ({
	page
}) => {
	await logIn(page, 'tree');
	expect(await order(page)).toEqual(['Waterfront', 'Harbour Herald', 'Market Courier']);
	// Closed, a folder hides what is in it.
	await sidebar(page).getByRole('button', { name: 'Close folder' }).click();
	await expect(node(page, 'Harbour Herald')).toHaveCount(0);
	await sidebar(page).getByRole('button', { name: 'Open folder' }).click();

	await node(page, 'Waterfront').hover();
	await page.getByRole('button', { name: 'Actions for the folder Waterfront' }).click();
	await page.getByRole('menuitem', { name: 'Rename' }).click();
	await page.getByLabel('Name of the folder').fill('Quay');
	await page.keyboard.press('Enter');
	await expect(node(page, 'Quay')).toBeVisible();

	await expect.poll(() => foldersOnServer(page)).toEqual(['Quay']);
	await page.reload();
	await expect(node(page, 'Quay')).toBeVisible();
	await node(page, 'Quay').hover();
	await page.getByRole('button', { name: 'Actions for the folder Quay' }).click();
	await page.getByRole('menuitem', { name: 'Remove the folder, keep what is in it' }).click();
	await expect(node(page, 'Quay')).toHaveCount(0);
	await expect.poll(() => foldersOnServer(page)).toEqual([]);
	await page.reload();
	await expect.poll(() => order(page)).toEqual(['Harbour Herald', 'Market Courier']);
});
