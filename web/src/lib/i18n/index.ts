// Texts of the page in its language: `m.settings.title`, `m.articles(3)`. Each
// language has a catalog in this folder with the shape of the English one (en.ts);
// a new language is a new catalog, its entry below and its code in locale.svelte.ts.
// Reading `m` in a component makes it follow a change of language.

import de from './de.ts';
import en, { type Messages } from './en.ts';
import es from './es.ts';
import fr from './fr.ts';
import it from './it.ts';
import ja from './ja.ts';
import ptBR from './pt-BR.ts';
import pl from './pl.ts';
import nl from './nl.ts';
import ko from './ko.ts';
import tr from './tr.ts';
import zhHans from './zh-Hans.ts';
import { language, languages, type Language } from './locale.svelte.ts';

export const catalogs: Record<Language, Messages> = {
	en,
	de,
	es,
	fr,
	'pt-BR': ptBR,
	it,
	ja,
	pl,
	nl,
	ko,
	tr,
	'zh-Hans': zhHans
};

/** The texts in the page's language. */
export const m: Messages = new Proxy({} as Messages, {
	get: (_, key) => catalogs[language()][key as keyof Messages]
});

/** Each language with its own name, for the language picker. */
export const languageNames = () =>
	languages.map((code) => ({ code, name: catalogs[code].languageName }));

export type { Messages };
export { language, languages, type Language };
