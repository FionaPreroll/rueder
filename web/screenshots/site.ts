// A made-up website for the screenshots: the feeds and articles of the reader
// with made-up data (src/lib/demo/data.ts), served as RSS, with their pictures,
// icons and recordings, and the folders as an OPML file. The rueder under test
// fetches from it as it would from the web.
//
//   node screenshots/site.ts
//
// POST /_control/until?at=<time in ms> serves only the articles published
// until then, so that they can arrive in the order they were published, as
// they would over days; rueder lists them in the order they arrived. Articles
// that changed in the made-up data are served as they were first, until POST
// /_control/current: from then on as they are now, so that the next fetch gives
// them an earlier version. From then on the feed with an error in the made-up
// data answers with that error.

import { createServer } from 'node:http';
import { deflateSync } from 'node:zlib';
import { demoData, type DemoArticle } from '../src/lib/demo/data.ts';
import type { Feed, TreeNode } from '../src/lib/model.ts';
import { SITE, SITE_PORT } from '../e2e/harness/ports.js';

const data = demoData(Date.now());

/** Whether the articles are served as they are now, not as they were first. */
let current = false;
/** Articles published later than this are not served yet. */
let until = Infinity;

// ------------------------------------------------------------------ pictures

/** The color of a CSS `hsl()` as bytes. */
function hsl(hue: number, saturation: number, lightness: number): [number, number, number] {
	const s = saturation / 100;
	const l = lightness / 100;
	const k = (n: number) => (n + hue / 30) % 12;
	const a = s * Math.min(l, 1 - l);
	const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1));
	return [f(0), f(8), f(4)].map((v) => Math.round(v * 255)) as [number, number, number];
}

const crcTable = Array.from({ length: 256 }, (_, n) => {
	let c = n;
	for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
	return c >>> 0;
});

function crc(bytes: Buffer) {
	let c = 0xffffffff;
	for (const byte of bytes) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
	return (c ^ 0xffffffff) >>> 0;
}

function chunk(type: string, body: Buffer) {
	const typed = Buffer.concat([Buffer.from(type), body]);
	const head = Buffer.alloc(4);
	head.writeUInt32BE(body.length);
	const tail = Buffer.alloc(4);
	tail.writeUInt32BE(crc(typed));
	return Buffer.concat([head, typed, tail]);
}

/** A PNG whose pixels `color` paints. */
function png(width: number, height: number, color: (x: number, y: number) => number[]) {
	const header = Buffer.alloc(13);
	header.writeUInt32BE(width, 0);
	header.writeUInt32BE(height, 4);
	header.set([8, 2, 0, 0, 0], 8);
	const pixels = Buffer.alloc(height * (1 + width * 3));
	for (let y = 0; y < height; y++) {
		const row = y * (1 + width * 3);
		for (let x = 0; x < width; x++) pixels.set(color(x, y), row + 1 + x * 3);
	}
	return Buffer.concat([
		Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
		chunk('IHDR', header),
		chunk('IDAT', deflateSync(pixels)),
		chunk('IEND', Buffer.alloc(0))
	]);
}

/** Whether (x, y) is inside a rectangle with rounded corners. */
function inRounded(
	x: number,
	y: number,
	left: number,
	top: number,
	w: number,
	h: number,
	r: number
) {
	if (x < left || y < top || x >= left + w || y >= top + h) return false;
	const dx = Math.max(left + r - x, x - (left + w - r), 0);
	const dy = Math.max(top + r - y, y - (top + h - r), 0);
	return dx * dx + dy * dy <= r * r;
}

/** The stand-in for a photo that the made-up data draws as SVG (`picture` in data.ts), as a PNG. */
function picture(hue: number, variant: number) {
	const ground = hsl(hue, 45, 82);
	const disc = hsl((hue + 40) % 360, 55, 62);
	const block = hsl((hue + 320) % 360, 40, 48);
	const cx = 140 + variant * 60;
	const r = 70 + variant * 8;
	return png(640, 280, (x, y) => {
		if (inRounded(x, y, 330 - variant * 20, 90, 230, 130, 12)) return block;
		if ((x - cx) ** 2 + (y - 150) ** 2 <= r * r) return disc;
		return ground;
	});
}

/** A feed's icon: a disc on a rounded square in the feed's color. */
function icon(hue: number) {
	const ground = hsl(hue, 55, 42);
	const disc = hsl(hue, 70, 85);
	return png(64, 64, (x, y) => {
		if (!inRounded(x, y, 0, 0, 64, 64, 14)) return [255, 255, 255];
		return (x - 32) ** 2 + (y - 32) ** 2 <= 14 * 14 ? disc : ground;
	});
}

const pictures = new Map<string, Buffer>();
const remembered = (key: string, make: () => Buffer) => {
	if (!pictures.has(key)) pictures.set(key, make());
	return pictures.get(key)!;
};

/** Every other feed has an icon of its own; the others show rueder's stand-in. */
const withIcon = new Set(Object.keys(data.feeds).filter((_, i) => i % 2 === 0));

// ------------------------------------------------------------------ feeds

const escape = (text: string) =>
	text.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]!);

/** What the made-up data keeps in the page itself, as addresses on this site. */
function addressed(html: string, article: DemoArticle, feed: Feed) {
	if (!article.thumbnail) return html;
	return html.replaceAll(article.thumbnail, `${SITE}/pic/${feed.hue}-${article.seq % 4}.png`);
}

/** The recordings of the made-up data, by their file name here. */
const recordings = new Map<string, Buffer>();

