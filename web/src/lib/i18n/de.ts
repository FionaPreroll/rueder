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
		groupedFolder: 'Neuer Ordner',
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
		appSettings: 'Server und App-Updates',
		unread: (count) => `${n(count)} ungelesen`,
		fetchFailed: (reason) => `Der letzte Abruf ist fehlgeschlagen: ${reason}`,
		waiting: 'Noch nicht abgerufen',
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
		splitView: 'Drei Bereiche: Baum, Liste und Artikel',
		magazineView: 'Magazin',
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
		waitingFeed:
			'Dieser Feed wurde noch nicht abgerufen. Seine Artikel kommen mit der nächsten Runde des Abrufs, in wenigen Minuten.',
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
		loadingList: 'Wird geladen …',
		toTop: 'Zurück nach oben',
		pick: 'Einen Artikel wählen, um ihn hier zu lesen.',
		closeArticle: 'Artikel schließen'
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
		disabled: 'Dieses Konto ist gesperrt. Frag die Person, die diesen rueder betreibt, warum.',
		removed: 'Dein Konto ist gelöscht, mit allem, was deins war.',
		note: 'Registrieren kann man sich nicht. Konten legt an, wer diesen rueder betreibt.'
	},

	register: {
		title: 'Konto anlegen',
		invited: 'Du wurdest zu diesem rueder eingeladen. Such dir einen Namen und ein Passwort aus.',
		again: 'Das Passwort noch einmal',
		passwordRule: (characters) => `Mindestens ${n(characters)} Zeichen.`,
		mismatch: 'Die beiden Passwörter sind verschieden.',
		submit: 'Konto anlegen',
		toRegister: 'Konto anlegen',
		toLogin: 'Bei einem vorhandenen Konto anmelden',
		closed: 'Ohne Einladung kann sich hier niemand registrieren.',
		inviteGone: 'Diese Einladung gilt nicht mehr. Bitte um eine neue.',
		full: 'Hier ist kein Platz für ein weiteres Konto.',
		today: 'Für heute wurden hier genug Konten angelegt. Versuch es morgen wieder.',
		taken: 'Ein Konto mit diesem Namen gibt es schon.',
		invalid: (reason) => `Das kann kein Konto werden. Der Server sagt: ${reason}`
	},

	invites: {
		intro:
			'Mit einem Link kann jemand ein Konto auf diesem rueder anlegen. Er wird einmal gezeigt, hier, wenn du ihn erzeugst.',
		left: (count) =>
			count === 0
				? 'Du hast keine Einladungen zu vergeben.'
				: `Du kannst noch ${n(count)} ${plural(count, { one: 'Person', other: 'Personen' })} einladen.`,
		admin: 'Als Admin kannst du immer einladen.',
		uses: 'Für wie viele Konten',
		make: 'Link erzeugen',
		noneLeft: 'Für so viele hast du keine Einladungen mehr.',
		failed: 'Das hat nicht geklappt. Versuch es gleich noch einmal.',
		link: 'Der Link, zum Weitergeben jetzt',
		copy: 'Kopieren',
		copied: 'Kopiert',
		goodFor: (days) => `Ein Link gilt ${n(days)} ${plural(days, { one: 'Tag', other: 'Tage' })}.`,
		yours: 'Deine Links',
		none: 'Du hast noch keine Links erzeugt.',
		used: (uses, of) => `${n(uses)} von ${n(of)} benutzt`,
		until: (when) => `gilt bis ${when}`,
		ran: (when) => `abgelaufen ${when}`,
		usedUp: 'aufgebraucht',
		revoked: 'zurückgezogen',
		revoke: 'Zurückziehen',
		revokeLink: (made) => `Den Link zurückziehen, der ${made} erzeugt wurde`
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
		tabs: {
			look: 'Aussehen',
			reading: 'Lesen',
			rules: 'Filterregeln',
			keys: 'Tastatur',
			invites: 'Einladungen',
			privacy: 'Datenschutz',
			move: 'Umzug',
			account: 'Konto'
		},
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
		views: { list: 'Liste', expanded: 'Erweitert', split: 'Drei Bereiche', magazine: 'Magazin' },
		markOnScroll: 'Artikel in der erweiterten Ansicht beim Vorbeiscrollen als gelesen markieren',
		teasers: 'In der Liste nach dem Titel die ersten Worte des Textes zeigen',
		wakeLock: 'Bildschirm anlassen, solange rueder offen ist',
		wakeLockMissing: 'Dieser Browser kann den Bildschirm nicht anlassen.',
		note: 'Änderungen gelten sofort, liegen bei deinem Konto und gelten auf jedem Gerät.',
		noteDemo: 'Änderungen gelten sofort und bleiben in diesem Browser.',
		reports: {
			send: 'Fehlerberichte aus diesem Browser an diesen Server schicken',
			intro:
				'Wenn auf der Seite etwas schiefgeht, erfahren die Admins davon und können es beheben. Berichte gehen an diesen Server und nirgendwo sonst hin.',
			what: 'Ein Bericht enthält:',
			holds: [
				'die Meldung des Fehlers, ohne Adressen und ohne Zahlen, die etwas benennen',
				'wo in den eigenen Dateien von rueder er auftrat',
				'die Version von rueder und deinen Browser mit seiner Hauptversion'
			],
			never:
				'Nie, was du liest, deine Feeds, Labels oder Regeln. Admins sehen, wie viele Konten ein Fehler trifft, nicht welche. Berichte verschwinden nach 30 Tagen.',
			off: 'Ausschalten löscht die Berichte, die du geschickt hast.'
		}
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
		note: 'Deine Tasten liegen bei den Einstellungen. Enter, Escape und g bleiben, wie sie sind.',
		change: (what) => `Taste ändern für: ${what}`,
		changeShort: 'Ändern',
		press: 'Taste drücken …',
		taken: (key, other) => `${key} ist schon die Taste für: ${other}`,
		fixed: (key) => `${key} lässt sich keinem Kürzel geben.`,
		reset: 'Zurück zu den Tasten von rueder',
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
		splitView: 'Drei Bereiche',
		magazineView: 'Magazin',
		goHome: 'Startseite',
		goAll: 'Alle Artikel',
		goStarred: 'Markierte Artikel',
		search: 'Suchen',
		sidebar: 'Seitenleiste ein- oder ausblenden',
		help: 'Diese Übersicht',
		then: 'dann'
	},

	labels: {
		title: 'Labels',
		add: 'Neues Label',
		name: 'Name des Labels',
		none: 'Noch keine Labels.',
		actions: (title) => `Aktionen für das Label ${title}`,
		rename: 'Umbenennen',
		color: 'Farbe',
		colors: ['Rot', 'Orange', 'Gelb', 'Grün', 'Türkis', 'Blau', 'Lila', 'Grau'],
		remove: 'Label löschen',
		forArticle: 'Labels',
		set: (title) => `Label: ${title}`
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
		movedTo: (url) => `Die Website hat die Anfrage weitergeleitet an ${url}.`,
		loginWall:
			'Manche Websites schicken alle, die nicht eingeloggt sind, auf ihre Login-Seite. Ihre Feeds lassen sich dann nicht lesen.',
		subscribed: 'Abonniert',
		show: 'Zeigen',
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

	removeAccount: {
		title: 'Konto löschen',
		intro:
			'Damit löschst du dein Konto auf diesem rueder endgültig. Das lässt sich nicht rückgängig machen.',
		goes: 'Mit ihm gehen: deine Sitzungen auf allen Geräten, deine Abos und Ordner, was du gelesen hast, Sterne, Labels, Filterregeln, Einstellungen, Fehlerberichte, Einladungen und die Verknüpfung mit Single Sign-on.',
		stays:
			'Feeds bleiben für die anderen, die sie lesen. Willst du dein Konto vorher mitnehmen, nimm „Umzug“.',
		password: 'Dein Passwort',
		name: (name) => `Gib zur Bestätigung deinen Namen ein, ${name}`,
		submit: 'Mein Konto endgültig löschen',
		wrong: 'Das stimmt nicht. Gelöscht wurde nichts.',
		busy: (minutes) =>
			`Zu viele falsche Versuche. Versuch es in ${n(minutes)} ${plural(minutes, { one: 'Minute', other: 'Minuten' })} wieder.`,
		unreachable: 'Der Server antwortet nicht. Gelöscht wurde nichts.'
	},

	move: {
		intro:
			'Nimm dein Konto zu einem anderen rueder mit, oder hol es von dort hierher: Einstellungen, Ordner und Feeds, was du gelesen hast, Sterne, Labels und Filterregeln, alles in einer Datei.',
		howItWorks:
			'Ein Import fügt zu dem hinzu, was hier ist, und nimmt nichts weg. Feeds werden an ihrer Adresse erkannt, Artikel an ihrer Kennung im Feed, denn die Nummern sind von rueder zu rueder verschieden. Deine Einstellungen werden die aus der Datei.',
		notInIt:
			'Nicht in der Datei: dein Passwort, deine Sitzungen, die Zustimmung zu Fehlerberichten und die Texte der Artikel. Einen Artikel, den dieser rueder nicht kennt, kann der Import deshalb nicht übernehmen.',
		export: 'Konto exportieren',
		import: 'Kontodatei importieren',
		importing: 'Importiere …',
		feeds: (feeds, known) =>
			`${n(feeds)} ${plural(feeds, { one: 'neuer Feed', other: 'neue Feeds' })}, ${n(known)} schon abonniert.`,
		folders: (count) =>
			`${n(count)} ${plural(count, { one: 'neuer Ordner', other: 'neue Ordner' })}.`,
		articles: (found) => `${n(found)} Artikel hier gefunden.`,
		missing: (count) =>
			`${n(count)} Artikel ${plural(count, { one: 'war', other: 'waren' })} hier nicht zu finden; was du davon gelesen, mit Stern oder mit Labels versehen hattest, geht verloren.`,
		marks: (labels, rules) =>
			`${n(labels)} ${plural(labels, { one: 'neues Label', other: 'neue Labels' })} und ${n(rules)} ${plural(rules, { one: 'neue Filterregel', other: 'neue Filterregeln' })}.`,
		skipped: (count) =>
			`${n(count)} ${plural(count, { one: 'Eintrag', other: 'Einträge' })} der Datei ließen sich nicht übernehmen.`,
		settings: 'Deine Einstellungen sind jetzt die aus der Datei.',
		later:
			'Feeds, die dieser rueder noch nicht hatte, bringen ihre Artikel mit der nächsten Runde des Abrufs. Was du vorher gelesen hattest, bleibt gelesen.',
		busy: (seconds) =>
			seconds
				? `Zu viele Importe. Versuch es in ${n(Math.ceil(seconds / 60))} ${plural(Math.ceil(seconds / 60), { one: 'Minute', other: 'Minuten' })} wieder.`
				: 'Zu viele Importe. Versuch es später noch einmal.'
	},

	status: {
		title: 'Serverstatus',
		open: 'Serverstatus',
		loading: 'Frage den Server …',
		failed: 'Der Server antwortet nicht. Versuch es gleich noch einmal.',
		refresh: 'Aktualisieren',
		asOf: (time) => `Stand ${time}. Gezählt wird die letzte Stunde.`,
		demo: 'Erfunden, denn hier gibt es keinen Server.',
		empty: '–',
		on: 'An',
		off: 'Aus',
		took: (median, p95) => `Median ${median}, 95 % in ${p95}`,
		hintsTitle: 'Einen Blick wert',
		allWell: 'Nichts fällt auf.',
		hints: {
			slowRoute: (route, took) =>
				`${route} ist langsam: Jede zwanzigste Anfrage braucht länger als ${took}.`,
			serverErrors: (route, count) =>
				`${route} hat ${n(count)}-mal mit einem Fehler des Servers geantwortet.`,
			poolBusy: (size) =>
				`Alle ${n(size)} Verbindungen zur Datenbank sind belegt, Anfragen warten auf eine.`,
			fetchBacklog: (count, late) =>
				`${n(count)} ${plural(count, { one: 'Feed wird', other: 'Feeds werden' })} zu spät abgerufen, der späteste um ${late}.`,
			feedsFailing: (failing, subscribed) =>
				`${n(failing)} von ${n(subscribed)} Feeds schlagen fehl.`,
			notFetching: 'Der Server ruft keine Feeds ab. Steht FETCH auf off?',
			fetchPaceFull: (most) =>
				`Jede Runde nimmt so viele Feeds, wie sie darf, und trotzdem warten Feeds: Der Takt erlaubt ${n(most)} Abrufe je Stunde. FETCH_ROUND, FETCH_AT_ONCE und FETCH_EVERY stellen ihn ein.`,
			cacheMisses: (share) =>
				`Die Datenbank findet nur ${share} dessen, was sie liest, im Speicher; mehr shared_buffers würden helfen.`,
			longQuery: (took) => `Eine Abfrage läuft seit ${took}.`,
			imageCacheFull: (share) => `Der Bild-Cache ist zu ${share} voll.`,
			errors: (count) => `Der Server hat in der letzten Stunde ${n(count)} Fehler geloggt.`,
			limited: (count) =>
				`${n(count)} ${plural(count, { one: 'Anfrage wurde', other: 'Anfragen wurden' })} als zu viele abgewiesen.`,
			browserErrors: (count) =>
				`Browser haben am letzten Tag ${n(count)} ${plural(count, { one: 'Fehler', other: 'verschiedene Fehler' })} gemeldet.`
		},
		server: {
			title: 'Server',
			version: 'Version',
			running: 'Läuft seit',
			memory: 'Speicher',
			accounts: 'Konten',
			feeds: 'Feeds',
			feedsValue: (subscribed, all) => `${n(subscribed)} abonniert, ${n(all)} insgesamt`,
			articles: 'Artikel',
			articlesValue: (count) => `etwa ${n(count)}`,
			live: 'Offene Tabs'
		},
		database: {
			title: 'Datenbank',
			pool: 'Verbindungen von rueder',
			poolValue: (size, idle, max) => `${n(size)} offen, ${n(idle)} frei, höchstens ${n(max)}`,
			connections: 'Alle Verbindungen',
			size: 'Größe',
			hitRatio: 'Aus dem Speicher gelesen',
			deadlocks: 'Deadlocks',
			longest: 'Längste laufende Abfrage',
			tables: 'Größte Tabellen',
			table: 'Tabelle',
			bytes: 'Größe',
			rows: 'Zeilen, etwa'
		},
		fetcher: {
			title: 'Abruf der Feeds',
			state: 'Abruf',
			lastRound: 'Letzte Runde',
			lastRoundValue: (ago, feeds, took) => `${ago}: ${n(feeds)} Feeds in ${took}`,
			fetches: 'Abrufe',
			fetchesValue: (fetched, unchanged, failed) =>
				`${n(fetched)} mit Inhalt, ${n(unchanged)} unverändert, ${n(failed)} fehlgeschlagen`,
			broken: 'An der Datenbank gescheitert',
			newArticles: 'Neue Artikel',
			took: 'Zeit je Abruf',
			overdue: 'Zu spät abgerufen',
			overdueValue: (count, late) => `${n(count)}, der späteste um ${late}`,
			waiting: 'Noch nie abgerufen',
			pace: 'Takt',
			paceValue: (round, every, atOnce, most) =>
				`${n(round)} Feeds alle ${every}, ${n(atOnce)} gleichzeitig: höchstens ${n(most)} je Stunde`,
			perHour: 'Abrufe in der letzten Stunde',
			behind: 'Rückstand',
			behindValue: (late) => `der Feed, der am längsten wartet, ist ${late} über seiner Zeit`,
			notBehind: 'Nichts wartet',
			failing: 'Fehlschlagende Feeds, nach Grund',
			noneFailing: 'Kein Feed schlägt fehl.',
			feeds: (count) => `${n(count)} ${plural(count, { one: 'Feed', other: 'Feeds' })}`,
			failures: {
				status: (code) =>
					code ? `Die Seite antwortet mit ${code}` : 'Die Seite antwortet mit einem Fehler',
				timeout: 'Die Seite braucht zu lange',
				blocked: 'Die Adresse liegt in einem Netz, aus dem rueder nicht abruft',
				dns: 'Der Name hat keine Adresse',
				network: 'Keine Verbindung, oder sie bricht ab',
				too_large: 'Zu groß',
				redirects: 'Zu viele Weiterleitungen, oder eine kaputte',
				unreadable: 'Kein Feed, den rueder lesen kann',
				other: 'Etwas anderes'
			}
		},
		finder: {
			title: 'Feed-Suche',
			state: 'Adressen ansehen',
			lookups: 'Suchen',
			lookupsValue: (found, none, failed) =>
				`${n(found)} mit Feeds, ${n(none)} ohne, ${n(failed)} fehlgeschlagen`,
			limited: 'Über dem Limit des Kontos',
			took: 'Zeit je Suche'
		},
		images: {
			title: 'Bild-Proxy',
			state: 'Proxy',
			cache: 'Cache',
			cacheValue: (used, max) => `${used} von ${max}`,
			served: 'Bilder',
			servedValue: (cached, fetched) => `${n(cached)} aus dem Cache, ${n(fetched)} abgerufen`,
			refused: 'Abgelehnt',
			refusedValue: (source, notImage, tooLarge, notFound) =>
				`${n(source)} Quelle fehlgeschlagen, ${n(notImage)} kein Bild, ${n(tooLarge)} zu groß, ${n(notFound)} falsche Adresse`,
			took: 'Zeit je abgerufenem Bild',
			failingSources: 'Vorerst gemiedene Quellen'
		},
		requests: {
			title: 'Anfragen',
			all: 'Alle Anfragen',
			allValue: (count, took) => `${n(count)}, ${took}`,
			answers: 'Antworten',
			answersValue: (fine, refused, errors) =>
				`${n(fine)} in Ordnung, ${n(refused)} abgelehnt, ${n(errors)} Fehler des Servers`,
			route: 'Route',
			count: 'Anzahl',
			median: 'Median',
			p95: '95 %',
			max: 'Längste',
			errors: 'Fehler',
			none: 'Keine Anfragen in der letzten Stunde.'
		},
		problems: {
			title: 'Warnungen und Fehler',
			counts: (errors, warnings) =>
				`In der letzten Stunde ${n(errors)} Fehler und ${n(warnings)} ${plural(warnings, { one: 'Warnung', other: 'Warnungen' })}.`,
			none: 'Seit dem Start wurde nichts geloggt.',
			error: 'Fehler',
			warning: 'Warnung',
			times: (count, last) => (count > 1 ? `${n(count)}-mal, zuletzt ${last}` : last)
		},
		browsers: {
			title: 'Fehler in Browsern',
			off: 'Dieser Server nimmt keine Fehlerberichte an (CLIENT_REPORTS=off).',
			consenting: (count) =>
				`${n(count)} ${plural(count, { one: 'Konto schickt', other: 'Konten schicken' })} Berichte. Fehler bleiben 30 Tage nach ihrem letzten Auftreten.`,
			none: 'Kein Browser hat einen Fehler gemeldet.',
			kinds: {
				error: 'Fehler',
				rejection: 'Fehlgeschlagenes Promise',
				component: 'Teil der Seite',
				request: 'Anfrage'
			},
			hits: (count, accounts, last) =>
				`${n(count)}-mal in ${n(accounts)} ${plural(accounts, { one: 'Konto', other: 'Konten' })}, zuletzt ${last}`,
			where: (versions, browsers) => `Version ${versions} · ${browsers}`
		}
	},

	update: {
		ready: 'Eine neue Version von rueder ist da.',
		reload: 'Neu laden',
		later: 'Später'
	},

	crash: {
		title: 'Auf dieser Seite ist etwas schiefgegangen.',
		text: 'Lade die Seite neu, um weiterzumachen. Was du gelesen und geändert hast, ist auf dem Server gespeichert.',
		reload: 'Neu laden'
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
