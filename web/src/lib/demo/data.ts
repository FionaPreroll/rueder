// Made-up feeds and articles for the draft of the interface. Nothing here is a real
// publication. The same seed and time give the same data, so tests and screenshots
// come out the same every time.

import type { Article, ArticleVersion, Feed, TreeNode } from '#lib/model.ts';

const HOUR = 3_600_000;

/** A small seeded random number generator (mulberry32): numbers from 0 up to 1. */
export function random(seed: number) {
	let a = seed >>> 0;
	return () => {
		a = (a + 0x6d2b79f5) >>> 0;
		let t = a;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

type Rng = () => number;
const pick = <T>(rng: Rng, items: readonly T[]) => items[Math.floor(rng() * items.length)];

interface FeedSeed {
	id: string;
	title: string;
	site: string;
	hue: number;
	lang: 'de' | 'en';
	authors: string[];
	tags: string[];
	/** Headlines are a head and a tail put together. */
	heads: string[];
	tails: string[];
	/** Articles at the start, and how many arrive per day. */
	count: number;
	perDay: number;
	/** Every so many articles one is an advertisement, for the filter rules to catch. */
	adEvery?: number;
	/** Its articles come with a recording, and with their picture beside the text. */
	recorded?: boolean;
	error?: string;
}

const seeds: FeedSeed[] = [
	{
		id: 'feed-ferris',
		title: 'Ferris Wochenblatt',
		site: 'https://ferris-wochenblatt.example',
		hue: 18,
		lang: 'de',
		authors: ['Mara Lindqvist', 'Jonas Erbe'],
		tags: ['rust', 'sprache', 'werkzeuge'],
		heads: [
			'Async-Traits',
			'Der Borrow Checker',
			'Cargo-Workspaces',
			'Fehlerbehandlung',
			'Lifetimes',
			'Unsafe-Code'
		],
		tails: [
			'ohne Kopfschmerzen',
			'in der Praxis',
			'nach einem Jahr im Einsatz',
			'für Umsteiger',
			'mit weniger Boilerplate',
			'und was dabei schiefging'
		],
		count: 38,
		perDay: 1.5
	},
	{
		id: 'feed-borrow',
		title: 'Borrow Checker Diaries',
		site: 'https://borrow-diaries.example',
		hue: 32,
		lang: 'en',
		authors: ['Priya Raman'],
		tags: ['rust', 'notes'],
		heads: [
			'A week with pinned futures',
			'Rewriting the parser',
			'Zero-copy feeds',
			'My first proc macro',
			'Property tests',
			'Compile times'
		],
		tails: [
			'and what I learned',
			'the slow way',
			'in forty lines',
			'without regrets',
			'on a rainy Sunday',
			'that finally clicked'
		],
		count: 24,
		perDay: 0.4
	},
	{
		id: 'feed-kernel',
		title: 'Kernel-Notizen',
		site: 'https://kernel-notizen.example',
		hue: 210,
		lang: 'de',
		authors: ['Henrik Paulsen'],
		tags: ['linux', 'kernel'],
		heads: ['Der Scheduler', 'io_uring', 'Speicherverwaltung', 'Das Dateisystem', 'eBPF'],
		tails: [
			'im neuen Release',
			'unter Last gemessen',
			'kurz erklärt',
			'und ein hartnäckiger Bug',
			'aus Sicht eines Treibers'
		],
		count: 31,
		perDay: 0.8
	},
	{
		id: 'feed-platine',
		title: 'Platinenpost',
		site: 'https://platinenpost.example',
		hue: 150,
		lang: 'de',
		authors: ['Selin Aydın', 'Tobias Kern'],
		tags: ['hardware', 'basteln'],
		heads: [
			'Ein Einplatinenrechner',
			'Das E-Paper-Display',
			'Der Lötofen',
			'Ein Sensor für fünf Euro',
			'Die Tastatur'
		],
		tails: [
			'im Dauertest',
			'zum Selberbauen',
			'mit offenem Schaltplan',
			'für die Fensterbank',
			'aus dem 3D-Drucker'
		],
		count: 44,
		perDay: 2.5,
		adEvery: 7
	},
	{
		id: 'feed-loetkolben',
		title: 'Der Lötkolben',
		site: 'https://der-loetkolben.example',
		hue: 95,
		lang: 'de',
		authors: ['Ruth Vogel'],
		tags: ['reparieren', 'werkstatt'],
		heads: ['Das alte Radio', 'Ein Netzteil', 'Der Plattenspieler', 'Die Kaffeemühle'],
		tails: ['wieder zum Leben erweckt', 'gründlich zerlegt', 'für drei Euro repariert'],
		count: 12,
		perDay: 0.2
	},
	{
		id: 'feed-netz',
		title: 'Netzgemüse',
		site: 'https://netzgemuese.example',
		hue: 280,
		lang: 'de',
		authors: ['Carla Brandt', 'Milan Hoffmann', 'Redaktion'],
		tags: ['netzpolitik', 'datenschutz', 'überwachung'],
		heads: [
			'Die Chatkontrolle',
			'Das Transparenzgesetz',
			'Offene Daten',
			'Der Bericht der Datenschützer',
			'Die digitale Verwaltung',
			'Freies WLAN'
		],
		tails: [
			'kommt erneut auf die Tagesordnung',
			'scheitert im Ausschuss',
			'in drei Grafiken',
			'und die Kritik daran',
			'verzögert sich um ein Jahr',
			'im Faktencheck'
		],
		count: 60,
		perDay: 6
	},
	{
		id: 'feed-tagesbote',
		title: 'Tagesbote',
		site: 'https://tagesbote.example',
		hue: 0,
		lang: 'de',
		authors: ['Redaktion', 'Anne Sievers', 'Paul Okafor'],
		tags: ['politik', 'wirtschaft', 'region'],
		heads: [
			'Der Stadtrat',
			'Die Bahn',
			'Das Hafenfest',
			'Der Haushalt',
			'Die neue Brücke',
			'Das Stadtbad'
		],
		tails: [
			'beschließt den Ausbau',
			'meldet Verspätungen',
			'zieht eine erste Bilanz',
			'bleibt umstritten',
			'öffnet nach zwei Jahren wieder',
			'wird teurer als geplant'
		],
		count: 60,
		perDay: 14,
		adEvery: 9
	},
	{
		id: 'feed-harbor',
		title: 'The Harbor Gazette',
		site: 'https://harbor-gazette.example',
		hue: 200,
		lang: 'en',
		authors: ['Imogen Clarke', 'Staff'],
		tags: ['world', 'business', 'climate'],
		heads: [
			'Ferry operators',
			'The city council',
			'Local fishermen',
			'A new tram line',
			'The old lighthouse',
			'Harbor pilots'
		],
		tails: [
			'vote for a longer season',
			'face a difficult winter',
			'welcome the first visitors',
			'ask for more time',
			'reopen after the storm',
			'agree on a new timetable'
		],
		count: 60,
		perDay: 9,
		adEvery: 8
	},
	{
		id: 'feed-teleskop',
		title: 'Teleskopnachrichten',
		site: 'https://teleskopnachrichten.example',
		hue: 245,
		lang: 'de',
		authors: ['Dr. Lena Abt'],
		tags: ['astronomie', 'raumfahrt'],
		heads: ['Ein ferner Gasriese', 'Der Komet', 'Das Radioteleskop', 'Ein Doppelstern'],
		tails: [
			'zeigt sich erstmals im Infrarot',
			'überrascht die Forschung',
			'liefert schärfere Bilder',
			'gibt ein altes Rätsel auf'
		],
		count: 26,
		perDay: 0.7
	},
	{
		id: 'feed-labnotes',
		title: 'Lab Notes Weekly',
		site: 'https://labnotes-weekly.example',
		hue: 170,
		lang: 'en',
		authors: ['Owen Hartley', 'Mei Tanaka'],
		tags: ['biology', 'methods'],
		heads: ['Sourdough microbes', 'A cheaper microscope', 'Tardigrades', 'Soil samples'],
		tails: [
			'under the lens',
			'survive another experiment',
			'explained with one chart',
			'tell an older story'
		],
		count: 18,
		perDay: 0.3,
		recorded: true
	},
	{
		id: 'feed-strich',
		title: 'Strichmännchen täglich',
		site: 'https://strichmaennchen.example',
		hue: 50,
		lang: 'de',
		authors: ['kritzel'],
		tags: ['comic'],
		heads: ['Montag', 'Kaffee', 'Das Meeting', 'Die Katze', 'Der Drucker', 'Feierabend'],
		tails: ['(schon wieder)', 'mit Fußnote', 'in vier Bildern', 'ohne Worte'],
		count: 40,
		perDay: 1
	},
	{
		id: 'feed-fundstuecke',
		title: 'Fundstücke der Woche',
		site: 'https://fundstuecke.example',
		hue: 320,
		lang: 'de',
		authors: ['Ina Roth'],
		tags: ['links', 'kurioses'],
		heads: ['Zehn Links', 'Eine Karte', 'Ein Archiv', 'Drei Werkzeuge'],
		tails: [
			'für verregnete Tage',
			'die ich nicht mehr hergebe',
			'aus den Tiefen des Netzes',
			'zum Staunen'
		],
		count: 16,
		perDay: 0.15
	},
	{
		id: 'feed-nebenan',
		title: 'Blog von nebenan',
		site: 'https://blog-von-nebenan.example',
		hue: 120,
		lang: 'de',
		authors: ['Fenja'],
		tags: ['alltag', 'garten'],
		heads: ['Der Balkon', 'Das Fahrrad', 'Die Nachbarn', 'Mein Sauerteig'],
		tails: ['im November', 'hat jetzt einen Namen', 'und ich', 'nach dem Umzug'],
		count: 9,
		perDay: 0.1
	},
	{
		id: 'feed-wort',
		title: 'Wort des Tages',
		site: 'https://wort-des-tages.example',
		hue: 265,
		lang: 'de',
		authors: ['Wörterbuch'],
		tags: ['sprache'],
		heads: ['Fisimatenten', 'Kokolores', 'blümerant', 'Schabernack', 'Firlefanz', 'Mumpitz'],
		tails: ['– Herkunft und Gebrauch', '– das Wort des Tages'],
		count: 30,
		perDay: 1,
		error: '404 Not Found'
	}
];

const sentences = {
	de: [
		'Auf den ersten Blick wirkt das unscheinbar, im Alltag macht es aber einen spürbaren Unterschied.',
		'Die Zahlen aus dem letzten Quartal stützen diese Sicht nur zum Teil.',
		'Wer genauer hinsieht, findet drei Stellen, an denen es hakt.',
		'Mehrere Beteiligte wollten sich auf Nachfrage nicht äußern.',
		'Das Ergebnis überraschte selbst diejenigen, die von Anfang an dabei waren.',
		'Bis zum Frühjahr soll eine überarbeitete Fassung vorliegen.',
		'Ob das reicht, wird sich frühestens im nächsten Jahr zeigen.',
		'Die Idee ist nicht neu, wurde bisher aber selten zu Ende gedacht.',
		'Im Test hielt das Versprechen länger als erwartet.',
		'Für die meisten ändert sich zunächst wenig.',
		'Der zweite Anlauf fiel deutlich gründlicher aus als der erste.',
		'Am Ende blieb eine einfache Frage offen: Wer bezahlt das?'
	],
	en: [
		'At first glance it looks minor, but it changes how the day goes.',
		'The figures from last quarter only partly support that view.',
		'A closer look shows three places where things get stuck.',
		'Several of those involved declined to comment.',
		'The result surprised even the people who had been there from the start.',
		'A revised draft is expected by spring.',
		'Whether that is enough will not be clear before next year.',
		'The idea is not new, but it has rarely been thought through.',
		'In testing, the promise held longer than expected.',
		'For most people, little changes at first.',
		'The second attempt was far more thorough than the first.',
		'One simple question remained: who pays for it?'
	]
};

const update = {
	de: 'Nachtrag: Die Zahlen im zweiten Absatz wurden nach einem Hinweis korrigiert.',
	en: 'Update: the figures in the second paragraph were corrected after a reader wrote in.'
};

const ad = {
	de: { prefix: 'Anzeige: ', author: 'Werbeteam', tag: 'anzeige' },
	en: { prefix: 'Sponsored: ', author: 'Sponsored Desk', tag: 'sponsored' }
};

/** A picture made of two colored shapes, as a stand-in for a photo. */
function picture(hue: number, variant: number) {
	const svg =
		`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 280">` +
		`<rect width="640" height="280" fill="hsl(${hue} 45% 82%)"/>` +
		`<circle cx="${140 + variant * 60}" cy="150" r="${70 + variant * 8}" fill="hsl(${(hue + 40) % 360} 55% 62%)"/>` +
		`<rect x="${330 - variant * 20}" y="90" width="230" height="130" rx="12" fill="hsl(${(hue + 320) % 360} 40% 48%)"/>` +
		`</svg>`;
	return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

function paragraphs(rng: Rng, lang: 'de' | 'en', count: number) {
	return Array.from({ length: count }, () =>
		Array.from({ length: 2 + Math.floor(rng() * 3) }, () => pick(rng, sentences[lang])).join(' ')
	);
}

const tones = new Map<number, string>();

/**
 * A second of a tone as a sound file, in an address that holds the file itself:
 * what a made-up episode plays without a server to fetch it from.
 */
function tone(hertz: number) {
	const known = tones.get(hertz);
	if (known) return known;
	const rate = 8000;
	const bytes = new Uint8Array(44 + rate);
	const view = new DataView(bytes.buffer);
	const text = (at: number, letters: string) =>
		[...letters].forEach((letter, i) => (bytes[at + i] = letter.charCodeAt(0)));
	// The head of a WAV file: one channel, eight bits a sample.
	text(0, 'RIFF');
	view.setUint32(4, 36 + rate, true);
	text(8, 'WAVEfmt ');
	view.setUint32(16, 16, true);
	view.setUint16(20, 1, true);
	view.setUint16(22, 1, true);
	view.setUint32(24, rate, true);
	view.setUint32(28, rate, true);
	view.setUint16(32, 1, true);
	view.setUint16(34, 8, true);
	text(36, 'data');
	view.setUint32(40, rate, true);
	for (let i = 0; i < rate; i++) {
		const fade = Math.min(1, i / 400, (rate - i) / 400);
		bytes[44 + i] = 128 + Math.round(50 * fade * Math.sin((2 * Math.PI * hertz * i) / rate));
	}
	let binary = '';
	for (const byte of bytes) binary += String.fromCharCode(byte);
	const address = `data:audio/wav;base64,${btoa(binary)}`;
	tones.set(hertz, address);
	return address;
}

const toHtml = (paras: string[], image: string | null, alt: string) =>
	(image ? `<p><img src="${image}" alt="${alt}" width="640" height="280" /></p>` : '') +
	paras.map((p) => `<p>${p}</p>`).join('');

/** One article of a feed, made up from the feed's seed and the article's number. */
function makeArticle(seed: FeedSeed, seq: number, publishedAt: number): DemoArticle {
	const rng = random(seq * 7919 + seed.hue * 31 + seed.title.length);
	const lang = seed.lang;
	const isAd = !!seed.adEvery && seq % seed.adEvery === 0;
	const headline = `${pick(rng, seed.heads)} ${pick(rng, seed.tails)}`;
	const title = isAd ? ad[lang].prefix + headline : headline;
	const paras = paragraphs(rng, lang, 3 + Math.floor(rng() * 4));
	const drawn = seed.recorded || rng() < 0.35 ? picture(seed.hue, seq % 4) : null;
	// A feed of recordings gives its picture beside the text, not in it.
	const image = seed.recorded ? null : drawn;

	// Some articles changed after they were first fetched.
	const versions: ArticleVersion[] = [];
	let updatedAt: number | null = null;
	let current = paras;
	if (!isAd && seq % 6 === 0) {
		const rounds = 1 + (seq % 3);
		for (let rev = 1; rev <= rounds; rev++) {
			const before = [...current];
			versions.push({
				rev,
				fetchedAt: publishedAt + (rev - 1) * 2 * HOUR,
				title: rev === 1 && seq % 12 === 0 ? `${title}?` : title,
				html: toHtml(before, image, title)
			});
			current = [...current];
			current[1 % current.length] = paragraphs(rng, lang, 1)[0];
			if (rev === rounds) current.push(update[lang]);
		}
		updatedAt = publishedAt + rounds * 2 * HOUR;
	}

	return {
		id: `${seed.id}-${seq}`,
		feedId: seed.id,
		seq,
		title,
		teaser: paras[0],
		author: isAd ? ad[lang].author : pick(rng, seed.authors),
		tags: isAd ? [ad[lang].tag] : [pick(rng, seed.tags)],
		link: `${seed.site}/${seq}`,
		publishedAt,
		html: toHtml(current, image, title),
		enclosures: seed.recorded
			? [{ url: tone(330 + 110 * (seq % 4)), type: 'audio/wav', length: 8044 }]
			: [],
		image: seed.recorded && drawn ? { url: drawn, title } : null,
		changes: versions.length,
		versions,
		updatedAt
	};
}

/** A made-up article has its text and its earlier versions with it from the start. */
export interface DemoArticle extends Article {
	html: string;
	/** Earlier versions, oldest first; empty for an article that never changed. */
	versions: ArticleVersion[];
}

export interface DemoData {
	tree: TreeNode[];
	feeds: Record<string, Feed>;
	/** Articles per feed, oldest first. */
	articles: Record<string, DemoArticle[]>;
	/** The next article of a feed, as if it had just been fetched. */
	nextArticle: (feedId: string, seq: number, now: number) => DemoArticle;
	/** How many articles a feed gets per day, for the made-up live updates. */
	pace: Record<string, number>;
}

const feed = (id: string) => ({ kind: 'feed', id }) as const;

const tree = (): TreeNode[] => [
	{
		kind: 'folder',
		id: 'folder-tech',
		title: 'Technik',
		children: [
			{
				kind: 'folder',
				id: 'folder-software',
				title: 'Software',
				children: [
					{
						kind: 'folder',
						id: 'folder-rust',
						title: 'Rust',
						children: [feed('feed-ferris'), feed('feed-borrow')]
					},
					feed('feed-kernel')
				]
			},
			{
				kind: 'folder',
				id: 'folder-hardware',
				title: 'Hardware',
				children: [feed('feed-platine'), feed('feed-loetkolben')]
			},
			feed('feed-netz')
		]
	},
	{
		kind: 'folder',
		id: 'folder-news',
		title: 'Nachrichten',
		children: [feed('feed-tagesbote'), feed('feed-harbor')]
	},
	{
		kind: 'folder',
		id: 'folder-science',
		title: 'Wissenschaft',
		children: [feed('feed-teleskop'), feed('feed-labnotes')]
	},
	{
		kind: 'folder',
		id: 'folder-fun',
		title: 'Comics & Kurioses',
		children: [feed('feed-strich'), feed('feed-fundstuecke')]
	},
	feed('feed-nebenan'),
	feed('feed-wort')
];

/** The made-up feeds with their articles, the newest ones shortly before `now`. */
export function demoData(now: number): DemoData {
	const feeds: Record<string, Feed> = {};
	const articles: Record<string, DemoArticle[]> = {};
	const pace: Record<string, number> = {};
	const byId = new Map(seeds.map((seed) => [seed.id, seed]));

	for (const seed of seeds) {
		const gap = (24 * HOUR) / seed.perDay;
		const rng = random(seed.hue + seed.count);
		const list: DemoArticle[] = [];
		// A feed with an error got its last article a few days ago.
		let at = now - (seed.error ? 3 * 24 * HOUR : rng() * Math.min(gap, 5 * HOUR));
		for (let seq = seed.count; seq >= 1; seq--) {
			list.unshift(makeArticle(seed, seq, Math.round(at)));
			at -= gap * (0.5 + rng());
		}
		articles[seed.id] = list;
		pace[seed.id] = seed.error ? 0 : seed.perDay;
		feeds[seed.id] = {
			id: seed.id,
			title: seed.title,
			siteUrl: seed.site,
			hue: seed.hue,
			lastSeq: seed.count,
			error: seed.error ?? null
		};
	}

	return {
		tree: tree(),
		feeds,
		articles,
		pace,
		nextArticle: (feedId, seq, at) => makeArticle(byId.get(feedId)!, seq, at)
	};
}
