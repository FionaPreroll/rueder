// The English texts. This catalog sets the shape every other language follows
// (see Messages); a text with values is a function of them.

import { formatNumber as n, plural } from './intl.ts';

const articles = (count: number) =>
	`${n(count)} ${plural(count, { one: 'article', other: 'articles' })}`;

const en = {
	/** The language's own name, for the language picker. */
	languageName: 'English',

	articles,

	common: {
		close: 'Close',
		cancel: 'Cancel',
		save: 'Save',
		delete: 'Delete',
		edit: 'Edit'
	},

	nav: {
		subscribe: 'Subscribe',
		home: 'Home',
		all: 'All items',
		starred: 'Starred items',
		hidden: 'Hidden by filters',
		subscriptions: 'Subscriptions',
		newFolder: 'New folder',
		groupedFolder: 'New folder',
		folderName: 'Name of the folder',
		renameFolder: 'Rename',
		removeFolder: 'Remove the folder, keep what is in it',
		folderActions: (title: string) => `Actions for the folder ${title}`,
		feedName: 'Name of the feed',
		feedNameHint: 'Empty for the feed’s own name',
		renameFeed: 'Rename',
		unsubscribe: 'Unsubscribe',
		feedActions: (title: string) => `Actions for the feed ${title}`,
		unsubscribeTitle: 'Unsubscribe?',
		unsubscribeText: (title: string) =>
			`Unsubscribe from “${title}”? What you read and starred there is lost.`,
		openFolder: 'Open folder',
		closeFolder: 'Close folder',
		openSidebar: 'Show the sidebar',
		closeSidebar: 'Hide the sidebar',
		search: 'Search',
		searchPlaceholder: 'Search these articles',
		settings: 'Settings',
		shortcuts: 'Keyboard shortcuts',
		live: 'Live',
		liveOn: 'New articles arrive by themselves. Click to pause.',
		liveOff: 'Paused. Click to let new articles arrive again.',
		paused: 'Paused',
		account: (name: string) => `Logged in as ${name}`,
		logout: 'Log out',
		unread: (count: number) => `${n(count)} unread`,
		fetchFailed: (reason: string) => `The last fetch failed: ${reason}`,
		waiting: 'Not fetched yet',
		moveHint: 'Drag to sort, or press Alt with the arrow keys',
		tooDeep: (levels: number) => `Folders nest ${levels} levels deep at most.`
	},

	toolbar: {
		refresh: 'Refresh',
		showAll: 'All items',
		showUnread: 'Unread only',
		show: 'Show',
		newestFirst: 'Newest first',
		oldestFirst: 'Oldest first',
		markAllRead: 'Mark all as read',
		markOptions: 'More ways to mark as read',
		markEverything: 'All items',
		markOlderDay: 'Items older than a day',
		markOlderWeek: 'Items older than a week',
		listView: 'List view',
		expandedView: 'Expanded view',
		previous: 'Previous item',
		next: 'Next item',
		newArticles: (count: number) =>
			`${n(count)} new ${plural(count, { one: 'article', other: 'articles' })}`
	},

	list: {
		empty: 'No articles here.',
		emptyUnread: 'Nothing unread here.',
		emptySearch: 'No article matches the search.',
		emptyFeed: 'This feed has no articles yet.',
		waitingFeed:
			'This feed has not been fetched yet. Its articles come with the next round of fetching, within a few minutes.',
		showAll: 'Show all items',
		more: 'Show more',
		star: 'Add star',
		unstar: 'Remove star',
		keepUnread: 'Keep unread',
		openOriginal: 'Open the original',
		from: 'from',
		by: (author: string) => `by ${author}`,
		updated: 'updated',
		updatedTitle: (count: number) =>
			`Changed since it was first fetched; ${n(count)} earlier ${plural(count, { one: 'version', other: 'versions' })}`,
		compare: 'Compare versions',
		hiddenBy: (rule: string) => `Hidden by the rule: ${rule}`,
		loading: 'Loading the text …',
		loadingList: 'Loading …'
	},

	media: {
		title: 'Attachments',
		play: 'Play'
	},

	notice: {
		unreachable: 'The server does not answer. What you see may be out of date.',
		notSaved: 'The server did not take your last change. It was undone here.'
	},

	login: {
		title: 'Log in to rueder',
		name: 'Name',
		password: 'Password',
		submit: 'Log in',
		checking: 'One moment …',
		wrong: 'Wrong name or password.',
		tooMany: (minutes: number) =>
			`Too many wrong tries. Try again in ${n(minutes)} ${plural(minutes, { one: 'minute', other: 'minutes' })}.`,
		unreachable: 'The server does not answer. Try again in a moment.',
		retry: 'Try again',
		or: 'or',
		oidc: 'Log in with single sign-on',
		oidcFailed: 'The login through single sign-on did not work. Try again.',
		oidcDenied: 'The login through single sign-on was cancelled.',
		oidcUnknown: 'No account here belongs to that login. Ask whoever runs this rueder to link it.',
		note: 'There is no sign-up. Accounts are made by whoever runs this rueder.'
	},

	home: {
		newIn: 'New in your subscriptions',
		allRead: 'You have read everything. New articles show up here as they arrive.',
		recentlyStarred: 'Recently starred',
		noStars: 'Nothing starred yet. Press s on an article.',
		problems: 'Feeds with problems',
		tips: 'Tips',
		tipKeys: 'Press ? for all keyboard shortcuts; j and k go through the articles.',
		tipDrag: 'Drag feeds and folders in the sidebar to sort them.',
		tipFilters: 'Filter rules in the settings hide articles by keyword or author.',
		showFeed: (title: string) => `Show ${title}`
	},

	versions: {
		title: 'Versions of this article',
		current: 'Current version',
		version: (rev: number) => `Version ${rev}`,
		fetched: (when: string) => `fetched ${when}`,
		compareWith: 'Compare the current version with',
		legendAdded: 'added',
		legendRemoved: 'removed',
		titleWas: 'The title was:',
		same: 'The text of this version is the same as the current one.'
	},

	settings: {
		title: 'Settings',
		tabs: { look: 'Appearance', reading: 'Reading', rules: 'Filter rules', keys: 'Keyboard' },
		theme: 'Theme',
		themes: { system: 'Like the system', light: 'Light', dark: 'Dark' },
		accent: 'Accent color',
		accents: {
			red: 'Red',
			blue: 'Blue',
			green: 'Green',
			orange: 'Orange',
			purple: 'Purple',
			graphite: 'Graphite'
		},
		density: 'Density of the list',
		densities: { compact: 'Compact', comfortable: 'Comfortable' },
		textSize: 'Text size of articles',
		textSizes: { small: 'Small', medium: 'Medium', large: 'Large' },
		readingFont: 'Font of articles',
		readingFonts: { sans: 'Sans-serif', serif: 'Serif' },
		counters: 'Unread counts',
		counterStyles: { parens: 'In parentheses', badge: 'As badges' },
		language: 'Language',
		languageAuto: 'Like the browser',
		view: 'View',
		views: { list: 'List', expanded: 'Expanded' },
		markOnScroll: 'Mark articles as read while scrolling past them in the expanded view',
		teasers: 'Show the first words of the text after the title in the list',
		note: 'Changes apply at once and are saved with your account, so they follow you to every device.',
		noteDemo: 'Changes apply at once and stay in this browser.'
	},

	rules: {
		intro:
			'Rules hide articles or mark them as read as they arrive. Hidden articles do not count as unread and stay under "Hidden by filters".',
		none: 'No rules yet.',
		add: 'Add a rule',
		new: 'New rule',
		on: 'On',
		scope: 'In',
		scopeAll: 'All feeds',
		scopeFolder: (title: string) => `Folder: ${title}`,
		scopeFeed: (title: string) => `Feed: ${title}`,
		field: 'Look at',
		fields: { any: 'Everything', title: 'Title', content: 'Text', author: 'Author', tag: 'Tag' },
		match: 'Match',
		matches: {
			contains: 'contains',
			word: 'has the word',
			regex: 'matches the regular expression'
		},
		pattern: 'Text to look for',
		action: 'Then',
		actions: { hide: 'hide the article', read: 'mark it as read' },
		preview: (count: number, total: number) =>
			`Applies to ${n(count)} of ${n(total)} articles here now.`,
		invalid: 'This is not a valid regular expression.',
		empty: 'Enter the text to look for.',
		checking: 'Looking what it would catch …',
		hits: (count: number) => `${n(count)} ${plural(count, { one: 'hit', other: 'hits' })}`,
		summary: (field: string, match: string, pattern: string, action: string) =>
			`${field} ${match} “${pattern}”: ${action}`
	},

	keys: {
		title: 'Keyboard shortcuts',
		note: 'Your keys are kept with the settings. Enter, Escape and g stay as they are.',
		change: (what: string) => `Change the key for: ${what}`,
		changeShort: 'Change',
		press: 'Press a key …',
		taken: (key: string, other: string) => `${key} is already the key for: ${other}`,
		fixed: (key: string) => `${key} cannot be given to a shortcut.`,
		reset: "Back to rueder's keys",
		groups: { articles: 'Articles', lists: 'Lists', go: 'Go to', app: 'Everywhere' },
		next: 'Next article, opened',
		previous: 'Previous article, opened',
		nextCursor: 'Next article, without opening',
		previousCursor: 'Previous article, without opening',
		toggle: 'Open or close the article',
		star: 'Add or remove the star',
		read: 'Mark as read or unread',
		original: 'Open the original in a new tab',
		markAll: 'Mark all as read',
		refresh: 'Refresh the list',
		unreadOnly: 'Unread only, or all items',
		listView: 'List view',
		expandedView: 'Expanded view',
		goHome: 'Home',
		goAll: 'All items',
		goStarred: 'Starred items',
		search: 'Search',
		sidebar: 'Show or hide the sidebar',
		help: 'This overview',
		then: 'then'
	},

	labels: {
		title: 'Labels',
		add: 'New label',
		name: 'Name of the label',
		none: 'No labels yet.',
		actions: (title: string) => `Actions for the label ${title}`,
		rename: 'Rename',
		color: 'Color',
		colors: ['Red', 'Orange', 'Yellow', 'Green', 'Teal', 'Blue', 'Purple', 'Gray'],
		remove: 'Delete the label',
		forArticle: 'Labels',
		set: (title: string) => `Label: ${title}`
	},

	subscribe: {
		title: 'Subscribe to a feed',
		address: 'Address of the feed or the website',
		find: 'Find feeds',
		finding: 'Looking …',
		note: 'The draft cannot fetch anything, so the feed stays empty.',
		found: (count: number) => (count === 1 ? 'There is one feed:' : `There are ${n(count)} feeds:`),
		none: 'There is no feed at this address.',
		movedTo: (url: string) => `The site sent the request on to ${url}.`,
		loginWall:
			'Some sites send whoever is not logged in to their login page. Their feeds cannot be read then.',
		subscribed: 'Subscribed',
		show: 'Show',
		untitled: 'Without a name',
		folder: 'Into the folder',
		topLevel: 'No folder',
		add: 'Subscribe',
		adding: 'Subscribing …',
		refused: 'That did not work.',
		serverSays: (reason: string) => `The server says: ${reason}`,
		busy: (seconds: number | null) =>
			seconds
				? `Too many lookups at once. Try again in ${n(seconds)} ${plural(seconds, { one: 'second', other: 'seconds' })}.`
				: 'Too many lookups at once. Try again in a moment.',
		noFetching: 'This server does not fetch feeds.',
		tooLarge: (megabytes: number) => `The file is larger than ${n(megabytes)} MB.`,
		unreachable: 'The server does not answer. Try again in a moment.'
	},

	opml: {
		title: 'Import and export',
		intro: 'Bring your subscriptions from another feed reader as an OPML file, or take them along.',
		import: 'Import an OPML file',
		importing: 'Importing …',
		imported: (feeds: number, folders: number) =>
			`${n(feeds)} new ${plural(feeds, { one: 'feed', other: 'feeds' })} and ${n(folders)} new ${plural(folders, { one: 'folder', other: 'folders' })}.`,
		known: (count: number) =>
			`${n(count)} ${plural(count, { one: 'feed was', other: 'feeds were' })} subscribed already.`,
		skipped: (count: number) =>
			`${n(count)} ${plural(count, { one: 'entry was', other: 'entries were' })} left out, not being feeds that can be read.`,
		later: 'Their articles arrive with the next round of fetching.',
		export: 'Export as OPML'
	},

	demo: {
		badge: 'Draft',
		note: 'A draft with made-up feeds. Nothing is saved on a server.',
		arrive: 'Let an article arrive',
		revise: 'Change the open article',
		revised: 'Update: this paragraph was added to try out the versions.',
		reviseHint: 'Open an article first.'
	}
};

export type Messages = typeof en;
export default en;
