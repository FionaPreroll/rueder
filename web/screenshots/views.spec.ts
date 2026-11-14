// Every view of rueder as a picture, for looking at the design: the login, the
// reader with its lists and views, menus, dialogs, the settings, the account's
// settings and the status of the server. Each project of
// playwright.screenshots.config.ts runs it for its language, device and theme.
//
// One test walks through all views. A view that fails is noted and the walk
// goes on from the start page, so one broken view does not cost the others;
// the test fails at the end, naming them.

import { expect, test, type Locator, type Page } from '@playwright/test';
import { PASSWORD, SITE } from '../e2e/harness/helpers.ts';
import { OUT, api, catalogs, importFeeds, shoot } from './harness.ts';
import { TERMS_VERSION, projects } from './projects.ts';
import { createHmac } from 'node:crypto';
import { rmSync } from 'node:fs';
import { join } from 'node:path';

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

interface ApiTree {
	folders: { id: number; title: string }[];
	feeds: { id: number; title: string; last_seq: number }[];
}

/** Stars, labels, filter rules and some reading, so the lists look used. */
async function useAccount(page: Page) {
	const tree = await api<ApiTree>(page, 'GET', '/tree');
	const feed = (title: string) => tree.feeds.find((feed) => feed.title === title)!;
	const state = (title: string, seq: number, change: object) =>
		api(page, 'PUT', `/feeds/${feed(title).id}/articles/${seq}/state`, change);

	for (const [title, seq] of [
		['Tagesbote', feed('Tagesbote').last_seq - 1],
		['The Harbor Gazette', feed('The Harbor Gazette').last_seq],
		['Kernel-Notizen', feed('Kernel-Notizen').last_seq - 3],
		['Lab Notes Weekly', feed('Lab Notes Weekly').last_seq]
	] as const) {
		await state(title, seq, { starred: true });
	}
	// Some of the newest are read already.
	for (const title of ['Platinenpost', 'Borrow Checker Diaries']) {
		const last = feed(title).last_seq;
		for (const seq of [last, last - 2]) await state(title, seq, { read: true });
	}
	// Everything in one feed is read, for "nothing unread here".
	await api(page, 'POST', `/feeds/${feed('Blog von nebenan').id}/read`, {
		upto: feed('Blog von nebenan').last_seq
	});

	const labels: number[] = [];
	for (const [title, color] of [
		['Später lesen', '#1a73e8'],
		['Projekt', '#188038'],
		['Zitat', '#e8710a']
	]) {
		labels.push((await api<{ id: number }>(page, 'POST', '/labels', { title, color })).id);
	}
	const label = (title: string, seq: number, id: number) =>
		api(page, 'PUT', `/feeds/${feed(title).id}/articles/${seq}/labels/${id}`);
	await label('Ferris Wochenblatt', feed('Ferris Wochenblatt').last_seq, labels[0]);
	await label('Tagesbote', feed('Tagesbote').last_seq - 2, labels[0]);
	await label('Kernel-Notizen', feed('Kernel-Notizen').last_seq, labels[1]);

	const rule = (pattern: string, field: string, action: string) =>
		api(page, 'POST', '/rules', {
			enabled: true,
			scope: { kind: 'all' },
			field,
			match: 'contains',
			pattern,
			action
		});
	await rule('Anzeige:', 'title', 'hide');
	await rule('Sponsored', 'title', 'hide');
	await rule('Werbeteam', 'author', 'read');
}

