// The page tells the server's refusals apart by their words, such as "second
// factor needed". Each of those words has to be one the server says: a word
// the server no longer uses would leave the page with the wrong hint, and no
// test against the made-up server would notice.

import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

/** The files under a folder whose names end in one of `endings`. */
function files(folder: string, endings: string[]): string[] {
	return readdirSync(folder).flatMap((name) => {
		const path = join(folder, name);
		if (statSync(path).isDirectory()) return files(path, endings);
		return endings.some((ending) => name.endsWith(ending)) ? [path] : [];
	});
}

/** What the page compares the server's words with, and where; a prefix ends in "…". */
function heard(): Map<string, string> {
	const page = files('src/lib', ['.ts', '.svelte']).filter(
		(file) => !/\.test\.ts$|\/test\/|\/i18n\//.test(file)
	);
	const words = new Map<string, string>();
	for (const file of page) {
		const source = readFileSync(file, 'utf8');
		const compared = /(?<!typeof )\b(?:said|word|error\.message\.trim\(\))\s*===\s*'([^']+)'/g;
		for (const [, said] of source.matchAll(compared)) words.set(said, file);
		for (const [, said] of source.matchAll(/\bsaid\.startsWith\('([^']+)'\)/g)) {
			words.set(`${said}…`, file);
		}
		for (const [, list] of source.matchAll(/const words = \[([^\]]+)\]/g)) {
			for (const [, said] of list.matchAll(/'([^']+)'/g)) words.set(said, file);
		}
	}
	return words;
}

describe('the words of the server’s refusals', () => {
	it('that the page tells apart are the server’s own', () => {
		const server = files('../server/src', ['.rs'])
			.map((file) => readFileSync(file, 'utf8'))
			.join('\n');
		const words = heard();
		// The ways of comparing above still find what the page compares.
		for (const known of ['second factor needed', 'an account has at most…', 'captcha']) {
			expect(words.has(known), known).toBe(true);
		}
		for (const [said, file] of words) {
			const prefix = said.endsWith('…');
			const written = prefix ? `"${said.slice(0, -1)}` : `"${said}"`;
			expect(server.includes(written), `${file} waits for "${said}"`).toBe(true);
		}
	});
});
