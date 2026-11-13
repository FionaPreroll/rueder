// The apps of other makers in the "Apps" tab: their pictures are there, as
// large as the list says, each with a caption in every language.

import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { catalogs, languages } from '#lib/i18n/index.ts';
import { knownApps, serverAddress, shot } from './apps.ts';

/** The size of a WebP picture, from its header. */
function webpSize(bytes: Uint8Array): { width: number; height: number } {
	const ascii = (from: number, to: number) => String.fromCharCode(...bytes.slice(from, to));
	expect(ascii(0, 4) + ascii(8, 12)).toBe('RIFFWEBP');
	const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
	const three = (at: number) => view.getUint16(at, true) + (bytes[at + 2] << 16);
	switch (ascii(12, 16)) {
		case 'VP8 ':
			return {
				width: view.getUint16(26, true) & 0x3fff,
				height: view.getUint16(28, true) & 0x3fff
			};
		case 'VP8L': {
			const bits = view.getUint32(21, true);
			return { width: (bits & 0x3fff) + 1, height: ((bits >> 14) & 0x3fff) + 1 };
		}
		case 'VP8X':
			return { width: three(24) + 1, height: three(27) + 1 };
		default:
			throw new Error(`not a WebP picture: ${ascii(12, 16)}`);
	}
}

describe('known apps', () => {
	it('have their pictures, as large as the list says', () => {
		for (const app of knownApps) {
			app.shots.forEach((size, i) => {
				// The tests run in web/.
				const file = join('static', shot(app, i + 1));
				expect(webpSize(readFileSync(file)), file).toEqual(size);
			});
		}
	});

	it('have a caption for every picture, in every language', () => {
		for (const code of languages) {
			for (const app of knownApps) {
				const captions = catalogs[code].settings.apps.shots[app.id];
				expect(captions, `${code}: ${app.id}`).toHaveLength(app.shots.length);
			}
		}
	});

	it('are told the server address their way', () => {
		expect(serverAddress('https://rueder.example', false)).toBe('https://rueder.example/greader');
		expect(serverAddress('https://rueder.example', true)).toBe('https://rueder.example/greader/');
	});
});
