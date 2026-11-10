// Intl formatters for the current locale (and time zone), made again only when
// that changes. Reading one in a component makes it follow the language.

import { formatLocale, language } from './locale.svelte.ts';

/** A thing made for the current locale, made once per locale. */
export function localized<T>(make: (locale: string) => T): () => T {
	let made: { locale: string; value: T } | undefined;
	return () => {
		const locale = formatLocale();
		if (made?.locale !== locale) made = { locale, value: make(locale) };
		return made.value;
	};
}

const pluralRules = localized(() => new Intl.PluralRules(language()));

/** The plural category of `n` in the page's language: "one", "few", "other" … */
export const pluralOf = (n: number) => pluralRules().select(n);

/** Picks the form for `n`; `other` is the fallback for categories left out. */
export const plural = (
	n: number,
	forms: Partial<Record<Intl.LDMLPluralRule, string>> & { other: string }
) => forms[pluralOf(n)] ?? forms.other;

const numbers = localized((locale) => new Intl.NumberFormat(locale));

/** A number in the current locale, e.g. "1,234" or "1.234". */
export const formatNumber = (n: number) => numbers().format(n);
