import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { after, describe, it } from 'node:test';
import { brokenLinks, linksOf, linksOfStyle, pagesIn, target } from '../links.js';

describe('linksOf', () => {
	it('reads href and src, in either quotes, but not in scripts and comments', () => {
		const html = `<a href="a.html">a</a><img src='b.png'><a class="x" href="c?x=1&amp;y=2">c</a>
			<!-- <a href="old.html"> --><script>const x = '<a href="gone.html">';</script>
			<script src="d.js"></script><link rel="stylesheet" href = "e.css">`;
		assert.deepEqual(linksOf(html), ['a.html', 'b.png', 'c?x=1&y=2', 'e.css']);
	});
});

describe('linksOfStyle', () => {
	it('reads @import and url(), but not data: and comments', () => {
		const css = `@import url('a.css'); @import "b.css"; x { background: url(c.png) }
			y { src: url( "d e.woff2" ) format('woff2') } z { background: url('data:image/svg+xml;x') }
			/* url(old.png) */`;
		assert.deepEqual(linksOfStyle(css), ['a.css', 'b.css', 'c.png', 'd e.woff2']);
	});
});

describe('target', () => {
	const base = '/rueder/';

	it('finds the file from the page it is on', () => {
		assert.equal(target('docs/a/index.html', '../b/', base), 'docs/b/');
		assert.equal(target('docs/a/index.html', 'x.html#part', base), 'docs/a/x.html');
		assert.equal(target('index.html', 'demo/?x=1', base), 'demo/');
		assert.equal(target('index.html', 'my%20file.html', base), 'my file.html');
	});

	it('finds a path from the top within the site', () => {
		assert.equal(target('docs/a/index.html', '/rueder/style.css', base), 'style.css');
	});

	it('marks a path out of the site as nothing there', () => {
		assert.equal(target('index.html', '/manifest.webmanifest', base), '../manifest.webmanifest');
		assert.equal(target('index.html', '../../etc/passwd', base), '../../etc/passwd');
		assert.equal(target('index.html', '%E0%A4%A', base), '../%E0%A4%A');
	});

	it('leaves other sites and anchors alone', () => {
		for (const link of [
			'https://example.org/',
			'mailto:a@example.org',
			'//example.org/x',
			'#top',
			'data:,x'
		]) {
			assert.equal(target('index.html', link, base), null, link);
		}
	});
});

describe('brokenLinks', () => {
	const dir = mkdtempSync(join(tmpdir(), 'rueder-links-'));
	after(() => rmSync(dir, { recursive: true, force: true }));
	const put = (path, text = '') => {
		mkdirSync(dirname(join(dir, path)), { recursive: true });
		writeFileSync(join(dir, path), text);
	};
	put(
		'index.html',
		'<a href="docs/">docs</a> <a href="docs/gone.html">gone</a> <a href="empty/">x</a>'
	);
	put(
		'docs/index.html',
		'<link href="../style.css"><a href="/rueder/">top</a> <a href="/elsewhere/">x</a>'
	);
	put('style.css', "@import 'fonts.css'; a { background: url(dot.png) }");
	put('dot.png');
	mkdirSync(join(dir, 'empty'));

	it('finds every page and stylesheet', () => {
		assert.deepEqual(pagesIn(dir), ['docs/index.html', 'index.html', 'style.css']);
	});

	it('lists what leads nowhere: a missing file, a folder without a page, a path off the site', () => {
		assert.deepEqual(brokenLinks(dir, '/rueder/'), [
			{ page: 'docs/index.html', link: '/elsewhere/' },
			{ page: 'index.html', link: 'docs/gone.html' },
			{ page: 'index.html', link: 'empty/' },
			{ page: 'style.css', link: 'fonts.css' }
		]);
	});
});
