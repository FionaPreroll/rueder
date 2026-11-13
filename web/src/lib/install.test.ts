// The installed app on a desktop: the number on its icon, and installing from the account menu.

import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ServerReader } from '#lib/api/reader.svelte.ts';
import { FakeServer } from '#lib/api/test/fake-server.ts';
import TopBar from '#lib/components/TopBar.svelte';
import { install, runsAsApp, showOnIcon, watchInstall } from './install.svelte.ts';

const chrome =
	'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36';
const safari =
	'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15';

/** A window of a browser, with the display modes it is in. */
function fakeWindow({
	userAgent = chrome,
	vendor = 'Google Inc.',
	modes = [] as string[],
	standalone
}: { userAgent?: string; vendor?: string; modes?: string[]; standalone?: boolean } = {}) {
	const target = new EventTarget();
	return Object.assign(target, {
		navigator: { userAgent, vendor, standalone },
		matchMedia: (query: string) => ({
			matches: modes.some((mode) => query === `(display-mode: ${mode})`)
		})
	}) as unknown as Window;
}

/** What Chromium fires when it would install the page. */
function offer() {
	const event = Object.assign(new Event('beforeinstallprompt', { cancelable: true }), {
		prompt: vi.fn(async () => ({ outcome: 'accepted' }))
	});
	return event;
}

afterEach(() => {
	vi.unstubAllGlobals();
	install.offer = null;
	install.safari = false;
});

describe('the number on the app’s icon', () => {
	it('follows the unread articles and goes at zero', () => {
		const nav = { setAppBadge: vi.fn(async () => {}), clearAppBadge: vi.fn(async () => {}) };
		showOnIcon(12, nav);
		expect(nav.setAppBadge).toHaveBeenCalledWith(12);
		showOnIcon(0, nav);
		expect(nav.clearAppBadge).toHaveBeenCalledOnce();
	});

	it('is left out where the browser has no badges, or refuses them', async () => {
		expect(() => showOnIcon(3, {})).not.toThrow();
		expect(() => showOnIcon(3, undefined)).not.toThrow();
		const refused = vi.fn(() => Promise.reject(new DOMException('No', 'NotAllowedError')));
		showOnIcon(3, { setAppBadge: refused, clearAppBadge: refused });
		// A refusal nobody waits for would be reported by Vitest.
		await new Promise((resolve) => setTimeout(resolve));
		expect(refused).toHaveBeenCalledOnce();
	});
});

describe('installing the page as an app', () => {
	it('knows when it runs as an app', () => {
		expect(runsAsApp(fakeWindow())).toBe(false);
		expect(runsAsApp(fakeWindow({ modes: ['standalone'] }))).toBe(true);
		expect(runsAsApp(fakeWindow({ modes: ['window-controls-overlay'] }))).toBe(true);
		expect(runsAsApp(fakeWindow({ standalone: true }))).toBe(true);
	});

	it('holds back the browser’s offer and shows its dialog when asked, once', async () => {
		const win = fakeWindow();
		const stop = watchInstall(win);
		expect(install.offer).toBeNull();
		const event = offer();
		win.dispatchEvent(event);
		expect(event.defaultPrevented).toBe(true);
		expect(install.offer).not.toBeNull();
		await install.offer!();
		expect(event.prompt).toHaveBeenCalledOnce();
		expect(install.offer).toBeNull();

		// A new offer, then the app is installed some other way.
		win.dispatchEvent(offer());
		expect(install.offer).not.toBeNull();
		win.dispatchEvent(new Event('appinstalled'));
		expect(install.offer).toBeNull();

		stop();
		win.dispatchEvent(offer());
		expect(install.offer).toBeNull();
	});

	it('takes a dialog the browser will not show as no harm', async () => {
		const win = fakeWindow();
		watchInstall(win);
		const event = Object.assign(new Event('beforeinstallprompt', { cancelable: true }), {
			prompt: vi.fn(() => Promise.reject(new DOMException('Shown', 'InvalidStateError')))
		});
		win.dispatchEvent(event);
		await expect(install.offer!()).resolves.toBeUndefined();
	});

	it('says where Safari installs, and not in other browsers', () => {
		const stop = watchInstall(fakeWindow({ userAgent: safari, vendor: 'Apple Computer, Inc.' }));
		expect(install.safari).toBe(true);
		stop();
		expect(install.safari).toBe(false);
		watchInstall(fakeWindow({ userAgent: chrome, vendor: 'Google Inc.' }));
		expect(install.safari).toBe(false);
		watchInstall(
			fakeWindow({
				userAgent: 'Mozilla/5.0 (X11; Linux x86_64; rv:145.0) Firefox/145.0',
				vendor: ''
			})
		);
		expect(install.safari).toBe(false);
	});

	it('offers nothing in an app already, nor in the Android app', () => {
		for (const win of [
			fakeWindow({ userAgent: safari, vendor: 'Apple Computer, Inc.', modes: ['standalone'] }),
			fakeWindow({ userAgent: `${chrome} rueder-android` })
		]) {
			watchInstall(win);
			win.dispatchEvent(offer());
			expect(install).toEqual({ offer: null, safari: false });
		}
	});
});

describe('the account menu', () => {
	async function menu() {
		vi.stubGlobal('fetch', new FakeServer().fetch);
		const session = { signOut: () => {}, expired: () => {} };
		render(TopBar, {
			reader: new ServerReader({ name: 'fiona', admin: false }, session),
			sidebarOpen: true,
			ontogglesidebar: () => {},
			onsettings: () => {},
			onaccount: () => {},
			onhelp: () => {},
			onstatus: () => {}
		});
		await userEvent.click(screen.getByRole('button', { name: 'Logged in as fiona' }));
	}

	it('installs as an app where the browser offers it', async () => {
		const win = fakeWindow();
		watchInstall(win);
		const event = offer();
		win.dispatchEvent(event);
		await menu();
		await userEvent.click(screen.getByRole('menuitem', { name: 'Install as an app' }));
		expect(event.prompt).toHaveBeenCalledOnce();
	});

	it('says where in Safari, and offers nothing where the browser does not', async () => {
		const stop = watchInstall(fakeWindow({ userAgent: safari, vendor: 'Apple Computer, Inc.' }));
		await menu();
		expect(screen.getByText(/File › Add to Dock/)).toBeInTheDocument();
		expect(screen.queryByRole('menuitem', { name: 'Install as an app' })).toBeNull();
		stop();
		await vi.waitFor(() => expect(screen.queryByText(/File › Add to Dock/)).toBeNull());
	});
});
