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

test('colors the bars of the app as the dark theme, within the page’s policy', async ({
	browser
}) => {
	const context = await browser.newContext({ colorScheme: 'dark' });
	const page = await context.newPage();
	// The script in app.html sets the colors before the first paint; the policy allows
	// it by its hash, so a changed script must not be refused.
	await page.addInitScript(() => {
		const seen: string[] = [];
		(window as unknown as { refused: string[] }).refused = seen;
		document.addEventListener('securitypolicyviolation', (event) =>
			seen.push(event.violatedDirective)
		);
	});
	await logIn(page, 'darkbars');
	const look = await page.evaluate(() => ({
		bars: [...document.querySelectorAll('meta[name="theme-color"]')].map(
			(meta) => (meta as HTMLMetaElement).content
		),
		manifest: document.querySelector('link[rel="manifest"]')?.getAttribute('href'),
		refused: (window as unknown as { refused: string[] }).refused
	}));
	expect(look).toEqual({
		bars: ['#1c1f25', '#1c1f25'],
		manifest: '/manifest-dark.webmanifest',
		refused: []
	});
	const manifest = await page.request.get('/manifest-dark.webmanifest');
	expect(manifest.headers()['content-type']).toContain('application/manifest+json');
	expect(await manifest.json()).toMatchObject({ name: 'rueder', theme_color: '#1c1f25' });
	await context.close();
});