function item(feed: Feed, article: DemoArticle) {
	const first = !current && article.versions.length > 0 ? article.versions[0] : null;
	const title = first?.title ?? article.title;
	const html = addressed(first?.html ?? article.html, article, feed);
	const author = article.author.replace(/[()]/g, '');
	const enclosures = article.enclosures.map((enclosure) => {
		const name = `${article.seq % 4}.wav`;
		recordings.set(name, Buffer.from(enclosure.url.split(',')[1], 'base64'));
		return `<enclosure url="${SITE}/tone/${name}" type="${enclosure.type}" length="${enclosure.length}"/>`;
	});
	const image = article.image
		? `<media:thumbnail url="${SITE}/pic/${feed.hue}-${article.seq % 4}.png"/>`
		: '';
	return `<item>
<title>${escape(title)}</title>
<link>${escape(article.link)}</link>
<guid isPermaLink="false">${article.id}</guid>
<pubDate>${new Date(article.publishedAt).toUTCString()}</pubDate>
<author>desk@example.org (${escape(author)})</author>
${article.tags.map((tag) => `<category>${escape(tag)}</category>`).join('')}
<description>${escape(html)}</description>
${enclosures.join('')}${image}
</item>`;
}

/** A feed of the made-up data; `weekly`, the same under another name, which nobody has subscribed to. */
function rss(id: string, weekly = false) {
	const feed = { ...data.feeds[id], title: data.feeds[id].title + (weekly ? ' Weekly' : '') };
	const items = data.articles[id]
		.filter((article) => article.publishedAt <= until)
		.reverse()
		.map((article) => item(feed, article));
	const image = withIcon.has(id)
		? `<image><url>${SITE}/icon/${feed.hue}.png</url><title>${escape(feed.title)}</title><link>${escape(feed.siteUrl)}</link></image>`
		: '';
	return `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/"><channel>
<title>${escape(feed.title)}</title>
<link>${escape(feed.siteUrl)}</link>
<description>A made-up feed</description>
${image}
${items.join('\n')}
</channel></rss>`;
}

/** The folders and feeds of the made-up data as an OPML file. */
function opml() {
	const outline = (node: TreeNode, depth: number): string => {
		const indent = '  '.repeat(depth);
		if (node.kind === 'feed') {
			const feed = data.feeds[node.id];
			return `${indent}<outline type="rss" text="${escape(feed.title)}" xmlUrl="${SITE}/${node.id}/feed.xml" htmlUrl="${escape(feed.siteUrl)}"/>`;
		}
		const inside = node.children.map((child) => outline(child, depth + 1)).join('\n');
		return `${indent}<outline text="${escape(node.title)}">\n${inside}\n${indent}</outline>`;
	};
	return `<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0"><head><title>Made-up feeds</title></head><body>
${data.tree.map((node) => outline(node, 1)).join('\n')}
</body></opml>`;
}

/** A website that names its feeds, for the search for feeds: its own, and one that is not in the OPML file. */
function page(id: string) {
	const feed = data.feeds[id];
	const links = [
		`<link rel="alternate" type="application/rss+xml" title="${escape(feed.title)}" href="/${id}/feed.xml">`,
		`<link rel="alternate" type="application/rss+xml" title="${escape(feed.title)} Weekly" href="/${id}/weekly.xml">`
	].join('\n');
	return `<!doctype html><html><head><title>${escape(feed.title)}</title>
${links}
</head><body><h1>${escape(feed.title)}</h1></body></html>`;
}

const server = createServer((request, response) => {
	const url = new URL(request.url ?? '/', SITE);
	const send = (status: number, type: string, body: string | Buffer) => {
		response.writeHead(status, { 'Content-Type': type, 'Cache-Control': 'no-store' });
		response.end(body);
	};
	if (url.pathname === '/_control/current' && request.method === 'POST') {
		current = true;
		return send(204, 'text/plain', '');
	}
	if (url.pathname === '/_control/until' && request.method === 'POST') {
		until = Number(url.searchParams.get('at'));
		return send(204, 'text/plain', '');
	}
	if (url.pathname === '/feeds.opml') return send(200, 'text/x-opml', opml());
	let found = /^\/pic\/(\d+)-(\d)\.png$/.exec(url.pathname);
	if (found) {
		const [hue, variant] = [Number(found[1]), Number(found[2])];
		return send(
			200,
			'image/png',
			remembered(url.pathname, () => picture(hue, variant))
		);
	}
	found = /^\/icon\/(\d+)\.png$/.exec(url.pathname);
	if (found) {
		const hue = Number(found[1]);
		return send(
			200,
			'image/png',
			remembered(url.pathname, () => icon(hue))
		);
	}
	found = /^\/tone\/(\d\.wav)$/.exec(url.pathname);
	if (found && recordings.has(found[1])) return send(200, 'audio/wav', recordings.get(found[1])!);
	found = /^\/([\w-]+)\/(feed\.xml|weekly\.xml)?$/.exec(url.pathname);
	const id = found?.[1];
	if (!id || !data.feeds[id]) return send(404, 'text/plain', 'not here');
	if (!found?.[2]) return send(200, 'text/html; charset=utf-8', page(id));
	const error = data.feeds[id].error;
	if (current && error) return send(Number(error.split(' ')[0]) || 500, 'text/plain', error);
	return send(200, 'application/rss+xml', rss(id, found[2] === 'weekly.xml'));
});

server.listen(SITE_PORT, '127.0.0.1', () => console.log(`the made-up site is at ${SITE}`));
