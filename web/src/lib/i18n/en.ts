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
		reportFeed: 'Report the feed …',
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
		appSettings: 'Server and app updates',
		unread: (count: number) => `${n(count)} unread`,
		fetchFailed: (reason: string) => `The last fetch failed: ${reason}`,
		waiting: 'Not fetched yet',
		blocked: 'Blocked by the operator of this rueder',
		moveHint: 'Drag to sort, or press Alt with the arrow keys',
		resizeSidebar: 'How wide the sidebar is: drag, or use the arrow keys',
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
		splitView: 'Three panes: tree, list and article',
		magazineView: 'Magazine',
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
		loadingList: 'Loading …',
		toTop: 'Back to the top',
		pick: 'Pick an article to read it here.',
		resizeList: 'How wide the list is: drag, or use the arrow keys',
		closeArticle: 'Close the article',
		blockedFeed:
			'The operator of this rueder blocked this feed. It is not fetched, and its articles are not shown.',
		articleActions: 'More actions for the article',
		reportArticle: 'Report the article …'
	},

	report: {
		feedTitle: 'Report the feed',
		articleTitle: 'Report the article',
		text: (title: string) =>
			`The operator of this rueder sees that “${title}” was reported, why and what you write, but not who reported it. They decide whether to block it for everyone.`,
		reason: 'Why',
		reasons: {
			illegal: 'Against the law',
			copyright: 'Somebody else’s work',
			malicious: 'Malware, phishing or fraud',
			spam: 'Spam',
			other: 'Something else'
		},
		note: 'What you would like to add (optional)',
		left: (count: number) => `${n(count)} characters left`,
		send: 'Send the report',
		sending: 'Sending …',
		sent: 'Thank you. The report reached the operator of this rueder.',
		tooMany: 'You sent many reports lately. Try again later.',
		unreachable: 'The report did not reach the server. Try again later.'
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
		codeNeeded: 'This account has a second factor. Enter the code your app shows.',
		wrongCode: 'That code is not right. Codes change every 30 seconds.',
		code: 'Code',
		codeNote: 'The six digits from your authenticator app, or one of your recovery codes.',
		tooMany: (minutes: number) =>
			`Too many wrong tries. Try again in ${n(minutes)} ${plural(minutes, { one: 'minute', other: 'minutes' })}.`,
		unreachable: 'The server does not answer. Try again in a moment.',
		retry: 'Try again',
		or: 'or',
		oidc: 'Log in with single sign-on',
		oidcFailed: 'The login through single sign-on did not work. Try again.',
		oidcDenied: 'The login through single sign-on was cancelled.',
		oidcUnknown: 'No account here belongs to that login. Ask whoever runs this rueder to link it.',
		disabled: 'This account is disabled. Ask whoever runs this rueder why.',
		removed: 'Your account is deleted, with everything that was yours.',
		note: 'There is no sign-up. Accounts are made by whoever runs this rueder.'
	},

	legal: {
		imprint: 'Imprint',
		privacy: 'Privacy policy',
		terms: 'Terms of use',
		agree: 'I agree to the terms of use',
		changed: 'The terms of use have changed',
		changedIntro: 'Please read the new terms of use. To go on using rueder, agree to them.',
		read: 'Read the terms of use',
		accept: 'I agree',
		decline: 'Log out',
		notSaved: 'That did not work. Try again in a moment.'
	},

	register: {
		title: 'Make an account',
		invited: 'You were invited to this rueder. Pick a name and a password.',
		again: 'The password once more',
		passwordRule: (characters: number) => `At least ${n(characters)} characters.`,
		mismatch: 'The two passwords differ.',
		submit: 'Make the account',
		toRegister: 'Make an account',
		toLogin: 'Log in to an account you have',
		closed: 'Nobody can register here without an invitation.',
		inviteGone: 'This invitation cannot be used any more. Ask for a new one.',
		full: 'There is no room for another account here.',
		today: 'Enough accounts were made here for one day. Try again tomorrow.',
		taken: 'There is an account with this name already.',
		terms: 'Agree to the terms of use to make an account.',
		invalid: (reason: string) => `That cannot be an account. The server says: ${reason}`
	},

	invites: {
		intro:
			'A link lets somebody make an account on this rueder. It is shown once, here, when you make it.',
		left: (count: number) =>
			count === 0
				? 'You have no invitations to give.'
				: `You can invite ${n(count)} more ${plural(count, { one: 'person', other: 'people' })}.`,
		admin: 'As an admin you can always invite.',
		uses: 'For how many accounts',
		make: 'Make a link',
		noneLeft: 'You have no invitations left for that many.',
		failed: 'That did not work. Try again in a moment.',
		link: 'The link, to pass on now',
		copy: 'Copy',
		copied: 'Copied',
		goodFor: (days: number) =>
			`A link is good for ${n(days)} ${plural(days, { one: 'day', other: 'days' })}.`,
		yours: 'Your links',
		none: 'You have made no links yet.',
		used: (uses: number, of: number) => `${n(uses)} of ${n(of)} used`,
		until: (when: string) => `good until ${when}`,
		ran: (when: string) => `ran out ${when}`,
		usedUp: 'used up',
		revoked: 'taken back',
		revoke: 'Take back',
		revokeLink: (made: string) => `Take back the link made ${made}`
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
		tabs: {
			look: 'Appearance',
			reading: 'Reading',
			rules: 'Filter rules',
			keys: 'Keyboard',
			invites: 'Invitations',
			privacy: 'Privacy',
			security: 'Security',
			apps: 'Apps',
			move: 'Move',
			account: 'Account'
		},
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
		textSize: 'Text size of articles, on this device',
		textSizes: { small: 'Small', medium: 'Medium', large: 'Large' },
		readingFont: 'Font of articles',
		readingFonts: { sans: 'Sans-serif', serif: 'Serif' },
		counters: 'Unread counts',
		counterStyles: { parens: 'In parentheses', badge: 'As badges' },
		language: 'Language',
		languageAuto: 'Like the browser',
		view: 'View',
		views: { list: 'List', expanded: 'Expanded', split: 'Three panes', magazine: 'Magazine' },
		markOnScroll: 'Mark articles as read while scrolling past them in the expanded view',
		teasers: 'Show the first words of the text after the title in the list',
		wakeLock: 'Keep the screen on while rueder is open',
		wakeLockMissing: 'This browser cannot keep the screen on.',
		note: 'Changes apply at once and are saved with your account, so they follow you to every device.',
		noteDemo: 'Changes apply at once and stay in this browser.',
		apps: {
			intro:
				'Feed reader apps of other makers can read and change what you read here, through the Google Reader API. Each app gets an app password of its own, which you can revoke at any time; your own password stays yours.',
			none: 'No app is set up yet.',
			made: (when: string) => `set up ${when}`,
			used: (when: string) => `last used ${when}`,
			unused: 'not used yet',
			revoke: 'Revoke',
			revokeName: (name: string) => `Revoke the app password ${name}`,
			add: 'Set up an app',
			full: (max: number) =>
				`An account has at most ${n(max)} app passwords. Revoke one to set up another app.`,
			choose: 'Which app do you want to use?',
			website: (app: string) => `Website of ${app}`,
			other: 'Another app',
			otherHint: 'Any app that offers the Google Reader API or FreshRSS.',
			platforms: {
				apple: 'Mac, iPhone and iPad',
				android: 'Android'
			},
			name: 'Name of the app password',
			password: 'Your password',
			code: 'Code of your authenticator app, or a recovery code',
			create: 'Create the app password',
			creating: 'One moment …',
			back: 'Back',
			how: {
				netnewswire:
					'In NetNewsWire, add an account (Settings, Accounts, then +) and choose FreshRSS. Enter the server address, name and app password below.',
				readyou:
					'In Read You, open Settings, Accounts, Add accounts, and choose Google Reader. Enter the server address, name and app password below.',
				other:
					'In your app, add an account of the kind “Google Reader API” or, where there is none, “FreshRSS”. Enter the server address, name and app password below.'
			},
			server: 'Server address',
			user: 'Name',
			secret: 'App password',
			copy: (what: string) => `Copy ${what}`,
			copied: 'Copied',
			once: 'The app password is shown only now. Should you lose it, revoke it and make a new one.',
			waiting: 'Waiting for the app to log in …',
			connected: 'Connected: the app has logged in.',
			done: 'Done',
			refusals: {
				wrongPassword: 'The password is not right.',
				codeNeeded: 'Enter a code of your authenticator app as well.',
				wrongCode: 'That code is not right. Codes change every 30 seconds.',
				tooManyApps: 'An account has at most 20 app passwords.',
				tooMany: 'Too many wrong tries. Try again in a few minutes.',
				unreachable: 'The server does not answer. Try again in a moment.'
			}
		},
		secondFactor: {
			title: 'Second factor',
			intro:
				'With a second factor, logging in takes a code from an authenticator app on your phone besides the password, so a password that got out is not enough.',
			on: 'On.',
			left: (count: number) =>
				`${n(count)} ${plural(count, { one: 'recovery code', other: 'recovery codes' })} left.`,
			off: 'Off.',
			adminNeeds:
				'This server makes admins admins only with a second factor. Set it up, or log in again with its code, to see the server status.',
			password: 'Your password',
			start: 'Set up',
			scan: 'Scan this code with your authenticator app, or type in the key below.',
			qr: 'QR code for the authenticator app',
			larger: 'Show the QR code larger',
			smaller: 'Show the QR code smaller',
			key: 'Key',
			copyKey: 'Copy the key',
			copied: 'Copied',
			code: 'The code the app shows now',
			codeOrRecovery: 'Code of the app, or a recovery code',
			confirm: 'Confirm',
			cancel: 'Cancel',
			codesTitle: 'Your recovery codes',
			codesIntro:
				'Each of them logs you in once when you do not have your phone. Keep them somewhere safe: they are shown only now.',
			copyCodes: 'Copy the codes',
			codesDone: 'I have kept them',
			switchOff: 'Switch off',
			switchOffNote: 'Logging in then takes the password alone again.',
			refusals: {
				wrongPassword: 'The password is not right.',
				wrongCode: 'That code is not right. Codes change every 30 seconds.',
				tooMany: 'Too many wrong tries. Try again in a few minutes.',
				unreachable: 'The server does not answer. Try again in a moment.'
			}
		},
		reports: {
			send: 'Send reports of errors in this browser to this server',
			intro:
				'When something on the page goes wrong, its admins learn of it and can fix it. Reports go to this server and nowhere else.',
			what: 'A report holds:',
			holds: [
				'the message of the error, without addresses and without numbers that name something',
				'where in rueder’s own files it happened',
				'the version of rueder and your browser with its major version'
			],
			never:
				'Never what you read, your feeds, labels or rules. Admins see how many accounts an error hits, not which. Reports go after 30 days.',
			off: 'Switching this off deletes the reports you sent.'
		}
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
		splitView: 'Three panes',
		magazineView: 'Magazine',
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
		show: 'Show the labels',
		hide: 'Hide the labels',
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

	removeAccount: {
		title: 'Delete the account',
		intro: 'This deletes your account on this rueder for good. It cannot be undone.',
		goes: 'What goes with it: your sessions on all devices, your subscriptions and folders, what you have read, stars, labels, filter rules, settings, reports of errors, invitations and the link to single sign-on.',
		stays:
			'Feeds stay for the others who read them. To take your account elsewhere first, use “Move”.',
		password: 'Your password',
		name: (name: string) => `Type your name, ${name}, to confirm`,
		code: 'Code from your app, or a recovery code',
		submit: 'Delete my account for good',
		wrong: 'That is not right. Nothing was deleted.',
		codeNeeded: 'Your account has a second factor. Type a code from your app as well.',
		busy: (minutes: number) =>
			`Too many wrong tries. Try again in ${n(minutes)} ${plural(minutes, { one: 'minute', other: 'minutes' })}.`,
		unreachable: 'The server does not answer. Nothing was deleted.'
	},

	move: {
		intro:
			'Take your account to another rueder, or bring it here from one: settings, folders and feeds, what you have read, stars, labels and filter rules, all in one file.',
		howItWorks:
			'An import adds to what is here and takes nothing away. Feeds are found by their address and articles by their id in the feed, as the numbers differ from one rueder to the next. Your settings become those of the file.',
		notInIt:
			'Not in the file: your password, your sessions, the consent to reports of errors and the texts of articles. So an article this rueder does not know cannot be brought in.',
		export: 'Export the account',
		import: 'Import an account file',
		importing: 'Importing …',
		feeds: (feeds: number, known: number) =>
			`${n(feeds)} new ${plural(feeds, { one: 'feed', other: 'feeds' })}, ${n(known)} subscribed already.`,
		folders: (count: number) =>
			`${n(count)} new ${plural(count, { one: 'folder', other: 'folders' })}.`,
		articles: (found: number) => `${articles(found)} found here.`,
		missing: (count: number) =>
			`${articles(count)} could not be found here; what you had read, starred or labelled of them is lost.`,
		marks: (labels: number, rules: number) =>
			`${n(labels)} new ${plural(labels, { one: 'label', other: 'labels' })} and ${n(rules)} new filter ${plural(rules, { one: 'rule', other: 'rules' })}.`,
		skipped: (count: number) =>
			`${n(count)} ${plural(count, { one: 'entry', other: 'entries' })} of the file could not be taken.`,
		settings: 'Your settings are those of the file now.',
		later:
			'Feeds this rueder did not have bring their articles with the next round of fetching. What you had read before stays read.',
		busy: (seconds: number | null) =>
			seconds
				? `Too many imports. Try again in ${n(Math.ceil(seconds / 60))} ${plural(Math.ceil(seconds / 60), { one: 'minute', other: 'minutes' })}.`
				: 'Too many imports. Try again later.'
	},

	status: {
		title: 'Server status',
		open: 'Server status',
		loading: 'Asking the server …',
		failed: 'The server did not answer. Try again in a moment.',
		refresh: 'Refresh',
		asOf: (time: string) => `As of ${time}. What is counted is the last hour.`,
		demo: 'Made up, as there is no server here.',
		empty: '–',
		on: 'On',
		off: 'Off',
		took: (median: string, p95: string) => `median ${median}, 95% within ${p95}`,
		hintsTitle: 'Worth a look',
		allWell: 'Nothing stands out.',
		hints: {
			slowRoute: (route: string, took: string) =>
				`${route} is slow: one request in twenty takes longer than ${took}.`,
			serverErrors: (route: string, count: number) =>
				`${route} answered with an error of the server ${n(count)} ${plural(count, { one: 'time', other: 'times' })}.`,
			poolBusy: (size: number) =>
				`All ${n(size)} connections to the database are in use, so requests wait for one.`,
			fetchBacklog: (count: number, late: string) =>
				`${n(count)} ${plural(count, { one: 'feed is', other: 'feeds are' })} fetched late, the latest by ${late}.`,
			feedsFailing: (failing: number, subscribed: number) =>
				`${n(failing)} of ${n(subscribed)} feeds fail.`,
			notFetching: 'The server does not fetch feeds. Is FETCH set to off?',
			fetchPaceFull: (most: number) =>
				`Every round takes as many feeds as it may, and feeds still wait: the pace allows ${n(most)} fetches an hour. FETCH_ROUND, FETCH_AT_ONCE and FETCH_EVERY set it.`,
			cacheMisses: (share: string) =>
				`The database finds only ${share} of what it reads in memory; more shared_buffers would help.`,
			longQuery: (took: string) => `A query has been running for ${took}.`,
			imageCacheFull: (share: string) => `The image cache is ${share} full.`,
			errors: (count: number) =>
				`The server logged ${n(count)} ${plural(count, { one: 'error', other: 'errors' })} in the last hour.`,
			limited: (count: number) =>
				`${n(count)} ${plural(count, { one: 'request was', other: 'requests were' })} turned away as too many.`,
			browserErrors: (count: number) =>
				`Browsers reported ${n(count)} ${plural(count, { one: 'error', other: 'different errors' })} in the last day.`,
			reportedFeeds: (count: number) =>
				`Readers reported ${n(count)} ${plural(count, { one: 'feed', other: 'feeds' })} that ${plural(count, { one: 'is', other: 'are' })} not blocked.`
		},
		server: {
			title: 'Server',
			version: 'Version',
			running: 'Running since',
			memory: 'Memory',
			accounts: 'Accounts',
			feeds: 'Feeds',
			feedsValue: (subscribed: number, all: number) =>
				`${n(subscribed)} subscribed, ${n(all)} in all`,
			articles: 'Articles',
			articlesValue: (count: number) => `about ${n(count)}`,
			live: 'Open tabs'
		},
		database: {
			title: 'Database',
			pool: 'Connections of rueder',
			poolValue: (size: number, idle: number, max: number) =>
				`${n(size)} open, ${n(idle)} idle, at most ${n(max)}`,
			connections: 'All connections',
			size: 'Size',
			hitRatio: 'Read from memory',
			deadlocks: 'Deadlocks',
			longest: 'Longest running query',
			tables: 'Largest tables',
			table: 'Table',
			bytes: 'Size',
			rows: 'Rows, about'
		},
		fetcher: {
			title: 'Fetching feeds',
			state: 'Fetching',
			lastRound: 'Last round',
			lastRoundValue: (ago: string, feeds: number, took: string) =>
				`${ago}: ${n(feeds)} ${plural(feeds, { one: 'feed', other: 'feeds' })} in ${took}`,
			fetches: 'Fetches',
			fetchesValue: (fetched: number, unchanged: number, failed: number) =>
				`${n(fetched)} brought the feed, ${n(unchanged)} unchanged, ${n(failed)} failed`,
			broken: 'Failed at the database',
			newArticles: 'New articles',
			took: 'Time per fetch',
			overdue: 'Fetched late',
			overdueValue: (count: number, late: string) => `${n(count)}, the latest by ${late}`,
			waiting: 'Not fetched yet',
			pace: 'Pace',
			paceValue: (round: number, every: string, atOnce: number, most: number) =>
				`${n(round)} ${plural(round, { one: 'feed', other: 'feeds' })} every ${every}, ${n(atOnce)} at once: at most ${n(most)} an hour`,
			perHour: 'Fetches in the last hour',
			behind: 'Behind',
			behindValue: (late: string) => `the feed waiting longest is ${late} past its time`,
			notBehind: 'Nothing is waiting',
			failing: 'Failing feeds, by why',
			noneFailing: 'No feed fails.',
			feeds: (count: number) => `${n(count)} ${plural(count, { one: 'feed', other: 'feeds' })}`,
			failures: {
				status: (code: number | null) =>
					code ? `The site answers ${code}` : 'The site answers with an error',
				timeout: 'The site takes too long',
				blocked: 'The address is in a network rueder does not fetch from',
				dns: 'The name has no address',
				network: 'No connection, or it broke off',
				too_large: 'Too large',
				redirects: 'Too many redirects, or a broken one',
				unreadable: 'Not a feed rueder can read',
				other: 'Something else'
			}
		},
		finder: {
			title: 'Feed search',
			state: 'Looking at addresses',
			lookups: 'Lookups',
			lookupsValue: (found: number, none: number, failed: number) =>
				`${n(found)} found feeds, ${n(none)} found none, ${n(failed)} failed`,
			limited: 'Over the limit of the account',
			took: 'Time per lookup'
		},
		images: {
			title: 'Image proxy',
			state: 'Proxy',
			cache: 'Cache',
			cacheValue: (used: string, max: string) => `${used} of ${max}`,
			served: 'Images',
			servedValue: (cached: number, fetched: number) =>
				`${n(cached)} from the cache, ${n(fetched)} fetched`,
			refused: 'Refused',
			refusedValue: (source: number, notImage: number, tooLarge: number, notFound: number) =>
				`${n(source)} source failed, ${n(notImage)} no image, ${n(tooLarge)} too large, ${n(notFound)} wrong address`,
			took: 'Time per fetched image',
			failingSources: 'Sources left alone for now'
		},
		requests: {
			title: 'Requests',
			all: 'All requests',
			allValue: (count: number, took: string) => `${n(count)}, ${took}`,
			answers: 'Answers',
			answersValue: (fine: number, refused: number, errors: number) =>
				`${n(fine)} fine, ${n(refused)} refused, ${n(errors)} errors of the server`,
			route: 'Route',
			count: 'Count',
			median: 'Median',
			p95: '95%',
			max: 'Longest',
			errors: 'Errors',
			none: 'No requests in the last hour.'
		},
		problems: {
			title: 'Warnings and errors',
			counts: (errors: number, warnings: number) =>
				`In the last hour ${n(errors)} ${plural(errors, { one: 'error', other: 'errors' })} and ${n(warnings)} ${plural(warnings, { one: 'warning', other: 'warnings' })}.`,
			none: 'Nothing was logged since the start.',
			error: 'Error',
			warning: 'Warning',
			times: (count: number, last: string) => (count > 1 ? `${n(count)} times, last ${last}` : last)
		},
		browsers: {
			title: 'Errors in browsers',
			off: 'This server takes no reports of errors (CLIENT_REPORTS=off).',
			consenting: (count: number) =>
				`${n(count)} ${plural(count, { one: 'account sends', other: 'accounts send' })} reports. Errors stay 30 days after they last happened.`,
			none: 'No browser reported an error.',
			kinds: {
				error: 'Error',
				rejection: 'Failed promise',
				component: 'Part of the page',
				request: 'Request'
			},
			hits: (count: number, accounts: number, last: string) =>
				`${n(count)} ${plural(count, { one: 'time', other: 'times' })} in ${n(accounts)} ${plural(accounts, { one: 'account', other: 'accounts' })}, last ${last}`,
			where: (versions: string, browsers: string) => `Version ${versions} · ${browsers}`
		},
		reported: {
			title: 'Reported feeds',
			text: 'Who reported is not shown here. “rueder feed block <address>” blocks a feed for everyone, “rueder feed unblock <address>” lifts it.',
			none: 'Nobody reported a feed.',
			reports: (count: number, last: string) =>
				`${n(count)} ${plural(count, { one: 'report', other: 'reports' })}, last ${last}`,
			blocked: 'Blocked'
		}
	},

	update: {
		ready: 'A new version of rueder is ready.',
		reload: 'Reload',
		later: 'Later'
	},

	crash: {
		title: 'Something went wrong on this page.',
		text: 'Reload the page to go on. What you read and changed is saved on the server.',
		reload: 'Reload'
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
