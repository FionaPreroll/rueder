// How the reader looks and behaves for one person: theme, accent color, density,
// text size, view and so on. They are kept in the browser, so the page looks right
// before the server answers, and with a server also with the account, so every
// device shows the same (see the reader's `saveSettings`).

import { languages, useLanguage, type Language } from './i18n/locale.svelte.ts';
import { understoodKeys, type KeyChoices } from './keys.ts';

export const themes = ['system', 'light', 'dark'] as const;
export const accents = ['red', 'blue', 'green', 'orange', 'purple', 'graphite'] as const;
export const densities = ['compact', 'comfortable'] as const;
export const textSizes = ['small', 'medium', 'large'] as const;
export const readingFonts = ['sans', 'serif'] as const;
export const counterStyles = ['parens', 'badge'] as const;
export const views = ['list', 'expanded', 'split', 'magazine'] as const;

export interface Settings {
	theme: (typeof themes)[number];
	accent: (typeof accents)[number];
	density: (typeof densities)[number];
	textSize: (typeof textSizes)[number];
	readingFont: (typeof readingFonts)[number];
	counters: (typeof counterStyles)[number];
	language: 'auto' | Language;
	view: (typeof views)[number];
	markOnScroll: boolean;
	teasers: boolean;
	/** Keep the screen on while the page is in sight (see wakelock.ts). */
	wakeLock: boolean;
	/** The keyboard shortcuts the reader gave keys of their own. */
	keys: KeyChoices;
}

export const defaults: Settings = {
	theme: 'system',
	accent: 'red',
	density: 'compact',
	textSize: 'medium',
	readingFont: 'sans',
	counters: 'parens',
	language: 'auto',
	view: 'list',
	markOnScroll: true,
	teasers: true,
	wakeLock: false,
	keys: {}
};

const KEY = 'rueder.settings';

const choices: { [K in keyof Settings]?: readonly string[] } = {
	theme: themes,
	accent: accents,
	density: densities,
	textSize: textSizes,
	readingFont: readingFonts,
	counters: counterStyles,
	language: ['auto', ...languages],
	view: views
};

/** The stored settings, with the default for everything missing or not understood. */
export function parse(stored: string | null): Settings {
	try {
		return understood(JSON.parse(stored ?? '{}'));
	} catch {
		// Not JSON: the defaults.
		return { ...defaults };
	}
}

/** Settings as somebody stored them, the server for one: what is not understood is the default. */
export function understood(stored: unknown): Settings {
	const found: Record<string, unknown> =
		stored && typeof stored === 'object' && !Array.isArray(stored)
			? (stored as Record<string, unknown>)
			: {};
	const settings: Record<string, unknown> = { ...defaults };
	for (const key of Object.keys(defaults) as (keyof Settings)[]) {
		if (key === 'keys') continue;
		const value = found[key];
		const allowed = choices[key];
		if (allowed ? allowed.includes(value as string) : typeof value === typeof defaults[key]) {
			settings[key] = value;
		}
	}
	settings.keys = understoodKeys(found.keys);
	return settings as unknown as Settings;
}

function load() {
	try {
		return parse(localStorage.getItem(KEY));
	} catch {
		return { ...defaults };
	}
}

export const settings = $state<Settings>(load());

const systemDark = () =>
	typeof matchMedia === 'function' && matchMedia('(prefers-color-scheme: dark)').matches;

/**
 * The color of the bars around an installed app, and of the browser's own bar
 * where it takes one: the top bar's `panel` of each theme. src/app.html sets the
 * same before the first paint.
 */
export const barColors = { light: '#f4f5f7', dark: '#1c1f25' } as const;

/**
 * The app's manifest for each theme. A manifest cannot follow the theme by itself,
 * and some browsers, Firefox on Android among them, color the bars of an installed
 * app only from it.
 */
export const manifests = {
	light: '/manifest.webmanifest',
	dark: '/manifest-dark.webmanifest'
} as const;

/** Gives the browser the colors of the theme for its bars. */
function colorBars(theme: 'light' | 'dark') {
	for (const meta of document.querySelectorAll<HTMLMetaElement>('meta[name="theme-color"]')) {
		meta.removeAttribute('media');
		meta.content = barColors[theme];
	}
	const manifest = document.querySelector<HTMLLinkElement>('link[rel="manifest"]');
	if (manifest && manifest.getAttribute('href') !== manifests[theme]) {
		manifest.setAttribute('href', manifests[theme]);
	}
}

/** Puts the settings on the page and remembers them. Call it again whenever they change. */
export function applySettings(now: Settings = settings) {
	const root = document.documentElement;
	const theme = now.theme === 'system' ? (systemDark() ? 'dark' : 'light') : now.theme;
	root.dataset.theme = theme;
	colorBars(theme);
	root.dataset.accent = now.accent;
	root.dataset.density = now.density;
	root.dataset.textSize = now.textSize;
	root.dataset.readingFont = now.readingFont;
	useLanguage(now.language === 'auto' ? null : now.language);
	try {
		localStorage.setItem(KEY, JSON.stringify(now));
	} catch {
		// Without storage the settings last until the page reloads.
	}
}
