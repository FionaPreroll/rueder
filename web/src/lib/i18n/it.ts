// The Italian texts, in the shape of the English catalog (en.ts), addressing the
// reader with “tu”. “Feed” and “passkey” stay as they are; an item is an
// “articolo”.

import type { Messages } from './en.ts';
import { formatNumber as n, plural } from './intl.ts';

const articles = (count: number) =>
	`${n(count)} ${plural(count, { one: 'articolo', other: 'articoli' })}`;

const minutes = (count: number) =>
	`${n(count)} ${plural(count, { one: 'minuto', other: 'minuti' })}`;

const it: Messages = {
	languageName: 'Italiano',

	articles,

	common: {
		close: 'Chiudi',
		cancel: 'Annulla',
		save: 'Salva',
		delete: 'Elimina',
		edit: 'Modifica'
	},

	nav: {
		subscribe: 'Iscriviti',
		home: 'Home',
		all: 'Tutti gli articoli',
		unreadItems: 'Articoli da leggere',
		starred: 'Articoli con stella',
		hidden: 'Nascosti dai filtri',
		subscriptions: 'Iscrizioni',
		newFolder: 'Nuova cartella',
		groupedFolder: 'Nuova cartella',
		folderName: 'Nome della cartella',
		renameFolder: 'Rinomina',
		removeFolder: 'Rimuovi la cartella, tieni il contenuto',
		folderActions: (title) => `Azioni per la cartella ${title}`,
		feedName: 'Nome del feed',
		feedNameHint: 'Vuoto per il nome proprio del feed',
		renameFeed: 'Rinomina',
		unsubscribe: 'Annulla iscrizione',
		reportFeed: 'Segnala il feed …',
		feedInfo: 'Informazioni sul feed …',
		feedActions: (title) => `Azioni per il feed ${title}`,
		unsubscribeTitle: 'Annullare l’iscrizione?',
		unsubscribeText: (title) =>
			`Annullare l’iscrizione a “${title}”? Quello che hai letto e contrassegnato con una stella lì andrà perso.`,
		openFolder: 'Apri la cartella',
		closeFolder: 'Chiudi la cartella',
		openSidebar: 'Mostra la barra laterale',
		closeSidebar: 'Nascondi la barra laterale',
		search: 'Cerca',
		searchPlaceholder: 'Cerca in questi articoli',
		clearSearch: 'Cancella la ricerca',
		settings: 'Impostazioni',
		accountSettings: 'Impostazioni dell’account',
		shortcuts: 'Scorciatoie da tastiera',
		live: 'In diretta',
		liveOn: 'I nuovi articoli arrivano da soli. Fai clic per mettere in pausa.',
		liveOff: 'In pausa. Fai clic per far arrivare di nuovo gli articoli nuovi.',
		paused: 'In pausa',
		account: (name) => `Accesso effettuato come ${name}`,
		logout: 'Esci',
		appSettings: 'Server e aggiornamenti dell’app',
		install: 'Installa come app',
		installSafari:
			'Per installare rueder come app: File › Aggiungi al Dock su un Mac, Condividi › Aggiungi alla schermata Home su un iPhone o un iPad.',
		unread: (count) => `${n(count)} da leggere`,
		hiddenCount: (count) =>
			`${n(count)} ${plural(count, { one: 'nascosto', other: 'nascosti' })} dai filtri`,
		fetchFailed: (reason) => `L’ultimo scaricamento non è riuscito: ${reason}`,
		waiting: 'Non ancora scaricato',
		blocked: 'Bloccato da chi gestisce questo rueder',
		moveHint: 'Trascina per ordinare, oppure premi Alt con le frecce',
		resizeSidebar: 'Larghezza della barra laterale: trascina, oppure usa le frecce',
		tooDeep: (levels) => `Le cartelle si annidano al massimo per ${n(levels)} livelli.`
	},

	toolbar: {
		refresh: 'Aggiorna',
		showAll: 'Tutti gli articoli',
		showUnread: 'Solo da leggere',
		show: 'Mostra',
		newestFirst: 'Prima i più recenti',
		oldestFirst: 'Prima i meno recenti',
		markAllRead: 'Segna tutto come letto',
		markOptions: 'Altri modi per segnare come letto',
		phoneMenu: (shown) => `Vista, mostra: ${shown}`,
		markMenu: 'Segna come letto …',
		markEverything: 'Tutti gli articoli',
		markOlderDay: 'Articoli più vecchi di un giorno',
		markOlderWeek: 'Articoli più vecchi di una settimana',
		listView: 'Vista elenco',
		expandedView: 'Vista espansa',
		splitView: 'Tre riquadri: albero, elenco e articolo',
		magazineView: 'Rivista',
		previous: 'Articolo precedente',
		next: 'Articolo successivo',
		newArticles: (count) =>
			`${n(count)} ${plural(count, { one: 'nuovo articolo', other: 'nuovi articoli' })}`
	},

	list: {
		empty: 'Nessun articolo qui.',
		emptyUnread: 'Niente da leggere qui.',
		emptySearch: 'Nessun articolo corrisponde alla ricerca.',
		found: (count, more, words) =>
			`${n(count)}${more ? '+' : ''} ${plural(count, { one: 'articolo', other: 'articoli' })} con “${words}”`,
		searchAll: 'Cerca in tutti i feed',
		feedFailing:
			'rueder riprova più tardi, e aspetta di più dopo ogni errore o quanto chiede il sito. Se continua a non riuscire, il feed potrebbe essersi spostato.',
		hiddenNote:
			'Le tue regole di filtro hanno nascosto questi articoli. Aprine uno per vedere quale regola è stata.',
		toRules: 'Regole di filtro',
		emptyFeed: 'Questo feed non ha ancora articoli.',
		waitingFeed:
			'Questo feed non è ancora stato scaricato. I suoi articoli arrivano con il prossimo giro di scaricamenti, entro pochi minuti.',
		showAll: 'Mostra tutti gli articoli',
		more: 'Mostra altri',
		star: 'Aggiungi stella',
		unstar: 'Rimuovi stella',
		keepUnread: 'Lascia da leggere',
		openOriginal: 'Apri l’originale',
		showFeed: 'Mostra tutti gli articoli di questo feed in rueder',
		from: 'da',
		by: (author) => `di ${author}`,
		updated: 'aggiornato',
		updatedHeadline: 'nuovo titolo',
		updatedTitle: (count) =>
			`Cambiato dal primo scaricamento; ${n(count)} ${plural(count, { one: 'versione precedente', other: 'versioni precedenti' })}`,
		updatedHeadlineTitle: (count) =>
			`Il titolo è stato riscritto, il testo è rimasto uguale; ${n(count)} ${plural(count, { one: 'versione precedente', other: 'versioni precedenti' })}`,
		compare: 'Confronta le versioni',
		hiddenBy: (rule) => `Nascosto dalla regola: ${rule}`,
		textDropped: 'Questo server non conserva più il testo di questo articolo, perché è vecchio.',
		textDroppedOriginal: 'Leggilo alla fonte',
		loading: 'Caricamento del testo …',
		loadingList: 'Caricamento …',
		toTop: 'Torna in cima',
		pick: 'Scegli un articolo per leggerlo qui.',
		resizeList: 'Larghezza dell’elenco: trascina, oppure usa le frecce',
		closeArticle: 'Chiudi l’articolo',
		blockedFeed:
			'Chi gestisce questo rueder ha bloccato questo feed. Non viene scaricato e i suoi articoli non vengono mostrati.',
		articleActions: 'Altre azioni per l’articolo',
		articleInfo: 'Informazioni sull’articolo …',
		articleSource: 'Codice sorgente …',
		reportArticle: 'Segnala l’articolo …'
	},

	report: {
		feedTitle: 'Segnala il feed',
		articleTitle: 'Segnala l’articolo',
		text: (title) =>
			`Chi gestisce questo rueder vede che “${title}” è stato segnalato, il motivo e quello che scrivi, ma non chi l’ha segnalato. Decide se bloccarlo per tutti.`,
		reason: 'Motivo',
		reasons: {
			illegal: 'Contro la legge',
			copyright: 'Opera di qualcun altro',
			malicious: 'Malware, phishing o truffa',
			spam: 'Spam',
			other: 'Altro'
		},
		note: 'Cosa vorresti aggiungere (facoltativo)',
		left: (count) =>
			`${plural(count, { one: 'Resta', other: 'Restano' })} ${n(count)} ${plural(count, { one: 'carattere', other: 'caratteri' })}`,
		send: 'Invia la segnalazione',
		sending: 'Invio …',
		sent: 'Grazie. La segnalazione è arrivata a chi gestisce questo rueder.',
		tooMany: 'Hai inviato molte segnalazioni di recente. Riprova più tardi.',
		unreachable: 'La segnalazione non è arrivata al server. Riprova più tardi.'
	},

	feedInfo: {
		title: 'Informazioni sul feed',
		text: (title) =>
			`Quello che questo rueder sa di “${title}” e come lo scarica. Se il feed non funziona, copialo nel tuo messaggio a chi lo gestisce.`,
		number: 'Numero',
		name: 'Nome',
		address: 'Indirizzo del feed',
		site: 'Sito web',
		noSite: 'Nessuno',
		agent: 'Scaricato come',
		agents: { rueder: 'rueder', browser: 'Browser' },
		every: 'Scaricato ogni',
		fetched: 'Ultimo scaricamento',
		succeeded: 'Ultimo scaricamento riuscito',
		failed: 'Ultimo errore',
		next: 'Prossimo scaricamento',
		blocked: 'Nessuno, chi gestisce ha bloccato il feed',
		articles: 'Articoli finora',
		never: 'Mai',
		fetchAs: 'Scaricalo come',
		fetchAsHint:
			'Per tutti quelli che leggono il feed. Come browser, per i siti che respingono rueder. Viene scaricato di nuovo subito.',
		notSwitched: 'Non ha funzionato. Riprova più tardi.',
		copy: 'Copia',
		copied: 'Copiato',
		loading: 'Caricamento …',
		unreachable: 'Il server non ha risposto. Riprova più tardi.'
	},
	articleInfo: {
		title: 'Informazioni sull’articolo',
		text: (title) =>
			`Quello che questo rueder sa di “${title}” e delle sue versioni. Se qualcosa non ti sembra giusto, copialo nel tuo messaggio a chi lo gestisce.`,
		feed: 'Feed',
		feedNumber: 'Numero del feed',
		number: 'Numero nel feed',
		name: 'Titolo',
		id: 'Identificativo nel feed',
		link: 'Link',
		noLink: 'Nessuno',
		published: 'Pubblicato',
		fetched: 'Primo scaricamento',
		changed: 'Ultima modifica',
		never: 'Mai',
		changedAt: (when, what) => `${when} (${what})`,
		what: { title: 'il titolo', text: 'il testo', both: 'il titolo e il testo' },
		version: 'Versione',
		kept: 'Versioni conservate',
		textLabel: 'Testo',
		textDropped: 'Non più conservato, perché è vecchio',
		copy: 'Copia',
		copied: 'Copiato',
		loading: 'Caricamento …',
		unreachable: 'Il server non ha risposto. Riprova più tardi.'
	},
	articleSource: {
		title: 'Codice sorgente dell’articolo',
		text: 'L’articolo in HTML, come questo rueder lo conserva dopo averlo ripulito, o come l’ha inviato il feed. Link e immagini mantengono qui i loro indirizzi, così si vede se è rimasto un parametro di tracciamento.',
		show: 'Mostra',
		kinds: { cleaned: 'Ripulito', original: 'Come l’ha inviato il feed' },
		noText: 'Questo server non conserva più il testo di questo articolo.',
		noOriginal: 'Questo server non conserva più quello che ha inviato il feed.',
		copy: 'Copia',
		copied: 'Copiato',
		loading: 'Caricamento …',
		unreachable: 'Il server non ha risposto. Riprova più tardi.'
	},

	media: {
		title: 'Allegati',
		play: 'Riproduci',
		image: 'Immagine',
		zoomIn: 'Ingrandisci',
		zoomOut: 'Riduci'
	},

	connection: {
		reconnecting: 'Riconnessione …',
		unreachable: (since) => `Il server non risponde da ${since}.`,
		offline: (since) => `Nessuna rete da ${since}.`,
		retry: 'Riprova ora',
		unsent: (count) =>
			plural(count, {
				one: '1 modifica è in attesa.',
				other: `${n(count)} modifiche sono in attesa.`
			})
	},

	notice: {
		unreachable: 'Il server non risponde. Quello che vedi potrebbe non essere aggiornato.',
		notSaved: 'Il server non ha accettato la tua ultima modifica. È stata annullata qui.',
		tooManyUnsent: 'Ci sono così tante modifiche in attesa del server che questa è stata annullata.'
	},

	login: {
		title: 'Accedi a rueder',
		about:
			'Un lettore di feed: scarica quello che pubblicano i siti che segui e ti mostra quello che non hai ancora letto.',
		name: 'Nome',
		password: 'Password',
		submit: 'Accedi',
		checking: 'Un momento …',
		wrong: 'Nome o password errati.',
		codeNeeded:
			'Questo account ha un secondo fattore. Inserisci il codice che mostra la tua app o un codice di recupero, oppure accedi con una passkey.',
		wrongCode: 'Quel codice non è giusto. I codici cambiano ogni 30 secondi.',
		code: 'Codice',
		codeNote: 'Le sei cifre della tua app di autenticazione, o uno dei tuoi codici di recupero.',
		tooMany: (count) => `Troppi tentativi sbagliati. Riprova tra ${minutes(count)}.`,
		unreachable: 'Il server non risponde. Riprova tra un attimo.',
		retry: 'Riprova',
		or: 'oppure',
		oidc: 'Accedi con il single sign-on',
		passkey: 'Accedi con una passkey',
		passkeyCancelled:
			'Non è stata usata nessuna passkey. Riprova, oppure accedi con nome e password.',
		passkeyFailed: 'Questo browser o dispositivo non può usare una passkey qui.',
		passkeyWrong: 'Questa passkey non appartiene a nessun account qui.',
		oidcFailed: 'L’accesso con il single sign-on non ha funzionato. Riprova.',
		oidcDenied: 'L’accesso con il single sign-on è stato annullato.',
		oidcUnknown:
			'Nessun account qui corrisponde a quell’accesso. Chiedi a chi gestisce questo rueder di collegarlo.',
		disabled: 'Questo account è disattivato. Chiedi il motivo a chi gestisce questo rueder.',
		unconfirmed:
			'Questo account aspetta che il suo indirizzo email venga confermato. Apri il link nella email, oppure fattelo inviare di nuovo.',
		resend: 'Invia di nuovo il link',
		resent: 'Il link è di nuovo in arrivo. Controlla la posta, anche nella cartella spam.',
		resendWrong: 'Inserisci il nome e la password dell’account per farti inviare di nuovo il link.',
		forgot: 'Password dimenticata?',
		removed: 'Il tuo account è stato eliminato, con tutto quello che era tuo.',
		note: 'Non c’è registrazione. Gli account li crea chi gestisce questo rueder.'
	},

	legal: {
		imprint: 'Note legali',
		privacy: 'Informativa sulla privacy',
		terms: 'Condizioni d’uso',
		agree: { before: 'Accetto le ', link: 'condizioni d’uso', after: '.' },
		changed: 'Le condizioni d’uso sono cambiate',
		changedIntro: 'Leggi le nuove condizioni d’uso. Per continuare a usare rueder, accettale.',
		read: 'Leggi le condizioni d’uso',
		accept: 'Accetto',
		decline: 'Esci',
		notSaved: 'Non ha funzionato. Riprova tra un attimo.'
	},

	register: {
		title: 'Crea un account',
		invited: 'Sei stato invitato su questo rueder. Scegli un nome e una password.',
		again: 'La password un’altra volta',
		passwordRule: (characters) => `Almeno ${n(characters)} caratteri.`,
		mismatch: 'Le due password sono diverse.',
		submit: 'Crea l’account',
		solving: 'Verifica di questo browser …',
		toRegister: 'Crea un account',
		toLogin: 'Accedi a un account che hai già',
		closed: 'Qui nessuno può registrarsi senza un invito.',
		inviteGone: 'Questo invito non si può più usare. Chiedine uno nuovo.',
		full: 'Qui non c’è posto per un altro account.',
		today: 'Oggi qui sono stati creati abbastanza account. Riprova domani.',
		captcha: 'La verifica di questo browser non è andata a buon fine. Riprova.',
		taken: 'Esiste già un account con questo nome.',
		terms: 'Accetta le condizioni d’uso per creare un account.',
		invalid: (reason) => `Questo non può essere un account. Il server dice: ${reason}`,
		email: 'Indirizzo email',
		emailOptional: 'Indirizzo email (facoltativo)',
		emailNote: 'Riceverai un link per confermarlo.',
		emailRequiredNote:
			'Riceverai un link per confermarlo; l’account potrà accedere quando lo avrai aperto.',
		emailInvalid: 'Inserisci un indirizzo email che possa ricevere posta.',
		unsent:
			'Non è stato possibile inviare il link, quindi nessun account è stato creato. Riprova più tardi.',
		tooMany: (count) => `Troppi tentativi. Riprova tra ${minutes(count)}.`,
		awaitingTitle: 'Controlla la posta',
		awaiting: (address, hours) =>
			`L’account è creato. Un link è stato inviato a ${address}: aprilo entro ${n(hours)} ore, poi accedi.`
	},

	forgot: {
		title: 'Password dimenticata?',
		intro:
			'Indica il tuo nome o il tuo indirizzo email. Un link per impostare una nuova password arriva all’indirizzo confermato.',
		field: 'Nome o indirizzo email',
		submit: 'Invia il link',
		asked: (count) =>
			`Se esiste un account così con un indirizzo confermato, gli sta arrivando un link, valido per ${minutes(count)}. Controlla anche la cartella spam.`,
		tooMany: (count) => `Richiesto troppe volte. Riprova tra ${minutes(count)}.`,
		back: 'Torna all’accesso'
	},

	reset: {
		title: 'Imposta una nuova password',
		intro: 'Tutti i browser e i dispositivi dell’account verranno disconnessi.',
		fresh: 'Nuova password',
		again: 'Nuova password un’altra volta',
		submit: 'Imposta la password',
		mismatch: 'Le due non coincidono.',
		gone: 'Questo link non funziona più. Chiedine uno nuovo.',
		invalid: (reason) => `Il server non ha accettato quella password. Dice: ${reason}`,
		done: 'La password è impostata. Accedi con quella.',
		cancel: 'Annulla'
	},

	email: {
		title: 'Indirizzo email',
		intro: 'rueder ti scrive solo a proposito del tuo account.',
		current: 'Il tuo indirizzo',
		none: 'L’account non ha un indirizzo.',
		pending: (address, hours) =>
			`Un link è stato inviato a ${address}. L’indirizzo vale una volta aperto, entro ${n(hours)} ore; fino ad allora resta quello di prima.`,
		address: 'Nuovo indirizzo',
		password: 'La tua password',
		passwordNote: 'Così nessun altro che usa questo browser può dirottare la tua posta.',
		code: 'Codice del secondo fattore',
		submit: 'Invia il link',
		sent: 'Il link è in arrivo. Controlla la posta, anche nella cartella spam.',
		remove: 'Rimuovi l’indirizzo',
		removeIntro:
			'Senza indirizzo non ti arriva nessuna email. Per il resto, l’account resta com’è.',
		removed: 'L’indirizzo è stato rimosso.',
		wrong: 'La password o il codice non è giusto.',
		codeNeeded: 'Questo account ha un secondo fattore. Inserisci anche il suo codice.',
		loginAgain:
			'Il tuo account non ha ancora una password né un secondo fattore. Accedi di nuovo, poi indica l’indirizzo entro dieci minuti.',
		invalid: 'Inserisci un indirizzo email che possa ricevere posta.',
		same: 'È già l’indirizzo dell’account.',
		required:
			'Ogni account di questo rueder ha bisogno di un indirizzo; si può cambiare, non rimuovere.',
		busy: (count) => `Troppi tentativi o email. Riprova tra ${minutes(count)}.`,
		unsent: 'Non è stato possibile inviare l’email. Riprova più tardi.',
		unreachable: 'Il server non risponde. Non è cambiato niente.',
		askTitle: 'Aggiungi un indirizzo email',
		askIntro:
			'Ora ogni account di questo rueder ha bisogno di un indirizzo email. Indica il tuo e apri il link che ti viene inviato.',
		askWaiting:
			'Quando apri il link, questo sparisce da solo. Non è arrivato niente? Invialo di nuovo, o a un altro indirizzo.',
		logOut: 'Esci',
		confirmed: (address) => `L’indirizzo ${address} è confermato.`,
		confirmedLogIn: 'L’account è pronto: accedi.',
		gone: 'Questo link non si può più usare: è già stato usato, ne è stato inviato uno più recente, oppure ha più di un giorno.',
		taken: 'Nel frattempo un altro account ha preso questo indirizzo.',
		confirmUnreachable: 'Il server non risponde. Riapri il link tra un attimo.',
		close: 'Chiudi'
	},

	invites: {
		intro:
			'Un link permette a qualcuno di creare un account su questo rueder. Viene mostrato una volta sola, qui, quando lo crei.',
		left: (count) =>
			count === 0
				? 'Non hai inviti da dare.'
				: `Puoi invitare ancora ${n(count)} ${plural(count, { one: 'persona', other: 'persone' })}.`,
		admin: 'Come admin puoi sempre invitare.',
		uses: 'Per quanti account',
		make: 'Crea un link',
		noneLeft: 'Non ti restano inviti per così tanti.',
		failed: 'Non ha funzionato. Riprova tra un attimo.',
		link: 'Il link, da passare ora',
		copy: 'Copia',
		copied: 'Copiato',
		goodFor: (days) =>
			`Un link vale ${n(days)} ${plural(days, { one: 'giorno', other: 'giorni' })}.`,
		yours: 'I tuoi link',
		none: 'Non hai ancora creato nessun link.',
		used: (uses, of) => `${n(uses)} di ${n(of)} usati`,
		until: (when) => `valido fino al ${when}`,
		ran: (when) => `scaduto ${when}`,
		usedUp: 'esaurito',
		revoked: 'ritirato',
		revoke: 'Ritira',
		revokeLink: (made) => `Ritira il link creato ${made}`
	},

	home: {
		newIn: 'Novità nelle tue iscrizioni',
		allRead: 'Hai letto tutto. I nuovi articoli compaiono qui appena arrivano.',
		recentlyStarred: 'Con stella di recente',
		noStars: 'Ancora niente con stella. Premi s su un articolo.',
		problems: 'Feed con problemi',
		tips: 'Suggerimenti',
		tipKeys: 'Premi ? per tutte le scorciatoie da tastiera; j e k scorrono gli articoli.',
		tipDrag: 'Trascina feed e cartelle nella barra laterale per ordinarli.',
		tipFilters:
			'Le regole di filtro nelle impostazioni nascondono articoli per parola chiave o autore.',
		showFeed: (title) => `Mostra ${title}`
	},

	versions: {
		title: 'Versioni di questo articolo',
		current: 'Versione attuale',
		version: (rev) => `Versione ${n(rev)}`,
		fetched: (when) => `scaricata ${when}`,
		compareWith: 'Confronta la versione attuale con',
		legendAdded: 'aggiunto',
		legendRemoved: 'rimosso',
		titleWas: 'Il titolo era:',
		same: 'Il testo di questa versione è uguale a quello attuale.',
		sameWords: 'Le parole sono le stesse; sono cambiate immagini o link.',
		markup: 'Parole, immagini e link sono gli stessi; è cambiata solo la struttura del testo.',
		pictures: 'Immagini cambiate',
		noPicture: 'Nessuna',
		address: 'Indirizzo dell’immagine',
		links: 'Link cambiati'
	},

	account: {
		title: 'Impostazioni dell’account',
		tabs: {
			password: 'Password',
			email: 'Email',
			security: 'Sicurezza',
			apps: 'App',
			invites: 'Inviti',
			move: 'Trasloco',
			privacy: 'Privacy',
			remove: 'Elimina l’account'
		},
		password: {
			title: 'Cambia la password',
			titleFirst: 'Imposta una password',
			intro:
				'Una nuova password disconnette tutti gli altri browser e dispositivi e revoca tutte le password per le app.',
			introFirst:
				'Con una password, questo account può accedere anche con il suo nome, non solo con il single sign-on.',
			current: 'Password attuale',
			fresh: 'Nuova password',
			again: 'Nuova password un’altra volta',
			submit: 'Cambia la password',
			submitFirst: 'Imposta la password',
			done: 'La password è cambiata. Tutti gli altri browser e dispositivi sono stati disconnessi, e le password per le app sono state revocate.',
			doneFirst: 'La password è impostata.',
			mismatch: 'Le due non coincidono.',
			code: 'Codice del secondo fattore',
			wrong: 'La password attuale o il codice non è giusto.',
			codeNeeded:
				'Questo account ha un secondo fattore. Inserisci anche un codice della tua app di autenticazione o un codice di recupero, oppure conferma con una passkey.',
			loginAgain:
				'Il tuo account non ha ancora una password né un secondo fattore. Accedi di nuovo, poi imposta la password entro dieci minuti.',
			invalid: (characters) =>
				`Il server non ha accettato quella password. Almeno ${n(characters)} caratteri.`,
			busy: (count) => `Troppi tentativi sbagliati. Riprova tra ${minutes(count)}.`,
			unreachable: 'Il server non risponde. La password non è cambiata.'
		},
		settingsNote: 'Aspetto, lettura, regole di filtro e tastiera sono in Impostazioni.'
	},

	settings: {
		title: 'Impostazioni',
		tabs: {
			look: 'Aspetto',
			reading: 'Lettura',
			rules: 'Regole di filtro',
			keys: 'Tastiera'
		},
		accountNote:
			'Password, secondo fattore, app e inviti sono in Impostazioni dell’account, nel menu del tuo account.',
		theme: 'Tema',
		themes: { system: 'Come il sistema', light: 'Chiaro', dark: 'Scuro' },
		accent: 'Colore d’accento',
		accents: {
			red: 'Rosso',
			blue: 'Blu',
			green: 'Verde',
			orange: 'Arancione',
			purple: 'Viola',
			graphite: 'Grafite'
		},
		density: 'Densità dell’elenco',
		densities: { compact: 'Compatta', comfortable: 'Comoda' },
		sidebarSide: 'Barra laterale',
		sidebarSides: { start: 'A sinistra', end: 'A destra' },
		textSize: 'Dimensione del testo degli articoli, su questo dispositivo',
		textSizes: { small: 'Piccola', medium: 'Media', large: 'Grande' },
		readingFont: 'Carattere degli articoli',
		readingFonts: { sans: 'Senza grazie', serif: 'Con grazie' },
		counters: 'Contatori da leggere',
		counterStyles: { parens: 'Tra parentesi', badge: 'Come badge' },
		language: 'Lingua',
		languageAuto: 'Come il browser',
		view: 'Vista',
		views: { list: 'Elenco', expanded: 'Espansa', split: 'Tre riquadri', magazine: 'Rivista' },
		markOnScroll: 'Segna gli articoli come letti scorrendoci sopra nella vista espansa',
		teasers: 'Mostra le prime parole del testo dopo il titolo nell’elenco',
		wakeLock: 'Tieni lo schermo acceso mentre rueder è aperto',
		wakeLockMissing: 'Questo browser non può tenere lo schermo acceso.',
		note: 'Le modifiche valgono subito e vengono salvate con il tuo account, così ti seguono su ogni dispositivo.',
		noteDemo: 'Le modifiche valgono subito e restano in questo browser.',
		apps: {
			intro:
				'Altre app per leggere feed possono usare il tuo account tramite l’API di Google Reader. Ognuna riceve una password per app tutta sua, che puoi revocare in qualsiasi momento.',
			none: 'Nessuna app è ancora configurata.',
			made: (when) => `configurata ${when}`,
			used: (when) => `ultimo uso ${when}`,
			unused: 'non ancora usata',
			revoke: 'Revoca',
			revokeName: (name) => `Revoca la password per app ${name}`,
			add: 'Configura un’app',
			full: (max) =>
				`Un account ha al massimo ${n(max)} password per app. Revocane una per configurare un’altra app.`,
			choose: 'Quale app vuoi usare?',
			website: (app) => `Sito web di ${app}`,
			other: 'Un’altra app',
			otherHint: 'Qualsiasi app che offra l’API di Google Reader o FreshRSS.',
			platforms: {
				apple: 'Mac, iPhone e iPad',
				android: 'Android'
			},
			name: 'Nome della password per app',
			password: 'La tua password',
			code: 'Codice della tua app di autenticazione, o un codice di recupero',
			create: 'Crea la password per app',
			creating: 'Un momento …',
			back: 'Indietro',
			how: {
				netnewswire:
					'In NetNewsWire, apri Impostazioni, poi Account, aggiungi un account (sul Mac con +) e scegli FreshRSS. Inserisci il nome qui sotto come Login (su iPhone e iPad: Username or Email), la password per app come Password e l’indirizzo del server come API URL.',
				readyou:
					'In Read You, apri Impostazioni, Account, Aggiungi account, e scegli Google Reader. Inserisci l’indirizzo del server qui sotto come URL del server, il nome come nome utente e la password per app come password.',
				other:
					'Nella tua app, aggiungi un account del tipo “Google Reader API” o, se non c’è, “FreshRSS”. Inserisci l’indirizzo del server, il nome e la password per app qui sotto.'
			},
			ownIcons: (app) =>
				`Privacy: ${app} carica da sola le icone dei tuoi feed, dai siti web dei feed, che in quel momento vedono il tuo indirizzo IP. Gli articoli e le loro immagini arrivano solo tramite rueder.`,
			shotsTitle: 'Passo per passo, con immagini',
			shots: {
				readyou: [
					'Tocca l’ingranaggio in alto a sinistra.',
					'Apri Account.',
					'Tocca Aggiungi account.',
					'Scegli Google Reader.',
					'Inserisci l’indirizzo del server come URL del server, il nome come nome utente e la password per app come password, poi tocca Aggiungi.'
				],
				netnewswire: [
					'Sul Mac: apri Impostazioni (⌘,), poi Account, e fai clic su + in basso a sinistra.',
					'Scegli FreshRSS e fai clic su Continua.',
					'Inserisci il nome come Login, la password per app come Password e l’indirizzo del server come API URL, poi fai clic su Crea.'
				]
			},
			shotAlt: (app, step) => `${app}, passo ${n(step)}`,
			server: 'Indirizzo del server',
			user: 'Nome',
			secret: 'Password per app',
			copy: (what) => `Copia ${what}`,
			copied: 'Copiato',
			once: 'La password per app viene mostrata solo ora. Se la perdi, revocala e creane una nuova.',
			waiting: 'In attesa che l’app acceda …',
			connected: 'Collegata: l’app ha effettuato l’accesso.',
			done: 'Fatto',
			refusals: {
				wrongPassword: 'La password non è giusta.',
				codeNeeded:
					'Inserisci anche un codice della tua app di autenticazione o un codice di recupero, oppure conferma con una passkey.',
				wrongCode: 'Quel codice non è giusto. I codici cambiano ogni 30 secondi.',
				loginAgain:
					'Il tuo account non ha ancora una password né un secondo fattore. Accedi di nuovo, poi crea la password per app entro dieci minuti.',
				tooManyApps: 'Un account ha al massimo 20 password per app.',
				tooMany: 'Troppi tentativi sbagliati. Riprova tra qualche minuto.',
				unreachable: 'Il server non risponde. Riprova tra un attimo.'
			}
		},
		secondFactor: {
			title: 'Secondo fattore',
			intro:
				'Con un secondo fattore, per accedere serve anche un codice di un’app di autenticazione, oppure una passkey.',
			app: 'App di autenticazione',
			on: 'Attivo.',
			left: (count) =>
				`${plural(count, { one: 'Resta', other: 'Restano' })} ${n(count)} ${plural(count, { one: 'codice di recupero', other: 'codici di recupero' })}.`,
			off: 'Disattivato.',
			byPasskeys: 'Non configurata. Le tue passkey tengono attivo il secondo fattore.',
			adminNeeds:
				'Questo server considera admin gli admin solo con un secondo fattore. Configuralo, oppure accedi di nuovo con il suo codice, per vedere lo stato del server.',
			password: 'La tua password',
			start: 'Configura',
			scan: 'Inquadra questo codice con la tua app di autenticazione, oppure inserisci la chiave qui sotto.',
			qr: 'Codice QR per l’app di autenticazione',
			larger: 'Mostra il codice QR più grande',
			smaller: 'Mostra il codice QR più piccolo',
			key: 'Chiave',
			copyKey: 'Copia la chiave',
			copied: 'Copiato',
			code: 'Il codice che l’app mostra ora',
			codeOrRecovery: 'Codice dell’app, o un codice di recupero',
			confirm: 'Conferma',
			cancel: 'Annulla',
			codesTitle: 'I tuoi codici di recupero',
			codesIntro:
				'Ognuno ti fa accedere una volta senza il telefono. Conservali bene: vengono mostrati solo ora.',
			copyCodes: 'Copia i codici',
			codesDone: 'Li ho conservati',
			switchOff: 'Disattiva',
			switchOffNote: 'Poi per accedere basta di nuovo la sola password.',
			switchOffNoteKeys: 'Le tue passkey tengono attivo il secondo fattore.',
			startWithPasskey: 'Configura con una passkey',
			switchOffWithPasskey: 'Disattiva con una passkey',
			refusals: {
				wrongPassword: 'La password non è giusta.',
				codeNeeded:
					'Le tue passkey tengono attivo il secondo fattore. Inserisci anche un codice di recupero.',
				wrongCode: 'Quel codice non è giusto. I codici cambiano ogni 30 secondi.',
				loginAgain:
					'Il tuo account non ha ancora una password né un secondo fattore. Accedi di nuovo, poi configura l’app entro dieci minuti.',
				tooMany: 'Troppi tentativi sbagliati. Riprova tra qualche minuto.',
				unreachable: 'Il server non risponde. Riprova tra un attimo.'
			}
		},
		passkeys: {
			title: 'Passkey',
			intro:
				'Una passkey ti fa accedere senza nome e password, con il PIN, l’impronta o il volto che chiede il tuo dispositivo. Vale come secondo fattore.',
			once: 'Con una passkey, la tua password da sola non apre più il tuo account.',
			none: 'Ancora nessuna passkey.',
			made: (when) => `aggiunta ${when}`,
			used: (when) => `ultimo uso ${when}`,
			unused: 'non ancora usata',
			add: 'Aggiungi una passkey',
			name: 'Nome, per esempio quello del dispositivo',
			defaultName: 'Passkey',
			password: 'La tua password',
			code: 'Codice della tua app di autenticazione, o un codice di recupero',
			create: 'Crea la passkey',
			working: 'Segui il tuo browser …',
			cancel: 'Annulla',
			remove: 'Rimuovi',
			removeName: (name) => `Rimuovi la passkey ${name}`,
			confirmRemove: 'Rimuovi la passkey',
			full: (max) => `Un account ha al massimo ${n(max)} passkey.`,
			or: 'oppure',
			confirmWith: 'Conferma con una passkey',
			codesTitle: 'I tuoi codici di recupero',
			codesIntro:
				'La tua prima passkey ha attivato il secondo fattore. Ogni codice ti fa accedere una volta se perdi il dispositivo. Conservali bene: vengono mostrati solo ora.',
			copyCodes: 'Copia i codici',
			copied: 'Copiato',
			codesDone: 'Li ho conservati',
			refusals: {
				cancelled: 'La finestra del browser è stata chiusa prima di usare la passkey.',
				known: 'Questo dispositivo ha già una passkey per il tuo account.',
				failed: 'Questo browser o dispositivo non è riuscito a creare o usare una passkey.',
				wrong: 'La passkey non è stata accettata. Riprova.',
				wrongPassword: 'La password non è giusta.',
				codeNeeded:
					'Il tuo account ha un secondo fattore. Inserisci anche un codice della tua app o un codice di recupero, oppure conferma con una passkey.',
				wrongCode: 'Quel codice non è giusto. I codici cambiano ogni 30 secondi.',
				tooManyPasskeys: 'Un account ha al massimo 10 passkey.',
				loginAgain:
					'Il tuo account non ha ancora una password né un secondo fattore. Accedi di nuovo, poi aggiungi la tua prima passkey entro dieci minuti.',
				tooMany: 'Troppi tentativi sbagliati. Riprova tra qualche minuto.',
				unreachable: 'Il server non risponde. Riprova tra un attimo.'
			}
		},
		reports: {
			send: 'Invia a questo server le segnalazioni di errori di questo browser',
			intro:
				'Quando qualcosa va storto sulla pagina, gli admin di questo server lo vengono a sapere.',
			what: 'Una segnalazione contiene:',
			holds: [
				'il messaggio dell’errore, senza indirizzi e senza numeri che identifichino qualcosa',
				'il punto dei file di rueder in cui è successo',
				'la versione di rueder e il tuo browser con la sua versione principale'
			],
			never:
				'Mai quello che leggi, i tuoi feed, etichette o regole. Le segnalazioni spariscono dopo 30 giorni.',
			off: 'Disattivare questa opzione elimina le segnalazioni che hai inviato.'
		}
	},

	rules: {
		intro:
			'Le regole nascondono articoli o li segnano come letti quando arrivano. Gli articoli nascosti non contano come da leggere e restano in “Nascosti dai filtri”.',
		none: 'Ancora nessuna regola.',
		add: 'Aggiungi una regola',
		new: 'Nuova regola',
		change: 'Modifica la regola',
		on: 'Attiva',
		scope: 'In',
		scopeAll: 'Tutti i feed',
		scopeFolder: (title) => `Cartella: ${title}`,
		scopeFeed: (title) => `Feed: ${title}`,
		field: 'Guarda in',
		fields: { any: 'Ovunque', title: 'Titolo', content: 'Testo', author: 'Autore', tag: 'Tag' },
		match: 'Confronto',
		matches: {
			contains: 'contiene',
			word: 'contiene la parola',
			starts: 'inizia con',
			ends: 'finisce con',
			regex: 'corrisponde all’espressione regolare',
			all: 'ogni articolo, qualunque cosa dica'
		},
		pattern: 'Testo da cercare',
		action: 'Allora',
		actions: { hide: 'nascondere l’articolo', read: 'segnarlo come letto' },
		preview: (count, total) => `Vale ora per ${n(count)} di ${n(total)} articoli qui.`,
		regexTip:
			'Le espressioni regolari vengono lette come in Rust, senza distinguere maiuscole e minuscole.',
		regexBuilder: 'Costruiscine e provane una su regex101',
		invalid: 'Questa non è un’espressione regolare valida.',
		empty: 'Inserisci il testo da cercare.',
		checking: 'Vedo cosa prenderebbe …',
		hits: (count) =>
			`${n(count)} ${plural(count, { one: 'corrispondenza', other: 'corrispondenze' })}`,
		summary: (field, match, pattern, action) => `${field} ${match} “${pattern}”: ${action}`,
		summaryAll: (action) => `Ogni articolo: ${action}`
	},

	keys: {
		title: 'Scorciatoie da tastiera',
		note: 'I tuoi tasti vengono salvati con le impostazioni. Invio, Esc e g restano come sono.',
		change: (what) => `Cambia il tasto per: ${what}`,
		changeShort: 'Cambia',
		press: 'Premi un tasto …',
		taken: (key, other) => `${key} è già il tasto per: ${other}`,
		fixed: (key) => `${key} non può essere assegnato a una scorciatoia.`,
		reset: 'Torna ai tasti di rueder',
		groups: { articles: 'Articoli', lists: 'Elenchi', go: 'Vai a', app: 'Ovunque' },
		next: 'Articolo successivo, aperto',
		previous: 'Articolo precedente, aperto',
		nextCursor: 'Articolo successivo, senza aprirlo',
		previousCursor: 'Articolo precedente, senza aprirlo',
		toggle: 'Apri o chiudi l’articolo',
		star: 'Aggiungi o rimuovi la stella',
		read: 'Segna come letto o da leggere',
		original: 'Apri l’originale in una nuova scheda',
		markAll: 'Segna tutto come letto',
		refresh: 'Aggiorna l’elenco',
		unreadOnly: 'Solo da leggere, o tutti gli articoli',
		listView: 'Vista elenco',
		expandedView: 'Vista espansa',
		splitView: 'Tre riquadri',
		magazineView: 'Rivista',
		goHome: 'Home',
		goAll: 'Tutti gli articoli',
		goUnread: 'Articoli da leggere',
		goStarred: 'Articoli con stella',
		search: 'Cerca',
		sidebar: 'Mostra o nascondi la barra laterale',
		help: 'Questa panoramica',
		then: 'poi'
	},

	labels: {
		title: 'Etichette',
		add: 'Nuova etichetta',
		name: 'Nome dell’etichetta',
		none: 'Ancora nessuna etichetta.',
		actions: (title) => `Azioni per l’etichetta ${title}`,
		rename: 'Rinomina',
		color: 'Colore',
		colors: ['Rosso', 'Arancione', 'Giallo', 'Verde', 'Turchese', 'Blu', 'Viola', 'Grigio'],
		remove: 'Elimina l’etichetta',
		show: 'Mostra le etichette',
		hide: 'Nascondi le etichette',
		forArticle: 'Etichette',
		set: (title) => `Etichetta: ${title}`
	},

	subscribe: {
		title: 'Iscriviti a un feed',
		address: 'Indirizzo del feed o del sito web',
		find: 'Trova feed',
		finding: 'Ricerca …',
		note: 'La bozza non può scaricare niente, quindi il feed resta vuoto.',
		found: (count) => (count === 1 ? 'C’è un feed:' : `Ci sono ${n(count)} feed:`),
		none: 'Non c’è nessun feed a questo indirizzo.',
		movedTo: (url) => `Il sito ha inoltrato la richiesta a ${url}.`,
		loginWall:
			'Alcuni siti mandano alla loro pagina di accesso chi non ha effettuato l’accesso. Allora i loro feed non si possono leggere.',
		subscribed: 'Iscritto',
		show: 'Mostra',
		untitled: 'Senza nome',
		page: 'Un sito web senza feed: rueder ne crea uno, solo con titoli e riassunti.',
		folder: 'Nella cartella',
		topLevel: 'Nessuna cartella',
		add: 'Iscriviti',
		adding: 'Iscrizione …',
		refused: 'Non ha funzionato.',
		serverSays: (reason) => `Il server dice: ${reason}`,
		busy: (seconds) =>
			seconds
				? `Troppe ricerche contemporaneamente. Riprova tra ${n(seconds)} ${plural(seconds, { one: 'secondo', other: 'secondi' })}.`
				: 'Troppe ricerche contemporaneamente. Riprova tra un attimo.',
		noFetching: 'Questo server non scarica feed.',
		tooLarge: (megabytes) => `Il file è più grande di ${n(megabytes)} MB.`,
		unreachable: 'Il server non risponde. Riprova tra un attimo.'
	},

	opml: {
		title: 'Importa ed esporta',
		intro:
			'Porta le tue iscrizioni da un altro lettore di feed come file OPML, oppure portale via.',
		import: 'Importa un file OPML',
		importing: 'Importazione …',
		imported: (feeds, folders) =>
			`${n(feeds)} ${plural(feeds, { one: 'nuovo feed', other: 'nuovi feed' })} e ${n(folders)} ${plural(folders, { one: 'nuova cartella', other: 'nuove cartelle' })}.`,
		known: (count) =>
			`${n(count)} ${plural(count, { one: 'feed era già seguito', other: 'feed erano già seguiti' })}.`,
		skipped: (count) =>
			`${n(count)} ${plural(count, { one: 'voce è stata tralasciata, non essendo un feed leggibile', other: 'voci sono state tralasciate, non essendo feed leggibili' })}.`,
		later: 'I loro articoli arrivano con il prossimo giro di scaricamenti.',
		export: 'Esporta come OPML'
	},

	removeAccount: {
		title: 'Elimina l’account',
		intro: 'Questo elimina per sempre il tuo account su questo rueder. Non si può annullare.',
		goes: 'Quello che se ne va con lui: le tue sessioni su tutti i dispositivi, le tue iscrizioni e cartelle, quello che hai letto, stelle, etichette, regole di filtro, impostazioni, segnalazioni di errori, inviti e il collegamento al single sign-on.',
		stays:
			'I feed restano per gli altri che li leggono. Per portare prima il tuo account altrove, usa “Trasloco”.',
		password: 'La tua password',
		name: (name) => `Scrivi il tuo nome, ${name}, per confermare`,
		code: 'Codice della tua app, o un codice di recupero',
		submit: 'Elimina per sempre il mio account',
		wrong: 'Non è giusto. Non è stato eliminato niente.',
		codeNeeded:
			'Il tuo account ha un secondo fattore. Scrivi anche un codice della tua app o un codice di recupero, oppure conferma con una passkey.',
		busy: (count) => `Troppi tentativi sbagliati. Riprova tra ${minutes(count)}.`,
		unreachable: 'Il server non risponde. Non è stato eliminato niente.'
	},

	move: {
		intro: 'Porta il tuo account su un altro rueder, o portalo qui da un altro, in un solo file.',
		howItWorks:
			'Un’importazione aggiunge a quello che c’è qui e non toglie niente. Le tue impostazioni diventano quelle del file.',
		notInIt: 'Non c’è nel file: la tua password, le tue sessioni e i testi degli articoli.',
		export: 'Esporta l’account',
		import: 'Importa un file di account',
		importing: 'Importazione …',
		feeds: (feeds, known) =>
			`${n(feeds)} ${plural(feeds, { one: 'nuovo feed', other: 'nuovi feed' })}, ${n(known)} già seguiti.`,
		folders: (count) =>
			`${n(count)} ${plural(count, { one: 'nuova cartella', other: 'nuove cartelle' })}.`,
		articles: (found) =>
			`${articles(found)} ${plural(found, { one: 'trovato', other: 'trovati' })} qui.`,
		missing: (count) =>
			`${articles(count)} non ${plural(count, { one: 'è stato trovato', other: 'sono stati trovati' })} qui; quello che ne avevi letto, contrassegnato con una stella o con un’etichetta è perso.`,
		marks: (labels, rules) =>
			`${n(labels)} ${plural(labels, { one: 'nuova etichetta', other: 'nuove etichette' })} e ${n(rules)} ${plural(rules, { one: 'nuova regola di filtro', other: 'nuove regole di filtro' })}.`,
		skipped: (count) =>
			`${n(count)} ${plural(count, { one: 'voce del file non è stata ripresa', other: 'voci del file non sono state riprese' })}.`,
		settings: 'Ora le tue impostazioni sono quelle del file.',
		later:
			'I nuovi feed portano i loro articoli con il prossimo giro di scaricamenti. Quello che avevi letto resta letto.',
		busy: (seconds) =>
			seconds
				? `Troppe importazioni. Riprova tra ${minutes(Math.ceil(seconds / 60))}.`
				: 'Troppe importazioni. Riprova più tardi.'
	},

	status: {
		title: 'Stato del server',
		open: 'Stato del server',
		loading: 'Interrogo il server …',
		failed: 'Il server non ha risposto. Riprova tra un attimo.',
		refresh: 'Aggiorna',
		asOf: (time) => `Alle ${time}. Quello che si conta è l’ultima ora.`,
		demo: 'Inventato, perché qui non c’è un server.',
		empty: '–',
		on: 'Attivo',
		off: 'Disattivato',
		took: (median, p95) => `mediana ${median}, il 95% entro ${p95}`,
		summary: {
			label: 'A colpo d’occhio',
			hints: 'Avvisi',
			failing: 'Feed che non funzionano',
			errors: 'Errori registrati',
			reported: 'Feed segnalati'
		},
		hintsTitle: 'Da guardare',
		allWell: 'Niente salta all’occhio.',
		hints: {
			slowRoute: (route, took) =>
				`${route} è lenta: una richiesta su venti impiega più di ${took}.`,
			serverErrors: (route, count) =>
				`${route} ha risposto con un errore del server ${n(count)} ${plural(count, { one: 'volta', other: 'volte' })}.`,
			poolBusy: (size) =>
				`Tutte le ${n(size)} connessioni al database sono in uso, quindi le richieste ne aspettano una.`,
			fetchBacklog: (count, late) =>
				`${n(count)} feed ${plural(count, { one: 'viene scaricato', other: 'vengono scaricati' })} in ritardo, il più in ritardo di ${late}.`,
			feedsFailing: (failing, subscribed) =>
				`${n(failing)} feed su ${n(subscribed)} non funzionano.`,
			notFetching: 'Il server non scarica feed. FETCH è impostato su off?',
			fetchPaceFull: (most) =>
				`Ogni giro prende tutti i feed che può, e alcuni aspettano ancora: il ritmo consente ${n(most)} scaricamenti all’ora. Lo impostano FETCH_ROUND, FETCH_AT_ONCE e FETCH_EVERY.`,
			cacheMisses: (share) =>
				`Il database trova in memoria solo il ${share} di quello che legge; più shared_buffers aiuterebbero.`,
			databaseLarge: (size, warn) =>
				`Il database è arrivato a ${size}; DATABASE_WARN_GB avvisa a partire da ${warn}. “rueder retention set” elimina gli articoli vecchi o i loro testi.`,
			longQuery: (took) => `Una query è in esecuzione da ${took}.`,
			imageCacheFull: (share) => `La cache delle immagini è piena al ${share}.`,
			errors: (count) =>
				`Il server ha registrato ${n(count)} ${plural(count, { one: 'errore', other: 'errori' })} nell’ultima ora.`,
			limited: (count) =>
				`${n(count)} ${plural(count, { one: 'richiesta è stata respinta', other: 'richieste sono state respinte' })} perché troppe.`,
			browserErrors: (count) =>
				`I browser hanno segnalato ${n(count)} ${plural(count, { one: 'errore', other: 'errori diversi' })} nell’ultimo giorno.`,
			reportedFeeds: (count) =>
				`I lettori hanno segnalato ${n(count)} feed che non ${plural(count, { one: 'è bloccato', other: 'sono bloccati' })}.`
		},
		build: {
			title: 'Build',
			commit: 'Commit',
			dirty: (commit) => `${commit}, con modifiche proprie`,
			branch: 'Branch',
			tag: 'Tag',
			committed: 'Data del commit',
			where: 'Gira',
			container: 'in un container',
			host: 'sulla macchina stessa',
			target: 'Per',
			profile: 'Compilato',
			profileValue: (profile, level) => `${profile}, ottimizzazione ${level}`,
			ci: 'Compilato da',
			ciValue: 'un’esecuzione della CI',
			byHand: 'a mano'
		},
		server: {
			title: 'Server',
			version: 'Versione',
			running: 'In funzione da',
			memory: 'Memoria',
			accounts: 'Account',
			feeds: 'Feed',
			feedsValue: (fetched, all) => `${n(fetched)} scaricati, ${n(all)} in tutto`,
			articles: 'Articoli',
			articlesValue: (count) => `circa ${n(count)}`,
			live: 'Schede aperte'
		},
		database: {
			title: 'Database',
			pool: 'Connessioni di rueder',
			poolValue: (size, idle, max) =>
				`${n(size)} aperte, ${n(idle)} inattive, al massimo ${n(max)}`,
			connections: 'Tutte le connessioni',
			size: 'Dimensione',
			sizeWarn: (size, warn) => `${size}, avviso a partire da ${warn}`,
			hitRatio: 'Letto dalla memoria',
			deadlocks: 'Deadlock',
			longest: 'Query più lunga in esecuzione',
			tables: 'Tabelle più grandi',
			table: 'Tabella',
			bytes: 'Dimensione',
			rows: 'Righe, circa'
		},
		fetcher: {
			title: 'Scaricamento dei feed',
			state: 'Scaricamento',
			lastRound: 'Ultimo giro',
			lastRoundValue: (ago, feeds, took) => `${ago}: ${n(feeds)} feed in ${took}`,
			fetches: 'Scaricamenti',
			fetchesValue: (fetched, unchanged, failed) =>
				`${n(fetched)} hanno portato il feed, ${n(unchanged)} invariati, ${n(failed)} non riusciti`,
			broken: 'Non riusciti nel database',
			newArticles: 'Nuovi articoli',
			took: 'Tempo per scaricamento',
			overdue: 'Scaricati in ritardo',
			overdueValue: (count, late) => `${n(count)}, il più in ritardo di ${late}`,
			waiting: 'Non ancora scaricati',
			pace: 'Ritmo',
			paceValue: (round, every, atOnce, most) =>
				`${n(round)} feed ogni ${every}, ${n(atOnce)} alla volta: al massimo ${n(most)} all’ora`,
			perHour: 'Scaricamenti nell’ultima ora',
			behind: 'Ritardo',
			behindValue: (late) => `il feed che aspetta da più tempo è in ritardo di ${late}`,
			notBehind: 'Niente in attesa',
			failing: 'Feed che non funzionano, per causa',
			noneFailing: 'Nessun feed ha problemi.',
			feeds: (count) => `${n(count)} feed`,
			failures: {
				status: (code) => (code ? `Il sito risponde ${code}` : 'Il sito risponde con un errore'),
				timeout: 'Il sito ci mette troppo',
				blocked: 'L’indirizzo è in una rete da cui rueder non scarica',
				dns: 'Il nome non ha un indirizzo',
				network: 'Nessuna connessione, oppure si è interrotta',
				too_large: 'Troppo grande',
				redirects: 'Troppi reindirizzamenti, o uno non valido',
				unreadable: 'Non è un feed che rueder sappia leggere',
				other: 'Altro'
			}
		},
		finder: {
			title: 'Ricerca di feed',
			state: 'Esame degli indirizzi',
			lookups: 'Ricerche',
			lookupsValue: (found, none, failed) =>
				`${n(found)} hanno trovato feed, ${n(none)} nessuno, ${n(failed)} non riuscite`,
			limited: 'Oltre il limite dell’account',
			took: 'Tempo per ricerca'
		},
		rssifier: {
			title: 'Siti web senza feed',
			state: 'RSSifier',
			pages: 'Siti web letti',
			pagesValue: (pages, failing) => `${n(pages)}, di cui ${n(failing)} con errori`,
			used: 'Uso del token',
			usedValue: (requests, fetches, underWay) =>
				`${n(requests)} richieste nell’ultimo minuto, ${n(fetches)} pagine scaricate nell’ultima ora, ${n(underWay)} in corso`,
			unanswered: (why) => `RSSifier non risponde: ${why}`,
			limits: 'Limiti del token',
			limitsValue: (perMinute, perHour, atOnce) =>
				`${perMinute} al minuto, ${perHour} pagine all’ora, ${atOnce} alla volta`,
			noLimit: 'nessun limite'
		},
		images: {
			title: 'Proxy delle immagini',
			state: 'Proxy',
			cache: 'Cache',
			cacheValue: (used, max) => `${used} di ${max}`,
			served: 'Immagini',
			servedValue: (cached, fetched) => `${n(cached)} dalla cache, ${n(fetched)} scaricate`,
			refused: 'Rifiutate',
			refusedValue: (source, notImage, tooLarge, notFound) =>
				`${n(source)} errore della fonte, ${n(notImage)} non immagini, ${n(tooLarge)} troppo grandi, ${n(notFound)} indirizzo sbagliato`,
			took: 'Tempo per immagine scaricata',
			failingSources: 'Fonti lasciate stare per ora'
		},
		requests: {
			title: 'Richieste',
			all: 'Tutte le richieste',
			allValue: (count, took) => `${n(count)}, ${took}`,
			answers: 'Risposte',
			answersValue: (fine, refused, errors) =>
				`${n(fine)} a buon fine, ${n(refused)} rifiutate, ${n(errors)} errori del server`,
			route: 'Percorso',
			count: 'Numero',
			median: 'Mediana',
			p95: '95%',
			max: 'La più lunga',
			errors: 'Errori',
			none: 'Nessuna richiesta nell’ultima ora.'
		},
		problems: {
			title: 'Avvisi ed errori',
			counts: (errors, warnings) =>
				`Nell’ultima ora ${n(errors)} ${plural(errors, { one: 'errore', other: 'errori' })} e ${n(warnings)} ${plural(warnings, { one: 'avviso', other: 'avvisi' })}.`,
			none: 'Non è stato registrato niente dall’avvio.',
			error: 'Errore',
			warning: 'Avviso',
			times: (count, last) => (count > 1 ? `${n(count)} volte, l’ultima ${last}` : last)
		},
		browsers: {
			title: 'Errori nei browser',
			off: 'Questo server non accetta segnalazioni di errori (CLIENT_REPORTS=off).',
			consenting: (count) =>
				`${n(count)} ${plural(count, { one: 'account invia', other: 'account inviano' })} segnalazioni. Gli errori restano 30 giorni dall’ultima volta che sono successi.`,
			none: 'Nessun browser ha segnalato errori.',
			kinds: {
				error: 'Errore',
				rejection: 'Promise rifiutata',
				component: 'Parte della pagina',
				request: 'Richiesta'
			},
			hits: (count, accounts, last) =>
				`${n(count)} ${plural(count, { one: 'volta', other: 'volte' })} in ${n(accounts)} account, l’ultima ${last}`,
			where: (versions, browsers) => `Versione ${versions} · ${browsers}`
		},
		reported: {
			title: 'Feed segnalati',
			text: 'Chi ha segnalato non viene mostrato qui. “rueder feed block <indirizzo>” blocca un feed per tutti, “rueder feed unblock <indirizzo>” lo sblocca.',
			none: 'Nessuno ha segnalato un feed.',
			reports: (count, last) =>
				`${n(count)} ${plural(count, { one: 'segnalazione', other: 'segnalazioni' })}, l’ultima ${last}`,
			blocked: 'Bloccato'
		}
	},

	update: {
		ready: 'È pronta una nuova versione di rueder.',
		reload: 'Ricarica',
		later: 'Più tardi'
	},

	crash: {
		title: 'Qualcosa è andato storto in questa pagina.',
		text: 'Ricarica la pagina per continuare. Quello che hai letto e cambiato è salvato sul server.',
		reload: 'Ricarica'
	},

	demo: {
		badge: 'Bozza',
		note: 'Una bozza con feed inventati. Niente viene salvato su un server.',
		arrive: 'Fai arrivare un articolo',
		revise: 'Cambia l’articolo aperto',
		revised: 'Aggiornamento: questo paragrafo è stato aggiunto per provare le versioni.',
		reviseHint: 'Apri prima un articolo.'
	}
};

export default it;
