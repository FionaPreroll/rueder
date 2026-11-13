// The pictures that are taken: every view in each language, on a desktop and
// on a phone, light and dark. Each combination is a project of its own
// (playwright.screenshots.config.ts) with an account of its own, as reading,
// starring and settings would otherwise show from one project in the next.

/** The version of the operator's terms of use that every account is asked to agree to. */
export const TERMS_VERSION = '2026-11';

export interface Shots {
	/** The project's name, also the folder its pictures land in. */
	name: string;
	language: 'en' | 'de';
	phone: boolean;
	theme: 'light' | 'dark';
	/** The account it logs in as. */
	account: string;
}

/** The languages asked for, as `en,de`; both if none are named. */
export function projects(languages?: string): Shots[] {
	const asked = (languages || 'en,de').split(',').map((language) => language.trim());
	const known = (['en', 'de'] as const).filter((language) => asked.includes(language));
	return known.flatMap((language) =>
		[false, true].flatMap((phone) =>
			(['light', 'dark'] as const).map((theme) => {
				const device = phone ? 'phone' : 'desktop';
				return {
					name: `${language}-${device}-${theme}`,
					language,
					phone,
					theme,
					account: `ada-${language}-${device}-${theme}`
				};
			})
		)
	);
}
