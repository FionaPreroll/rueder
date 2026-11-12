// The texts of the start page, in English and in German, as everywhere in rueder.

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
	version: (name, build) => `App version ${name} (build ${build})`
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
	version: (name, build) => `App-Version ${name} (Build ${build})`
};

export const catalogs = { en, de };

/** The texts for a language such as "de-DE"; English for any other. */
export function textsFor(language) {
	return String(language ?? '')
		.toLowerCase()
		.startsWith('de')
		? de
		: en;
}
