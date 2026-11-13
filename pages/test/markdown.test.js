import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';
import { address, render, slug } from '../markdown.js';

const commit = '0123456789abcdef0123456789abcdef01234567';
const from = { repo: 'someone/rueder', commit, day: '2026-11-13' };
const blob = `https://github.com/someone/rueder/blob/${commit}`;
const pages = { 'deploy/README.md': './', 'app/README.md': '../android-app/' };
const at = { file: 'deploy/README.md', pages, from };

describe('slug', () => {
	it('is the anchor GitHub gives a heading', () => {
		assert.equal(slug('Bringing over the old rueder'), 'bringing-over-the-old-rueder');
		assert.equal(slug('What’s new?'), 'whats-new');
		assert.equal(slug('code & more'), 'code--more');
		assert.equal(slug('Step 2: the proxy (nginx)'), 'step-2-the-proxy-nginx');
		assert.equal(slug('Über_alles-gut'), 'über_alles-gut');
	});
});

describe('address', () => {
	it('leaves other sites and anchors as they are', () => {
		for (const href of [
			'https://example.org/a',
			'mailto:a@example.org',
			'//example.org',
			'#backup'
		]) {
			assert.equal(address(href, at), href);
		}
	});

	it('keeps a README that is a page on the site, with its anchor', () => {
		assert.equal(address('../app/README.md#building', at), '../android-app/#building');
		assert.equal(address('README.md', at), './');
	});

	it('sends every other file to GitHub, at the commit', () => {
		assert.equal(address('../server/README.md', at), `${blob}/server/README.md`);
		assert.equal(
			address('../docs/implementation.md#stand', at),
			`${blob}/docs/implementation.md#stand`
		);
		assert.equal(address('compose.yaml', at), `${blob}/deploy/compose.yaml`);
		assert.equal(
			address('../docs/images/logo.png', { ...at, image: true }),
			`https://github.com/someone/rueder/raw/${commit}/docs/images/logo.png`
		);
	});

	it('refuses a link out of the repository', () => {
		assert.throws(() => address('../../elsewhere.md', at), /leads out of the repository/);
	});
});

describe('render', () => {
	it('gives headings GitHub’s anchors, the same one twice numbered', () => {
		const { title, html } = render(
			'# Installing\n\n## Backup\n\n### Backup\n\n## `rueder` & co\n',
			at
		);
		assert.equal(title, 'Installing');
		assert.ok(html.includes('<h1 id="installing">Installing</h1>'));
		assert.ok(html.includes('<h2 id="backup">Backup</h2>'));
		assert.ok(html.includes('<h3 id="backup-1">Backup</h3>'));
		assert.ok(html.includes('<h2 id="rueder--co"><code>rueder</code> &amp; co</h2>'));
	});

	it('rewrites the links and keeps GitHub’s tables and code', () => {
		const { html } = render(
			'# A\n\nSee [the app](../app/README.md) and [the server](../server/README.md).\n\n' +
				'| a | b |\n| - | - |\n| 1 | 2 |\n\n```sh\ndocker compose up\n```\n',
			at
		);
		assert.ok(html.includes('<a href="../android-app/">the app</a>'));
		assert.ok(html.includes(`<a href="${blob}/server/README.md">the server</a>`));
		assert.ok(html.includes('<table>'));
		assert.ok(html.includes('<code class="language-sh">docker compose up'));
	});

	it('refuses a README without a heading', () => {
		assert.throws(() => render('Only text.', at), /has no heading/);
	});

	it('finds every anchor the READMEs on the site link to', () => {
		for (const file of ['deploy/README.md', 'app/README.md']) {
			const { html } = render(readFileSync(new URL(`../../${file}`, import.meta.url), 'utf8'), {
				file,
				pages,
				from
			});
			const ids = new Set([...html.matchAll(/ id="([^"]+)"/g)].map((match) => match[1]));
			for (const [, anchor] of html.matchAll(/href="#([^"]+)"/g)) {
				assert.ok(ids.has(anchor), `${file}: #${anchor}`);
			}
		}
	});
});
