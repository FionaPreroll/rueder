import { afterEach, describe, expect, it } from 'vitest';
import { language } from '#lib/i18n/index.ts';
import { useLanguage } from '#lib/i18n/locale.svelte.ts';
import { applySettings, defaults, parse } from '#lib/settings.svelte.ts';

afterEach(() => {
	localStorage.clear();
	useLanguage(null);
});

describe('parse', () => {
	it('is the defaults for nothing, and for what is not JSON', () => {
		expect(parse(null)).toEqual(defaults);
		expect(parse('not json')).toEqual(defaults);
		expect(parse('[1, 2]')).toEqual(defaults);
		expect(parse('null')).toEqual(defaults);
	});

	it('keeps what it knows and drops the rest', () => {
		const stored = JSON.stringify({
			theme: 'dark',
			accent: 'pink',
			density: 'comfortable',
			view: 'expanded',
			language: 'de',
			markOnScroll: false,
			teasers: 'yes',
			keys: { next: 'l', previous: 'Enter' },
			unknown: true
		});
		expect(parse(stored)).toEqual({
			...defaults,
			theme: 'dark',
			density: 'comfortable',
			view: 'expanded',
			language: 'de',
			markOnScroll: false,
			keys: { next: 'l' }
		});
		expect(parse(JSON.stringify({ keys: 'j' })).keys).toEqual({});
	});
});

describe('applySettings', () => {
	it('puts the look on the page, sets the language and remembers', () => {
		applySettings({
			...defaults,
			theme: 'dark',
			accent: 'green',
			density: 'comfortable',
			textSize: 'large',
			readingFont: 'serif',
			language: 'de'
		});
		const root = document.documentElement;
		expect(root.dataset).toMatchObject({
			theme: 'dark',
			accent: 'green',
			density: 'comfortable',
			textSize: 'large',
			readingFont: 'serif'
		});
		expect(root.lang).toBe('de');
		expect(language()).toBe('de');
		expect(parse(localStorage.getItem('rueder.settings'))).toMatchObject({
			theme: 'dark',
			accent: 'green',
			language: 'de'
		});
	});

	it('takes the light theme where the system does not say dark', () => {
		applySettings({ ...defaults });
		expect(document.documentElement.dataset.theme).toBe('light');
		expect(language()).toBe('en');
	});

	it('colors the bars of the browser and of the installed app as the theme', () => {
		document.head.innerHTML = `
			<link rel="manifest" href="/manifest.webmanifest" />
			<meta name="theme-color" content="#f4f5f7" media="(prefers-color-scheme: light)" />
			<meta name="theme-color" content="#1c1f25" media="(prefers-color-scheme: dark)" />`;
		const bars = () =>
			[...document.querySelectorAll('meta[name="theme-color"]')].map((meta) => [
				meta.getAttribute('content'),
				meta.getAttribute('media')
			]);
		const manifest = () => document.querySelector('link[rel="manifest"]')?.getAttribute('href');

		applySettings({ ...defaults, theme: 'dark' });
		expect(bars()).toEqual([
			['#1c1f25', null],
			['#1c1f25', null]
		]);
		expect(manifest()).toBe('/manifest-dark.webmanifest');

		applySettings({ ...defaults, theme: 'light' });
		expect(bars()).toEqual([
			['#f4f5f7', null],
			['#f4f5f7', null]
		]);
		expect(manifest()).toBe('/manifest.webmanifest');
		document.head.innerHTML = '';
	});
});
