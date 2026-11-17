// The texts of the start page, in every language of rueder's page
// (web/src/lib/i18n/locale.svelte.ts), with the same form of address.

const en = {
	intro: 'Your feed reader, on your own server.',
	address: 'Address of your rueder',
	open: 'Open',
	opening: 'Opening …',
	updates: 'Look for a new version of this app at every start',
	updatesHint:
		'The app then asks GitHub once at every start whether there is a newer build, and offers to install it. Nothing else is sent.',
	lookNow: 'Look now',
	looking: 'Looking …',
	current: 'This is the newest version.',
	lookFailed: 'GitHub did not answer. Try again later.',
	unreachable: (address) =>
		`${address} does not answer. Check the address, or try again once you are online.`,
	empty: 'Enter the address of your rueder.',
	invalid: 'This is not the address of a web page.',
	insecure: 'The address must start with https://: rueder logs in only over a secure connection.',
	version: (name, build) => `App version ${name} (build ${build})`,
	imprint: 'Imprint',
	privacy: 'Privacy policy',
	terms: 'Terms of use'
};

const de = {
	intro: 'Dein Feedreader, auf deinem eigenen Server.',
	address: 'Adresse deines rueder',
	open: 'Öffnen',
	opening: 'Öffne …',
	updates: 'Bei jedem Start nach einer neuen Version dieser App suchen',
	updatesHint:
		'Die App fragt dann bei jedem Start einmal bei GitHub, ob es einen neueren Build gibt, und bietet an, ihn zu installieren. Sonst wird nichts gesendet.',
	lookNow: 'Jetzt suchen',
	looking: 'Suche …',
	current: 'Das ist die neueste Version.',
	lookFailed: 'GitHub hat nicht geantwortet. Versuch es später noch einmal.',
	unreachable: (address) =>
		`${address} antwortet nicht. Prüf die Adresse, oder versuch es noch einmal, wenn du online bist.`,
	empty: 'Gib die Adresse deines rueder ein.',
	invalid: 'Das ist keine Adresse einer Webseite.',
	insecure:
		'Die Adresse muss mit https:// beginnen: rueder meldet nur über eine sichere Verbindung an.',
	version: (name, build) => `App-Version ${name} (Build ${build})`,
	imprint: 'Impressum',
	privacy: 'Datenschutzerklärung',
	terms: 'Nutzungsbedingungen'
};

const es = {
	intro: 'Tu lector de feeds, en tu propio servidor.',
	address: 'Dirección de tu rueder',
	open: 'Abrir',
	opening: 'Abriendo …',
	updates: 'Buscar una versión nueva de esta app en cada inicio',
	updatesHint:
		'La app pregunta entonces a GitHub una vez en cada inicio si hay un build más nuevo, y ofrece instalarlo. No se envía nada más.',
	lookNow: 'Buscar ahora',
	looking: 'Buscando …',
	current: 'Esta es la versión más nueva.',
	lookFailed: 'GitHub no respondió. Inténtalo de nuevo más tarde.',
	unreachable: (address) =>
		`${address} no responde. Revisa la dirección, o inténtalo de nuevo cuando tengas conexión.`,
	empty: 'Escribe la dirección de tu rueder.',
	invalid: 'Esta no es la dirección de una página web.',
	insecure:
		'La dirección debe empezar con https://: rueder solo inicia sesión mediante una conexión segura.',
	version: (name, build) => `Versión de la app ${name} (build ${build})`,
	imprint: 'Aviso legal',
	privacy: 'Política de privacidad',
	terms: 'Condiciones de uso'
};

