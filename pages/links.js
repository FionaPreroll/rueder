// Whether the links of the site lead somewhere: every address in an href or
// src of a page, and in an @import or url() of a stylesheet, that stays on the
// site has to name a file the site has. Links to other sites are not followed.
// Scripts and comments are left out, as their text is not a link.

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, posix, relative, sep } from 'node:path';

/** All pages and stylesheets below `dir`, as paths from `dir` with forward slashes. */
export function pagesIn(dir) {
	const found = [];
	for (const entry of readdirSync(dir, { recursive: true, withFileTypes: true })) {
		if (entry.isFile() && /\.(html|css)$/.test(entry.name)) {
			found.push(relative(dir, join(entry.parentPath, entry.name)).split(sep).join('/'));
		}
	}
	return found.sort();
}

/** The addresses a stylesheet loads: its @import and url(), but not the data: ones. */
export function linksOfStyle(css) {
	const text = css.replace(/\/\*[\s\S]*?\*\//g, '');
	const links = [];
	for (const match of text.matchAll(
		/@import\s+(?:url\(\s*)?(?:"([^"]*)"|'([^']*)')|url\(\s*(?:"([^"]*)"|'([^']*)'|([^)'"\s]*))\s*\)/gi
	)) {
		const link = match.slice(1).find((part) => part !== undefined);
		if (link && !link.startsWith('data:')) links.push(link);
	}
	return links;
}

/**
 * The page in one pass: a comment (to `-->` or `--!>`), a script element (to
 * its end tag, however that is written), or an href or src with its address.
 * Comments and scripts are passed over, so what they hold is not read as a link.
 */
const pieces =
	/<!--[\s\S]*?(?:--!?>|$)|<script\b[\s\S]*?(?:<\/script\b[^>]*>|$)|\s(?:href|src)\s*=\s*(?:"([^"]*)"|'([^']*)')/gi;

/** The addresses in the href and src attributes of a page, outside its scripts and comments. */
export function linksOf(html) {
	const links = [];
	for (const match of html.matchAll(pieces)) {
		const address = match[1] ?? match[2];
		if (address !== undefined) links.push(address.replace(/&amp;/g, '&'));
	}
	return links;
}

/**
 * The file a link of the page `page` names on the site, as a path from the
 * top of the site, or null where it leads off the site. `base` is the path the
 * site is served under, like /rueder/. A link that leaves the site's path
 * altogether comes back as '../…', which no file is.
 */
export function target(page, link, base) {
	if (/^[a-z][a-z0-9+.-]*:/i.test(link) || link.startsWith('//')) return null;
	let path = link.split('#')[0].split('?')[0];
	if (path === '') return null;
	try {
		path = decodeURIComponent(path);
	} catch {
		return `../${path}`;
	}
	if (path.startsWith('/')) {
		return path.startsWith(base) ? path.slice(base.length) : `..${path}`;
	}
	return posix.normalize(posix.join(posix.dirname(page), path));
}

/** Whether the site in `dir` has the file a link names: a folder counts with its index.html. */
function has(dir, path) {
	if (path.startsWith('..')) return false;
	const file = join(dir, path);
	if (!existsSync(file)) return false;
	return statSync(file).isDirectory() ? existsSync(join(file, 'index.html')) : true;
}

/** Every link of the site in `dir` that names no file of it: { page, link }. */
export function brokenLinks(dir, base) {
	const broken = [];
	for (const page of pagesIn(dir)) {
		const seen = new Set();
		const text = readFileSync(join(dir, page), 'utf8');
		for (const link of page.endsWith('.css') ? linksOfStyle(text) : linksOf(text)) {
			const path = target(page, link, base);
			if (path === null || seen.has(path)) continue;
			seen.add(path);
			if (!has(dir, path)) broken.push({ page, link });
		}
	}
	return broken;
}
