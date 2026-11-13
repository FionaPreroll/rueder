// Puts rueder's site for GitHub Pages together in one folder, from what the
// other parts of the repository build, and checks that its links lead
// somewhere. pages/README.md says what to build first.
//
//   node build.js [--out <folder>]    (default: ../_site)

import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, posix, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { brokenLinks } from './links.js';
import { render } from './markdown.js';
import { fill, footer, origin, repoUrl, sitePath } from './site.js';

const here = dirname(fileURLToPath(import.meta.url));

/** What the other parts build, where it goes on the site, and how it is built. */
export const parts = [
	{ from: 'web/build', to: 'demo', how: 'pnpm build:single in web/' },
	{ from: 'web/build-docs', to: 'docs/frontend', how: 'pnpm build:docs in web/' },
	{
		from: 'server/target/doc',
		to: 'docs/rust',
		how: 'cargo doc --no-deps --lib --document-private-items in server/'
	},
	{
		from: 'app/android/app/build/docs/javadoc',
		to: 'docs/android-app/code',
		how: './gradlew javadoc in app/android/'
	}
];

/** The READMEs that are pages of the site, with what goes above their text. */
export const readmes = [
	{ file: 'deploy/README.md', to: 'docs/self-hosting', aside: '' },
	{
		file: 'app/README.md',
		to: 'docs/android-app',
		aside:
			'<p class="aside">The app’s Java code, with its documentation: ' +
			'<a href="code/">Javadoc</a>.</p>'
	}
];

/**
 * Links that lead nowhere on the site and are not ours to mend: rustdoc shows
 * the documentation of other crates' traits on the types that implement them,
 * and these links in it lead somewhere only on those crates' own pages.
 */
export const foreignLinks = [
	// tracing's WithSubscriber, on every type.
	{ under: 'docs/rust/', link: /^dispatcher#setting-the-default-subscriber$/ },
	// tracing-subscriber's Layer, on monitor::ProblemLayer.
	{ under: 'docs/rust/', link: /^tracing_core::Subscriber(::\w+\(\))?$/ }
];

const foreign = ({ page, link }) =>
	foreignLinks.some((known) => page.startsWith(known.under) && known.link.test(link));

/** A page that sends the browser on to `to`, for addresses that only lead somewhere. */
export const redirect = (to) =>
	`<!doctype html>\n<html lang="en">\n<head>\n<meta charset="utf-8" />\n` +
	`<meta http-equiv="refresh" content="0; url=${to}" />\n<title>rueder</title>\n</head>\n` +
	`<body>\n<p><a href="${to}">Continue</a></p>\n</body>\n</html>\n`;

/**
 * The single-file build of the page writes its own links in the head (the
 * manifest, the icon) from the top of the site, as the server serves it. The
 * demo is the top of its own folder, so they start there.
 */
export const demoLinks = (html) => html.replace(/(<link\b[^>]*\bhref=")\/(?!\/)/g, '$1./');

/** Javadoc's stylesheet without the @import of the fonts it comes with. */
export const withoutFonts = (css) =>
	css.replace(/@import url\('resources\/fonts\/dejavu\.css'\);\n?/, '');

/** Puts the site together in `out` from the repository in `repo`. */
export function build({ repo, out, from }) {
	const missing = parts.filter((part) => !existsSync(join(repo, part.from)));
	if (missing.length) {
		throw new Error(
			'build these first:\n' + missing.map((part) => `  ${part.from}: ${part.how}`).join('\n')
		);
	}
	rmSync(out, { recursive: true, force: true });
	mkdirSync(out, { recursive: true });
	const foot = footer(from);

	for (const part of parts) cpSync(join(repo, part.from), join(out, part.to), { recursive: true });
	const demo = join(out, 'demo', 'index.html');
	writeFileSync(demo, demoLinks(readFileSync(demo, 'utf8')));
	// Some JDKs, Debian's and Ubuntu's among them, leave Javadoc's fonts out
	// but not the stylesheet's @import of them; then the system's fonts serve.
	const javadoc = join(out, 'docs/android-app/code');
	if (!existsSync(join(javadoc, 'resources/fonts/dejavu.css'))) {
		const style = join(javadoc, 'stylesheet.css');
		writeFileSync(style, withoutFonts(readFileSync(style, 'utf8')));
	}
	// rustdoc writes one folder per crate and no page above them.
	writeFileSync(join(out, 'docs/rust/index.html'), redirect('rueder/index.html'));
	writeFileSync(join(out, 'docs/index.html'), redirect('../'));

	// The overview, its look, and the pictures it shares with the pages.
	cpSync(join(here, 'src/style.css'), join(out, 'style.css'));
	cpSync(join(repo, 'web/static/icon.svg'), join(out, 'icon.svg'));
	cpSync(join(repo, 'docs/images/logo.png'), join(out, 'logo.png'));
	const overview = readFileSync(join(here, 'src/index.html'), 'utf8');
	writeFileSync(join(out, 'index.html'), fill(overview, { repo: repoUrl(from), footer: foot }));
	const missingPage = readFileSync(join(here, 'src/404.html'), 'utf8');
	writeFileSync(join(out, '404.html'), fill(missingPage, { root: sitePath(from), footer: foot }));

	const page = readFileSync(join(here, 'src/page.html'), 'utf8');
	for (const readme of readmes) {
		// Links between the READMEs stay on the site.
		const pages = Object.fromEntries(
			readmes.map((other) => [other.file, `${posix.relative(readme.to, other.to) || '.'}/`])
		);
		const markdown = readFileSync(join(repo, readme.file), 'utf8');
		const { title, html } = render(markdown, { file: readme.file, pages, from });
		const root = '../'.repeat(readme.to.split('/').length);
		mkdirSync(join(out, readme.to), { recursive: true });
		writeFileSync(
			join(out, readme.to, 'index.html'),
			fill(page, { title, root, aside: readme.aside, content: html, footer: foot })
		);
	}

	const broken = brokenLinks(out, sitePath(from)).filter((found) => !foreign(found));
	if (broken.length) {
		throw new Error(
			`${broken.length} links lead nowhere:\n` +
				broken.map(({ page, link }) => `  ${page}: ${link}`).join('\n')
		);
	}
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
	const at = process.argv.indexOf('--out');
	const out = resolve(at === -1 ? join(here, '../_site') : process.argv[at + 1]);
	build({ repo: resolve(here, '..'), out, from: origin() });
	console.log(`The site is in ${out}.`);
}
