// What the screenshots share: asking the server as the page would, the
// made-up feeds as subscriptions, the texts of the page, and taking a picture.

import { execFileSync } from 'node:child_process';
import { appendFileSync, mkdirSync, renameSync } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';
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
	const take = async (path: string) => {
		await page.screenshot({ path, animations: 'disabled', caret: 'hide' });
		await measure(page, path);
	};
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

/** What `measure` notes of a picture: its targets too small for a finger or a pointer, and the lines of article text. */
export interface Measured {
	/** The picture, as its name in its folder. */
	picture: string;
	/** Targets that can be pressed and are seen on top, as the name they are known by and their size in CSS pixels. */
	targets: { name: string; width: number; height: number }[];
	/** For each paragraph of article text of three lines or more, its characters to the line. */
	lines: number[];
}

/**
 * Notes, next to a picture, what can be measured of the page as it is shown,
 * in `measure.jsonl` of its folder; report.ts adds it up (docs/design_review.md).
 * A target counts when it is on top at its middle: hidden behind a dialog, it
 * cannot be pressed. A link in running text does not count, as WCAG leaves it out;
 * one marked `reach` counts by the area around it that can be pressed.
 */
export async function measure(page: Page, picture: string) {
	const seen = await page.evaluate(() => {
		const pressable =
			'button, a[href], input:not([type=hidden]), select, textarea, summary, [role=button], [role=tab], [role=menuitem], [role=menuitemradio]';
		const targets: { name: string; width: number; height: number }[] = [];
		for (const element of document.querySelectorAll<HTMLElement>(pressable)) {
			// A checkbox or a radio button is pressed through its label, around it or named
			// by `for`; a text field is pressed itself.
			const ticked = element.matches('input[type="checkbox"], input[type="radio"]');
			const target =
				element.closest('label') ??
				(ticked && element instanceof HTMLInputElement ? element.labels?.[0] : null) ??
				element;
			if (element.matches('a') && element.closest('p, li, .article-body')) continue;
			const box = target.getBoundingClientRect();
			if (box.width === 0 || box.height === 0) continue;
			const x = Math.min(Math.max(box.left + box.width / 2, 0), innerWidth - 1);
			const y = Math.min(Math.max(box.top + box.height / 2, 0), innerHeight - 1);
			if (box.bottom <= 0 || box.top >= innerHeight) continue;
			const top = document.elementFromPoint(x, y);
			if (!top || !(target.contains(top) || top.contains(target))) continue;
			const name = (
				element.getAttribute('aria-label') ||
				element.title ||
				target.textContent ||
				element.getAttribute('placeholder') ||
				element.tagName
			)
				.trim()
				.replace(/\s+/g, ' ')
				.slice(0, 60);
			// A target in a line of text is pressed around it, by 44 pixels (layout.css).
			const around = target.matches('.reach') && matchMedia('(pointer: coarse)').matches ? 44 : 0;
			targets.push({
				name,
				width: Math.max(Math.round(box.width), around),
				height: Math.max(Math.round(box.height), around)
			});
		}
		const lines: number[] = [];
		for (const paragraph of document.querySelectorAll('.article-body p')) {
			const range = document.createRange();
			range.selectNodeContents(paragraph);
			const rows = new Set([...range.getClientRects()].map((rect) => Math.round(rect.top))).size;
			if (rows >= 3) lines.push(Math.round((paragraph.textContent ?? '').length / rows));
		}
		return { targets, lines };
	});
	const measured: Measured = { picture: basename(picture), ...seen };
	appendFileSync(join(dirname(picture), 'measure.jsonl'), JSON.stringify(measured) + '\n');
}
