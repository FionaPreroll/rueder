// Accounts that make themselves: invitations, open registration, and the limits
// the operator sets on the command line.

import { expect, test, type Browser, type Page } from '@playwright/test';
import { PASSWORD, accountSettings, logIn, rueder, sidebar } from './harness/helpers.ts';

test.describe.configure({ mode: 'serial' });

// Whatever a test opened is closed again for the specs after it.
test.afterAll(() => {
	rueder(
		'registration',
		'set',
		...['--open', 'false', '--max-accounts', 'none', '--per-day', 'none', '--invites', '0']
	);
});

/** Somebody who is not logged in, in a browser of their own. */
const stranger = async (browser: Browser) => (await browser.newContext()).newPage();

/** Makes a link in the settings and returns it. */
async function makeLink(page: Page) {
	const dialog = await accountSettings(page, 'Invitations');
	await dialog.getByRole('button', { name: 'Make a link' }).click();
	const link = await dialog.getByLabel('The link, to pass on now').inputValue();
	expect(link).toMatch(/^http:\/\/localhost:3210\/\?invite=[\w-]{43}$/);
	return link;
}

async function fillIn(page: Page, name: string) {
	await page.getByLabel('Name').fill(name);
	await page.getByLabel('Password', { exact: true }).fill(PASSWORD);
	await page.getByLabel('The password once more').fill(PASSWORD);
	await page.getByRole('button', { name: 'Make the account' }).click();
}

test('nobody registers until the operator opens something', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByText('There is no sign-up.')).toBeVisible();
	await expect(page.getByRole('button', { name: 'Make an account' })).toHaveCount(0);
	const refused = await page.request.post('/api/register', {
		data: { username: 'eve', password: PASSWORD }
	});
	expect([refused.status(), await refused.text()]).toEqual([403, 'closed']);

	// A reader has no invitations to give either.
	await logIn(page, 'inviter');
	await accountSettings(page, 'Invitations');
	await expect(page.getByText('You have no invitations to give.')).toBeVisible();
	await expect(page.getByRole('button', { name: 'Make a link' })).toHaveCount(0);
});

test('an admin invites with a link that makes one account', async ({ page, browser }) => {
	await logIn(page, 'login');
	const link = await makeLink(page);
	await expect(page.getByText('As an admin you can always invite.')).toBeVisible();

	const guest = await stranger(browser);
	await guest.goto(link);
	await expect(guest.getByText('You were invited to this rueder.')).toBeVisible();
	await fillIn(guest, 'invited-by-admin');
	// The new account is logged in, with an empty rueder of its own.
	await expect(sidebar(guest)).toBeVisible();
	await expect(guest.getByRole('button', { name: 'Logged in as invited-by-admin' })).toBeVisible();
	expect(new URL(guest.url()).search).toBe('');

	// The link is used up.
	const late = await stranger(browser);
	await late.goto(link);
	await expect(late.getByRole('alert')).toHaveText(
		'This invitation cannot be used any more. Ask for a new one.'
	);
	await expect(late.getByRole('button', { name: 'Make an account' })).toHaveCount(0);
});

test('a reader invites as many as the operator gave them', async ({ page, browser }) => {
	rueder('account', 'invites', 'inviter', '1');
	await logIn(page, 'inviter');
	const link = await makeLink(page);
	await expect(page.getByText('You have no invitations to give.')).toBeVisible();

	// The limits count for readers' invitations: with no room, nobody gets in.
	rueder('registration', 'set', '--max-accounts', '1');
	const guest = await stranger(browser);
	await guest.goto(link);
	await fillIn(guest, 'invited-by-reader');
	await expect(guest.getByRole('alert')).toHaveText('There is no room for another account here.');

	rueder('registration', 'set', '--max-accounts', 'none');
	await guest.getByRole('button', { name: 'Make the account' }).click();
	await expect(guest.getByRole('button', { name: 'Logged in as invited-by-reader' })).toBeVisible();
});

test('anybody registers once the operator opens it, up to the day’s limit', async ({ browser }) => {
	// One account today already came by a reader's invitation.
	rueder('registration', 'set', '--open', 'true', '--per-day', '2');
	const first = await stranger(browser);
	await first.goto('/');
	await first.getByRole('button', { name: 'Make an account' }).click();
	await fillIn(first, 'walk-in');
	await expect(first.getByRole('button', { name: 'Logged in as walk-in' })).toBeVisible();

	// The day is full: the login no longer offers it.
	const second = await stranger(browser);
	await second.goto('/');
	await expect(second.getByRole('heading', { name: 'Log in to rueder' })).toBeVisible();
	await expect(second.getByText('There is no sign-up.')).toBeVisible();
	const refused = await second.request.post('/api/register', {
		data: { username: 'too-late', password: PASSWORD }
	});
	expect([refused.status(), await refused.text()]).toEqual([403, 'today']);

	// The new account logs in like any other.
	await logIn(second, 'walk-in');
});
