// The language of the page: what was picked in the settings,
// else the first of the browser's languages that rueder speaks, else English.
// A language counts whatever its region: a browser in pt-PT gets pt-BR.
// Numbers and dates are formatted for the browser's own locale when it is a
// variant of that language (de-AT, en-GB), else for the language itself.

/** The languages rueder speaks; each has a catalog in this folder (see index.ts). */
export const languages = ['en', 'de', 'es', 'fr', 'pt-BR', 'it'] as const;
export type Language = (typeof languages)[number];

const isLanguage = (code: string): code is Language =>
	(languages as readonly string[]).includes(code);

/** The language of a BCP 47 tag without its region: "de" for "de-AT". */
const primary = (tag: string) => tag.split(/[-_]/)[0].toLowerCase();

/** The language rueder speaks for a tag such as "de-AT" or "pt-PT", if any. */
const spoken = (tag: string) => languages.find((code) => primary(code) === primary(tag));

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
	const variant = locales.find((l) => primary(l) === primary(language));
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
	if (typeof document !== 'undefined') document.documentElement.lang = current.language;
}
