// Logging in and out.

import { expect, test } from '@playwright/test';
import { PASSWORD, logIn, sidebar } from './harness/helpers.ts';

test('the login is all that somebody without a session gets', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByRole('heading', { name: 'Log in to rueder' })).toBeVisible();
	await expect(sidebar(page)).toHaveCount(0);
	// The API says no as well, and an image of the proxy too.
	expect((await page.request.get('/api/tree')).status()).toBe(401);
	expect((await page.request.get('/img/x/icon/y')).status()).toBe(401);
});

test('a wrong password is refused, the right one logs in, and the session holds', async ({
	page
}) => {
	await page.goto('/');
	await page.getByLabel('Name').fill('login');
	await page.getByLabel('Password').fill('not the password');
	await page.getByRole('button', { name: 'Log in', exact: true }).click();
	await expect(page.getByRole('alert')).toHaveText('Wrong name or password.');
	await expect(page.getByLabel('Password')).toHaveValue('');

	await page.getByLabel('Password').fill(PASSWORD);
	await page.getByLabel('Password').press('Enter');
	await expect(page.getByText('New in your subscriptions')).toBeVisible();

	// Scripts cannot read the session: the cookie is the browser's alone.
	const cookies = await page.context().cookies();
	expect(cookies.map((cookie) => cookie.name)).toContain('__Host-rueder');
	expect(cookies.find((cookie) => cookie.name === '__Host-rueder')).toMatchObject({
		httpOnly: true,
		secure: true,
		sameSite: 'Lax'
	});
	expect(await page.evaluate(() => document.cookie)).not.toContain('__Host-rueder');

	await page.reload();
	await expect(page.getByText('New in your subscriptions')).toBeVisible();
});

test('logging out ends the session', async ({ page }) => {
	await logIn(page, 'login');
	await page.getByRole('button', { name: 'Logged in as login' }).click();
	await page.getByRole('menuitem', { name: 'Log out' }).click();
	await expect(page.getByRole('heading', { name: 'Log in to rueder' })).toBeVisible();
	await page.reload();
	await expect(page.getByRole('heading', { name: 'Log in to rueder' })).toBeVisible();
	expect((await page.request.get('/api/me')).status()).toBe(401);
});
