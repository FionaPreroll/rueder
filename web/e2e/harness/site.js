// A made-up website for the end-to-end tests: a few feeds, a page that names its
// feed, an image, a page without a feed and a made-up RSSifier that makes one of
// it. The rueder under test fetches from it as it would from the
// web. Tests change what it serves through /_control, to let an article arrive
// or change.
//
//   node e2e/harness/site.js

import { createServer } from 'node:http';
import { deflateSync } from 'node:zlib';
import { RSSIFIER_ENV, SITE, SITE_PORT } from './ports.js';

/** A plain PNG of one color. */
function png(width, height, [r, g, b]) {
	const crcTable = Array.from({ length: 256 }, (_, n) => {
		let c = n;
		for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
		return c >>> 0;
	});
	const crc = (bytes) => {
		let c = 0xffffffff;
		for (const byte of bytes) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
		return (c ^ 0xffffffff) >>> 0;
	};
	const chunk = (type, data) => {
		const body = Buffer.concat([Buffer.from(type), data]);
		const head = Buffer.alloc(4);
		head.writeUInt32BE(data.length);
		const tail = Buffer.alloc(4);
		tail.writeUInt32BE(crc(body));
		return Buffer.concat([head, body, tail]);
	};
	const header = Buffer.alloc(13);
	header.writeUInt32BE(width, 0);
	header.writeUInt32BE(height, 4);
	header.set([8, 2, 0, 0, 0], 8);
	const row = Buffer.concat([
		Buffer.from([0]),
		Buffer.alloc(width * 3).fill(Buffer.from([r, g, b]))
	]);
	const pixels = Buffer.concat(Array.from({ length: height }, () => row));
	return Buffer.concat([
		Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
		chunk('IHDR', header),
		chunk('IDAT', deflateSync(pixels)),
		chunk('IEND', Buffer.alloc(0))
	]);
}

const picture = png(320, 120, [200, 64, 46]);

/** When the first article of a feed was published; each later one an hour after. */
const START = Date.parse('2026-11-01T08:00:00Z');

/** What an article says besides its own words: a picture, and what the cleaning has to take out. */
const extras = (n, picture = { src: '/pic.png', alt: 'The ferry' }) =>
	`<p><img src="${picture.src}" alt="${picture.alt}"><img src="${SITE}/pixel.gif" width="1" height="1"></p>` +
	`<script>window.tracked = true</script>` +
	`<p><a href="${SITE}/harbour/more?utm_source=feed&id=${n}" onclick="track()">Read more</a></p>`;

const article = (feed, n, text) => ({ n, title: `${feed} news ${n}`, text });

/**
 * The feeds by their path, each with the word its titles begin with and its articles, oldest first.
 * A path with a dash serves a copy under an address of its own (`/harbour-tree/` is the feed
 * `harbour`), so that each spec reads a feed nobody else does, whatever ran before it.
 */
function freshFeeds() {
	return {
		// Twelve articles: more than the ten a new subscription starts with as unread.
		harbour: {
			title: 'Harbour Herald',
			name: 'Harbour',
			articles: Array.from({ length: 12 }, (_, i) =>
				article('Harbour', i + 1, `The ferry <b>Seute Deern</b> runs again, trip ${i + 1}.`)
			)
		},
		// Changed by the tests while somebody reads.
		live: {
			title: 'Live Wire',
			name: 'Wire',
			articles: [article('Wire', 1, 'The first report.'), article('Wire', 2, 'The second report.')]
		},
		// Long enough that an article does not fit on a screen: for scrolling.
		long: {
			title: 'Long Story',
			name: 'Story',
			articles: Array.from({ length: 4 }, (_, i) =>
				article(
					'Story',
					i + 1,
					Array.from(
						{ length: 40 },
						(_, p) =>
							`Paragraph ${p + 1} of story ${i + 1}. ` +
							'The ferry keeps to its timetable, and the harbour keeps to the ferry. '.repeat(4)
					).join('</p><p>')
				)
			)
		},
		// One article tagged with many and long categories, as some sites do.
		tagged: {
			title: 'Tagged Times',
			name: 'Tagged',
			articles: [
				{
					...article('Tagged', 1, 'An article about nearly everything.'),
					tags: [
						'International Congress of Mathematicians',
						'Artificial intelligence in mathematics',
						'Millennium Prize Problems',
						'Poincaré conjecture',
						'Fourth dimension',
						'Rainforest regeneration',
						'Ice caves and climate history',
						'Nobel Prizes',
						'Donaudampfschifffahrtsgesellschaftskapitänsmützenabzeichen'
					]
				}
			]
		},
		// For the filter rules: some of its titles are advertising.
		market: {
			title: 'Market Courier',
			name: 'Market',
			articles: [
				article('Market', 1, 'Apples are cheap this week.'),
				{ n: 2, title: 'Sponsored: the new soldering iron', text: 'Buy it now.' },
				article('Market', 3, 'The fish stall moved.'),
				{ n: 4, title: 'Sponsored: a week of offers', text: 'Buy more.' }
			]
		}
	};
}

let feeds = freshFeeds();

const escape = (text) =>
	text.replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' })[c]);

