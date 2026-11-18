// The language of the page: what was picked in the settings,
// else the first of the browser's languages that rueder speaks, else English.
// A language counts whatever its region: a browser in pt-PT gets pt-BR.
// Chinese goes by its script instead: traditional characters where the tag
// names them or Taiwan, Hong Kong or Macao, else simplified ones.
// Numbers and dates are formatted for the browser's own locale when it is a
// variant of that language (de-AT, en-GB), else for the language itself.

/** The languages rueder speaks; each has a catalog in this folder (see index.ts). */
export const languages = [
	'en',
	'de',
	'es',
	'fr',
	'pt-BR',
	'it',
	'ja',
	'pl',
	'nl',
	'ko',
	'tr',
	'zh-Hans',
	'zh-Hant'
] as const;
export type Language = (typeof languages)[number];

const isLanguage = (code: string): code is Language =>
	(languages as readonly string[]).includes(code);

/** The languages written from right to left, by their primary subtag. */
const RIGHT_TO_LEFT = ['ar', 'fa', 'he', 'ur'];

/** The language of a BCP 47 tag without its region: "de" for "de-AT". */
const primary = (tag: string) => tag.split(/[-_]/)[0].toLowerCase();

/** Which way a language is written. */
export const directionOf = (tag: string): 'ltr' | 'rtl' =>
	RIGHT_TO_LEFT.includes(primary(tag)) ? 'rtl' : 'ltr';

/** The script of Chinese for a tag such as "zh-TW", "zh-Hant-HK" or "zh-CN". */
function chinese(tag: string): Language {
	const subtags = tag.toLowerCase().split(/[-_]/).slice(1);
	const script = subtags.find((subtag) => subtag.length === 4);
	const traditional = script
		? script === 'hant'
		: subtags.some((subtag) => ['tw', 'hk', 'mo'].includes(subtag));
	return traditional ? 'zh-Hant' : 'zh-Hans';
}

/** The language rueder speaks for a tag such as "de-AT", "pt-PT" or "zh-TW", if any. */
const spoken = (tag: string) =>
	primary(tag) === 'zh' ? chinese(tag) : languages.find((code) => primary(code) === primary(tag));

const browserLocales = () =>
	typeof navigator === 'undefined' ? [] : [...(navigator.languages ?? [navigator.language])];

/** The first of `locales` (BCP 47 tags such as "de-AT") that rueder speaks. */
export function pickLanguage(locales: readonly string[]): Language | null {
	for (const locale of locales) {
		const code = spoken(locale);
		if (code) return code;
	}
	return null;
}

/** The locale numbers and dates are formatted in for `language`: the browser's own variant of it, if any. */
export function formatLocaleFor(language: Language, locales: readonly string[]) {
	const variant = locales.find((l) => spoken(l) === language);
	if (!variant) return language;
	try {
		return Intl.getCanonicalLocales(variant)[0];
	} catch {
		return language;
	}
}

/** The language of the browser, or English. */
export const browserLanguage = () => pickLanguage(browserLocales()) ?? 'en';

/** The language a page starts in: the one picked in the settings, or the browser's for "auto". */
export const startLanguage = (picked: 'auto' | Language): Language =>
	picked === 'auto' ? browserLanguage() : picked;

const current = $state(settle(null));

function settle(picked: string | null | undefined) {
	const locales = browserLocales();
	const language = picked && isLanguage(picked) ? picked : browserLanguage();
	return { language, formatLocale: formatLocaleFor(language, locales) };
}

/** The language of the page. Reactive in components. */
export const language = () => current.language;

/** The locale for Intl formatters, e.g. "de-AT". Reactive in components. */
export const formatLocale = () => current.formatLocale;

/** Takes the language the account picked (a code such as "de"), or the browser's for null. */
export function useLanguage(picked: string | null | undefined) {
	Object.assign(current, settle(picked));
	if (typeof document === 'undefined') return;
	document.documentElement.lang = current.language;
	document.documentElement.dir = directionOf(current.language);
}
