// A README of the repository as a page of the site: GitHub's Markdown, its
// headings with the anchors GitHub gives them, and its links rewritten for
// where the page is. The READMEs are the repository's own text, so their HTML
// is taken as it is.

import { posix } from 'node:path';
import { Marked } from 'marked';
import { repoUrl } from './site.js';

/** The anchor GitHub gives a heading: lowercase, without punctuation, spaces as hyphens. */
export function slug(text) {
	return text
		.toLowerCase()
		.trim()
		.replace(/[^\p{L}\p{N}\s_-]/gu, '')
		.replace(/\s/g, '-');
}

const entities = { amp: '&', lt: '<', gt: '>', quot: '"', '#39': "'" };

/** The text of a piece of HTML, as a heading shows it. */
const textOf = (html) =>
	html.replace(/<[^>]*>/g, '').replace(/&(amp|lt|gt|quot|#39);/g, (_, name) => entities[name]);

/**
 * Where a link of the README `file` leads on the site: an address with a
 * scheme or only an anchor stays as it is; a file that is a page of the site
 * (`pages`, by its path in the repository) leads there; any other file of the
 * repository leads to it on GitHub, at the commit the site is built from.
 */
export function address(href, { file, pages, from, image = false }) {
	if (/^[a-z][a-z0-9+.-]*:/i.test(href) || href.startsWith('//') || href.startsWith('#')) {
		return href;
	}
	const [path, ...rest] = href.split('#');
	const anchor = rest.length ? `#${rest.join('#')}` : '';
	const target = posix.normalize(posix.join(posix.dirname(file), path));
	if (target.startsWith('..') || posix.isAbsolute(target)) {
		throw new Error(`${file}: ${href} leads out of the repository`);
	}
	if (target in pages) return pages[target] + anchor;
	return `${repoUrl(from)}/${image ? 'raw' : 'blob'}/${from.commit}/${target}${anchor}`;
}

/**
 * Turns the README `file` into HTML. Returns its title, the text of its first
 * heading, and the HTML.
 */
export function render(markdown, { file, pages, from }) {
	const seen = new Map();
	let title = null;
	const marked = new Marked({
		gfm: true,
		walkTokens(token) {
			if (token.type === 'link' || token.type === 'image') {
				token.href = address(token.href, { file, pages, from, image: token.type === 'image' });
			}
		},
		renderer: {
			heading({ tokens, depth }) {
				const html = this.parser.parseInline(tokens);
				const text = textOf(html);
				title ??= text;
				// The same heading twice gets -1, -2, … as on GitHub.
				const base = slug(text);
				const count = seen.get(base) ?? 0;
				seen.set(base, count + 1);
				const id = count ? `${base}-${count}` : base;
				return `<h${depth} id="${id}">${html}</h${depth}>\n`;
			}
		}
	});
	const html = marked.parse(markdown);
	if (!title) throw new Error(`${file} has no heading`);
	return { title, html };
}
