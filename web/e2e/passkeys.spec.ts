// Passkeys against the real server, with Chromium's virtual authenticator as the
// device: added in the account settings, logging in without name or password,
// standing in for the code after a password, and removed again. The device is
// one that keeps its keys and asks for its PIN, as a phone or a laptop does.

import { expect, test, type CDPSession, type Page } from '@playwright/test';
import { PASSWORD, accountSettings, logIn, sidebar } from './harness/helpers.ts';

/** Gives the browser a passkey device and returns a handle to look into it. */
async function device(page: Page) {
	const cdp = await page.context().newCDPSession(page);
	await cdp.send('WebAuthn.enable');
	const { authenticatorId } = await cdp.send('WebAuthn.addVirtualAuthenticator', {
		options: {
			protocol: 'ctap2',
			transport: 'internal',
			hasResidentKey: true,
			hasUserVerification: true,
			isUserVerified: true,
			automaticPresenceSimulation: true
		}
	});
	return { cdp, authenticatorId };
}

async function keys(cdp: CDPSession, authenticatorId: string) {
	const { credentials } = await cdp.send('WebAuthn.getCredentials', { authenticatorId });
	return credentials;
}

test('a passkey is added, logs in without name or password, and is removed', async ({ page }) => {
	const { cdp, authenticatorId } = await device(page);
	await logIn(page, 'keyholder');
	const dialog = await accountSettings(page, 'Security');
	await expect(dialog.getByText('No passkey yet.')).toBeVisible();
	await dialog.getByRole('button', { name: 'Add a passkey' }).click();
	await dialog.getByLabel('Name, such as the device').fill('Laptop');
	const adding = dialog.locator('form', { hasText: 'Create the passkey' });
	await adding.getByLabel('Your password').fill(PASSWORD);
	await dialog.getByRole('button', { name: 'Create the passkey' }).click();

	// The first passkey turns the second factor on and brings the recovery codes.
	const codes = dialog.locator('section', { hasText: 'Your recovery codes' }).locator('li');
	await expect(codes).toHaveCount(10);
	await dialog.getByRole('button', { name: 'I have kept them' }).click();
	await expect(dialog.getByText('Laptop')).toBeVisible();
	await expect(dialog.getByText(/Your passkeys keep the second factor on/)).toBeVisible();

	// The device keeps the key itself, for this site, with nothing but a random user handle.
	const [key] = await keys(cdp, authenticatorId);
	expect(key.isResidentCredential).toBe(true);
	expect(key.rpId).toBe('localhost');
	expect(Buffer.from(key.userHandle ?? '', 'base64').toString()).not.toContain('keyholder');

	// Logged out, the passkey alone logs in: no name, no password.
	await page.context().clearCookies();
	await page.goto('/');
	await page.getByRole('button', { name: 'Log in with a passkey' }).click();
	await expect(sidebar(page)).toBeVisible();
	await expect(page.getByRole('button', { name: 'Logged in as keyholder' })).toBeVisible();

	// The password alone no longer opens the account; the passkey does.
	await page.context().clearCookies();
	await page.goto('/');
	await page.getByLabel('Name').fill('keyholder');
	await page.getByLabel('Password').fill(PASSWORD);
	await page.getByRole('button', { name: 'Log in', exact: true }).click();
	await expect(page.getByRole('status')).toContainText('This account has a second factor.');
	await page.getByRole('button', { name: 'Log in with a passkey' }).click();
	await expect(sidebar(page)).toBeVisible();

	// Removing it: the passkey confirms it is the owner.
	await accountSettings(page, 'Security');
	await dialog.getByRole('button', { name: 'Remove the passkey Laptop' }).click();
	await dialog.getByRole('button', { name: 'Confirm with a passkey' }).click();
	await expect(dialog.getByText('No passkey yet.')).toBeVisible();
	await expect(dialog.getByText('Off.')).toBeVisible();

	// The device still has the key, but it opens nothing any more.
	expect(await keys(cdp, authenticatorId)).toHaveLength(1);
	await page.context().clearCookies();
	await page.goto('/');
	await page.getByRole('button', { name: 'Log in with a passkey' }).click();
	await expect(page.getByRole('alert')).toContainText('This passkey belongs to no account here.');
	// And the password alone is enough again.
	await logIn(page, 'keyholder');
});

test('a device that does not verify its user logs nobody in', async ({ page }) => {
	const { cdp, authenticatorId } = await device(page);
	await logIn(page, 'unverified');
	const dialog = await accountSettings(page, 'Security');
	await dialog.getByRole('button', { name: 'Add a passkey' }).click();
	await dialog
		.locator('form', { hasText: 'Create the passkey' })
		.getByLabel('Your password')
		.fill(PASSWORD);
	await dialog.getByRole('button', { name: 'Create the passkey' }).click();
	await dialog.getByRole('button', { name: 'I have kept them' }).click();
	await expect(dialog.getByText('Passkey', { exact: true })).toBeVisible();

	// The same device, without its PIN or biometrics now.
	await cdp.send('WebAuthn.setUserVerified', { authenticatorId, isUserVerified: false });
	await page.context().clearCookies();
	await page.goto('/');
	await page.getByRole('button', { name: 'Log in with a passkey' }).click();
	// The browser says no more than that no passkey was used, which the page takes as a hint.
	await expect(page.getByRole('status')).toContainText('No passkey was used.');
	await expect(sidebar(page)).toBeHidden();
	await page.reload();
	await expect(page.getByRole('button', { name: 'Log in with a passkey' })).toBeVisible();

	// With it again, it does.
	await cdp.send('WebAuthn.setUserVerified', { authenticatorId, isUserVerified: true });
	await page.getByRole('button', { name: 'Log in with a passkey' }).click();
	await expect(sidebar(page)).toBeVisible();
	await expect(page.getByRole('button', { name: 'Logged in as unverified' })).toBeVisible();
});
