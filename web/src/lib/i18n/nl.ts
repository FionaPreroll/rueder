// The Dutch texts, in the shape of the English catalog (en.ts), addressing the
// reader with “je”. “Feed” stays as it is; an item is an “artikel”.

import type { Messages } from './en.ts';
import { formatNumber as n, plural } from './intl.ts';

const articles = (count: number) =>
	`${n(count)} ${plural(count, { one: 'artikel', other: 'artikelen' })}`;

const minutes = (count: number) =>
	`${n(count)} ${plural(count, { one: 'minuut', other: 'minuten' })}`;

const nl: Messages = {
	languageName: 'Nederlands',

	articles,

	common: {
		close: 'Sluiten',
		cancel: 'Annuleren',
		save: 'Opslaan',
		delete: 'Verwijderen',
		edit: 'Bewerken'
	},

	nav: {
		subscribe: 'Abonneren',
		home: 'Start',
		all: 'Alle artikelen',
		unreadItems: 'Ongelezen artikelen',
		starred: 'Artikelen met ster',
		hidden: 'Verborgen door filters',
		subscriptions: 'Abonnementen',
		newFolder: 'Nieuwe map',
		groupedFolder: 'Nieuwe map',
		folderName: 'Naam van de map',
		renameFolder: 'Naam wijzigen',
		removeFolder: 'Map verwijderen, inhoud houden',
		folderActions: (title) => `Acties voor de map ${title}`,
		feedName: 'Naam van de feed',
		feedNameHint: 'Leeg voor de eigen naam van de feed',
		renameFeed: 'Naam wijzigen',
		unsubscribe: 'Abonnement opzeggen',
		reportFeed: 'Feed melden …',
		feedInfo: 'Feedinformatie …',
		feedActions: (title) => `Acties voor de feed ${title}`,
		unsubscribeTitle: 'Abonnement opzeggen?',
		unsubscribeText: (title) =>
			`Abonnement op ‘${title}’ opzeggen? Wat je daar gelezen en met een ster gemarkeerd hebt, gaat verloren.`,
		openFolder: 'Map openen',
		closeFolder: 'Map sluiten',
		openSidebar: 'Zijbalk tonen',
		closeSidebar: 'Zijbalk verbergen',
		search: 'Zoeken',
		searchPlaceholder: 'Zoeken in deze artikelen',
		clearSearch: 'Zoekopdracht wissen',
		settings: 'Instellingen',
		accountSettings: 'Accountinstellingen',
		shortcuts: 'Sneltoetsen',
		live: 'Live',
		liveOn: 'Nieuwe artikelen komen vanzelf binnen. Klik om te pauzeren.',
		liveOff: 'Gepauzeerd. Klik om nieuwe artikelen weer binnen te laten komen.',
		paused: 'Gepauzeerd',
		account: (name) => `Ingelogd als ${name}`,
		logout: 'Uitloggen',
		appSettings: 'Server en app-updates',
		install: 'Installeren als app',
		installSafari:
			'Om rueder als app te installeren: Archief › Voeg toe aan Dock op een Mac, Deel › Zet op beginscherm op een iPhone of iPad.',
		unread: (count) => `${n(count)} ongelezen`,
		hiddenCount: (count) => `${n(count)} verborgen door filters`,
		fetchFailed: (reason) => `Het laatste ophalen is mislukt: ${reason}`,
		waiting: 'Nog niet opgehaald',
		blocked: 'Geblokkeerd door de beheerder van deze rueder',
		moveHint: 'Sleep om te sorteren, of druk op Alt met de pijltjestoetsen',
		resizeSidebar: 'Breedte van de zijbalk: sleep of gebruik de pijltjestoetsen',
		tooDeep: (levels) => `Mappen kunnen hoogstens ${n(levels)} niveaus diep genest worden.`
	},

	toolbar: {
		refresh: 'Vernieuwen',
		showAll: 'Alle artikelen',
		showUnread: 'Alleen ongelezen',
		show: 'Tonen',
		newestFirst: 'Nieuwste eerst',
		oldestFirst: 'Oudste eerst',
		markAllRead: 'Alles als gelezen markeren',
		markOptions: 'Meer manieren om als gelezen te markeren',
		phoneMenu: (shown) => `Weergave, toont: ${shown}`,
		markMenu: 'Als gelezen markeren …',
		markEverything: 'Alle artikelen',
		markOlderDay: 'Artikelen ouder dan een dag',
		markOlderWeek: 'Artikelen ouder dan een week',
		listView: 'Lijstweergave',
		expandedView: 'Uitgebreide weergave',
		splitView: 'Drie panelen: boom, lijst en artikel',
		magazineView: 'Magazine',
		previous: 'Vorig artikel',
		next: 'Volgend artikel',
		newArticles: (count) =>
			`${n(count)} ${plural(count, { one: 'nieuw artikel', other: 'nieuwe artikelen' })}`
	},

	list: {
		empty: 'Geen artikelen hier.',
		emptyUnread: 'Niets ongelezen hier.',
		emptySearch: 'Geen artikel past bij de zoekopdracht.',
		found: (count, more, words) =>
			`${n(count)}${more ? '+' : ''} ${plural(count, { one: 'artikel', other: 'artikelen' })} met ‘${words}’`,
		searchAll: 'Zoeken in alle feeds',
		feedFailing:
			'rueder probeert het later opnieuw en wacht na elke mislukking langer, of zo lang als de site vraagt. Blijft het mislukken, dan is de feed misschien verhuisd.',
		hiddenNote:
			'Je filterregels hebben deze artikelen verborgen. Open er een om te zien welke regel het was.',
		toRules: 'Filterregels',
		emptyFeed: 'Deze feed heeft nog geen artikelen.',
		waitingFeed:
			'Deze feed is nog niet opgehaald. De artikelen komen met de volgende ophaalronde, binnen een paar minuten.',
		showAll: 'Alle artikelen tonen',
		more: 'Meer tonen',
		star: 'Ster toevoegen',
		unstar: 'Ster verwijderen',
		keepUnread: 'Ongelezen houden',
		openOriginal: 'Origineel openen',
		showFeed: 'Alle artikelen van deze feed in rueder tonen',
		from: 'uit',
		by: (author) => `door ${author}`,
		updated: 'bijgewerkt',
		updatedHeadline: 'nieuwe kop',
		updatedTitle: (count) =>
			`Gewijzigd sinds het eerste ophalen; ${n(count)} ${plural(count, { one: 'eerdere versie', other: 'eerdere versies' })}`,
		updatedHeadlineTitle: (count) =>
			`De kop is herschreven, de tekst is gebleven; ${n(count)} ${plural(count, { one: 'eerdere versie', other: 'eerdere versies' })}`,
		compare: 'Versies vergelijken',
		hiddenBy: (rule) => `Verborgen door de regel: ${rule}`,
		textDropped: 'Deze server bewaart de tekst van dit artikel niet meer, omdat het oud is.',
		textDroppedOriginal: 'Lees het bij de bron',
		loading: 'Tekst laden …',
		loadingList: 'Laden …',
		toTop: 'Terug naar boven',
		pick: 'Kies een artikel om het hier te lezen.',
		resizeList: 'Breedte van de lijst: sleep of gebruik de pijltjestoetsen',
		closeArticle: 'Artikel sluiten',
		blockedFeed:
			'De beheerder van deze rueder heeft deze feed geblokkeerd. Hij wordt niet opgehaald en de artikelen worden niet getoond.',
		articleActions: 'Meer acties voor het artikel',
		articleInfo: 'Artikelinformatie …',
		articleSource: 'Broncode …',
		reportArticle: 'Artikel melden …'
	},

	report: {
		feedTitle: 'Feed melden',
		articleTitle: 'Artikel melden',
		text: (title) =>
			`De beheerder van deze rueder ziet dat ‘${title}’ gemeld is, waarom en wat je schrijft, maar niet wie het gemeld heeft. De beheerder beslist of het voor iedereen geblokkeerd wordt.`,
		reason: 'Waarom',
		reasons: {
			illegal: 'In strijd met de wet',
			copyright: 'Werk van iemand anders',
			malicious: 'Malware, phishing of fraude',
			spam: 'Spam',
			other: 'Iets anders'
		},
		note: 'Wat je wilt toevoegen (optioneel)',
		left: (count) => `Nog ${n(count)} ${plural(count, { one: 'teken', other: 'tekens' })}`,
		send: 'Melding versturen',
		sending: 'Versturen …',
		sent: 'Bedankt. De melding is bij de beheerder van deze rueder aangekomen.',
		tooMany: 'Je hebt de laatste tijd veel meldingen verstuurd. Probeer het later opnieuw.',
		unreachable: 'De melding heeft de server niet bereikt. Probeer het later opnieuw.'
	},

	feedInfo: {
		title: 'Feedinformatie',
		text: (title) =>
			`Wat deze rueder van ‘${title}’ weet en hoe hij de feed ophaalt. Werkt de feed niet, kopieer dit dan in je bericht aan de beheerder.`,
		number: 'Nummer',
		name: 'Naam',
		address: 'Feedadres',
		site: 'Website',
		noSite: 'Geen',
		agent: 'Opgehaald als',
		agents: { rueder: 'rueder', browser: 'Browser' },
		every: 'Opgehaald elke',
		fetched: 'Laatst opgehaald',
		succeeded: 'Laatst gelukt',
		failed: 'Laatste fout',
		next: 'Volgende keer ophalen',
		blocked: 'Nooit, de beheerder heeft de feed geblokkeerd',
		articles: 'Artikelen tot nu toe',
		never: 'Nooit',
		fetchAs: 'Ophalen als',
		fetchAsHint:
			'Voor iedereen die de feed leest. Als browser voor sites die rueder weigeren. Hij wordt meteen opnieuw opgehaald.',
		notSwitched: 'Dat lukte niet. Probeer het later opnieuw.',
		copy: 'Kopiëren',
		copied: 'Gekopieerd',
		loading: 'Laden …',
		unreachable: 'De server antwoordde niet. Probeer het later opnieuw.'
	},
	articleInfo: {
		title: 'Artikelinformatie',
		text: (title) =>
			`Wat deze rueder van ‘${title}’ en de versies ervan weet. Lijkt er iets niet te kloppen, kopieer dit dan in je bericht aan de beheerder.`,
		feed: 'Feed',
		feedNumber: 'Feednummer',
		number: 'Nummer in de feed',
		name: 'Titel',
		id: 'Kenmerk in de feed',
		link: 'Link',
		noLink: 'Geen',
		published: 'Gepubliceerd',
		fetched: 'Eerst opgehaald',
		changed: 'Laatst gewijzigd',
		never: 'Nooit',
		changedAt: (when, what) => `${when} (${what})`,
		what: { title: 'de titel', text: 'de tekst', both: 'de titel en de tekst' },
		version: 'Versie',
		kept: 'Bewaarde versies',
		textLabel: 'Tekst',
		textDropped: 'Niet meer bewaard, omdat hij oud is',
		copy: 'Kopiëren',
		copied: 'Gekopieerd',
		loading: 'Laden …',
		unreachable: 'De server antwoordde niet. Probeer het later opnieuw.'
	},
	articleSource: {
		title: 'Broncode van het artikel',
		text: 'Het artikel als HTML, zoals deze rueder het na het opschonen bewaart, of zoals de feed het stuurde. Links en afbeeldingen houden hier hun eigen adressen, zodat je ziet of er nog een trackingparameter over is.',
		show: 'Tonen',
		kinds: { cleaned: 'Opgeschoond', original: 'Zoals de feed het stuurde' },
		noText: 'Deze server bewaart de tekst van dit artikel niet meer.',
		noOriginal: 'Deze server bewaart niet meer wat de feed stuurde.',
		copy: 'Kopiëren',
		copied: 'Gekopieerd',
		loading: 'Laden …',
		unreachable: 'De server antwoordde niet. Probeer het later opnieuw.'
	},

	media: {
		title: 'Bijlagen',
		play: 'Afspelen'
	},

	connection: {
		reconnecting: 'Opnieuw verbinden …',
		unreachable: (since) => `De server antwoordt niet sinds ${since}.`,
		offline: (since) => `Geen netwerk sinds ${since}.`,
		retry: 'Nu opnieuw proberen',
		unsent: (count) =>
			plural(count, {
				one: '1 wijziging wacht.',
				other: `${n(count)} wijzigingen wachten.`
			})
	},

	notice: {
		unreachable: 'De server antwoordt niet. Wat je ziet is misschien niet actueel.',
		notSaved: 'De server heeft je laatste wijziging niet aangenomen. Ze is hier ongedaan gemaakt.',
		tooManyUnsent: 'Er wachten zoveel wijzigingen op de server dat deze is teruggedraaid.'
	},

	login: {
		title: 'Inloggen bij rueder',
		about:
			'Een feedlezer: hij haalt op wat de sites die je volgt publiceren en toont wat je nog niet gelezen hebt.',
		name: 'Naam',
		password: 'Wachtwoord',
		submit: 'Inloggen',
		checking: 'Een moment …',
		wrong: 'Verkeerde naam of verkeerd wachtwoord.',
		codeNeeded:
			'Dit account heeft een tweede factor. Voer de code van je app of een herstelcode in, of log in met een toegangssleutel.',
		wrongCode: 'Die code klopt niet. Codes veranderen elke 30 seconden.',
		code: 'Code',
		codeNote: 'De zes cijfers uit je authenticatie-app, of een van je herstelcodes.',
		tooMany: (count) => `Te veel verkeerde pogingen. Probeer het over ${minutes(count)} opnieuw.`,
		unreachable: 'De server antwoordt niet. Probeer het zo opnieuw.',
		retry: 'Opnieuw proberen',
		or: 'of',
		oidc: 'Inloggen met single sign-on',
		passkey: 'Inloggen met een toegangssleutel',
		passkeyCancelled:
			'Er is geen toegangssleutel gebruikt. Probeer het opnieuw, of log in met naam en wachtwoord.',
		passkeyFailed: 'Deze browser of dit apparaat kan hier geen toegangssleutel gebruiken.',
		passkeyWrong: 'Deze toegangssleutel hoort bij geen enkel account hier.',
		oidcFailed: 'Inloggen met single sign-on is niet gelukt. Probeer het opnieuw.',
		oidcDenied: 'Inloggen met single sign-on is geannuleerd.',
		oidcUnknown:
			'Geen account hier hoort bij die login. Vraag de beheerder van deze rueder om hem te koppelen.',
		disabled: 'Dit account is uitgeschakeld. Vraag de beheerder van deze rueder waarom.',
		unconfirmed:
			'Dit account wacht tot het e-mailadres bevestigd is. Open de link in de e-mail, of laat hem opnieuw sturen.',
		resend: 'Link opnieuw sturen',
		resent: 'De link is opnieuw onderweg. Kijk in je mail, ook in de spammap.',
		resendWrong:
			'Voer de naam en het wachtwoord van het account in om de link opnieuw te laten sturen.',
		forgot: 'Wachtwoord vergeten?',
		removed: 'Je account is verwijderd, met alles wat van jou was.',
		note: 'Je kunt je hier niet zelf aanmelden. Accounts worden gemaakt door de beheerder van deze rueder.'
	},

	legal: {
		imprint: 'Colofon',
		privacy: 'Privacybeleid',
		terms: 'Gebruiksvoorwaarden',
		agree: { before: 'Ik ga akkoord met de ', link: 'gebruiksvoorwaarden', after: '.' },
		changed: 'De gebruiksvoorwaarden zijn gewijzigd',
		changedIntro:
			'Lees de nieuwe gebruiksvoorwaarden. Ga ermee akkoord om rueder te blijven gebruiken.',
		read: 'Gebruiksvoorwaarden lezen',
		accept: 'Akkoord',
		decline: 'Uitloggen',
		notSaved: 'Dat lukte niet. Probeer het zo opnieuw.'
	},

	register: {
		title: 'Account maken',
		invited: 'Je bent uitgenodigd voor deze rueder. Kies een naam en een wachtwoord.',
		again: 'Het wachtwoord nog een keer',
		passwordRule: (characters) => `Minstens ${n(characters)} tekens.`,
		mismatch: 'De twee wachtwoorden verschillen.',
		submit: 'Account maken',
		solving: 'Deze browser controleren …',
		toRegister: 'Account maken',
		toLogin: 'Inloggen bij een bestaand account',
		closed: 'Niemand kan zich hier zonder uitnodiging aanmelden.',
		inviteGone: 'Deze uitnodiging kan niet meer gebruikt worden. Vraag om een nieuwe.',
		full: 'Er is hier geen ruimte meer voor nog een account.',
		today: 'Er zijn hier vandaag genoeg accounts gemaakt. Probeer het morgen opnieuw.',
		captcha: 'De controle van deze browser is niet gelukt. Probeer het opnieuw.',
		taken: 'Er is al een account met deze naam.',
		terms: 'Ga akkoord met de gebruiksvoorwaarden om een account te maken.',
		invalid: (reason) => `Dat kan geen account zijn. De server zegt: ${reason}`,
		email: 'E-mailadres',
		emailOptional: 'E-mailadres (optioneel)',
		emailNote: 'Je krijgt een link om het te bevestigen.',
		emailRequiredNote:
			'Je krijgt een link om het te bevestigen; het account kan inloggen zodra je hem geopend hebt.',
		emailInvalid: 'Voer een e-mailadres in waar mail naartoe kan.',
		unsent:
			'De link kon niet gemaild worden, dus er is geen account gemaakt. Probeer het later opnieuw.',
		tooMany: (count) => `Te veel pogingen. Probeer het over ${minutes(count)} opnieuw.`,
		awaitingTitle: 'Kijk in je mail',
		awaiting: (address, hours) =>
			`Het account is gemaakt. Er is een link naar ${address} gestuurd: open hem binnen ${n(hours)} uur en log dan in.`
	},

	forgot: {
		title: 'Wachtwoord vergeten?',
		intro:
			'Geef je naam of e-mailadres. Een link om een nieuw wachtwoord in te stellen gaat naar het bevestigde adres.',
		field: 'Naam of e-mailadres',
		submit: 'Link sturen',
		asked: (count) =>
			`Als er zo’n account met een bevestigd adres is, is er een link onderweg, geldig voor ${minutes(count)}. Kijk ook in de spammap.`,
		tooMany: (count) => `Te vaak gevraagd. Probeer het over ${minutes(count)} opnieuw.`,
		back: 'Terug naar inloggen'
	},

	reset: {
		title: 'Nieuw wachtwoord instellen',
		intro: 'Alle browsers en apparaten van het account worden uitgelogd.',
		fresh: 'Nieuw wachtwoord',
		again: 'Nieuw wachtwoord nog een keer',
		submit: 'Wachtwoord instellen',
		mismatch: 'De twee komen niet overeen.',
		gone: 'Deze link werkt niet meer. Vraag om een nieuwe.',
		invalid: (reason) => `De server heeft dat wachtwoord niet aangenomen. Hij zegt: ${reason}`,
		done: 'Het wachtwoord is ingesteld. Log ermee in.',
		cancel: 'Annuleren'
	},

	email: {
		title: 'E-mailadres',
		intro: 'rueder mailt je alleen over je account.',
		current: 'Je adres',
		none: 'Het account heeft geen adres.',
		pending: (address, hours) =>
			`Er is een link naar ${address} gestuurd. Het adres telt zodra hij geopend is, binnen ${n(hours)} uur; tot dan blijft het vorige.`,
		address: 'Nieuw adres',
		password: 'Je wachtwoord',
		passwordNote: 'Zodat niemand anders die deze browser heeft je mail ergens anders heen stuurt.',
		code: 'Code van de tweede factor',
		submit: 'Link sturen',
		sent: 'De link is onderweg. Kijk in je mail, ook in de spammap.',
		remove: 'Adres verwijderen',
		removeIntro:
			'Zonder adres wordt er niets naar je gemaild. Verder blijft het account zoals het is.',
		removed: 'Het adres is verwijderd.',
		wrong: 'Het wachtwoord of de code klopt niet.',
		codeNeeded: 'Dit account heeft een tweede factor. Voer ook de code in.',
		loginAgain:
			'Je account heeft nog geen wachtwoord en geen tweede factor. Log opnieuw in en geef het adres dan binnen tien minuten op.',
		invalid: 'Voer een e-mailadres in waar mail naartoe kan.',
		same: 'Dat is het adres dat het account al heeft.',
		required:
			'Elk account op deze rueder heeft een adres nodig; het kan gewijzigd worden, niet verwijderd.',
		busy: (count) => `Te veel pogingen of mails. Probeer het over ${minutes(count)} opnieuw.`,
		unsent: 'De mail kon niet verstuurd worden. Probeer het later opnieuw.',
		unreachable: 'De server antwoordt niet. Er is niets gewijzigd.',
		askTitle: 'Voeg een e-mailadres toe',
		askIntro:
			'Elk account op deze rueder heeft nu een e-mailadres nodig. Geef het jouwe en open de link die ernaar gemaild wordt.',
		askWaiting:
			'Zodra je de link opent, verdwijnt dit vanzelf. Niets ontvangen? Stuur hem opnieuw, of naar een ander adres.',
		logOut: 'Uitloggen',
		confirmed: (address) => `Het adres ${address} is bevestigd.`,
		confirmedLogIn: 'Het account is klaar: log in.',
		gone: 'Deze link kan niet meer gebruikt worden: hij is gebruikt, er is een nieuwere gestuurd, of hij is ouder dan een dag.',
		taken: 'Een ander account heeft dit adres intussen.',
		confirmUnreachable: 'De server antwoordt niet. Open de link zo opnieuw.',
		close: 'Sluiten'
	},

	invites: {
		intro:
			'Met een link kan iemand een account maken op deze rueder. Hij wordt één keer getoond, hier, als je hem maakt.',
		left: (count) =>
			count === 0
				? 'Je hebt geen uitnodigingen om weg te geven.'
				: `Je kunt nog ${n(count)} ${plural(count, { one: 'persoon', other: 'mensen' })} uitnodigen.`,
		admin: 'Als beheerder kun je altijd uitnodigen.',
		uses: 'Voor hoeveel accounts',
		make: 'Link maken',
		noneLeft: 'Je hebt niet genoeg uitnodigingen over voor zoveel.',
		failed: 'Dat lukte niet. Probeer het zo opnieuw.',
		link: 'De link, om nu door te geven',
		copy: 'Kopiëren',
		copied: 'Gekopieerd',
		goodFor: (days) =>
			`Een link is ${n(days)} ${plural(days, { one: 'dag', other: 'dagen' })} geldig.`,
		yours: 'Je links',
		none: 'Je hebt nog geen links gemaakt.',
		used: (uses, of) => `${n(uses)} van ${n(of)} gebruikt`,
		until: (when) => `geldig tot ${when}`,
		ran: (when) => `verlopen ${when}`,
		usedUp: 'opgebruikt',
		revoked: 'ingetrokken',
		revoke: 'Intrekken',
		revokeLink: (made) => `De link van ${made} intrekken`
	},

	home: {
		newIn: 'Nieuw in je abonnementen',
		allRead: 'Je hebt alles gelezen. Nieuwe artikelen verschijnen hier zodra ze binnenkomen.',
		recentlyStarred: 'Onlangs met ster',
		noStars: 'Nog niets met een ster. Druk op s bij een artikel.',
		problems: 'Feeds met problemen',
		tips: 'Tips',
		tipKeys: 'Druk op ? voor alle sneltoetsen; j en k gaan door de artikelen.',
		tipDrag: 'Sleep feeds en mappen in de zijbalk om ze te sorteren.',
		tipFilters: 'Filterregels in de instellingen verbergen artikelen op trefwoord of auteur.',
		showFeed: (title) => `${title} tonen`
	},

	versions: {
		title: 'Versies van dit artikel',
		current: 'Huidige versie',
		version: (rev) => `Versie ${n(rev)}`,
		fetched: (when) => `opgehaald ${when}`,
		compareWith: 'De huidige versie vergelijken met',
		legendAdded: 'toegevoegd',
		legendRemoved: 'verwijderd',
		titleWas: 'De titel was:',
		same: 'De tekst van deze versie is gelijk aan de huidige.',
		sameWords: 'De woorden zijn gelijk; afbeeldingen of links zijn veranderd.',
		markup:
			'Woorden, afbeeldingen en links zijn gelijk; alleen de opbouw van de tekst is veranderd.',
		pictures: 'Veranderde afbeeldingen',
		noPicture: 'Geen',
		address: 'Adres van de afbeelding',
		links: 'Veranderde links'
	},

	account: {
		title: 'Accountinstellingen',
		tabs: {
			password: 'Wachtwoord',
			email: 'E-mail',
			security: 'Beveiliging',
			apps: 'Apps',
			invites: 'Uitnodigingen',
			move: 'Verhuizen',
			privacy: 'Privacy',
			remove: 'Account verwijderen'
		},
		password: {
			title: 'Wachtwoord wijzigen',
			titleFirst: 'Wachtwoord instellen',
			intro:
				'Een nieuw wachtwoord logt elke andere browser en elk ander apparaat uit en trekt alle app-wachtwoorden in.',
			introFirst:
				'Met een wachtwoord kan dit account ook met zijn naam inloggen, niet alleen via single sign-on.',
			current: 'Huidig wachtwoord',
			fresh: 'Nieuw wachtwoord',
			again: 'Nieuw wachtwoord nog een keer',
			submit: 'Wachtwoord wijzigen',
			submitFirst: 'Wachtwoord instellen',
			done: 'Het wachtwoord is gewijzigd. Elke andere browser en elk ander apparaat is uitgelogd, en de app-wachtwoorden zijn ingetrokken.',
			doneFirst: 'Het wachtwoord is ingesteld.',
			mismatch: 'De twee komen niet overeen.',
			code: 'Code van de tweede factor',
			wrong: 'Het huidige wachtwoord of de code klopt niet.',
			codeNeeded:
				'Dit account heeft een tweede factor. Voer ook een code van je authenticatie-app of een herstelcode in, of bevestig met een toegangssleutel.',
			loginAgain:
				'Je account heeft nog geen wachtwoord en geen tweede factor. Log opnieuw in en stel het wachtwoord dan binnen tien minuten in.',
			invalid: (characters) =>
				`De server heeft dat wachtwoord niet aangenomen. Minstens ${n(characters)} tekens.`,
			busy: (count) => `Te veel verkeerde pogingen. Probeer het over ${minutes(count)} opnieuw.`,
			unreachable: 'De server antwoordt niet. Het wachtwoord is niet gewijzigd.'
		},
		settingsNote: 'Weergave, lezen, filterregels en het toetsenbord staan onder Instellingen.'
	},

	settings: {
		title: 'Instellingen',
		tabs: {
			look: 'Weergave',
			reading: 'Lezen',
			rules: 'Filterregels',
			keys: 'Toetsenbord'
		},
		accountNote:
			'Wachtwoord, tweede factor, apps en uitnodigingen staan onder Accountinstellingen, in het menu van je account.',
		theme: 'Thema',
		themes: { system: 'Zoals het systeem', light: 'Licht', dark: 'Donker' },
		accent: 'Accentkleur',
		accents: {
			red: 'Rood',
			blue: 'Blauw',
			green: 'Groen',
			orange: 'Oranje',
			purple: 'Paars',
			graphite: 'Grafiet'
		},
		density: 'Dichtheid van de lijst',
		densities: { compact: 'Compact', comfortable: 'Ruim' },
		sidebarSide: 'Zijbalk',
		sidebarSides: { start: 'Links', end: 'Rechts' },
		textSize: 'Tekstgrootte van artikelen, op dit apparaat',
		textSizes: { small: 'Klein', medium: 'Middel', large: 'Groot' },
		readingFont: 'Lettertype van artikelen',
		readingFonts: { sans: 'Schreefloos', serif: 'Met schreef' },
		counters: 'Aantallen ongelezen',
		counterStyles: { parens: 'Tussen haakjes', badge: 'Als badges' },
		language: 'Taal',
		languageAuto: 'Zoals de browser',
		view: 'Weergave',
		views: { list: 'Lijst', expanded: 'Uitgebreid', split: 'Drie panelen', magazine: 'Magazine' },
		markOnScroll:
			'Artikelen als gelezen markeren als je er in de uitgebreide weergave langs scrolt',
		teasers: 'De eerste woorden van de tekst na de titel in de lijst tonen',
		wakeLock: 'Het scherm aan houden zolang rueder open is',
		wakeLockMissing: 'Deze browser kan het scherm niet aan houden.',
		note: 'Wijzigingen gelden meteen en worden bij je account bewaard, dus ze volgen je naar elk apparaat.',
		noteDemo: 'Wijzigingen gelden meteen en blijven in deze browser.',
		apps: {
			intro:
				'Andere feedlezer-apps kunnen je account gebruiken via de Google Reader API. Elke app krijgt een eigen app-wachtwoord, dat je op elk moment kunt intrekken.',
			none: 'Er is nog geen app ingesteld.',
			made: (when) => `ingesteld ${when}`,
			used: (when) => `laatst gebruikt ${when}`,
			unused: 'nog niet gebruikt',
			revoke: 'Intrekken',
			revokeName: (name) => `Het app-wachtwoord ${name} intrekken`,
			add: 'App instellen',
			full: (max) =>
				`Een account heeft hoogstens ${n(max)} app-wachtwoorden. Trek er een in om een andere app in te stellen.`,
			choose: 'Welke app wil je gebruiken?',
			website: (app) => `Website van ${app}`,
			other: 'Een andere app',
			otherHint: 'Elke app die de Google Reader API of FreshRSS ondersteunt.',
			platforms: {
				apple: 'Mac, iPhone en iPad',
				android: 'Android'
			},
			name: 'Naam van het app-wachtwoord',
			password: 'Je wachtwoord',
			code: 'Code van je authenticatie-app, of een herstelcode',
			create: 'App-wachtwoord maken',
			creating: 'Een moment …',
			back: 'Terug',
			how: {
				netnewswire:
					'Open in NetNewsWire de Instellingen en dan Accounts, voeg een account toe (op de Mac met +) en kies FreshRSS. Voer de naam hieronder in als Login (op iPhone en iPad: Username or Email), het app-wachtwoord als Password en het serveradres als API URL.',
				readyou:
					'Open in Read You Instellingen, Accounts, Accounts toevoegen, en kies Google Reader. Voer het serveradres hieronder in als server-URL, de naam als gebruikersnaam en het app-wachtwoord als wachtwoord.',
				other:
					'Voeg in je app een account toe van het type ‘Google Reader API’ of, als dat er niet is, ‘FreshRSS’. Voer het serveradres, de naam en het app-wachtwoord hieronder in.'
			},
			ownIcons: (app) =>
				`Privacy: ${app} laadt de pictogrammen van je feeds zelf, van de websites van de feeds, die dan je IP-adres zien. Artikelen en hun afbeeldingen komen alleen via rueder.`,
			shotsTitle: 'Stap voor stap, met afbeeldingen',
			shots: {
				readyou: [
					'Tik linksboven op het tandwiel.',
					'Open Accounts.',
					'Tik op Accounts toevoegen.',
					'Kies Google Reader.',
					'Voer het serveradres in als server-URL, de naam als gebruikersnaam en het app-wachtwoord als wachtwoord, en tik op Toevoegen.'
				],
				netnewswire: [
					'Op de Mac: open de Instellingen (⌘,), dan Accounts, en klik linksonder op +.',
					'Kies FreshRSS en klik op Ga door.',
					'Voer de naam in als Login, het app-wachtwoord als Password en het serveradres als API URL, en klik op Maak aan.'
				]
			},
			shotAlt: (app, step) => `${app}, stap ${n(step)}`,
			server: 'Serveradres',
			user: 'Naam',
			secret: 'App-wachtwoord',
			copy: (what) => `${what} kopiëren`,
			copied: 'Gekopieerd',
			once: 'Het app-wachtwoord wordt alleen nu getoond. Raak je het kwijt, trek het dan in en maak een nieuw.',
			waiting: 'Wachten tot de app inlogt …',
			connected: 'Verbonden: de app is ingelogd.',
			done: 'Klaar',
			refusals: {
				wrongPassword: 'Het wachtwoord klopt niet.',
				codeNeeded:
					'Voer ook een code van je authenticatie-app of een herstelcode in, of bevestig met een toegangssleutel.',
				wrongCode: 'Die code klopt niet. Codes veranderen elke 30 seconden.',
				loginAgain:
					'Je account heeft nog geen wachtwoord en geen tweede factor. Log opnieuw in en maak het app-wachtwoord dan binnen tien minuten.',
				tooManyApps: 'Een account heeft hoogstens 20 app-wachtwoorden.',
				tooMany: 'Te veel verkeerde pogingen. Probeer het over een paar minuten opnieuw.',
				unreachable: 'De server antwoordt niet. Probeer het zo opnieuw.'
			}
		},
		secondFactor: {
			title: 'Tweede factor',
			intro:
				'Met een tweede factor vraagt inloggen ook een code van een authenticatie-app, of een toegangssleutel.',
			app: 'Authenticatie-app',
			on: 'Aan.',
			left: (count) =>
				`Nog ${n(count)} ${plural(count, { one: 'herstelcode', other: 'herstelcodes' })}.`,
			off: 'Uit.',
			byPasskeys: 'Niet ingesteld. Je toegangssleutels houden de tweede factor aan.',
			adminNeeds:
				'Deze server maakt beheerders alleen met een tweede factor tot beheerder. Stel die in, of log opnieuw in met de code ervan, om de serverstatus te zien.',
			password: 'Je wachtwoord',
			start: 'Instellen',
			scan: 'Scan deze code met je authenticatie-app, of typ de sleutel hieronder in.',
			qr: 'QR-code voor de authenticatie-app',
			larger: 'QR-code groter tonen',
			smaller: 'QR-code kleiner tonen',
			key: 'Sleutel',
			copyKey: 'Sleutel kopiëren',
			copied: 'Gekopieerd',
			code: 'De code die de app nu toont',
			codeOrRecovery: 'Code van de app, of een herstelcode',
			confirm: 'Bevestigen',
			cancel: 'Annuleren',
			codesTitle: 'Je herstelcodes',
			codesIntro:
				'Met elke code log je één keer in zonder je telefoon. Bewaar ze goed: ze worden alleen nu getoond.',
			copyCodes: 'Codes kopiëren',
			codesDone: 'Ik heb ze bewaard',
			switchOff: 'Uitschakelen',
			switchOffNote: 'Inloggen vraagt dan weer alleen het wachtwoord.',
			switchOffNoteKeys: 'Je toegangssleutels houden de tweede factor aan.',
			startWithPasskey: 'Instellen met een toegangssleutel',
			switchOffWithPasskey: 'Uitschakelen met een toegangssleutel',
			refusals: {
				wrongPassword: 'Het wachtwoord klopt niet.',
				codeNeeded: 'Je toegangssleutels houden de tweede factor aan. Voer ook een herstelcode in.',
				wrongCode: 'Die code klopt niet. Codes veranderen elke 30 seconden.',
				loginAgain:
					'Je account heeft nog geen wachtwoord en geen tweede factor. Log opnieuw in en stel de app dan binnen tien minuten in.',
				tooMany: 'Te veel verkeerde pogingen. Probeer het over een paar minuten opnieuw.',
				unreachable: 'De server antwoordt niet. Probeer het zo opnieuw.'
			}
		},
		passkeys: {
			title: 'Toegangssleutels',
			intro:
				'Met een toegangssleutel log je in zonder naam en wachtwoord, met de pincode, vingerafdruk of het gezicht dat je apparaat vraagt. Hij telt als tweede factor.',
			once: 'Met een toegangssleutel opent je wachtwoord alleen je account niet meer.',
			none: 'Nog geen toegangssleutel.',
			made: (when) => `toegevoegd ${when}`,
			used: (when) => `laatst gebruikt ${when}`,
			unused: 'nog niet gebruikt',
			add: 'Toegangssleutel toevoegen',
			name: 'Naam, zoals die van het apparaat',
			defaultName: 'Toegangssleutel',
			password: 'Je wachtwoord',
			code: 'Code van je authenticatie-app, of een herstelcode',
			create: 'Toegangssleutel maken',
			working: 'Volg je browser …',
			cancel: 'Annuleren',
			remove: 'Verwijderen',
			removeName: (name) => `De toegangssleutel ${name} verwijderen`,
			confirmRemove: 'Toegangssleutel verwijderen',
			full: (max) => `Een account heeft hoogstens ${n(max)} toegangssleutels.`,
			or: 'of',
			confirmWith: 'Bevestigen met een toegangssleutel',
			codesTitle: 'Je herstelcodes',
			codesIntro:
				'Je eerste toegangssleutel heeft de tweede factor aangezet. Met elke code log je één keer in als je je apparaat kwijtraakt. Bewaar ze goed: ze worden alleen nu getoond.',
			copyCodes: 'Codes kopiëren',
			copied: 'Gekopieerd',
			codesDone: 'Ik heb ze bewaard',
			refusals: {
				cancelled:
					'Het venster van de browser is gesloten voordat de toegangssleutel gebruikt werd.',
				known: 'Dit apparaat heeft al een toegangssleutel voor je account.',
				failed: 'Deze browser of dit apparaat kon geen toegangssleutel maken of gebruiken.',
				wrong: 'De toegangssleutel is niet aangenomen. Probeer het opnieuw.',
				wrongPassword: 'Het wachtwoord klopt niet.',
				codeNeeded:
					'Je account heeft een tweede factor. Voer ook een code van je app of een herstelcode in, of bevestig met een toegangssleutel.',
				wrongCode: 'Die code klopt niet. Codes veranderen elke 30 seconden.',
				tooManyPasskeys: 'Een account heeft hoogstens 10 toegangssleutels.',
				loginAgain:
					'Je account heeft nog geen wachtwoord en geen tweede factor. Log opnieuw in en voeg je eerste toegangssleutel dan binnen tien minuten toe.',
				tooMany: 'Te veel verkeerde pogingen. Probeer het over een paar minuten opnieuw.',
				unreachable: 'De server antwoordt niet. Probeer het zo opnieuw.'
			}
		},
		reports: {
			send: 'Foutmeldingen uit deze browser naar deze server sturen',
			intro: 'Als er iets misgaat op de pagina, horen de beheerders van deze server het.',
			what: 'Een melding bevat:',
			holds: [
				'de foutmelding, zonder adressen en zonder getallen die iets aanduiden',
				'waar in de eigen bestanden van rueder het gebeurde',
				'de versie van rueder en je browser met zijn hoofdversie'
			],
			never: 'Nooit wat je leest, je feeds, labels of regels. Meldingen verdwijnen na 30 dagen.',
			off: 'Als je dit uitschakelt, worden de meldingen die je stuurde verwijderd.'
		}
	},

	rules: {
		intro:
			'Regels verbergen artikelen of markeren ze als gelezen zodra ze binnenkomen. Verborgen artikelen tellen niet als ongelezen en blijven onder ‘Verborgen door filters’.',
		none: 'Nog geen regels.',
		add: 'Regel toevoegen',
		new: 'Nieuwe regel',
		on: 'Aan',
		scope: 'In',
		scopeAll: 'Alle feeds',
		scopeFolder: (title) => `Map: ${title}`,
		scopeFeed: (title) => `Feed: ${title}`,
		field: 'Kijk naar',
		fields: { any: 'Alles', title: 'Titel', content: 'Tekst', author: 'Auteur', tag: 'Tag' },
		match: 'Vergelijking',
		matches: {
			contains: 'bevat',
			word: 'bevat het woord',
			starts: 'begint met',
			ends: 'eindigt op',
			regex: 'past bij de reguliere expressie',
			all: 'elk artikel, wat er ook in staat'
		},
		pattern: 'Te zoeken tekst',
		action: 'Dan',
		actions: { hide: 'het artikel verbergen', read: 'als gelezen markeren' },
		preview: (count, total) => `Geldt nu voor ${n(count)} van de ${n(total)} artikelen hier.`,
		regexTip: 'Reguliere expressies worden gelezen zoals Rust ze leest, ongeacht hoofdletters.',
		regexBuilder: 'Er een maken en uitproberen op regex101',
		invalid: 'Dit is geen geldige reguliere expressie.',
		empty: 'Voer de te zoeken tekst in.',
		checking: 'Kijken wat hij zou vangen …',
		hits: (count) => `${n(count)} ${plural(count, { one: 'treffer', other: 'treffers' })}`,
		summary: (field, match, pattern, action) => `${field} ${match} ‘${pattern}’: ${action}`,
		summaryAll: (action) => `Elk artikel: ${action}`
	},

	keys: {
		title: 'Sneltoetsen',
		note: 'Je toetsen worden bij de instellingen bewaard. Enter, Escape en g blijven zoals ze zijn.',
		change: (what) => `De toets wijzigen voor: ${what}`,
		changeShort: 'Wijzigen',
		press: 'Druk op een toets …',
		taken: (key, other) => `${key} is al de toets voor: ${other}`,
		fixed: (key) => `${key} kan niet aan een sneltoets gegeven worden.`,
		reset: 'Terug naar de toetsen van rueder',
		groups: { articles: 'Artikelen', lists: 'Lijsten', go: 'Ga naar', app: 'Overal' },
		next: 'Volgend artikel, geopend',
		previous: 'Vorig artikel, geopend',
		nextCursor: 'Volgend artikel, zonder te openen',
		previousCursor: 'Vorig artikel, zonder te openen',
		toggle: 'Het artikel openen of sluiten',
		star: 'Ster toevoegen of verwijderen',
		read: 'Als gelezen of ongelezen markeren',
		original: 'Het origineel in een nieuw tabblad openen',
		markAll: 'Alles als gelezen markeren',
		refresh: 'De lijst vernieuwen',
		unreadOnly: 'Alleen ongelezen, of alle artikelen',
		listView: 'Lijstweergave',
		expandedView: 'Uitgebreide weergave',
		splitView: 'Drie panelen',
		magazineView: 'Magazine',
		goHome: 'Start',
		goAll: 'Alle artikelen',
		goUnread: 'Ongelezen artikelen',
		goStarred: 'Artikelen met ster',
		search: 'Zoeken',
		sidebar: 'De zijbalk tonen of verbergen',
		help: 'Dit overzicht',
		then: 'dan'
	},

	labels: {
		title: 'Labels',
		add: 'Nieuw label',
		name: 'Naam van het label',
		none: 'Nog geen labels.',
		actions: (title) => `Acties voor het label ${title}`,
		rename: 'Naam wijzigen',
		color: 'Kleur',
		colors: ['Rood', 'Oranje', 'Geel', 'Groen', 'Turquoise', 'Blauw', 'Paars', 'Grijs'],
		remove: 'Label verwijderen',
		show: 'Labels tonen',
		hide: 'Labels verbergen',
		forArticle: 'Labels',
		set: (title) => `Label: ${title}`
	},

	subscribe: {
		title: 'Abonneren op een feed',
		address: 'Adres van de feed of de website',
		find: 'Feeds zoeken',
		finding: 'Zoeken …',
		note: 'Het concept kan niets ophalen, dus de feed blijft leeg.',
		found: (count) => (count === 1 ? 'Er is één feed:' : `Er zijn ${n(count)} feeds:`),
		none: 'Er is geen feed op dit adres.',
		movedTo: (url) => `De site stuurde het verzoek door naar ${url}.`,
		loginWall:
			'Sommige sites sturen wie niet ingelogd is naar hun inlogpagina. Hun feeds kunnen dan niet gelezen worden.',
		subscribed: 'Geabonneerd',
		show: 'Tonen',
		untitled: 'Zonder naam',
		folder: 'In de map',
		topLevel: 'Geen map',
		add: 'Abonneren',
		adding: 'Abonneren …',
		refused: 'Dat lukte niet.',
		serverSays: (reason) => `De server zegt: ${reason}`,
		busy: (seconds) =>
			seconds
				? `Te veel zoekopdrachten tegelijk. Probeer het over ${n(seconds)} ${plural(seconds, { one: 'seconde', other: 'seconden' })} opnieuw.`
				: 'Te veel zoekopdrachten tegelijk. Probeer het zo opnieuw.',
		noFetching: 'Deze server haalt geen feeds op.',
		tooLarge: (megabytes) => `Het bestand is groter dan ${n(megabytes)} MB.`,
		unreachable: 'De server antwoordt niet. Probeer het zo opnieuw.'
	},

	opml: {
		title: 'Importeren en exporteren',
		intro:
			'Neem je abonnementen mee uit een andere feedlezer als OPML-bestand, of neem ze mee naar elders.',
		import: 'OPML-bestand importeren',
		importing: 'Importeren …',
		imported: (feeds, folders) =>
			`${n(feeds)} ${plural(feeds, { one: 'nieuwe feed', other: 'nieuwe feeds' })} en ${n(folders)} ${plural(folders, { one: 'nieuwe map', other: 'nieuwe mappen' })}.`,
		known: (count) =>
			`${n(count)} ${plural(count, { one: 'feed was', other: 'feeds waren' })} al geabonneerd.`,
		skipped: (count) =>
			`${n(count)} ${plural(count, { one: 'item is', other: 'items zijn' })} overgeslagen, omdat het geen leesbare feeds zijn.`,
		later: 'Hun artikelen komen met de volgende ophaalronde.',
		export: 'Exporteren als OPML'
	},

	removeAccount: {
		title: 'Het account verwijderen',
		intro:
			'Dit verwijdert je account op deze rueder voorgoed. Het kan niet ongedaan gemaakt worden.',
		goes: 'Wat ermee verdwijnt: je sessies op alle apparaten, je abonnementen en mappen, wat je gelezen hebt, sterren, labels, filterregels, instellingen, foutmeldingen, uitnodigingen en de koppeling met single sign-on.',
		stays:
			'Feeds blijven voor de anderen die ze lezen. Wil je je account eerst ergens anders heen brengen, gebruik dan ‘Verhuizen’.',
		password: 'Je wachtwoord',
		name: (name) => `Typ je naam, ${name}, om te bevestigen`,
		code: 'Code van je app, of een herstelcode',
		submit: 'Mijn account voorgoed verwijderen',
		wrong: 'Dat klopt niet. Er is niets verwijderd.',
		codeNeeded:
			'Je account heeft een tweede factor. Typ ook een code van je app of een herstelcode, of bevestig met een toegangssleutel.',
		busy: (count) => `Te veel verkeerde pogingen. Probeer het over ${minutes(count)} opnieuw.`,
		unreachable: 'De server antwoordt niet. Er is niets verwijderd.'
	},

	move: {
		intro: 'Breng je account naar een andere rueder, of haal het hierheen, in één bestand.',
		howItWorks:
			'Een import voegt toe aan wat hier is en haalt niets weg. Je instellingen worden die uit het bestand.',
		notInIt: 'Niet in het bestand: je wachtwoord, je sessies en de teksten van artikelen.',
		export: 'Het account exporteren',
		import: 'Een accountbestand importeren',
		importing: 'Importeren …',
		feeds: (feeds, known) =>
			`${n(feeds)} ${plural(feeds, { one: 'nieuwe feed', other: 'nieuwe feeds' })}, ${n(known)} al geabonneerd.`,
		folders: (count) =>
			`${n(count)} ${plural(count, { one: 'nieuwe map', other: 'nieuwe mappen' })}.`,
		articles: (found) => `${articles(found)} hier gevonden.`,
		missing: (count) =>
			`${articles(count)} niet hier gevonden; wat je ervan gelezen, met een ster of label gemarkeerd had, is verloren.`,
		marks: (labels, rules) =>
			`${n(labels)} ${plural(labels, { one: 'nieuw label', other: 'nieuwe labels' })} en ${n(rules)} ${plural(rules, { one: 'nieuwe filterregel', other: 'nieuwe filterregels' })}.`,
		skipped: (count) =>
			`${n(count)} ${plural(count, { one: 'item', other: 'items' })} uit het bestand kon${count === 1 ? '' : 'den'} niet overgenomen worden.`,
		settings: 'Je instellingen zijn nu die uit het bestand.',
		later:
			'Nieuwe feeds brengen hun artikelen met de volgende ophaalronde. Wat je gelezen had, blijft gelezen.',
		busy: (seconds) =>
			seconds
				? `Te veel imports. Probeer het over ${minutes(Math.ceil(seconds / 60))} opnieuw.`
				: 'Te veel imports. Probeer het later opnieuw.'
	},

	status: {
		title: 'Serverstatus',
		open: 'Serverstatus',
		loading: 'De server vragen …',
		failed: 'De server antwoordde niet. Probeer het zo opnieuw.',
		refresh: 'Vernieuwen',
		asOf: (time) => `Stand van ${time}. Wat geteld wordt, is het laatste uur.`,
		demo: 'Verzonnen, want hier is geen server.',
		empty: '–',
		on: 'Aan',
		off: 'Uit',
		took: (median, p95) => `mediaan ${median}, 95% binnen ${p95}`,
		summary: {
			label: 'In één oogopslag',
			hints: 'Aandachtspunten',
			failing: 'Feeds die mislukken',
			errors: 'Gelogde fouten',
			reported: 'Gemelde feeds'
		},
		hintsTitle: 'Het bekijken waard',
		allWell: 'Er valt niets op.',
		hints: {
			slowRoute: (route, took) =>
				`${route} is traag: één op de twintig verzoeken duurt langer dan ${took}.`,
			serverErrors: (route, count) =>
				`${route} antwoordde ${n(count)} keer met een fout van de server.`,
			poolBusy: (size) =>
				`Alle ${n(size)} verbindingen met de database zijn in gebruik, dus verzoeken wachten erop.`,
			fetchBacklog: (count, late) =>
				`${n(count)} ${plural(count, { one: 'feed wordt', other: 'feeds worden' })} te laat opgehaald, de laatste ${late} te laat.`,
			feedsFailing: (failing, subscribed) =>
				`${n(failing)} van de ${n(subscribed)} feeds mislukken.`,
			notFetching: 'De server haalt geen feeds op. Staat FETCH op off?',
			fetchPaceFull: (most) =>
				`Elke ronde neemt zoveel feeds als mag, en toch wachten er feeds: het tempo laat ${n(most)} ophaalacties per uur toe. FETCH_ROUND, FETCH_AT_ONCE en FETCH_EVERY stellen het in.`,
			cacheMisses: (share) =>
				`De database vindt maar ${share} van wat hij leest in het geheugen; meer shared_buffers zou helpen.`,
			databaseLarge: (size, warn) =>
				`De database is gegroeid tot ${size}; DATABASE_WARN_GB waarschuwt vanaf ${warn}. ‘rueder retention set’ verwijdert oude artikelen of hun teksten.`,
			longQuery: (took) => `Er loopt al ${took} een query.`,
			imageCacheFull: (share) => `De afbeeldingscache is voor ${share} vol.`,
			errors: (count) =>
				`De server heeft in het laatste uur ${n(count)} ${plural(count, { one: 'fout', other: 'fouten' })} gelogd.`,
			limited: (count) =>
				`${n(count)} ${plural(count, { one: 'verzoek is', other: 'verzoeken zijn' })} geweigerd als te veel.`,
			browserErrors: (count) =>
				`Browsers meldden de afgelopen dag ${n(count)} ${plural(count, { one: 'fout', other: 'verschillende fouten' })}.`,
			reportedFeeds: (count) =>
				`Lezers meldden ${n(count)} ${plural(count, { one: 'feed die niet geblokkeerd is', other: 'feeds die niet geblokkeerd zijn' })}.`
		},
		build: {
			title: 'Build',
			commit: 'Commit',
			dirty: (commit) => `${commit}, met eigen wijzigingen`,
			branch: 'Branch',
			tag: 'Tag',
			committed: 'Gecommit',
			where: 'Draait',
			container: 'in een container',
			host: 'op de machine zelf',
			target: 'Voor',
			profile: 'Gecompileerd',
			profileValue: (profile, level) => `${profile}, optimalisatie ${level}`,
			ci: 'Gebouwd door',
			ciValue: 'een run van de CI',
			byHand: 'met de hand'
		},
		server: {
			title: 'Server',
			version: 'Versie',
			running: 'Draait sinds',
			memory: 'Geheugen',
			accounts: 'Accounts',
			feeds: 'Feeds',
			feedsValue: (fetched, all) => `${n(fetched)} opgehaald, ${n(all)} in totaal`,
			articles: 'Artikelen',
			articlesValue: (count) => `ongeveer ${n(count)}`,
			live: 'Open tabbladen'
		},
		database: {
			title: 'Database',
			pool: 'Verbindingen van rueder',
			poolValue: (size, idle, max) => `${n(size)} open, ${n(idle)} inactief, hoogstens ${n(max)}`,
			connections: 'Alle verbindingen',
			size: 'Grootte',
			sizeWarn: (size, warn) => `${size}, waarschuwing vanaf ${warn}`,
			hitRatio: 'Uit het geheugen gelezen',
			deadlocks: 'Deadlocks',
			longest: 'Langst lopende query',
			tables: 'Grootste tabellen',
			table: 'Tabel',
			bytes: 'Grootte',
			rows: 'Rijen, ongeveer'
		},
		fetcher: {
			title: 'Feeds ophalen',
			state: 'Ophalen',
			lastRound: 'Laatste ronde',
			lastRoundValue: (ago, feeds, took) =>
				`${ago}: ${n(feeds)} ${plural(feeds, { one: 'feed', other: 'feeds' })} in ${took}`,
			fetches: 'Ophaalacties',
			fetchesValue: (fetched, unchanged, failed) =>
				`${n(fetched)} brachten de feed, ${n(unchanged)} ongewijzigd, ${n(failed)} mislukt`,
			broken: 'Mislukt bij de database',
			newArticles: 'Nieuwe artikelen',
			took: 'Tijd per ophaalactie',
			overdue: 'Te laat opgehaald',
			overdueValue: (count, late) => `${n(count)}, de laatste ${late} te laat`,
			waiting: 'Nog niet opgehaald',
			pace: 'Tempo',
			paceValue: (round, every, atOnce, most) =>
				`${n(round)} ${plural(round, { one: 'feed', other: 'feeds' })} elke ${every}, ${n(atOnce)} tegelijk: hoogstens ${n(most)} per uur`,
			perHour: 'Ophaalacties in het laatste uur',
			behind: 'Achterstand',
			behindValue: (late) => `de feed die het langst wacht is ${late} over zijn tijd`,
			notBehind: 'Er wacht niets',
			failing: 'Mislukte feeds, per oorzaak',
			noneFailing: 'Geen feed mislukt.',
			feeds: (count) => `${n(count)} ${plural(count, { one: 'feed', other: 'feeds' })}`,
			failures: {
				status: (code) => (code ? `De site antwoordt ${code}` : 'De site antwoordt met een fout'),
				timeout: 'De site doet er te lang over',
				blocked: 'Het adres ligt in een netwerk waar rueder niet uit ophaalt',
				dns: 'De naam heeft geen adres',
				network: 'Geen verbinding, of ze werd verbroken',
				too_large: 'Te groot',
				redirects: 'Te veel omleidingen, of een kapotte',
				unreadable: 'Geen feed die rueder kan lezen',
				other: 'Iets anders'
			}
		},
		finder: {
			title: 'Feeds zoeken',
			state: 'Adressen bekijken',
			lookups: 'Zoekopdrachten',
			lookupsValue: (found, none, failed) =>
				`${n(found)} vonden feeds, ${n(none)} vonden niets, ${n(failed)} mislukt`,
			limited: 'Boven de limiet van het account',
			took: 'Tijd per zoekopdracht'
		},
		images: {
			title: 'Afbeeldingsproxy',
			state: 'Proxy',
			cache: 'Cache',
			cacheValue: (used, max) => `${used} van ${max}`,
			served: 'Afbeeldingen',
			servedValue: (cached, fetched) => `${n(cached)} uit de cache, ${n(fetched)} opgehaald`,
			refused: 'Geweigerd',
			refusedValue: (source, notImage, tooLarge, notFound) =>
				`${n(source)} bron mislukt, ${n(notImage)} geen afbeelding, ${n(tooLarge)} te groot, ${n(notFound)} verkeerd adres`,
			took: 'Tijd per opgehaalde afbeelding',
			failingSources: 'Bronnen die voorlopig met rust gelaten worden'
		},
		requests: {
			title: 'Verzoeken',
			all: 'Alle verzoeken',
			allValue: (count, took) => `${n(count)}, ${took}`,
			answers: 'Antwoorden',
			answersValue: (fine, refused, errors) =>
				`${n(fine)} goed, ${n(refused)} geweigerd, ${n(errors)} fouten van de server`,
			route: 'Route',
			count: 'Aantal',
			median: 'Mediaan',
			p95: '95%',
			max: 'Langste',
			errors: 'Fouten',
			none: 'Geen verzoeken in het laatste uur.'
		},
		problems: {
			title: 'Waarschuwingen en fouten',
			counts: (errors, warnings) =>
				`In het laatste uur ${n(errors)} ${plural(errors, { one: 'fout', other: 'fouten' })} en ${n(warnings)} ${plural(warnings, { one: 'waarschuwing', other: 'waarschuwingen' })}.`,
			none: 'Sinds de start is er niets gelogd.',
			error: 'Fout',
			warning: 'Waarschuwing',
			times: (count, last) => (count > 1 ? `${n(count)} keer, laatst ${last}` : last)
		},
		browsers: {
			title: 'Fouten in browsers',
			off: 'Deze server neemt geen foutmeldingen aan (CLIENT_REPORTS=off).',
			consenting: (count) =>
				`${n(count)} ${plural(count, { one: 'account stuurt', other: 'accounts sturen' })} meldingen. Fouten blijven 30 dagen nadat ze het laatst gebeurden.`,
			none: 'Geen browser heeft een fout gemeld.',
			kinds: {
				error: 'Fout',
				rejection: 'Mislukte promise',
				component: 'Deel van de pagina',
				request: 'Verzoek'
			},
			hits: (count, accounts, last) =>
				`${n(count)} keer in ${n(accounts)} ${plural(accounts, { one: 'account', other: 'accounts' })}, laatst ${last}`,
			where: (versions, browsers) => `Versie ${versions} · ${browsers}`
		},
		reported: {
			title: 'Gemelde feeds',
			text: 'Wie meldde, staat hier niet. ‘rueder feed block <adres>’ blokkeert een feed voor iedereen, ‘rueder feed unblock <adres>’ heft dat op.',
			none: 'Niemand heeft een feed gemeld.',
			reports: (count, last) =>
				`${n(count)} ${plural(count, { one: 'melding', other: 'meldingen' })}, laatst ${last}`,
			blocked: 'Geblokkeerd'
		}
	},

	update: {
		ready: 'Er is een nieuwe versie van rueder klaar.',
		reload: 'Opnieuw laden',
		later: 'Later'
	},

	crash: {
		title: 'Er is iets misgegaan op deze pagina.',
		text: 'Laad de pagina opnieuw om verder te gaan. Wat je gelezen en gewijzigd hebt, staat op de server.',
		reload: 'Opnieuw laden'
	},

	demo: {
		badge: 'Concept',
		note: 'Een concept met verzonnen feeds. Er wordt niets op een server bewaard.',
		arrive: 'Een artikel laten binnenkomen',
		revise: 'Het geopende artikel wijzigen',
		revised: 'Update: deze alinea is toegevoegd om de versies uit te proberen.',
		reviseHint: 'Open eerst een artikel.'
	}
};

export default nl;
