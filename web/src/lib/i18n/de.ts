// Die deutschen Texte, in der Form des englischen Katalogs (en.ts). Angeredet
// wird mit „du“; „Feed“ und „Artikel“ bleiben, „Item“ heißt hier „Artikel“.

import type { Messages } from './en.ts';
import { formatNumber as n, plural } from './intl.ts';

const de: Messages = {
	languageName: 'Deutsch',

	articles: (count) => `${n(count)} Artikel`,

	common: {
		close: 'Schließen',
		cancel: 'Abbrechen',
		save: 'Speichern',
		delete: 'Löschen',
		edit: 'Bearbeiten'
	},

	nav: {
		subscribe: 'Abonnieren',
		home: 'Startseite',
		all: 'Alle Artikel',
		starred: 'Markierte Artikel',
		hidden: 'Von Filtern ausgeblendet',
		subscriptions: 'Abonnements',
		newFolder: 'Neuer Ordner',
		folderName: 'Name des Ordners',
		renameFolder: 'Umbenennen',
		removeFolder: 'Ordner entfernen, Inhalt behalten',
		folderActions: (title) => `Aktionen für den Ordner ${title}`,
		feedName: 'Name des Feeds',
		feedNameHint: 'Leer für den eigenen Namen des Feeds',
		renameFeed: 'Umbenennen',
		unsubscribe: 'Abbestellen',
		feedActions: (title) => `Aktionen für den Feed ${title}`,
		unsubscribeTitle: 'Abbestellen?',
		unsubscribeText: (title) =>
			`„${title}“ abbestellen? Was du dort gelesen und markiert hast, geht verloren.`,
		openFolder: 'Ordner aufklappen',
		closeFolder: 'Ordner zuklappen',
		openSidebar: 'Seitenleiste einblenden',
		closeSidebar: 'Seitenleiste ausblenden',
		search: 'Suchen',
		searchPlaceholder: 'In diesen Artikeln suchen',
		settings: 'Einstellungen',
		shortcuts: 'Tastaturkürzel',
		live: 'Live',
		liveOn: 'Neue Artikel kommen von selbst an. Klick zum Anhalten.',
		liveOff: 'Angehalten. Klick, damit wieder neue Artikel ankommen.',
		paused: 'Angehalten',
		account: (name) => `Angemeldet als ${name}`,
		logout: 'Abmelden',
		unread: (count) => `${n(count)} ungelesen`,
		fetchFailed: (reason) => `Der letzte Abruf ist fehlgeschlagen: ${reason}`,
		moveHint: 'Zum Sortieren ziehen, oder Alt mit den Pfeiltasten drücken',
		tooDeep: (levels) => `Ordner lassen sich höchstens ${levels} Ebenen tief verschachteln.`
	},

	toolbar: {
		refresh: 'Aktualisieren',
		showAll: 'Alle Artikel',
		showUnread: 'Nur ungelesene',
		show: 'Anzeigen',
		newestFirst: 'Neueste zuerst',
		oldestFirst: 'Älteste zuerst',
		markAllRead: 'Alle als gelesen markieren',
		markOptions: 'Weitere Arten, als gelesen zu markieren',
		markEverything: 'Alle Artikel',
		markOlderDay: 'Artikel, die älter als ein Tag sind',
		markOlderWeek: 'Artikel, die älter als eine Woche sind',
		listView: 'Listenansicht',
		expandedView: 'Erweiterte Ansicht',
		previous: 'Vorheriger Artikel',
		next: 'Nächster Artikel',
		newArticles: (count) =>
			`${n(count)} ${plural(count, { one: 'neuer Artikel', other: 'neue Artikel' })}`
	},

	list: {
		empty: 'Hier sind keine Artikel.',
		emptyUnread: 'Hier ist nichts ungelesen.',
		emptySearch: 'Kein Artikel passt zur Suche.',
		emptyFeed: 'Dieser Feed hat noch keine Artikel.',
		showAll: 'Alle Artikel anzeigen',
		more: 'Mehr anzeigen',
		star: 'Markierung hinzufügen',
		unstar: 'Markierung entfernen',
		keepUnread: 'Als ungelesen behalten',
		openOriginal: 'Original öffnen',
		from: 'aus',
		by: (author) => `von ${author}`,
		updated: 'aktualisiert',
		updatedTitle: (count) =>
			`Seit dem ersten Abruf geändert; ${n(count)} ${plural(count, { one: 'frühere Version', other: 'frühere Versionen' })}`,
		compare: 'Versionen vergleichen',
		hiddenBy: (rule) => `Ausgeblendet von der Regel: ${rule}`,
		loading: 'Der Text wird geladen …',
		loadingList: 'Wird geladen …'
	},

	media: {
		title: 'Anhänge',
		play: 'Abspielen'
	},

	notice: {
		unreachable: 'Der Server antwortet nicht. Was du siehst, ist vielleicht nicht mehr aktuell.',
		notSaved:
			'Der Server hat deine letzte Änderung nicht angenommen. Sie wurde hier zurückgenommen.'
	},

	login: {
		title: 'Bei rueder anmelden',
		name: 'Name',
		password: 'Passwort',
		submit: 'Anmelden',
		checking: 'Einen Moment …',
		wrong: 'Name oder Passwort stimmt nicht.',
		tooMany: (minutes) =>
			`Zu viele falsche Versuche. Versuch es in ${n(minutes)} ${plural(minutes, { one: 'Minute', other: 'Minuten' })} wieder.`,
		unreachable: 'Der Server antwortet nicht. Versuch es gleich noch einmal.',
		retry: 'Noch einmal versuchen',
		or: 'oder',
		oidc: 'Mit Single Sign-on anmelden',
		oidcFailed: 'Die Anmeldung über Single Sign-on hat nicht geklappt. Versuch es noch einmal.',
		oidcDenied: 'Die Anmeldung über Single Sign-on wurde abgebrochen.',
		oidcUnknown:
			'Zu dieser Anmeldung gehört hier kein Konto. Bitte die Person, die diesen rueder betreibt, es zu verknüpfen.',
		note: 'Registrieren kann man sich nicht. Konten legt an, wer diesen rueder betreibt.'
	},

	home: {
		newIn: 'Neu in meinen Abonnements',
		allRead: 'Du hast alles gelesen. Neue Artikel erscheinen hier, sobald sie ankommen.',
		recentlyStarred: 'Zuletzt markiert',
		noStars: 'Noch nichts markiert. Drück s auf einem Artikel.',
		problems: 'Feeds mit Problemen',
		tips: 'Tipps',
		tipKeys: 'Mit ? siehst du alle Tastaturkürzel; j und k gehen durch die Artikel.',
		tipDrag: 'Zieh Feeds und Ordner in der Seitenleiste, um sie zu sortieren.',
		tipFilters: 'Filterregeln in den Einstellungen blenden Artikel nach Stichwort oder Autor aus.',
		showFeed: (title) => `${title} anzeigen`
	},

	versions: {
		title: 'Versionen dieses Artikels',
		current: 'Aktuelle Version',
		version: (rev) => `Version ${rev}`,
		fetched: (when) => `abgerufen ${when}`,
		compareWith: 'Aktuelle Version vergleichen mit',
		legendAdded: 'hinzugefügt',
		legendRemoved: 'entfernt',
		titleWas: 'Der Titel lautete:',
		same: 'Der Text dieser Version ist derselbe wie der aktuelle.'
	},

	settings: {
		title: 'Einstellungen',
		tabs: { look: 'Aussehen', reading: 'Lesen', rules: 'Filterregeln', keys: 'Tastatur' },
		theme: 'Farbschema',
		themes: { system: 'Wie das System', light: 'Hell', dark: 'Dunkel' },
		accent: 'Akzentfarbe',
		accents: {
			red: 'Rot',
			blue: 'Blau',
			green: 'Grün',
			orange: 'Orange',
			purple: 'Violett',
			graphite: 'Graphit'
		},
		density: 'Dichte der Liste',
		densities: { compact: 'Kompakt', comfortable: 'Luftig' },
		textSize: 'Schriftgröße der Artikel',
		textSizes: { small: 'Klein', medium: 'Mittel', large: 'Groß' },
		readingFont: 'Schrift der Artikel',
		readingFonts: { sans: 'Serifenlos', serif: 'Mit Serifen' },
		counters: 'Ungelesen-Zahlen',
		counterStyles: { parens: 'In Klammern', badge: 'Als Plaketten' },
		language: 'Sprache',
		languageAuto: 'Wie der Browser',
		view: 'Ansicht',
		views: { list: 'Liste', expanded: 'Erweitert' },
		markOnScroll: 'Artikel in der erweiterten Ansicht beim Vorbeiscrollen als gelesen markieren',
		teasers: 'In der Liste nach dem Titel die ersten Worte des Textes zeigen',
		note: 'Änderungen gelten sofort und bleiben in diesem Browser. Später liegen sie bei deinem Konto und gelten auf jedem Gerät.'
	},

	rules: {
		intro:
			'Regeln blenden Artikel aus oder markieren sie als gelesen, sobald sie ankommen. Ausgeblendete Artikel zählen nicht als ungelesen und bleiben unter „Von Filtern ausgeblendet“.',
		none: 'Noch keine Regeln.',
		add: 'Regel hinzufügen',
		new: 'Neue Regel',
		on: 'An',
		scope: 'In',
		scopeAll: 'Alle Feeds',
		scopeFolder: (title) => `Ordner: ${title}`,
		scopeFeed: (title) => `Feed: ${title}`,
		field: 'Feld',
		fields: { any: 'Alles', title: 'Titel', content: 'Text', author: 'Autor', tag: 'Schlagwort' },
		match: 'Vergleich',
		matches: {
			contains: 'enthält',
			word: 'enthält das Wort',
			regex: 'passt zum regulären Ausdruck'
		},
		pattern: 'Gesuchter Text',
		action: 'Dann',
		actions: { hide: 'den Artikel ausblenden', read: 'als gelesen markieren' },
		preview: (count, total) => `Trifft jetzt auf ${n(count)} von ${n(total)} Artikeln hier zu.`,
		invalid: 'Das ist kein gültiger regulärer Ausdruck.',
		empty: 'Gib den gesuchten Text ein.',
		checking: 'Es wird nachgesehen, was die Regel träfe …',
		hits: (count) => `${n(count)} Treffer`,
		summary: (field, match, pattern, action) => `${field} ${match} „${pattern}“: ${action}`
	},

	keys: {
		title: 'Tastaturkürzel',
		later: 'Die Belegung wird änderbar, sobald die Einstellungen auf dem Server liegen.',
		groups: { articles: 'Artikel', lists: 'Listen', go: 'Gehe zu', app: 'Überall' },
		next: 'Nächster Artikel, geöffnet',
		previous: 'Vorheriger Artikel, geöffnet',
		nextCursor: 'Nächster Artikel, ohne ihn zu öffnen',
		previousCursor: 'Vorheriger Artikel, ohne ihn zu öffnen',
		toggle: 'Artikel öffnen oder schließen',
		star: 'Markierung setzen oder entfernen',
		read: 'Als gelesen oder ungelesen markieren',
		original: 'Original in einem neuen Tab öffnen',
		markAll: 'Alle als gelesen markieren',
		refresh: 'Liste aktualisieren',
		unreadOnly: 'Nur ungelesene oder alle Artikel',
		listView: 'Listenansicht',
		expandedView: 'Erweiterte Ansicht',
		goHome: 'Startseite',
		goAll: 'Alle Artikel',
		goStarred: 'Markierte Artikel',
		search: 'Suchen',
		sidebar: 'Seitenleiste ein- oder ausblenden',
		help: 'Diese Übersicht',
		then: 'dann'
	},

	subscribe: {
		title: 'Feed abonnieren',
		address: 'Adresse des Feeds oder der Website',
		find: 'Feeds suchen',
		finding: 'Suche …',
		note: 'Der Entwurf kann nichts abrufen, der Feed bleibt deshalb leer.',
		found: (count) =>
			count === 1 ? 'Dort gibt es einen Feed:' : `Dort gibt es ${n(count)} Feeds:`,
		none: 'Unter dieser Adresse gibt es keinen Feed.',
		untitled: 'Ohne Namen',
		folder: 'In den Ordner',
		topLevel: 'Kein Ordner',
		add: 'Abonnieren',
		adding: 'Abonniere …',
		refused: 'Das hat nicht geklappt.',
		serverSays: (reason) => `Der Server sagt: ${reason}`,
		busy: (seconds) =>
			seconds
				? `Zu viele Abfragen auf einmal. Versuch es in ${n(seconds)} ${plural(seconds, { one: 'Sekunde', other: 'Sekunden' })} wieder.`
				: 'Zu viele Abfragen auf einmal. Versuch es gleich noch einmal.',
		noFetching: 'Dieser Server ruft keine Feeds ab.',
		tooLarge: (megabytes) => `Die Datei ist größer als ${n(megabytes)} MB.`,
		unreachable: 'Der Server antwortet nicht. Versuch es gleich noch einmal.'
	},

	opml: {
		title: 'Import und Export',
		intro: 'Bring deine Abos aus einem anderen Feedreader als OPML-Datei mit, oder nimm sie mit.',
		import: 'OPML-Datei importieren',
		importing: 'Importiere …',
		imported: (feeds, folders) =>
			`${n(feeds)} ${plural(feeds, { one: 'neuer Feed', other: 'neue Feeds' })} und ${n(folders)} ${plural(folders, { one: 'neuer Ordner', other: 'neue Ordner' })}.`,
		known: (count) =>
			`${n(count)} ${plural(count, { one: 'Feed war', other: 'Feeds waren' })} schon abonniert.`,
		skipped: (count) =>
			`${n(count)} ${plural(count, { one: 'Eintrag wurde', other: 'Einträge wurden' })} ausgelassen, weil sie keine lesbaren Feeds sind.`,
		later: 'Ihre Artikel kommen mit der nächsten Runde des Abrufs.',
		export: 'Als OPML exportieren'
	},

	demo: {
		badge: 'Entwurf',
		note: 'Ein Entwurf mit erfundenen Feeds. Nichts wird auf einem Server gespeichert.',
		arrive: 'Einen Artikel ankommen lassen',
		revise: 'Den offenen Artikel ändern',
		revised: 'Nachtrag: Dieser Absatz wurde ergänzt, um die Versionen auszuprobieren.',
		reviseHint: 'Öffne zuerst einen Artikel.'
	}
};

export default de;
