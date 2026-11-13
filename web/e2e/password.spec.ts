// The account's own password, changed in the settings of the account: every
// other browser of the account is logged out, and the new password is the one
// that logs in from then on.

import { expect, test } from '@playwright/test';
import { accountSettings, logIn, sidebar } from './harness/helpers.ts';
import { PASSWORD } from './harness/ports.js';

// The second test logs in with the password the first one set.
test.describe.configure({ mode: 'serial' });

const FRESH = 'a longer secret than that';

test('a new password logs the other browsers out and logs in from then on', async ({ browser }) => {
	const context = await browser.newContext();
	const page = await context.newPage();
	await logIn(page, 'secrets');
	// A second browser of the same account, which this change logs out.
	const elsewhere = await browser.newContext();
	const other = await elsewhere.newPage();
	await logIn(other, 'secrets');

	const dialog = await accountSettings(page, 'Password');
	await dialog.getByLabel('Current password').fill(PASSWORD);
	await dialog.getByLabel('New password', { exact: true }).fill(FRESH);
	await dialog.getByLabel('New password again').fill(FRESH);
	await dialog.getByRole('button', { name: 'Change the password' }).click();
	await expect(dialog.getByRole('status')).toHaveText(
		'The password is changed. Every other browser and device is logged out, and app passwords are revoked.'
	);

	// This browser got a new session with the answer and reads on.
	await dialog.getByRole('button', { name: 'Close' }).click();
	await expect(sidebar(page)).toBeVisible();
	await page.reload();
	await expect(sidebar(page)).toBeVisible();

	// The other one is out, and the password it logged in with no longer works.
	await other.reload();
	await expect(other.getByRole('button', { name: 'Log in' })).toBeVisible();
	await other.getByLabel('Name').fill('secrets');
	await other.getByLabel('Password').fill(PASSWORD);
	await other.getByRole('button', { name: 'Log in' }).click();
	await expect(other.getByRole('alert')).toContainText('Wrong name or password.');
	await other.getByLabel('Password').fill(FRESH);
	await other.getByRole('button', { name: 'Log in' }).click();
	await expect(sidebar(other)).toBeVisible();

	await context.close();
	await elsewhere.close();
});

test('the one so far has to be right', async ({ page }) => {
	await logIn(page, 'secrets', FRESH);
	const dialog = await accountSettings(page, 'Password');
	await dialog.getByLabel('Current password').fill('not it at all');
	await dialog.getByLabel('New password', { exact: true }).fill('something else entirely');
	await dialog.getByLabel('New password again').fill('something else entirely');
	await dialog.getByRole('button', { name: 'Change the password' }).click();
	await expect(dialog.getByRole('alert')).toHaveText('The current password is not right.');
});
