// A reader looks up what the server knows of a feed, for a message to the
// operator; the operator finds the same on the command line and says as what
// the feed is fetched, and so does an admin who reads it, in the same window,
// where the admin also says how often it is fetched at most, as the operator
// does on the command line.

import { expect, test, type Locator, type Page } from '@playwright/test';
import { logIn, node, rueder, SITE, subscribe } from './harness/helpers.ts';

const ADDRESS = `${SITE}/harbour-info/feed.xml`;

async function feedInfo(page: Page) {
	await node(page, 'Harbour Herald').hover();
	await page.getByRole('button', { name: 'Actions for the feed Harbour Herald' }).click();
	await page.getByRole('menuitem', { name: 'Feed info …' }).click();
	const dialog = page.getByRole('dialog', { name: 'Feed info' });
	await expect(dialog).toContainText(ADDRESS);
	return dialog;
}

/** The value next to a label of the window's list. */
const line = (dialog: Locator, label: string) =>
	dialog.locator('dt', { hasText: new RegExp(`^${label}$`) }).locator('+ dd');

test('a reader sees how a feed is fetched, and only the operator and admins change it', async ({
	browser
}) => {
	const page = await (await browser.newContext()).newPage();
	await logIn(page, 'support');
	await subscribe(page, 'harbour-info', 'Harbour Herald');
	let dialog = await feedInfo(page);
	await expect(line(dialog, 'Fetched as')).toHaveText('rueder');
	await expect(line(dialog, 'Articles so far')).toHaveText('12');
	await expect(line(dialog, 'Last error')).toHaveText('Never');
	await expect(dialog.getByRole('radio')).toHaveCount(0);
	const number = (await line(dialog, 'Number').textContent()) ?? '';

	// The operator sees the same by its number, and how many read it, but not who.
	const shown = rueder('feed', 'show', number);
	expect(shown).toContain(`Address:      ${ADDRESS}`);
	expect(shown).toContain('Fetched as:   rueder');
	expect(shown).toContain('Readers:      1');
	expect(shown).not.toContain('support');
	expect(rueder('feed', 'agent', ADDRESS, 'browser')).toContain('is fetched as browser');
	await dialog.getByRole('button', { name: 'Close' }).last().click();
	dialog = await feedInfo(page);
	await expect(line(dialog, 'Fetched as')).toHaveText('Browser');

	// An admin who reads the feed says it in the window.
	rueder('admin', 'add', 'helper');
	const admin = await (await browser.newContext()).newPage();
	await logIn(admin, 'helper');
	await subscribe(admin, 'harbour-info', 'Harbour Herald');
	const adminDialog = await feedInfo(admin);
	await adminDialog.locator('label', { hasText: 'rueder' }).click();
	await expect(line(adminDialog, 'Fetched as')).toHaveText('rueder');
	expect(rueder('feed', 'show', ADDRESS)).toContain('Fetched as:   rueder');
	expect(rueder('feed', 'show', ADDRESS)).toContain('Readers:      2');
	// And how often it is fetched at most.
	await expect(line(adminDialog, 'At most every')).toHaveText('No limit');
	await adminDialog.locator('label', { hasText: '4 hr' }).click();
	await expect(line(adminDialog, 'At most every')).toHaveText('4 hr');
	expect(rueder('feed', 'show', ADDRESS)).toContain('At least:     14400 s');
	// The operator takes it back for any feed.
	expect(rueder('feed', 'min-interval', ADDRESS, 'none')).toContain('as often as it publishes');
	await adminDialog.getByRole('button', { name: 'Close' }).last().click();
	await expect(line(await feedInfo(admin), 'At most every')).toHaveText('No limit');
});
