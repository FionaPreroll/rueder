// What the pictures of the README are taken as: on a desktop and on a phone,
// light and dark, each a project of playwright.showcase.config.ts.

import { resolve } from 'node:path';

/** Where the page seems to be served from; the test answers for it from `BUILD`. */
export const DEMO = 'https://rueder.example/';

/** The single-file build of the page (`pnpm build:single`). */
export const BUILD = resolve(import.meta.dirname, '../build');

/** Where the pictures go, as `<picture>-<theme>.png`. */
export const OUT = resolve(import.meta.dirname, 'out');

export interface Look {
	/** The project's name. */
	name: string;
	phone: boolean;
	theme: 'light' | 'dark';
}

export const looks: Look[] = [false, true].flatMap((phone) =>
	(['light', 'dark'] as const).map((theme) => ({
		name: `${phone ? 'phone' : 'desktop'}-${theme}`,
		phone,
		theme
	}))
);