const fr = {
	intro: 'Votre lecteur de flux, sur votre propre serveur.',
	address: 'Adresse de votre rueder',
	open: 'Ouvrir',
	opening: 'Ouverture …',
	updates: 'Chercher une nouvelle version de cette app à chaque démarrage',
	updatesHint:
		'L’app demande alors à GitHub, une fois à chaque démarrage, s’il existe un build plus récent, et propose de l’installer. Rien d’autre n’est envoyé.',
	lookNow: 'Chercher maintenant',
	looking: 'Recherche …',
	current: 'C’est la version la plus récente.',
	lookFailed: 'GitHub n’a pas répondu. Réessayez plus tard.',
	unreachable: (address) =>
		`${address} ne répond pas. Vérifiez l’adresse, ou réessayez une fois en ligne.`,
	empty: 'Saisissez l’adresse de votre rueder.',
	invalid: 'Ce n’est pas l’adresse d’une page web.',
	insecure:
		'L’adresse doit commencer par https:// : rueder ne se connecte que par une connexion sécurisée.',
	version: (name, build) => `Version de l’app ${name} (build ${build})`,
	imprint: 'Mentions légales',
	privacy: 'Politique de confidentialité',
	terms: 'Conditions d’utilisation'
};

const ptBR = {
	intro: 'Seu leitor de feeds, no seu próprio servidor.',
	address: 'Endereço do seu rueder',
	open: 'Abrir',
	opening: 'Abrindo …',
	updates: 'Procurar uma nova versão deste app a cada início',
	updatesHint:
		'O app então pergunta ao GitHub uma vez a cada início se há um build mais novo, e oferece instalá-lo. Nada mais é enviado.',
	lookNow: 'Procurar agora',
	looking: 'Procurando …',
	current: 'Esta é a versão mais nova.',
	lookFailed: 'O GitHub não respondeu. Tente de novo mais tarde.',
	unreachable: (address) =>
		`${address} não responde. Confira o endereço, ou tente de novo quando estiver on-line.`,
	empty: 'Digite o endereço do seu rueder.',
	invalid: 'Este não é o endereço de uma página da web.',
	insecure: 'O endereço precisa começar com https://: o rueder só entra por uma conexão segura.',
	version: (name, build) => `Versão do app ${name} (build ${build})`,
	imprint: 'Informações legais',
	privacy: 'Política de privacidade',
	terms: 'Termos de uso'
};

const it = {
	intro: 'Il tuo lettore di feed, sul tuo server.',
	address: 'Indirizzo del tuo rueder',
	open: 'Apri',
	opening: 'Apertura …',
	updates: 'Cerca una nuova versione di questa app a ogni avvio',
	updatesHint:
		'L’app chiede allora a GitHub, una volta a ogni avvio, se c’è una build più recente, e propone di installarla. Non viene inviato nient’altro.',
	lookNow: 'Cerca ora',
	looking: 'Ricerca …',
	current: 'Questa è la versione più recente.',
	lookFailed: 'GitHub non ha risposto. Riprova più tardi.',
	unreachable: (address) =>
		`${address} non risponde. Controlla l’indirizzo, oppure riprova quando sei online.`,
	empty: 'Inserisci l’indirizzo del tuo rueder.',
	invalid: 'Questo non è l’indirizzo di una pagina web.',
	insecure:
		'L’indirizzo deve iniziare con https://: rueder accede solo tramite una connessione sicura.',
	version: (name, build) => `Versione dell’app ${name} (build ${build})`,
	imprint: 'Note legali',
	privacy: 'Informativa sulla privacy',
	terms: 'Condizioni d’uso'
};

export const catalogs = { en, de, es, fr, 'pt-BR': ptBR, it };

/** The language of a tag such as "de-AT" without its region: "de". */
const primary = (tag) =>
	String(tag ?? '')
		.split(/[-_]/)[0]
		.toLowerCase();

/** The code of the language for a tag such as "de-AT" or "pt-PT", whatever its region; "en" for any other. */
export const codeFor = (language) =>
	Object.keys(catalogs).find((code) => primary(code) === primary(language)) ?? 'en';

/** The texts for a language such as "de-DE"; English for any other. */
export const textsFor = (language) => catalogs[codeFor(language)];
