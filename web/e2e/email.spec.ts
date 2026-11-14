// The address of an account (work package 5.4): registering where every
// account needs one, the link from the mail, and an account from before that
// is asked for its address. The mail goes to the made-up relay (harness/mail.js).

import { expect, test } from '@playwright/test';
import { PASSWORD, accountSettings, linkIn, mailsTo, rueder, sidebar } from './harness/helpers.ts';

test.describe.configure({ mode: 'serial' });

test.beforeAll(() => {
	rueder('registration', 'set', '--open', 'true', '--email', 'required');
});

// The specs after this one see a rueder without addresses again, and the
// account made here does not count against their limits for the day.
test.afterAll(() => {
	rueder('registration', 'set', '--open', 'false', '--email', 'off');
	rueder('account', 'remove', 'newcomer');
});

test('a new account logs in once the link from its mail is opened', async ({ page }) => {
	await page.goto('/');
	await page.getByRole('button', { name: 'Make an account' }).click();
	await page.getByLabel('Name').fill('newcomer');
	await page.getByLabel('Password', { exact: true }).fill(PASSWORD);
	await page.getByLabel('The password once more').fill(PASSWORD);
	await page.getByLabel('Email address').fill('newcomer@mail.test');
	await page.getByRole('button', { name: 'Make the account' }).click();
	await expect(page.getByText('Check your mail')).toBeVisible();
	await expect(page.getByText(/A link went to newcomer@mail\.test/)).toBeVisible();

	// Not before the link is opened.
	await page.getByRole('button', { name: 'Log in', exact: true }).click();
	await expect(page.getByText(/waits for its email address to be confirmed/)).toBeVisible();
	await page.getByRole('button', { name: 'Send the link again' }).click();
	await expect(page.getByText(/The link is on its way again/)).toBeVisible();
	await expect.poll(async () => (await mailsTo('newcomer@mail.test')).length).toBe(2);
	const [first] = await mailsTo('newcomer@mail.test');
	expect(first.subject).toBe('Confirm your address for rueder');
	expect(first.body).toContain('newcomer');

	// The newest link is the one that counts.
	const link = await linkIn('newcomer@mail.test');
	expect(link).toMatch(/^http:\/\/localhost:3210\/\?confirm=[\w-]{43}$/);
	await page.goto(link);
	await expect(page.getByText('The address newcomer@mail.test is confirmed.')).toBeVisible();
	await expect(page.getByText('The account is ready: log in.')).toBeVisible();
	// Said once: the address of the page no longer carries the link.
	expect(new URL(page.url()).search).toBe('');
	await page.getByRole('button', { name: 'Close' }).click();

	await page.getByLabel('Name').fill('newcomer');
	await page.getByLabel('Password').fill(PASSWORD);
	await page.getByRole('button', { name: 'Log in', exact: true }).click();
	await expect(sidebar(page)).toBeVisible();

	// The link is good for once only.
	await page.goto(link);
	await expect(page.getByText(/This link cannot be used any more/)).toBeVisible();
});

test('an account from before is asked for its address after logging in', async ({ page }) => {
	await page.goto('/');
	await page.getByLabel('Name').fill('addressee');
	await page.getByLabel('Password').fill(PASSWORD);
	await page.getByRole('button', { name: 'Log in', exact: true }).click();
	const asked = page.getByRole('alertdialog', { name: 'Add an email address' });
	await expect(asked).toBeVisible();
	await asked.getByLabel('New address').fill('addressee@mail.test');
	await asked.getByLabel('Your password').fill(PASSWORD);
	await asked.getByRole('button', { name: 'Send the link' }).click();
	await expect(asked.getByText(/A link went to addressee@mail\.test/)).toBeVisible();

	// The link, opened in another tab, lets this one go on.
	const other = await page.context().newPage();
	await other.goto(await linkIn('addressee@mail.test'));
	await expect(other.getByText('The address addressee@mail.test is confirmed.')).toBeVisible();
	await expect(asked).toBeHidden();
	await expect(sidebar(page)).toBeVisible();

	const settings = await accountSettings(page, 'Email');
	await expect(settings.getByText('addressee@mail.test')).toBeVisible();
	// Every account here needs one: it can be changed, not removed.
	await expect(settings.getByRole('button', { name: 'Remove the address' })).toHaveCount(0);
});
