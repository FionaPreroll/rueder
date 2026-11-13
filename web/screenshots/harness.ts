// What the screenshots share: asking the server as the page would, the
// made-up feeds as subscriptions, the texts of the page, and taking a picture.

import { execFileSync } from 'node:child_process';
import { mkdirSync, renameSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import type { Locator, Page } from '@playwright/test';
import { SITE, database } from '../e2e/harness/ports.js';
// The catalogs read the page's language through a rune of Svelte, which is
// a plain value outside of it; this has to come before them.
import './runes.ts';
import de from '../src/lib/i18n/de.ts';
import en, { type Messages } from '../src/lib/i18n/en.ts';

export const catalogs: Record<'en' | 'de', Messages> = { en, de };

/** Asks the database of the run, as the operator would; the answer as text. */
export function psql(sql: string) {
	return execFileSync('psql', [database(), '--quiet', '--tuples-only', '--no-align', '-c', sql], {
		encoding: 'utf8',
		stdio: ['ignore', 'pipe', 'inherit']
	}).trim();
}

/**
 * Sends a request to the API from the page, with its session and its origin,
 * and gives back the answer's JSON, if any. A string is sent as it is, as XML.
 */
export async function api<T = unknown>(page: Page, method: string, path: string, body?: unknown) {
	return page.evaluate(
		async ([method, path, body]) => {
			const xml = typeof body === 'string';
			const answer = await fetch(`/api${path}`, {
				method,
				headers:
					body === undefined ? {} : { 'Content-Type': xml ? 'text/x-opml' : 'application/json' },
				body: body === undefined ? undefined : xml ? body : JSON.stringify(body)
			});
			if (!answer.ok) throw new Error(`${method} ${path}: ${answer.status} ${await answer.text()}`);
			const text = await answer.text();
			return (text ? JSON.parse(text) : null) as unknown;
		},
		[method, path, body] as const
	) as Promise<T>;
}

/** Subscribes the account logged in on `page` to the made-up feeds, in their folders. */
export async function importFeeds(page: Page) {
	const opml = await (await fetch(`${SITE}/feeds.opml`)).text();
	await api(page, 'POST', '/opml', opml);
}

/** Where the pictures go: screenshots/out/<project>/. */
export const OUT = resolve(import.meta.dirname, 'out');

/** Waits until the pictures on the page are loaded and nothing moves any more. */
export async function settled(page: Page) {
	await page.evaluate(async () => {
		await document.fonts.ready;
		await Promise.all(
			[...document.images]
				.filter((image) => !image.complete && image.loading !== 'lazy')
				.map(
					(image) => new Promise((done) => image.addEventListener('loadend', done, { once: true }))
				)
		);
	});
	// Transitions of menus and dialogs end.
	await page.waitForTimeout(250);
}

/**
 * Takes a picture of the window. With `scrolling`, an element that scrolls,
 * as long dialogs do, it takes one more for every screen further down, up to
 * a few.
 */
export async function shoot(page: Page, file: string, scrolling?: Locator) {
	await settled(page);
	mkdirSync(dirname(file), { recursive: true });
	const take = (path: string) => page.screenshot({ path, animations: 'disabled', caret: 'hide' });
	await take(`${file}.png`);
	if (!scrolling) return;
	for (let part = 2; part <= 6; part++) {
		const moved = await scrolling.evaluate((element) => {
			const before = element.scrollTop;
			element.scrollBy(0, element.clientHeight * 0.85);
			return element.scrollTop > before;
		});
		if (!moved) return;
		// The first of several is the first part, so that they sort in order.
		if (part === 2) renameSync(`${file}.png`, `${file}-1.png`);
		await settled(page);
		await take(`${file}-${part}.png`);
	}
}
