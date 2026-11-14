// A reader reports a feed and an article, an admin sees the feed on the status
// page, and the operator blocks it on the command line and lifts the block.

import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, test, type Page } from '@playwright/test';
import { article, articles, logIn, node, rueder, SITE, subscribe } from './harness/helpers.ts';

const ADDRESS = `${SITE}/harbour-reporting/feed.xml`;

/** The command line runs beside the server; its image cache is one of its own here. */
process.env.IMAGE_CACHE_DIR = mkdtempSync(join(tmpdir(), 'rueder-e2e-block-'));

async function report(page: Page, reason: string, note: string) {
	await page.locator('label', { hasText: reason }).click();
	await expect(page.getByRole('radio', { name: reason })).toBeChecked();
	await page.getByLabel('What you would like to add (optional)').fill(note);
	await page.getByRole('button', { name: 'Send the report' }).click();
	await expect(page.getByRole('status')).toHaveText(
		'Thank you. The report reached the operator of this rueder.'
	);
	await page.getByRole('dialog').getByRole('button', { name: 'Close' }).last().click();
	await expect(page.getByRole('dialog')).toHaveCount(0);
}

async function reportedOnStatus(page: Page) {
	await page.getByRole('button', { name: 'Logged in as login' }).click();
	await page.getByRole('menuitem', { name: 'Server status' }).click();
	const dialog = page.getByRole('dialog', { name: 'Server status' });
	const section = dialog.locator('details', {
		has: page.getByRole('heading', { name: 'Reported feeds' })
	});
	return section.getByRole('listitem').filter({ hasText: ADDRESS });
}

test('a reported feed shows on the status page, and blocking hides it for everybody', async ({
	browser
}) => {
	const page = await (await browser.newContext()).newPage();
	await logIn(page, 'reporting');
	await subscribe(page, 'harbour-reporting', 'Harbour Herald');
	await node(page, 'Harbour Herald').click();
	await expect(articles(page)).toHaveCount(12);

	// The feed from its menu in the tree.
	await node(page, 'Harbour Herald').hover();
	await page.getByRole('button', { name: 'Actions for the feed Harbour Herald' }).click();
	await page.getByRole('menuitem', { name: 'Report the feed …' }).click();
	await expect(page.getByRole('dialog', { name: 'Report the feed' })).toContainText(
		'but not who reported it'
	);
	await report(page, 'Spam', 'Only ads');

	// An article from the menu under its text.
	await article(page, 'Harbour news 12')
		.getByRole('button', { name: /Harbour news 12/ })
		.click();
	await page.getByRole('button', { name: 'More actions for the article' }).click();
	await page.getByRole('menuitem', { name: 'Report the article …' }).click();
	await report(page, 'Somebody else’s work', 'Copied');

	// An admin sees the feed, its address and why, never who or what they wrote.
	const admin = await (await browser.newContext()).newPage();
	await logIn(admin, 'login');
	const reported = await reportedOnStatus(admin);
	await expect(reported).toContainText('Harbour Herald');
	await expect(reported).toContainText('2 reports');
	await expect(reported).toContainText('Spam 1');
	await expect(reported).toContainText('Somebody else’s work 1');
	// The account's name is in the feed's address here, so it is looked for beside it.
	await expect(reported).not.toContainText(/reporting(?!\/feed\.xml)/);
	await expect(reported).not.toContainText('Only ads');
	await expect(reported).not.toContainText('Copied');
	await expect(admin.getByText(/Readers reported \d+ feeds? that/)).toBeVisible();

	// Blocked: nobody sees its articles.
	expect(rueder('feed', 'block', ADDRESS)).toContain('Blocked');
	await page.reload();
	await node(page, 'Harbour Herald').click();
	await expect(page.getByText('The operator of this rueder blocked this feed.')).toBeVisible();
	await expect(articles(page)).toHaveCount(0);
	// Its list is empty, not missing: the page does not take it for a failure.
	await expect(page.getByText('The server does not answer.')).toHaveCount(0);
	await expect(node(page, 'Harbour Herald')).toContainText(
		'Blocked by the operator of this rueder'
	);
	await expect(node(page, 'Harbour Herald')).not.toContainText('(');
	await admin.reload();
	await expect(await reportedOnStatus(admin)).toContainText('Blocked');

	// Lifted: everything is back.
	expect(rueder('feed', 'unblock', ADDRESS)).toContain('not blocked any more');
	await page.reload();
	await node(page, 'Harbour Herald').click();
	await expect(articles(page)).toHaveCount(12);
	await expect(node(page, 'Harbour Herald')).not.toContainText('Blocked');
});
