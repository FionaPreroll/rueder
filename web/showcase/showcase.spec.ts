// The pictures in the README: what rueder does best, in the reader with
// made-up data. Each picture is a test of its own, taken on a desktop or on a
// phone, light and dark (playwright.showcase.config.ts), and lands in
// showcase/out/ as <picture>-<theme>.png. The README names them by these names.

import { existsSync } from 'node:fs';
import { join, normalize, sep } from 'node:path';
import { expect, test, type Locator, type Page } from '@playwright/test';
import { catalogs } from '../screenshots/harness.ts';
import { BUILD, DEMO, OUT, looks } from './looks.ts';

const m = catalogs.en;

test.beforeEach(async ({ page }) => {
	// The build answers the page's requests, as GitHub Pages does for the demo.
	await page.route(`${DEMO}**`, async (route) => {
		const path = normalize(decodeURIComponent(new URL(route.request().url()).pathname));
		const file = join(BUILD, path.endsWith('/') ? `${path}index.html` : path);
		if (file.startsWith(BUILD + sep) && existsSync(file)) await route.fulfill({ path: file });
		else await route.fulfill({ status: 404 });
	});
	// Made-up articles arrive every nine seconds (`DemoReader.startLive`); no
	// interval of five seconds or more starts here, so that none comes in the
	// middle of a picture. The page's clock and its frames run as they would:
	// the list measures its rows in frames and draws only those in sight.
	await page.addInitScript(() => {
		const every = window.setInterval;
		window.setInterval = ((handler: TimerHandler, timeout?: number, ...rest: unknown[]) =>
			(timeout ?? 0) >= 5000 ? 0 : every(handler, timeout, ...rest)) as typeof window.setInterval;
	});
});

/** Opens the reader at its start page, as it is for someone who comes the first time. */
async function open(page: Page) {
	await page.goto('/');
	await expect(page.getByRole('searchbox')).toBeVisible();
}

/** A button that can be seen: some come twice, for wide windows and for narrow ones. */
const button = (within: Page | Locator, name: string | RegExp) =>
	within.getByRole('button', { name, exact: typeof name === 'string' }).filter({ visible: true });

const articles = (page: Page) => page.locator('[data-article]');

/** Shows all items, in one of the views of the toolbar. */
async function showAll(page: Page, view?: 'list' | 'split' | 'magazine') {
	const sidebar = page.getByRole('navigation', { name: m.nav.subscriptions });
	if (!(await sidebar.isVisible())) await button(page, m.nav.openSidebar).click();
	await sidebar
		.getByRole('button', { name: new RegExp(`^${m.nav.all}`) })
		.first()
		.click();
	if (view) {
		const label = {
			list: m.toolbar.listView,
			split: m.toolbar.splitView,
			magazine: m.toolbar.magazineView
		}[view];
		await page
			.getByRole('group', { name: m.settings.view })
			.getByRole('button', { name: label })
			.click();
	}
	await expect(articles(page).first()).toBeVisible();
}

/**
 * Takes the picture, once the pictures on the page are there and nothing moves
 * any more: of the window, or of `part` of it with a margin around it, so that
 * a dialog can still be read where the README shows it small.
 */
async function shoot(page: Page, file: string, part?: Locator) {
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
	const box = part && (await part.boundingBox());
	const viewport = page.viewportSize();
	const clip =
		box && viewport
			? {
					x: Math.max(box.x - MARGIN, 0),
					y: Math.max(box.y - MARGIN, 0),
					width: Math.min(box.width + 2 * MARGIN, viewport.width),
					height: Math.min(box.height + 2 * MARGIN, viewport.height)
				}
			: undefined;
	if (part && !clip) throw new Error(`${file}: the part to show is not on the page`);
	await page.screenshot({ path: file, clip, animations: 'disabled', caret: 'hide' });
}

/** The margin around a part of the window, in CSS pixels. */
const MARGIN = 24;

