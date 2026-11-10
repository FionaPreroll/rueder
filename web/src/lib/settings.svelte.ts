// How the reader looks and behaves for one person: theme, accent color, density,
// text size, view and so on. The draft keeps them in the browser; later they are
// saved with the account, so every device shows the same.

import { languages, useLanguage, type Language } from './i18n/locale.svelte.ts';

export const themes = ['system', 'light', 'dark'] as const;
export const accents = ['red', 'blue', 'green', 'orange', 'purple', 'graphite'] as const;
export const densities = ['compact', 'comfortable'] as const;
export const textSizes = ['small', 'medium', 'large'] as const;
export const readingFonts = ['sans', 'serif'] as const;
export const counterStyles = ['parens', 'badge'] as const;
export const views = ['list', 'expanded'] as const;

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
	teasers: true
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
	let found: Record<string, unknown> = {};
	try {
		const parsed: unknown = JSON.parse(stored ?? '{}');
		if (parsed && typeof parsed === 'object') found = parsed as Record<string, unknown>;
	} catch {
		// Not JSON: the defaults.
	}
	const settings: Record<string, unknown> = { ...defaults };
	for (const key of Object.keys(defaults) as (keyof Settings)[]) {
		const value = found[key];
		const allowed = choices[key];
		if (allowed ? allowed.includes(value as string) : typeof value === typeof defaults[key]) {
			settings[key] = value;
		}
	}
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

/** Puts the settings on the page and remembers them. Call it again whenever they change. */
export function applySettings(now: Settings = settings) {
	const root = document.documentElement;
	root.dataset.theme = now.theme === 'system' ? (systemDark() ? 'dark' : 'light') : now.theme;
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
