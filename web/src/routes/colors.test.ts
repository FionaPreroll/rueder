// The colors of layout.css against the measures of the design review
// (docs/design_review.md): WCAG 2.2 AA everywhere, and from AAA the contrast
// of 7:1 for the text of the list and of articles. Read from the file itself,
// for both themes and every accent color, so a new shade is checked before it
// is seen.

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { accents } from '#lib/settings.svelte.ts';

// From the root of web/, where Vitest runs; Vite would hand a CSS import over empty.
const css = readFileSync('src/routes/layout.css', 'utf8');

/** The `--color-…` variables a block of the file sets, by the start of the block. */
function block(start: string): Record<string, string> {
	const at = css.indexOf(`${start} {`);
	if (at < 0) return {};
	const body = css.slice(at, css.indexOf('}', at));
	return Object.fromEntries(
		[...body.matchAll(/--color-([\w-]+):\s*([^;]+);/g)].map(([, name, value]) => [
			name,
			value.trim()
		])
	);
}

type Rgb = [number, number, number];

function hex(value: string): Rgb {
	const digits = value.replace('#', '');
	return [0, 2, 4].map((at) => parseInt(digits.slice(at, at + 2), 16)) as Rgb;
}

/** The color a variable stands for, with `var()` and `color-mix(in srgb, …)` worked out. */
function resolve(colors: Record<string, string>, value: string): Rgb {
	const reference = /^var\(--color-([\w-]+)\)$/.exec(value);
	if (reference) return resolve(colors, colors[reference[1]]);
	const mix = /^color-mix\(in srgb, (.+) (\d+)%, (.+)\)$/.exec(value);
	if (mix) {
		const [one, share, other] = [
			resolve(colors, mix[1]),
			Number(mix[2]) / 100,
			resolve(colors, mix[3])
		];
		return one.map((part, index) => Math.round(part * share + other[index] * (1 - share))) as Rgb;
	}
	if (/^#[0-9a-f]{6}$/i.test(value)) return hex(value);
	throw new Error(`cannot read the color ${value}`);
}

/** Relative luminance after WCAG 2.2. */
function luminance([r, g, b]: Rgb) {
	const linear = (part: number) => {
		const c = part / 255;
		return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
	};
	return 0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b);
}

function contrast(one: Rgb, other: Rgb) {
	const [light, dark] = [luminance(one), luminance(other)].sort((a, b) => b - a);
	return (light + 0.05) / (dark + 0.05);
}

const light = block('@theme');
const dark = { ...light, ...block(":root[data-theme='dark']") };

/** The colors as the page shows them, for a theme and an accent. */
function colors(theme: 'light' | 'dark', accent: (typeof accents)[number]) {
	const set = {
		...(theme === 'light' ? light : dark),
		...block(`:root[data-accent='${accent}']`),
		...(theme === 'dark' ? block(`:root[data-theme='dark'][data-accent='${accent}']`) : {})
	};
	return (name: string) => resolve(set, `var(--color-${name})`);
}

const grounds = ['paper', 'panel', 'hover', 'chosen'];

describe.each(['light', 'dark'] as const)('the %s theme', (theme) => {
	describe.each(accents)('with the accent %s', (accent) => {
		const color = colors(theme, accent);
		const at = (fore: string, ground: string) => contrast(color(fore), color(ground));

		it('keeps text readable on every ground (AA, 4.5:1)', () => {
			for (const text of ['ink', 'soft', 'link'])
				for (const ground of grounds)
					expect(at(text, ground), `${text} on ${ground}`).toBeGreaterThanOrEqual(4.5);
			for (const text of ['warn'])
				for (const ground of ['paper', 'panel'])
					expect(at(text, ground), `${text} on ${ground}`).toBeGreaterThanOrEqual(4.5);
			expect(at('on-accent', 'accent'), 'on-accent on accent').toBeGreaterThanOrEqual(4.5);
			expect(at('on-warn', 'warn'), 'on-warn on warn').toBeGreaterThanOrEqual(4.5);
		});

		it('keeps the text of the list and of articles at 7:1 (AAA)', () => {
			for (const text of ['ink', 'soft'])
				for (const ground of ['paper', 'panel'])
					expect(at(text, ground), `${text} on ${ground}`).toBeGreaterThanOrEqual(7);
		});

		it('shows fields, the star and the accent as a line (3:1)', () => {
			for (const ground of ['paper', 'panel'])
				expect(at('field', ground), `field on ${ground}`).toBeGreaterThanOrEqual(3);
			expect(at('star', 'paper'), 'star on paper').toBeGreaterThanOrEqual(3);
			for (const ground of grounds)
				expect(at('accent-line', ground), `accent-line on ${ground}`).toBeGreaterThanOrEqual(3);
			expect(at('link', 'paper'), 'the ring of the focus').toBeGreaterThanOrEqual(3);
		});
	});
});

it('reads every accent of the settings from the file', () => {
	for (const accent of accents.filter((accent) => accent !== 'red'))
		expect(block(`:root[data-accent='${accent}']`).accent, accent).toBeDefined();
});