interface Picture {
	/** Its name in the README, without the theme. */
	name: string;
	phone: boolean;
	/** Shows what the picture is of, and gives back the part of the window it is of, if not all. */
	show(page: Page): Promise<Locator | void>;
}

const pictures: Picture[] = [
	{
		// Folders, list and article side by side.
		name: 'three-panes',
		phone: false,
		async show(page) {
			await showAll(page, 'split');
			await articles(page).first().click();
		}
	},
	{
		name: 'magazine',
		phone: false,
		async show(page) {
			await showAll(page, 'magazine');
		}
	},
	{
		// An article that changed at its source, compared with what it was.
		name: 'versions',
		phone: false,
		async show(page) {
			await showAll(page, 'list');
			const updated = articles(page)
				.filter({ has: page.getByText(m.list.updated, { exact: true }) })
				.first();
			// The list draws only the rows near what is in sight, and which
			// articles are near the top depends on the time of day.
			for (let screen = 0; screen < 20 && (await updated.count()) === 0; screen++) {
				await articles(page).last().scrollIntoViewIfNeeded();
				await page.waitForTimeout(100);
			}
			await updated.click();
			await button(page, m.list.compare).first().click();
			const dialog = page.getByRole('dialog', { name: m.versions.title });
			await expect(dialog).toBeVisible();
			return dialog;
		}
	},
	{
		// A new filter rule, with what it would catch.
		name: 'filter-rules',
		phone: false,
		async show(page) {
			await button(page, m.nav.settings).click();
			const dialog = page.getByRole('dialog', { name: m.settings.title });
			await dialog.getByRole('tab', { name: m.settings.tabs.rules }).click();
			await button(dialog, m.rules.add).click();
			await dialog.getByLabel(m.rules.pattern).fill('council');
			// What it would catch, counted.
			const counted = m.rules.preview(0, 0).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
			await expect(dialog.getByText(new RegExp(counted.replace(/0/g, '\\d+')))).toBeVisible();
			return dialog;
		}
	},
	{
		// An app password for NetNewsWire, with what to enter in the app.
		name: 'apps',
		phone: false,
		async show(page) {
			await button(page, m.demo.badge).click();
			await page.getByRole('menuitem', { name: m.nav.accountSettings }).click();
			const dialog = page.getByRole('dialog', { name: m.account.title });
			await dialog.getByRole('tab', { name: m.account.tabs.apps }).click();
			await button(dialog, m.settings.apps.add).click();
			await dialog
				.getByRole('button', { name: /NetNewsWire/ })
				.first()
				.click();
			// The draft takes any password.
			await dialog.getByLabel(m.settings.apps.password).fill('password');
			await button(dialog, m.settings.apps.create).click();
			await expect(dialog.getByText(m.settings.apps.once)).toBeVisible();
			return dialog;
		}
	},
	{
		// The start page: what is new, feed by feed.
		name: 'phone-home',
		phone: true,
		async show(page) {
			await expect(page.getByText(m.home.newIn)).toBeVisible();
		}
	},
	{
		name: 'phone-sidebar',
		phone: true,
		async show(page) {
			await button(page, m.nav.openSidebar).click();
			await expect(page.getByRole('navigation', { name: m.nav.subscriptions })).toBeVisible();
		}
	},
	{
		name: 'phone-article',
		phone: true,
		async show(page) {
			await showAll(page);
			await articles(page).first().click();
			await expect(page.getByText(m.list.keepUnread).first()).toBeVisible();
		}
	}
];

for (const picture of pictures) {
	test(picture.name, { tag: picture.phone ? '@phone' : '@desktop' }, async ({ page }, info) => {
		const look = looks.find((look) => look.name === info.project.name);
		if (!look) throw new Error(`no such look: ${info.project.name}`);
		await open(page);
		const part = await picture.show(page);
		await shoot(page, join(OUT, `${picture.name}-${look.theme}.png`), part ?? undefined);
	});
}
