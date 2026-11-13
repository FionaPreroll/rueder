// Moving an account: what one account read and starred goes out as a file and
// comes in to another, which finds the same feed by its address.

import { readFile } from 'node:fs/promises';
import { expect, test, type Page } from '@playwright/test';
import {
	accountSettings,
	article,
	articles,
	logIn,
	node,
	sidebar,
	subscribe
} from './harness/helpers.ts';

test.describe.configure({ mode: 'serial' });

const openMove = (page: Page) => accountSettings(page, 'Move');

let file = '';

test('gives out the account as a file', async ({ page }) => {
	await logIn(page, 'moving');
	await subscribe(page, 'harbour-move', 'Harbour Herald');
	await expect(node(page, 'Harbour Herald')).toContainText('(12)');
	await node(page, 'Harbour Herald').click();
	await article(page, 'Harbour news 12')
		.getByRole('button', { name: /Harbour news 12/ })
		.click();
	await page.keyboard.press('s');
	await expect(node(page, 'Harbour Herald')).toContainText('(11)');

	const dialog = await openMove(page);
	const download = page.waitForEvent('download');
	await dialog.getByRole('button', { name: 'Export the account' }).click();
	const saved = await download;
	expect(saved.suggestedFilename()).toMatch(/^rueder-\d{4}-\d{2}-\d{2}\.json$/);
	file = await readFile(await saved.path(), 'utf8');
	const account = JSON.parse(file);
	expect(account.format).toBe('rueder-account');
	expect(account.tree[0].articles[0]).toMatchObject({ read: true, starred: true });
});

test('brings the file into another account', async ({ page }) => {
	expect(file).not.toBe('');
	await logIn(page, 'moved');
	await expect(node(page, 'Harbour Herald')).toHaveCount(0);
	const dialog = await openMove(page);
	await dialog.getByLabel('Import an account file').setInputFiles({
		name: 'rueder.json',
		mimeType: 'application/json',
		buffer: Buffer.from(file)
	});
	await expect(dialog.getByRole('status')).toContainText('1 new feed, 0 subscribed already.');
	await expect(dialog.getByRole('status')).toContainText('1 article found here');
	await dialog.getByRole('button', { name: 'Close' }).click();

	// What was read and starred came along, and stays after a reload.
	await expect(node(page, 'Harbour Herald')).toContainText('(11)');
	await page.reload();
	await expect(node(page, 'Harbour Herald')).toContainText('(11)');
	await sidebar(page).getByRole('button', { name: 'Starred items' }).click();
	await expect(articles(page)).toHaveCount(1);
	await expect(articles(page).first()).toContainText('Harbour news 12');
});
