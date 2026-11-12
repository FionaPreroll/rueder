// Which requests the service worker answers from its cache: never the reader's.

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { SHELL, filesToKeep, way } from './offline.ts';
import { barColors, manifests } from './settings.svelte.ts';

const origin = 'https://rueder.example';
const get = (path: string, mode = 'cors') => ({ method: 'GET', url: `${origin}${path}`, mode });

describe('way', () => {
	it('keeps the app, and leaves the reader’s data and images to the server', () => {
		expect(way(get('/', 'navigate'), origin)).toBe('shell');
		expect(way(get('/feeds/12', 'navigate'), origin)).toBe('shell');
		expect(way(get('/_app/immutable/entry/app.js'), origin)).toBe('file');
		expect(way(get('/icon-192.png'), origin)).toBe('file');
		for (const path of [
			'/api/tree',
			'/api',
			'/api/oidc/start',
			'/img/abc/image/def',
			'/service-worker.js'
		]) {
			expect(way(get(path, 'navigate'), origin), path).toBe('network');
			expect(way(get(path), origin), path).toBe('network');
		}
		// A page that starts with "api" is still the app.
		expect(way(get('/apiary', 'navigate'), origin)).toBe('shell');
		expect(way({ ...get('/'), method: 'POST' }, origin)).toBe('network');
		expect(way(get('/x'), 'https://elsewhere.example')).toBe('network');
		expect(way({ method: 'GET', url: 'https://cdn.example/a.mp3', mode: 'no-cors' }, origin)).toBe(
			'network'
		);
	});
});

describe('filesToKeep', () => {
	it('names every file from the top of the site, the page first and once', () => {
		expect(filesToKeep(['_app/a.js', '/icon.svg', '_app/a.js', '/'])).toEqual([
			SHELL,
			'/_app/a.js',
			'/icon.svg'
		]);
	});
});

describe('the manifest', () => {
	it('names icons that are there, and the app as a whole', () => {
		const manifest = JSON.parse(readFileSync('static/manifest.webmanifest', 'utf8'));
		expect(manifest).toMatchObject({
			name: 'rueder',
			start_url: '/',
			scope: '/',
			display: 'standalone'
		});
		const icons = manifest.icons as { src: string; sizes: string; purpose: string }[];
		for (const icon of icons) expect(() => readFileSync(`static${icon.src}`)).not.toThrow();
		expect(icons.some((icon) => icon.sizes === '512x512' && icon.purpose === 'maskable')).toBe(
			true
		);
		expect(icons.some((icon) => icon.sizes === '192x192')).toBe(true);
	});

	it('is the same app in the dark theme, in its colors', () => {
		const light = JSON.parse(readFileSync('static/manifest.webmanifest', 'utf8'));
		const dark = JSON.parse(readFileSync('static/manifest-dark.webmanifest', 'utf8'));
		expect({ ...dark, theme_color: '', background_color: '' }).toEqual({
			...light,
			theme_color: '',
			background_color: ''
		});
		expect(light.theme_color).toBe(barColors.light);
		expect(dark.theme_color).toBe(barColors.dark);
		expect(manifests).toEqual({
			light: '/manifest.webmanifest',
			dark: '/manifest-dark.webmanifest'
		});
	});
});
