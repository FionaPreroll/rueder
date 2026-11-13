// The page inside the Android app, and the way back to the app's own settings.

import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ServerReader } from '#lib/api/reader.svelte.ts';
import { FakeServer } from '#lib/api/test/fake-server.ts';
import TopBar from '#lib/components/TopBar.svelte';
import { APP_SETTINGS, inAndroidApp } from './app.ts';

const chrome =
	'Mozilla/5.0 (Linux; Android 15) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Mobile Safari/537.36';

afterEach(() => {
	vi.unstubAllGlobals();
	vi.restoreAllMocks();
});

describe('inAndroidApp', () => {
	it('knows the app by the name it adds to the browser’s', () => {
		expect(inAndroidApp(`${chrome} rueder-android`)).toBe(true);
		expect(inAndroidApp(chrome)).toBe(false);
		expect(inAndroidApp('rueder-androidx')).toBe(false);
	});
});

describe('the account menu', () => {
	async function menu(userAgent: string) {
		vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue(userAgent);
		vi.stubGlobal('fetch', new FakeServer().fetch);
		const session = { signOut: () => {}, expired: () => {} };
		const reader = new ServerReader({ name: 'fiona', admin: false }, session);
		render(TopBar, {
			reader,
			sidebarOpen: true,
			ontogglesidebar: () => {},
			onsettings: () => {},
			onaccount: () => {},
			onhelp: () => {},
			onstatus: () => {}
		});
		await userEvent.click(screen.getByRole('button', { name: 'Logged in as fiona' }));
	}

	it('leads back to the app’s settings inside the app', async () => {
		await menu(`${chrome} rueder-android`);
		expect(screen.getByRole('menuitem', { name: 'Server and app updates' })).toHaveAttribute(
			'href',
			APP_SETTINGS
		);
	});

	it('has no such item in a browser', async () => {
		await menu(chrome);
		expect(screen.queryByRole('menuitem', { name: 'Server and app updates' })).toBeNull();
	});
});