function rss(path, feed) {
	const items = [...feed.articles]
		.reverse()
		.map(
			({ n, title, text, picture, tags = [path] }) => `<item>
<title>${escape(title)}</title>
<link>${SITE}/${path}/${n}</link>
<guid>${path}-${n}</guid>
<pubDate>${new Date(START + n * 3_600_000).toUTCString()}</pubDate>
<author>desk@example.org (Ada Desk)</author>
${tags.map((tag) => `<category>${escape(tag)}</category>`).join('\n')}
<description>${escape(`<p>${text}</p>${extras(n, picture)}`)}</description>
</item>`
		)
		.join('\n');
	return `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0"><channel>
<title>${feed.title}</title>
<link>${SITE}/${path}/</link>
<description>A made-up feed for tests</description>
${items}
</channel></rss>`;
}

/** What the tests may change, as POST /_control/<what>?feed=<path>. */
const controls = {
	/** Everything as it was at the start. */
	reset: () => {
		feeds = freshFeeds();
	},
	/** A new article arrives. */
	publish: (feed, _path, text) => {
		const n = feed.articles.length + 1;
		feed.articles.push(article(feed.name, n, text || `Report ${n}.`));
	},
	/** The newest article gets a sentence more. */
	revise: (feed, _path, text) => {
		const newest = feed.articles.at(-1);
		newest.text += ` ${text || 'Update: one more sentence.'}`;
	},
	/** The newest article gets another headline; its text stays as it is. */
	rename: (feed, _path, text) => {
		const newest = feed.articles.at(-1);
		newest.title = text || `${newest.title}, corrected`;
	},
	/** The newest article gets another picture; its words stay as they are. */
	picture: (feed, _path, text) => {
		feed.articles.at(-1).picture = { src: '/pic.png?in-port', alt: text || 'The ferry in port' };
	}
};

/** The news of the club, whose pages under /club… name no feed. */
const club = [
	{ n: 1, title: 'Summer party at the harbour', text: 'With music and fish rolls.' },
	{ n: 2, title: 'General meeting in November', text: 'The board asks everybody to come.' }
];

/** What the made-up RSSifier answers, as RSSifier does (rssifier/README.md). */
function rssifier(request, url) {
	const problem = (status, type, detail) => [
		status,
		'application/problem+json',
		JSON.stringify({ type, status, detail })
	];
	if (request.headers.authorization !== `Bearer ${RSSIFIER_ENV.RSSIFIER_TOKEN}`)
		return problem(401, 'token', 'this token is not known');
	const page = new URL(url.searchParams.get('url') ?? '', SITE);
	if (url.pathname !== '/_rssifier/v1/feed' || !page.pathname.startsWith('/club'))
		return problem(422, 'no-items', 'the page lists nothing RSSifier can tell apart');
	const items = [...club].reverse().map(({ n, title, text }) => ({
		id: `${SITE}/club/news/${n}`,
		url: `${SITE}/club/news/${n}`,
		title,
		content_html: `<p>${escape(text)}</p>`,
		date_published: new Date(START + n * 3_600_000).toISOString()
	}));
	const feed = {
		version: 'https://jsonfeed.org/version/1.1',
		title: 'The Club',
		home_page_url: `${SITE}${page.pathname}`,
		items,
		_rssifier: { source: 'guess' }
	};
	return [200, 'application/feed+json', JSON.stringify(feed)];
}

const server = createServer((request, response) => {
	const url = new URL(request.url, SITE);
	const send = (status, type, body) => {
		response.writeHead(status, { 'Content-Type': type, 'Cache-Control': 'no-store' });
		response.end(body);
	};
	const control = /^\/_control\/(\w+)$/.exec(url.pathname);
	if (control && request.method === 'POST') {
		const change = controls[control[1]];
		const path = url.searchParams.get('feed');
		const feed = feeds[path];
		if (!change || (control[1] !== 'reset' && !feed)) return send(404, 'text/plain', 'unknown');
		change(feed, path, url.searchParams.get('text'));
		return send(204, 'text/plain', '');
	}
	if (url.pathname === '/pic.png') return send(200, 'image/png', picture);
	if (url.pathname.startsWith('/_rssifier/')) return send(...rssifier(request, url));
	if (url.pathname.startsWith('/club'))
		return send(
			200,
			'text/html; charset=utf-8',
			`<!doctype html><html><head><title>The Club</title></head><body><ul>${club
				.map(({ n, title }) => `<li><a href="/club/news/${n}">${title}</a></li>`)
				.join('')}</ul></body></html>`
		);
	const [, path, rest] = /^\/([\w-]+)(?:\/(.*))?$/.exec(url.pathname) ?? [];
	const feed = path && feeds[path.split('-')[0]];
	if (!feed) return send(404, 'text/plain', 'not here');
	if (rest === 'feed.xml') return send(200, 'application/rss+xml', rss(path, feed));
	// The site's page names its feed, for rueder's search for feeds.
	return send(
		200,
		'text/html; charset=utf-8',
		`<!doctype html><html><head><title>${feed.title}</title>
<link rel="alternate" type="application/rss+xml" title="${feed.title}" href="/${path}/feed.xml">
</head><body><h1>${feed.title}</h1></body></html>`
	);
});

server.listen(SITE_PORT, '127.0.0.1', () => console.log(`the made-up site is at ${SITE}`));
