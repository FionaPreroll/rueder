// The catalogs and picking the language.

import { afterEach, describe, expect, it, vi } from 'vitest';
import { shortcutGroups } from '#lib/keys.ts';
import en from './en.ts';
import { catalogs, language, languageNames, languages, m } from './index.ts';
import { formatNumber, plural } from './intl.ts';
import {
	browserLanguage,
	directionOf,
	startLanguage,
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
			{ code: 'de', name: 'Deutsch' },
			{ code: 'es', name: 'Español' },
			{ code: 'fr', name: 'Français' },
			{ code: 'pt-BR', name: 'Português (Brasil)' },
			{ code: 'it', name: 'Italiano' },
			{ code: 'ja', name: '日本語' },
			{ code: 'pl', name: 'Polski' },
			{ code: 'nl', name: 'Nederlands' },
			{ code: 'ko', name: '한국어' },
			{ code: 'tr', name: 'Türkçe' },
			{ code: 'zh-Hans', name: '简体中文' },
			{ code: 'zh-Hant', name: '繁體中文' },
			{ code: 'id', name: 'Bahasa Indonesia' },
			{ code: 'ar', name: 'العربية' },
			{ code: 'ru', name: 'Русский' }
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
		useLanguage('es');
		expect(m.articles(1)).toBe('1 artículo');
		expect(m.toolbar.newArticles(3)).toBe('3 artículos nuevos');
		useLanguage('fr');
		expect(m.articles(0)).toBe('0 article');
		expect(m.articles(2)).toBe('2 articles');
		// Japanese counts without a plural.
		useLanguage('ja');
		expect(m.articles(1)).toBe('1 件の記事');
		expect(m.articles(1200)).toBe('1,200 件の記事');
		// Polish counts in three forms.
		useLanguage('pl');
		expect([1, 2, 5, 22, 25].map((count) => m.articles(count))).toEqual([
			'1 artykuł',
			'2 artykuły',
			'5 artykułów',
			'22 artykuły',
			'25 artykułów'
		]);
		// Arabic says one and two with the noun alone, 3 to 10 in the plural,
		// 11 to 99 in the singular with tanwin, 100 in the singular.
		useLanguage('ar');
		expect([1, 2, 3, 11, 100].map((count) => m.articles(count))).toEqual([
			'مقالة واحدة',
			'مقالتان',
			'3 مقالات',
			'11 مقالة',
			'100 مقالة'
		]);
		expect(m.invites.goodFor(11)).toBe('الرابط صالح لمدة 11 يومًا.');
		expect(m.invites.goodFor(100)).toBe('الرابط صالح لمدة 100 يوم.');
		// Russian counts in three forms, by the last digits.
		useLanguage('ru');
		expect([1, 3, 5, 11, 21, 22, 25].map((count) => m.articles(count))).toEqual([
			'1 статья',
			'3 статьи',
			'5 статей',
			'11 статей',
			'21 статья',
			'22 статьи',
			'25 статей'
		]);
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
		expect(pickLanguage(['tlh-Latn', 'de-AT', 'en'])).toBe('de');
		expect(pickLanguage(['EN-gb'])).toBe('en');
		expect(pickLanguage(['tlh', 'x-pig-latin'])).toBeNull();
		expect(pickLanguage([])).toBeNull();
		// Whatever the region: Portugal gets the Portuguese of Brazil.
		expect(pickLanguage(['pt-PT'])).toBe('pt-BR');
		expect(pickLanguage(['pt'])).toBe('pt-BR');
	});

	it('takes Chinese by its script, else by Taiwan, Hong Kong and Macao', () => {
		const picked = (tag: string) => pickLanguage([tag]);
		expect(['zh-TW', 'zh-HK', 'zh_mo', 'zh-Hant', 'zh-Hant-SG'].map(picked)).toEqual(
			Array(5).fill('zh-Hant')
		);
		expect(['zh', 'zh-CN', 'zh-SG', 'zh-Hans', 'zh-Hans-HK'].map(picked)).toEqual(
			Array(5).fill('zh-Hans')
		);
	});

	it('formats for the browser’s own variant of the language', () => {
		expect(formatLocaleFor('de', ['en-US', 'de-at'])).toBe('de-AT');
		expect(formatLocaleFor('de', ['en-US'])).toBe('de');
		expect(formatLocaleFor('en', ['en_US'])).toBe('en');
		expect(formatLocaleFor('pt-BR', ['pt-PT'])).toBe('pt-PT');
		expect(formatLocaleFor('pt-BR', ['en'])).toBe('pt-BR');
		expect(formatLocaleFor('zh-Hant', ['zh-CN', 'zh-HK'])).toBe('zh-HK');
		expect(formatLocaleFor('zh-Hans', ['zh-TW'])).toBe('zh-Hans');
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

	it('says which way a language is written, and turns the page with it', () => {
		expect(['ar-EG', 'he', 'fa-IR', 'de', 'ja'].map(directionOf)).toEqual([
			'rtl',
			'rtl',
			'rtl',
			'ltr',
			'ltr'
		]);
		useLanguage('ar');
		expect(document.documentElement.dir).toBe('rtl');
		useLanguage('de');
		expect(document.documentElement.dir).toBe('ltr');
	});

	it('falls back to English', () => {
		vi.stubGlobal('navigator', { language: 'tlh-Latn' });
		expect(browserLanguage()).toBe('en');
	});

	it('starts in the language picked, else in the browser’s', () => {
		vi.stubGlobal('navigator', { languages: ['de-AT'] });
		expect(startLanguage('auto')).toBe('de');
		expect(startLanguage('en')).toBe('en');
		vi.stubGlobal('navigator', { languages: ['tlh'] });
		expect(startLanguage('auto')).toBe('en');
		expect(startLanguage('de')).toBe('de');
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
