// The second factor against the real server: set up with a code an app would
// show, then needed at login, where a recovery code counts once.

import { createHmac } from 'node:crypto';
import { expect, test, type Page } from '@playwright/test';
import { PASSWORD, logIn, sidebar } from './harness/helpers.ts';

/** The bytes of a key in base32, as the page shows it in groups. */
function fromBase32(text: string): Buffer {
	const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
	let bits = 0;
	let value = 0;
	const bytes: number[] = [];
	for (const char of text.replace(/\s/g, '')) {
		value = (value << 5) | alphabet.indexOf(char);
		bits += 5;
		if (bits >= 8) {
			bits -= 8;
			bytes.push((value >>> bits) & 0xff);
		}
	}
	return Buffer.from(bytes);
}

/** The code an authenticator app shows now, after RFC 6238. */
function totp(key: Buffer, at = Date.now()): string {
	const step = Buffer.alloc(8);
	step.writeBigUInt64BE(BigInt(Math.floor(at / 1000 / 30)));
	const hash = createHmac('sha1', key).update(step).digest();
	const offset = hash[hash.length - 1] & 0x0f;
	return String((hash.readUInt32BE(offset) & 0x7fffffff) % 1_000_000).padStart(6, '0');
}

async function logInWith(page: Page, code: string) {
	await page.goto('/');
	await page.getByLabel('Name').fill('guarded');
	await page.getByLabel('Password').fill(PASSWORD);
	await page.getByRole('button', { name: 'Log in' }).click();
	await expect(page.getByRole('alert')).toContainText('This account has a second factor.');
	await page.getByLabel('Code').fill(code);
	await page.getByRole('button', { name: 'Log in' }).click();
}

test('a second factor is set up and then needed to log in', async ({ page }) => {
	await logIn(page, 'guarded');
	await page.getByRole('button', { name: 'Settings' }).click();
	const dialog = page.getByRole('dialog', { name: 'Settings' });
	await dialog.getByRole('tab', { name: 'Security' }).click();
	await expect(dialog.getByText('Off.')).toBeVisible();
	await dialog.getByLabel('Your password').fill(PASSWORD);
	await dialog.getByRole('button', { name: 'Set up' }).click();
	await expect(
		dialog.getByRole('img', { name: 'QR code for the authenticator app' })
	).toBeVisible();
	const key = fromBase32(await dialog.locator('code').innerText());
	expect(key).toHaveLength(20);
	await dialog.getByLabel('The code the app shows now').fill(totp(key));
	await dialog.getByRole('button', { name: 'Confirm' }).click();
	const codes = dialog.locator('li');
	await expect(codes).toHaveCount(10);
	const recovery = await codes.first().innerText();
	await dialog.getByRole('button', { name: 'I have kept them' }).click();
	await expect(dialog.getByText('10 recovery codes left.')).toBeVisible();

	// After logging out, the password alone is not enough.
	await page.context().clearCookies();
	await logInWith(page, recovery);
	await expect(sidebar(page)).toBeVisible();
	// A recovery code counts once.
	await page.context().clearCookies();
	await logInWith(page, recovery);
	await expect(page.getByRole('alert')).toContainText('That code is not right.');
	// The code of the next step is one the server has not taken yet.
	await page.getByLabel('Code').fill(totp(key, Date.now() + 30_000));
	await page.getByRole('button', { name: 'Log in' }).click();
	await expect(sidebar(page)).toBeVisible();
});
