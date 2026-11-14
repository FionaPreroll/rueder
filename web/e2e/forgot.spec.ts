// Forgot password (work package 5.5): the link mailed to the account's
// confirmed address sets a new password, every session ends, and the login
// goes on as before. The mail goes to the made-up relay (harness/mail.js).

import { expect, test } from '@playwright/test';
import { PASSWORD, linkIn, logIn, mailsTo, rueder, sidebar, sql } from './harness/helpers.ts';

const FRESH = 'a brand new password';
const ADDRESS = 'forgetful@mail.test';

test.beforeAll(() => {
	rueder('registration', 'set', '--email', 'optional');
	sql(`UPDATE accounts SET email = '${ADDRESS}' WHERE username = 'forgetful'`);
});

// The specs after this one see a rueder without addresses again.
test.afterAll(() => {
	rueder('registration', 'set', '--email', 'off');
});

test('a forgotten password is set anew with the link from the mail', async ({ page, browser }) => {
	// Another browser is logged in to the account all along.
	const kept = await (await browser.newContext()).newPage();
	await logIn(kept, 'forgetful');

	await page.goto('/');
	await page.getByRole('button', { name: 'Forgot your password?' }).click();
	const field = page.getByLabel('Name or email address');
	// Somebody nobody knows gets the same answer, and no mail goes anywhere.
	await field.fill('nobody-at-all');
	await page.getByRole('button', { name: 'Send the link' }).click();
	const asked = page.getByText(/If there is such an account with a confirmed address/);
	await expect(asked).toBeVisible();
	await field.fill('forgetful');
	await page.getByRole('button', { name: 'Send the link' }).click();
	await expect(asked).toBeVisible();

	const link = await linkIn(ADDRESS, 'reset');
	expect(link).toMatch(/^http:\/\/localhost:3210\/\?reset=[\w-]{43}$/);
	const [mail] = await mailsTo(ADDRESS);
	expect(mail.subject).toBe('Set a new password for rueder');
	expect(mail.body).toContain('30 minutes');

	await page.goto(link);
	await expect(page.getByRole('heading', { name: 'Set a new password' })).toBeVisible();
	// The token is no longer in the address, nor in the history.
	expect(new URL(page.url()).search).toBe('');
	await page.getByLabel('New password', { exact: true }).fill(FRESH);
	await page.getByLabel('New password again').fill(FRESH);
	await page.getByRole('button', { name: 'Set the password' }).click();
	await expect(page.getByText('The password is set. Log in with it.')).toBeVisible();

	// The address hears of it.
	await expect.poll(async () => (await mailsTo(ADDRESS)).length).toBe(2);
	expect((await mailsTo(ADDRESS))[1].subject).toBe('The password of your rueder account changed');

	// The old password opens nothing, the new one does.
	await page.getByLabel('Name').fill('forgetful');
	await page.getByLabel('Password').fill(PASSWORD);
	await page.getByRole('button', { name: 'Log in', exact: true }).click();
	await expect(page.getByText('Wrong name or password.')).toBeVisible();
	await page.getByLabel('Password').fill(FRESH);
	await page.getByRole('button', { name: 'Log in', exact: true }).click();
	await expect(sidebar(page)).toBeVisible();

	// The other browser is logged out.
	await kept.reload();
	await expect(kept.getByRole('heading', { name: 'Log in to rueder' })).toBeVisible();

	// The link works once.
	await kept.goto(link);
	await kept.getByLabel('New password', { exact: true }).fill('yet another password');
	await kept.getByLabel('New password again').fill('yet another password');
	await kept.getByRole('button', { name: 'Set the password' }).click();
	await expect(kept.getByText(/This link cannot be used any more/)).toBeVisible();
});
