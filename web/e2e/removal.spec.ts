// Deleting the own account in the settings, and an account the operator
// disabled on the command line.

import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { expect, test, type Page } from '@playwright/test';
import { database, PASSWORD } from './harness/ports.js';
import { accountSettings, logIn, node, sidebar, subscribe } from './harness/helpers.ts';

/** Runs `rueder` on the command line against the database of the run. */
function rueder(...args: string[]) {
	const binary = resolve(
		import.meta.dirname,
		'..',
		process.env.RUEDER_BIN ?? '../server/target/debug/rueder'
	);
	execFileSync(binary, args, {
		env: { ...process.env, DATABASE_URL: database() },
		stdio: ['ignore', 'ignore', 'inherit']
	});
}

const openAccount = (page: Page) => accountSettings(page, 'Delete account');

const tryLogIn = async (page: Page, name: string, password: string) => {
	await page.goto('/');
	await page.getByLabel('Name').fill(name);
	await page.getByLabel('Password').fill(password);
	await page.getByRole('button', { name: 'Log in' }).click();
};

test('an account deletes itself, and its other tabs are logged out', async ({ browser }) => {
	const context = await browser.newContext();
	const page = await context.newPage();
	await logIn(page, 'leaving');
	await subscribe(page, 'harbour-leaving', 'Harbour Herald');
	await expect(node(page, 'Harbour Herald')).toBeVisible();
	const other = await context.newPage();
	await other.goto('/');
	await expect(sidebar(other)).toBeVisible();

	const dialog = await openAccount(page);
	await expect(dialog.getByText(/Feeds stay for the others/)).toBeVisible();
	await dialog.getByLabel('Your password').fill('not the password');
	await dialog.getByRole('button', { name: 'Delete my account for good' }).click();
	await expect(dialog.getByRole('alert')).toHaveText('That is not right. Nothing was deleted.');

	await dialog.getByLabel('Your password').fill(PASSWORD);
	await dialog.getByRole('button', { name: 'Delete my account for good' }).click();
	await expect(page.getByRole('status')).toHaveText(
		'Your account is deleted, with everything that was yours.'
	);
	await expect(page.getByRole('button', { name: 'Log in' })).toBeVisible();
	// The other tab hears of it over its live connection and shows the login.
	await expect(other.getByRole('button', { name: 'Log in' })).toBeVisible();
	expect((await other.request.get('/api/me')).status()).toBe(401);

	await tryLogIn(page, 'leaving', PASSWORD);
	await expect(page.getByRole('alert')).toHaveText('Wrong name or password.');
	await context.close();
});

test('a disabled account is told so after the right password', async ({ page }) => {
	await logIn(page, 'barred');
	rueder('account', 'disable', 'barred');
	await page.reload();
	await expect(page.getByRole('button', { name: 'Log in' })).toBeVisible();

	await tryLogIn(page, 'barred', 'not the password');
	await expect(page.getByRole('alert')).toHaveText('Wrong name or password.');
	await tryLogIn(page, 'barred', PASSWORD);
	await expect(page.getByRole('alert')).toHaveText(
		'This account is disabled. Ask whoever runs this rueder why.'
	);

	rueder('account', 'enable', 'barred');
	await logIn(page, 'barred');
});