test('every view', async ({ page, isMobile }, info) => {
	const shots = projects(process.env.SCREENSHOT_LANGUAGES).find(
		(p) => p.name === info.project.name
	);
	if (!shots) throw new Error(`no such project: ${info.project.name}`);
	const m = catalogs[shots.language];
	const phone = isMobile;
	const folder = join(OUT, shots.name);
	// What a run before measured (harness.ts, measure).
	rmSync(join(folder, 'measure.jsonl'), { force: true });

	let number = 0;
	const failed: string[] = [];

	const sidebar = page.getByRole('navigation', { name: m.nav.subscriptions });
	const dialog = (name: string) => page.getByRole('dialog', { name });
	const menuItem = (name: string | RegExp) => page.getByRole('menuitem', { name });
	const node = (title: string) =>
		sidebar.locator('[data-node]', { has: page.getByText(title, { exact: true }) });
	/** What an opened article shows below its text. */
	const opened = page.getByText(m.list.keepUnread).first();
	const articles = page.locator('[data-article]');
	/** A button that can be seen: some come twice, one for wide windows and one for narrow ones. */
	const button = (name: string | RegExp, within: Page | Locator = page) =>
		within.getByRole('button', { name, exact: typeof name === 'string' }).filter({ visible: true });

	/** Takes the picture of a view, and with `scrolling` of what is further down in it. */
	const picture = (name: string, scrolling?: Locator) =>
		shoot(page, join(folder, `${String(++number).padStart(2, '0')}-${name}`), scrolling);

	/** The scrolling backdrop of an open dialog. */
	const backdrop = (open: Locator) => open.locator('xpath=..');

	/** Back to the start: no dialog, no menu, the list of all items. */
	async function reset() {
		await page.keyboard.press('Escape');
		// Without the view kept from before, the page starts at home.
		await page.evaluate(() => localStorage.removeItem('rueder.view')).catch(() => {});
		await page.goto('/');
		await expect(page.getByRole('searchbox').or(page.getByLabel(m.login.name))).toBeVisible();
	}

	/** One view or a few: what goes wrong is noted, and the next starts from the start page. */
	async function view(name: string, show: () => Promise<void>) {
		await test.step(name, async () => {
			try {
				await show();
			} catch (error) {
				failed.push(`${name}: ${String(error).split('\n')[0]}`);
				await reset().catch(() => {});
			}
		});
	}

	/** Opens the sidebar where it is hidden, as on a phone. */
	async function showSidebar() {
		if (!(await sidebar.isVisible())) await button(m.nav.openSidebar).click();
		await expect(sidebar).toBeVisible();
	}

	/** Shows a list from the sidebar: a stream such as All items, or a feed or a folder by its name. */
	async function show(name: string, inTree = false) {
		await showSidebar();
		if (inTree) await node(name).click();
		else
			await sidebar
				.getByRole('button', { name: new RegExp(`^${name}`) })
				.first()
				.click();
		if ((await sidebar.isVisible()) && phone) await button(m.nav.closeSidebar).first().click();
	}

	/** Picks the view of the lists through the toolbar. */
	async function pickView(view: 'list' | 'expanded' | 'split' | 'magazine') {
		const label = {
			list: m.toolbar.listView,
			expanded: m.toolbar.expandedView,
			split: m.toolbar.splitView,
			magazine: m.toolbar.magazineView
		}[view];
		const direct = page.getByRole('group', { name: m.settings.view }).getByRole('button', {
			name: label
		});
		if (await direct.isVisible()) return direct.click();
		await button(phoneMenu).click();
		await page.getByRole('menuitemradio', { name: m.settings.views[view] }).click();
	}

	/** On a phone, what to show, the views and marking as read are one menu. */
	const phoneMenu = new RegExp(
		`^${m.toolbar.phoneMenu('').replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`
	);

	/** Opens the menu of what to show, which says what it shows now. */
	async function openShow(shown: string) {
		const combined = button(phoneMenu);
		if (await combined.isVisible()) return combined.click();
		await button(shown).click();
	}

	async function logIn() {
		await page.getByLabel(m.login.name).fill(shots!.account);
		await page.getByLabel(m.login.password).fill(PASSWORD);
		await button(m.login.submit).click();
	}

	// ------------------------------------------------------------ logging in

	await view('login', async () => {
		await page.goto('/');
		await expect(button(m.login.submit)).toBeVisible();
		await picture('login');
		await page.getByLabel(m.login.name).fill(shots.account);
		await page.getByLabel(m.login.password).fill('not the password');
		await button(m.login.submit).click();
		await expect(page.getByRole('alert')).toContainText(m.login.wrong);
		await picture('login-wrong-password');
	});

	await view('register', async () => {
		await page.goto('/');
		await page
			.getByRole('button', { name: m.register.toRegister })
			.or(page.getByRole('link', { name: m.register.toRegister }))
			.click();
		await expect(page.getByLabel(m.register.again)).toBeVisible();
		await picture('register');
	});

	await view('terms', async () => {
		await page.goto('/');
		await logIn();
		await expect(dialog(m.legal.changed).or(page.getByRole('alertdialog'))).toBeVisible();
		await picture('terms-to-accept');
		await button(m.legal.accept).click();
		await expect(page.getByRole('searchbox')).toBeVisible();
	});

	// Whatever the walk above left undone: logged in, terms agreed, feeds there.
	await page.goto('/');
	if (await page.getByLabel(m.login.name).isVisible()) await logIn();
	await expect(sidebar.or(button(m.nav.openSidebar))).toBeVisible();
	await api(page, 'PUT', '/terms', { version: TERMS_VERSION });
	const tree = await api<ApiTree>(page, 'GET', '/tree');
	if (tree.feeds.length === 0) {
		await importFeeds(page);
		await useAccount(page);
	}
	await page.reload();

	// ------------------------------------------------------------ the reader

	await view('home', async () => {
		await expect(page.getByText(m.home.newIn)).toBeVisible();
		await picture('home');
	});

	if (phone) {
		await view('sidebar', async () => {
			await showSidebar();
			await picture('sidebar');
		});
	}

	await view('list', async () => {
		await show(m.nav.all);
		await pickView('list');
		await expect(articles.first()).toBeVisible();
		await picture('list');
		await articles.nth(1).click();
		await expect(opened).toBeVisible();
		await picture('list-article-open');
	});

	await view('expanded', async () => {
		await reset();
		await show(m.nav.all);
		await pickView('expanded');
		await expect(articles.first()).toBeVisible();
		await picture('expanded');
	});

	await view('split', async () => {
		await reset();
		await show(m.nav.all);
		await pickView('split');
		await articles.first().click();
		await picture('three-panes');
	});

	await view('magazine', async () => {
		await reset();
		await show(m.nav.all);
		await pickView('magazine');
		await expect(articles.first()).toBeVisible();
		await picture('magazine');
		await articles.first().click();
		await expect(page.getByRole('dialog')).toBeVisible();
		await picture('magazine-article');
		await page.keyboard.press('Escape');
		await pickView('list');
	});

	await view('folder', async () => {
		await reset();
		await show('Nachrichten', true);
		await expect(articles.first()).toBeVisible();
		await picture('folder');
	});

	await view('feed with an error', async () => {
		await reset();
		await show('Wort des Tages', true);
		await picture('feed-with-error');
	});

	await view('podcast', async () => {
		await reset();
		await show('Lab Notes Weekly', true);
		await articles.first().click();
		await expect(page.getByText(m.media.title)).toBeVisible();
		await picture('article-with-recording');
	});

	await view('versions', async () => {
		await reset();
		await show(m.nav.all);
		await pickView('list');
		const updated = m.list.updatedTitle(1).split(';')[0];
		const changed = articles.filter({
			has: page.getByRole('img', { name: new RegExp(`^${updated}`) })
		});
		await changed.first().click();
		await button(m.list.compare).first().click();
		const open = dialog(m.versions.title);
		await expect(open).toBeVisible();
		await picture('versions', backdrop(open));
	});

	await view('starred', async () => {
		await reset();
		await show(m.nav.starred);
		await expect(articles.first()).toBeVisible();
		await picture('starred');
	});

	await view('hidden', async () => {
		await reset();
		await show(m.nav.hidden);
		await expect(articles.first()).toBeVisible();
		await picture('hidden-by-filters');
	});

	await view('label', async () => {
		await reset();
		await show('Später lesen');
		await expect(articles.first()).toBeVisible();
		await picture('label');
	});

	await view('search', async () => {
		await reset();
		// In one feed, which offers to search all of them.
		await show('Ferris Wochenblatt', true);
		await page.getByRole('searchbox').fill('Borrow');
		await expect(articles.first()).toBeVisible();
		await expect(button(m.list.searchAll)).toBeVisible();
		await picture('search');
	});

	await view('nothing unread', async () => {
		await reset();
		await show('Blog von nebenan', true);
		await openShow(m.toolbar.showAll);
		await page.getByRole('menuitemradio', { name: m.toolbar.showUnread }).click();
		await expect(page.getByText(m.list.emptyUnread)).toBeVisible();
		await picture('nothing-unread');
		await openShow(m.toolbar.showUnread);
		await page.getByRole('menuitemradio', { name: m.toolbar.showAll }).click();
	});

	// ------------------------------------------------------------ menus

	await view('menus of the toolbar', async () => {
		await reset();
		await show(m.nav.all);
		await openShow(m.toolbar.showAll);
		await picture('menu-show');
		await page.keyboard.press('Escape');
		// On a phone that one menu holds marking as read too.
		if (!phone) {
			await page.getByRole('button', { name: m.toolbar.markOptions }).click();
			await picture('menu-mark-as-read');
			await page.keyboard.press('Escape');
		}
	});

	await view('menus of the sidebar', async () => {
		await reset();
		await showSidebar();
		await node('Technik').hover();
		await page.getByRole('button', { name: m.nav.folderActions('Technik') }).click();
		await picture('menu-folder');
		await page.keyboard.press('Escape');
		await node('Tagesbote').hover();
		await page.getByRole('button', { name: m.nav.feedActions('Tagesbote') }).click();
		await picture('menu-feed');
		await menuItem(m.nav.unsubscribe).click();
		await picture('unsubscribe');
		await button(m.common.cancel).click();
		await node('Tagesbote').hover();
		await page.getByRole('button', { name: m.nav.feedActions('Tagesbote') }).click();
		await menuItem(m.nav.reportFeed).click();
		await expect(dialog(m.report.feedTitle)).toBeVisible();
		await picture('report-feed');
		await page.keyboard.press('Escape');
		await sidebar.getByRole('button', { name: m.labels.actions('Projekt') }).click();
		await picture('menu-label');
		await page.keyboard.press('Escape');
		await button(m.nav.newFolder, sidebar).click();
		await picture('new-folder');
	});

	await view('menus of an article', async () => {
		await reset();
		await show('Kernel-Notizen', true);
		await articles.first().click();
		await expect(opened).toBeVisible();
		await button(m.list.articleActions).first().click();
		await picture('menu-article');
		await page.keyboard.press('Escape');
		await button(m.labels.forArticle).first().click();
		await picture('menu-article-labels');
		await page.keyboard.press('Escape');
	});

	await view('menu of the account', async () => {
		await reset();
		await page.getByRole('button', { name: m.nav.account(shots.account) }).click();
		await picture('menu-account');
	});

	// ------------------------------------------------------------ dialogs

	await view('subscribe', async () => {
		await reset();
		await showSidebar();
		await button(m.nav.subscribe, sidebar).click();
		const open = dialog(m.subscribe.title);
		await expect(open).toBeVisible();
		await picture('subscribe');
		await open.getByLabel(m.subscribe.address).fill(`${SITE}/feed-teleskop/`);
		await button(m.subscribe.find, open).click();
		await expect(open.getByText(m.subscribe.subscribed)).toBeVisible();
		await picture('subscribe-found', backdrop(open));
	});

	await view('settings', async () => {
		await reset();
		await button(m.nav.settings).click();
		const open = dialog(m.settings.title);
		// A phone has no keyboard, so no tab for its keys.
		const tabs = (['look', 'reading', 'rules', 'keys'] as const).filter(
			(tab) => !phone || tab !== 'keys'
		);
		for (const tab of tabs) {
			await open.getByRole('tab', { name: m.settings.tabs[tab] }).click();
			await picture(`settings-${tab}`, backdrop(open));
		}
		await open.getByRole('tab', { name: m.settings.tabs.rules }).click();
		await button(m.rules.add, open).click();
		await picture('settings-rules-new', backdrop(open));
	});

	await view('settings of the account', async () => {
		await reset();
		await page.getByRole('button', { name: m.nav.account(shots.account) }).click();
		await menuItem(m.nav.accountSettings).click();
		const open = dialog(m.account.title);
		for (const tab of [
			'password',
			'security',
			'apps',
			'invites',
			'move',
			'privacy',
			'remove'
		] as const) {
			const name = open.getByRole('tab', { name: m.account.tabs[tab] });
			if (!(await name.isVisible())) continue;
			await name.click();
			await picture(`account-${tab}`, backdrop(open));
		}
	});

	await view('an app password', async () => {
		await reset();
		await page.getByRole('button', { name: m.nav.account(shots.account) }).click();
		await menuItem(m.nav.accountSettings).click();
		const open = dialog(m.account.title);
		await open.getByRole('tab', { name: m.account.tabs.apps }).click();
		await button(m.settings.apps.add, open).click();
		await picture('app-choose', backdrop(open));
		await open
			.getByRole('button', { name: /NetNewsWire/ })
			.first()
			.click();
		await picture('app-netnewswire', backdrop(open));
		await open.getByLabel(m.settings.apps.password).fill(PASSWORD);
		await button(m.settings.apps.create, open).click();
		await expect(open.getByText(m.settings.apps.once)).toBeVisible();
		await picture('app-created', backdrop(open));
	});

	await view('an invitation', async () => {
		await reset();
		await page.getByRole('button', { name: m.nav.account(shots.account) }).click();
		await menuItem(m.nav.accountSettings).click();
		const open = dialog(m.account.title);
		await open.getByRole('tab', { name: m.account.tabs.invites }).click();
		await button(m.invites.make, open).click();
		await expect(open.getByText(m.invites.link)).toBeVisible();
		await picture('invitation-made', backdrop(open));
	});

	await view('status', async () => {
		await reset();
		await page.getByRole('button', { name: m.nav.account(shots.account) }).click();
		await menuItem(m.status.open).click();
		const open = dialog(m.status.title);
		await expect(open.getByText(m.status.loading)).toBeHidden();
		await picture('status', backdrop(open));
	});

	await view('second factor', async () => {
		await reset();
		await page.getByRole('button', { name: m.nav.account(shots.account) }).click();
		await menuItem(m.nav.accountSettings).click();
		const open = dialog(m.account.title);
		await open.getByRole('tab', { name: m.account.tabs.security }).click();
		await open.getByLabel(m.settings.secondFactor.password).first().fill(PASSWORD);
		await button(m.settings.secondFactor.start, open).click();
		await expect(open.getByRole('img', { name: m.settings.secondFactor.qr })).toBeVisible();
		await picture('second-factor-setup', backdrop(open));
		const key = fromBase32(await open.locator('code').innerText());
		await open.getByLabel(m.settings.secondFactor.code).fill(totp(key));
		await button(m.settings.secondFactor.confirm, open).click();
		await expect(open.getByText(m.settings.secondFactor.codesTitle)).toBeVisible();
		await picture('second-factor-recovery-codes', backdrop(open));
		await button(m.settings.secondFactor.codesDone, open).click();
		await picture('second-factor-on', backdrop(open));

		// Logging in now asks for the code.
		await page.context().clearCookies();
		await page.goto('/');
		await logIn();
		await expect(page.getByLabel(m.login.code)).toBeVisible();
		await picture('login-code');
	});

	if (failed.length > 0) throw new Error(`views that failed:\n${failed.join('\n')}`);
});
