// An app of another maker, set up in the settings and then driven the way
// NetNewsWire does it, against the real server: logging in with the app
// password shown, reading, marking as read, images, and revoking it.

import { expect, test } from '@playwright/test';
import { accountSettings, logIn, node, PASSWORD, subscribe } from './harness/helpers.ts';

test('an app is set up in the settings, syncs, and stops when revoked', async ({ page }) => {
	await logIn(page, 'apps');
	await subscribe(page, 'harbour-apps', 'Harbour Herald');
	await expect(node(page, 'Harbour Herald')).toContainText('(12)');

	const dialog = await accountSettings(page, 'Apps');
	await dialog.getByRole('button', { name: 'Set up an app' }).click();
	await expect(dialog.getByRole('link', { name: 'Website of NetNewsWire' })).toHaveAttribute(
		'href',
		'https://netnewswire.com/'
	);
	await dialog.getByRole('button', { name: 'NetNewsWire' }).click();
	await dialog.getByLabel('Your password').fill(PASSWORD);
	await dialog.getByRole('button', { name: 'Create the app password' }).click();
	await expect(dialog.getByText(/choose FreshRSS/)).toBeVisible();
	await expect(dialog.getByRole('status')).toHaveText('Waiting for the app to log in …');

	// What the page shows is what the app gets.
	const origin = new URL(page.url()).origin;
	const address = `${origin}/greader`;
	await expect(dialog.getByText(address, { exact: true })).toBeVisible();
	const secret = (await dialog.locator('code').nth(2).innerText()).trim();
	expect(secret).toMatch(/^[a-z2-7]{4}(-[a-z2-7]{4}){5}$/);

	// The app logs in: no cookie, only the name and the app password.
	const login = await fetch(`${address}/accounts/ClientLogin`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
		body: new URLSearchParams({ Email: 'apps', Passwd: secret })
	});
	expect(login.status).toBe(200);
	const auth = (await login.text())
		.split('\n')
		.find((line) => line.startsWith('Auth='))!
		.slice('Auth='.length);
	await expect(dialog.getByRole('status')).toHaveText('Connected: the app has logged in.');
	const headers = { Authorization: `GoogleLogin auth=${auth}` };
	const api = `${address}/reader/api/0`;

	const unread = await fetch(
		`${api}/stream/items/ids?output=json&n=1000&s=user/-/state/com.google/reading-list&xt=user/-/state/com.google/read`,
		{ headers }
	).then((answer) => answer.json());
	const ids: string[] = unread.itemRefs.map((item: { id: string }) => item.id);
	expect(ids).toHaveLength(12);

	// Its texts, with the image through the proxy, for this app password.
	const contents = await fetch(`${api}/stream/items/contents`, {
		method: 'POST',
		headers: { ...headers, 'Content-Type': 'application/x-www-form-urlencoded' },
		body: `output=json&i=${ids[0]}`
	}).then((answer) => answer.json());
	const html: string = contents.items[0].summary.content;
	// Links go to the article's site; images never do.
	const sources = [...html.matchAll(/src="([^"]+)"/g)].map((found) => found[1]!);
	expect(sources).toHaveLength(1);
	const image = sources[0]!;
	expect(image.startsWith(`${address}/img/`)).toBe(true);
	const picture = await fetch(image);
	expect(picture.status).toBe(200);
	expect(picture.headers.get('content-type')).toBe('image/png');

	// Read in the app: the page counts down by itself.
	const read = await fetch(`${api}/edit-tag`, {
		method: 'POST',
		headers: { ...headers, 'Content-Type': 'application/x-www-form-urlencoded' },
		body: `T=x&i=${ids[0]}&a=user/-/state/com.google/read`
	});
	expect(await read.text()).toBe('OK');
	await expect(node(page, 'Harbour Herald')).toContainText('(11)');

	// Revoked: the app is out, and so are its images.
	await dialog.getByRole('button', { name: 'Done' }).click();
	await dialog.getByRole('button', { name: 'Revoke the app password NetNewsWire' }).click();
	await expect(dialog.getByText('No app is set up yet.')).toBeVisible();
	expect((await fetch(`${api}/user-info`, { headers })).status).toBe(401);
	expect((await fetch(image)).status).toBe(404);
});
