// The keyboard shortcuts, for the overview. They follow Google Reader's where it had
// one. A space between keys means one after the other, a plus means together; the
// page (routes/+page.svelte) is what acts on them.

import type { Messages } from './i18n/index.ts';

export interface Shortcut {
	/** Ways to trigger it, e.g. ['o', 'Enter'] or ['g h'] or ['Shift+A']. */
	keys: string[];
	text: string;
}

export const shortcutGroups = (m: Messages): { title: string; shortcuts: Shortcut[] }[] => [
	{
		title: m.keys.groups.articles,
		shortcuts: [
			{ keys: ['j'], text: m.keys.next },
			{ keys: ['k'], text: m.keys.previous },
			{ keys: ['n'], text: m.keys.nextCursor },
			{ keys: ['p'], text: m.keys.previousCursor },
			{ keys: ['o', 'Enter'], text: m.keys.toggle },
			{ keys: ['s'], text: m.keys.star },
			{ keys: ['m'], text: m.keys.read },
			{ keys: ['v'], text: m.keys.original }
		]
	},
	{
		title: m.keys.groups.lists,
		shortcuts: [
			{ keys: ['Shift+A'], text: m.keys.markAll },
			{ keys: ['r'], text: m.keys.refresh },
			{ keys: ['Shift+U'], text: m.keys.unreadOnly },
			{ keys: ['1'], text: m.keys.expandedView },
			{ keys: ['2'], text: m.keys.listView }
		]
	},
	{
		title: m.keys.groups.go,
		shortcuts: [
			{ keys: ['g h'], text: m.keys.goHome },
			{ keys: ['g a'], text: m.keys.goAll },
			{ keys: ['g s'], text: m.keys.goStarred }
		]
	},
	{
		title: m.keys.groups.app,
		shortcuts: [
			{ keys: ['/'], text: m.keys.search },
			{ keys: ['u'], text: m.keys.sidebar },
			{ keys: ['?'], text: m.keys.help }
		]
	}
];
