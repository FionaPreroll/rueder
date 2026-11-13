// Prints the footer of the site's pages: the commit and the day they are built
// from. The generators of the documentation put it on their pages as well
// (TypeDoc's customFooterHtml and Javadoc's -bottom through RUEDER_DOCS_FOOTER,
// rustdoc's --html-after-content from a file), so every page of the site says
// what it shows.
//
//   node footer.js            the footer as HTML
//   node footer.js --rustdoc  the same for rustdoc, whose pages are a row of
//                             columns: a small note at the bottom right instead

import { footer, origin } from './site.js';

const line = footer(origin());

if (process.argv.includes('--rustdoc')) {
	console.log(
		`<div class="rueder-built">${line}</div>\n<style>\n` +
			'.rueder-built { position: fixed; right: 0; bottom: 0; padding: 2px 10px; ' +
			'font-size: 12px; color: var(--main-color); background: var(--main-background-color); ' +
			'border: 1px solid var(--border-color); border-right: 0; border-bottom: 0; ' +
			'border-top-left-radius: 6px; }\n</style>'
	);
} else {
	console.log(line);
}
