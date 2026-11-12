// The keyboard shortcuts: what each one does, its keys as Google Reader had them,
// and the keys a reader chose instead (kept with the settings, `settings.keys`).
// A key is what the browser says it is (`KeyboardEvent.key`): "j", "A" for Shift
// and a, "?", "Enter". A key after "g " is the second of two ("g h").

import type { Messages } from './i18n/index.ts';

export const actions = [
	'next',
	'previous',
	'nextCursor',
	'previousCursor',
	'toggle',
	'star',
	'read',
	'original',
	'markAll',
	'refresh',
	'unreadOnly',
	'expandedView',
	'listView',
	'splitView',
	'goHome',
	'goAll',
	'goStarred',
	'search',
	'sidebar',
	'help'
] as const;

export type Action = (typeof actions)[number];

/** The keys of each action as rueder has them. */
export const defaultKeys: Record<Action, string> = {
	next: 'j',
	previous: 'k',
	nextCursor: 'n',
	previousCursor: 'p',
	toggle: 'o',
	star: 's',
	read: 'm',
	original: 'v',
	markAll: 'A',
	refresh: 'r',
	unreadOnly: 'U',
	expandedView: '1',
	listView: '2',
	splitView: '3',
	goHome: 'g h',
	goAll: 'g a',
	goStarred: 'g s',
	search: '/',
	sidebar: 'u',
	help: '?'
};

/** Keys that do one thing always: Enter opens, Escape closes, g starts two keys. */
export const fixedKeys = ['Enter', 'Escape', 'g', 'Tab', ' '];

/** The actions that go somewhere with a second key after g. */
const goActions: readonly Action[] = ['goHome', 'goAll', 'goStarred'];
export const isGo = (action: Action) => goActions.includes(action);

/** The keys a reader chose, by action; an action that is not named keeps its own. */
export type KeyChoices = Partial<Record<Action, string>>;

/** The longest name of a key a browser gives, as "ArrowRight" or "PageDown". */
const MAX_KEY = 16;

/** Whether a key can be given to an action: one key, not one of the fixed ones. */
export function canBeKey(action: Action, key: string) {
	const single = isGo(action) ? key.slice(2) : key;
	if (isGo(action) && !key.startsWith('g ')) return false;
	return (
		single.length > 0 &&
		single.length <= MAX_KEY &&
		!/\s/.test(single) &&
		(isGo(action) || !fixedKeys.includes(single))
	);
}

/** The keys a reader chose, as stored: what is no action or no key is left out. */
export function understoodKeys(stored: unknown): KeyChoices {
	if (!stored || typeof stored !== 'object' || Array.isArray(stored)) return {};
	const choices: KeyChoices = {};
	for (const action of actions) {
		const key = (stored as Record<string, unknown>)[action];
		if (typeof key === 'string' && canBeKey(action, key)) choices[action] = key;
	}
	return choices;
}

/** The key of an action, the reader's own or rueder's. */
export const keyOf = (action: Action, choices: KeyChoices) =>
	choices[action] ?? defaultKeys[action];

/** The action another one would take the key from, if any. */
export function takenBy(action: Action, key: string, choices: KeyChoices): Action | null {
	return actions.find((other) => other !== action && keyOf(other, choices) === key) ?? null;
}

/** What the page looks up: the action of a key alone, and of a second key after g. */
export interface KeyMap {
	single: Map<string, Action>;
	afterG: Map<string, Action>;
}

export function keyMap(choices: KeyChoices): KeyMap {
	const map: KeyMap = { single: new Map(), afterG: new Map() };
	for (const action of actions) {
		const key = keyOf(action, choices);
		if (isGo(action)) map.afterG.set(key.slice(2), action);
		else map.single.set(key, action);
	}
	return map;
}

/** A key as the overview writes it: "Shift+A" for "A", "Space" for " ". */
export function keyName(key: string) {
	if (/^[A-Z]$/.test(key)) return `Shift+${key}`;
	return key === ' ' ? 'Space' : key;
}

export interface Shortcut {
	action: Action;
	text: string;
}

export const shortcutGroups = (m: Messages): { title: string; shortcuts: Shortcut[] }[] => [
	{
		title: m.keys.groups.articles,
		shortcuts: [
			{ action: 'next', text: m.keys.next },
			{ action: 'previous', text: m.keys.previous },
			{ action: 'nextCursor', text: m.keys.nextCursor },
			{ action: 'previousCursor', text: m.keys.previousCursor },
			{ action: 'toggle', text: m.keys.toggle },
			{ action: 'star', text: m.keys.star },
			{ action: 'read', text: m.keys.read },
			{ action: 'original', text: m.keys.original }
		]
	},
	{
		title: m.keys.groups.lists,
		shortcuts: [
			{ action: 'markAll', text: m.keys.markAll },
			{ action: 'refresh', text: m.keys.refresh },
			{ action: 'unreadOnly', text: m.keys.unreadOnly },
			{ action: 'expandedView', text: m.keys.expandedView },
			{ action: 'listView', text: m.keys.listView },
			{ action: 'splitView', text: m.keys.splitView }
		]
	},
	{
		title: m.keys.groups.go,
		shortcuts: [
			{ action: 'goHome', text: m.keys.goHome },
			{ action: 'goAll', text: m.keys.goAll },
			{ action: 'goStarred', text: m.keys.goStarred }
		]
	},
	{
		title: m.keys.groups.app,
		shortcuts: [
			{ action: 'search', text: m.keys.search },
			{ action: 'sidebar', text: m.keys.sidebar },
			{ action: 'help', text: m.keys.help }
		]
	}
];
