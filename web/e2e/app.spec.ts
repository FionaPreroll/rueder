// The page as an app: the manifest, and the service worker that keeps the app's
// files and never the reader's.

import { expect, test } from '@playwright/test';
import { logIn } from './harness/helpers.ts';

test('installs as an app, keeps its own files and nothing of the reader’s', async ({ page }) => {
	await logIn(page, 'login');
	const manifest = await page.request.get('/manifest.webmanifest');
	expect(manifest.headers()['content-type']).toContain('application/manifest+json');
	expect(await manifest.json()).toMatchObject({ name: 'rueder', display: 'standalone' });

	// The worker takes over the page and keeps the files of this version.
	await page.evaluate(() => navigator.serviceWorker.ready);
	await expect.poll(() => page.evaluate(() => !!navigator.serviceWorker.controller)).toBe(true);
	const kept = await page.evaluate(async () => {
		const names = await caches.keys();
		const urls = [];
		for (const name of names) {
			for (const request of await (await caches.open(name)).keys()) urls.push(request.url);
		}
		return { names, paths: urls.map((url) => new URL(url).pathname) };
	});
	expect(kept.names).toHaveLength(1);
	expect(kept.paths).toContain('/');
	expect(kept.paths.some((path) => path.startsWith('/_app/immutable/'))).toBe(true);
	expect(kept.paths.filter((path) => /^\/(api|img)\//.test(path))).toEqual([]);

	// What the reader sees still comes from the server, after a reload too.
	await page.reload();
	await expect(page.getByText('New in your subscriptions')).toBeVisible();
	const after = await page.evaluate(async () => {
		const urls = [];
		for (const name of await caches.keys()) {
			for (const request of await (await caches.open(name)).keys()) urls.push(request.url);
		}
		return urls.filter((url) => /\/(api|img)\//.test(new URL(url).pathname));
	});
	expect(after).toEqual([]);
});
