// The site put together from made-up parts, as small as they can be.

import assert from 'node:assert/strict';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { after, beforeEach, describe, it } from 'node:test';
import { build, demoLinks, foreignLinks, redirect, withoutFonts } from '../build.js';

const commit = '0123456789abcdef0123456789abcdef01234567';
const from = { repo: 'someone/rueder', commit, day: '2026-11-13' };
const root = mkdtempSync(join(tmpdir(), 'rueder-site-'));
after(() => rmSync(root, { recursive: true, force: true }));

const repo = join(root, 'repo');
const out = join(root, 'site');
const put = (path, text = '') => {
	mkdirSync(dirname(join(repo, path)), { recursive: true });
	writeFileSync(join(repo, path), text);
};
const read = (path) => readFileSync(join(out, path), 'utf8');

/** A repository with everything the site is made of, each part one page. */
function madeUp() {
	rmSync(repo, { recursive: true, force: true });
	put(
		'web/build/index.html',
		'<link rel="manifest" href="/manifest.webmanifest" /><link rel="icon" href="data:," />' +
			'<script>const a = \'<a href="/api/x">\';</script>'
	);
	put('web/build/manifest.webmanifest', '{}');
	put('web/build-docs/index.html', '<p>frontend</p>');
	put(
		'server/target/doc/rueder/index.html',
		'<a href="dispatcher#setting-the-default-subscriber">x</a>'
	);
	put(
		'app/android/app/build/docs/javadoc/index.html',
		'<link rel="stylesheet" href="stylesheet.css">'
	);
	put(
		'app/android/app/build/docs/javadoc/stylesheet.css',
		"/* Javadoc */\n@import url('resources/fonts/dejavu.css');\nbody { color: red }\n"
	);
	put(
		'deploy/README.md',
		'# Installing rueder\n\nThe [app](../app/README.md#building), the [server](../server/README.md).\n'
	);
	put(
		'app/README.md',
		'# rueder for Android\n\n## Building\n\nBack to [installing](../deploy/README.md).\n'
	);
	put('web/static/icon.svg', '<svg xmlns="http://www.w3.org/2000/svg"/>');
	put('docs/images/logo.png', 'png');
}

describe('build', () => {
	beforeEach(madeUp);

	it('puts every part where the overview says it is', () => {
		build({ repo, out, from });
		for (const page of [
			'index.html',
			'404.html',
			'demo/index.html',
			'docs/index.html',
			'docs/frontend/index.html',
			'docs/rust/index.html',
			'docs/rust/rueder/index.html',
			'docs/self-hosting/index.html',
			'docs/android-app/index.html',
			'docs/android-app/code/index.html'
		]) {
			assert.ok(existsSync(join(out, page)), page);
		}
		for (const card of [
			'demo/',
			'docs/self-hosting/',
			'docs/android-app/',
			'docs/rust/',
			'docs/frontend/'
		]) {
			assert.ok(read('index.html').includes(`href="${card}"`), card);
		}
		assert.equal(read('docs/index.html'), redirect('../'));
		assert.equal(read('docs/rust/index.html'), redirect('rueder/index.html'));
	});

	it('says on its own pages what it was built from', () => {
		build({ repo, out, from });
		for (const page of [
			'index.html',
			'404.html',
			'docs/self-hosting/index.html',
			'docs/android-app/index.html'
		]) {
			assert.ok(read(page).includes(`/commit/${commit}">0123456</a>`), page);
			assert.ok(!read(page).includes('{{'), page);
		}
	});

	it('makes pages of the READMEs, linked to each other on the site', () => {
		build({ repo, out, from });
		const installing = read('docs/self-hosting/index.html');
		assert.ok(installing.includes('<title>Installing rueder · rueder</title>'));
		assert.ok(installing.includes('href="../android-app/#building"'));
		assert.ok(
			installing.includes(
				`href="https://github.com/someone/rueder/blob/${commit}/server/README.md"`
			)
		);
		const app = read('docs/android-app/index.html');
		assert.ok(app.includes('<h2 id="building">Building</h2>'));
		assert.ok(app.includes('href="code/"'));
		assert.ok(app.includes('href="../self-hosting/"'));
		assert.ok(read('404.html').includes('href="/rueder/"'));
	});

	it('serves the demo from its own folder', () => {
		build({ repo, out, from });
		const demo = read('demo/index.html');
		assert.ok(demo.includes('href="./manifest.webmanifest"'));
		// What the scripts hold is theirs.
		assert.ok(demo.includes('<a href="/api/x">'));
	});

	it('leaves Javadoc’s fonts out where the JDK did', () => {
		build({ repo, out, from });
		assert.equal(
			read('docs/android-app/code/stylesheet.css'),
			'/* Javadoc */\nbody { color: red }\n'
		);
		madeUp();
		put('app/android/app/build/docs/javadoc/resources/fonts/dejavu.css', '');
		build({ repo, out, from });
		assert.ok(read('docs/android-app/code/stylesheet.css').includes('@import'));
	});

	it('names what to build first', () => {
		rmSync(join(repo, 'server/target'), { recursive: true });
		rmSync(join(repo, 'web/build-docs'), { recursive: true });
		assert.throws(
			() => build({ repo, out, from }),
			(error) =>
				error.message.includes('web/build-docs: pnpm build:docs in web/') &&
				error.message.includes('server/target/doc: cargo doc') &&
				!error.message.includes('web/build:')
		);
	});

	it('stops at a link that leads nowhere', () => {
		put('web/build-docs/index.html', '<a href="modules/gone.html">gone</a>');
		put(
			'app/README.md',
			'# rueder for Android\n\nSee [nothing](#nothing) and [a picture](pic.png).\n'
		);
		assert.throws(
			() => build({ repo, out, from }),
			(error) =>
				error.message.startsWith('1 links lead nowhere') &&
				error.message.includes('modules/gone.html')
		);
	});

	it('lets other crates’ links pass only in the Rust documentation', () => {
		put('web/build-docs/index.html', '<a href="dispatcher#setting-the-default-subscriber">x</a>');
		assert.throws(() => build({ repo, out, from }), /docs\/frontend\/index.html: dispatcher/);
		assert.ok(foreignLinks.every((known) => known.under === 'docs/rust/'));
	});
});

describe('demoLinks', () => {
	it('starts the head’s links from the demo’s folder, and only those', () => {
		assert.equal(
			demoLinks(
				'<link rel="manifest" href="/manifest.webmanifest" /><link href="//cdn.example/x" /><a href="/x">'
			),
			'<link rel="manifest" href="./manifest.webmanifest" /><link href="//cdn.example/x" /><a href="/x">'
		);
	});
});

describe('withoutFonts', () => {
	it('takes out the @import of Javadoc’s fonts and nothing else', () => {
		assert.equal(withoutFonts("@import url('resources/fonts/dejavu.css');\na{}\n"), 'a{}\n');
		assert.equal(withoutFonts("@import url('other.css');\n"), "@import url('other.css');\n");
	});
});
