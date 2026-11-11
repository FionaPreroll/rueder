// The catalogs and picking the language.

import { afterEach, describe, expect, it, vi } from 'vitest';
import { shortcutGroups } from '#lib/keys.ts';
import en from './en.ts';
import { catalogs, language, languageNames, languages, m } from './index.ts';
import { formatNumber, plural } from './intl.ts';
import {
	browserLanguage,
	formatLocale,
	formatLocaleFor,
	pickLanguage,
	useLanguage
} from './locale.svelte.ts';

afterEach(() => {
	vi.unstubAllGlobals();
	useLanguage(null);
});

/** Every key of `shape` with the kind of its value, functions with their arity. */
function outline(shape: unknown, path = ''): string[] {
	if (typeof shape === 'function') return [`${path}: function(${shape.length})`];
	if (shape && typeof shape === 'object') {
		return Object.entries(shape).flatMap(([k, v]) => outline(v, path ? `${path}.${k}` : k));
	}
	return [`${path}: ${typeof shape}`];
}

describe('catalogs', () => {
	it('have the shape of the English one', () => {
		for (const code of languages) {
			expect(outline(catalogs[code]), code).toEqual(outline(en));
		}
	});

	it('leave no text empty', () => {
		const texts = (shape: unknown): string[] =>
			typeof shape === 'string'
				? [shape]
				: shape && typeof shape === 'object'
					? Object.values(shape).flatMap(texts)
					: [];
		for (const code of languages) {
			expect(
				texts(catalogs[code]).filter((text) => !text.trim()),
				code
			).toEqual([]);
		}
	});

	it('name themselves for the language picker', () => {
		expect(languageNames()).toEqual([
			{ code: 'en', name: 'English' },
			{ code: 'de', name: 'Deutsch' }
		]);
	});

	it('count in their language', () => {
		expect(m.articles(1)).toBe('1 article');
		expect(m.articles(1200)).toBe('1,200 articles');
		expect(m.toolbar.newArticles(1)).toBe('1 new article');
		expect(m.rules.hits(2)).toBe('2 hits');
		useLanguage('de');
		expect(m.articles(1)).toBe('1 Artikel');
		expect(m.articles(1200)).toBe('1.200 Artikel');
		expect(m.toolbar.newArticles(1)).toBe('1 neuer Artikel');
		expect(m.toolbar.newArticles(3)).toBe('3 neue Artikel');
		expect(m.rules.hits(1)).toBe('1 Treffer');
	});

	it('describe every shortcut once', () => {
		for (const code of languages) {
			const all = shortcutGroups(catalogs[code]).flatMap((group) => group.shortcuts);
			const actions = all.map((shortcut) => shortcut.action);
			expect(new Set(actions).size, code).toBe(actions.length);
			expect(new Set(all.map((shortcut) => shortcut.text)).size, code).toBe(all.length);
		}
	});
});

describe('picking the language', () => {
	it('takes the first language rueder speaks', () => {
		expect(pickLanguage(['fr-FR', 'de-AT', 'en'])).toBe('de');
		expect(pickLanguage(['EN-gb'])).toBe('en');
		expect(pickLanguage(['fr', 'ja'])).toBeNull();
		expect(pickLanguage([])).toBeNull();
	});

	it('formats for the browser’s own variant of the language', () => {
		expect(formatLocaleFor('de', ['en-US', 'de-at'])).toBe('de-AT');
		expect(formatLocaleFor('de', ['en-US'])).toBe('de');
		expect(formatLocaleFor('en', ['en_US'])).toBe('en');
	});

	it('follows the browser until something is picked', () => {
		vi.stubGlobal('navigator', { languages: ['de-CH', 'en'] });
		expect(browserLanguage()).toBe('de');
		useLanguage(null);
		expect(language()).toBe('de');
		expect(formatLocale()).toBe('de-CH');
		expect(document.documentElement.lang).toBe('de');
		useLanguage('en');
		expect(language()).toBe('en');
		expect(formatLocale()).toBe('en');
		useLanguage('tlh');
		expect(language()).toBe('de');
	});

	it('falls back to English', () => {
		vi.stubGlobal('navigator', { language: 'fr-FR' });
		expect(browserLanguage()).toBe('en');
	});
});

describe('numbers and plurals', () => {
	it('follow the language', () => {
		expect(formatNumber(12345)).toBe('12,345');
		expect(plural(1, { one: 'feed', other: 'feeds' })).toBe('feed');
		expect(plural(0, { one: 'feed', other: 'feeds' })).toBe('feeds');
		useLanguage('de');
		expect(formatNumber(12345)).toBe('12.345');
	});
});
