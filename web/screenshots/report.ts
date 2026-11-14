// Adds up what the walk measured next to its pictures (harness.ts, measure)
// into screenshots/out/measure.md: for each project the targets too small to
// press and the length of the lines of article text, against the measures of
// the design review (docs/design_review.md): 44 by 44 pixels for a finger on a
// phone, 24 by 24 for a pointer, at most 80 characters to the line. A smaller
// target for a pointer passes with room around it, as WCAG 2.5.8 says (`spaced`).
//
//   node screenshots/report.ts

import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import type { Measured, Target } from './harness.ts';

/** Where the pictures are, as in harness.ts. */
const OUT = resolve(import.meta.dirname, 'out');

/** The smallest a target may be, in CSS pixels, on a phone and elsewhere. */
export const SMALLEST = { phone: 44, pointer: 24 };
/** The most characters to a line of article text. */
export const LONGEST_LINE = 80;

/** The middle of a target's box. */
const middle = ({ x, y, width, height }: Target) => [x + width / 2, y + height / 2] as const;

/**
 * Whether a target under the smallest size has room enough around it (WCAG 2.5.8):
 * a circle of that size around its middle meets no other target, nor the circle
 * of another target under the size. A target counted twice, as a field and its
 * label, does not meet itself.
 */
export function spaced(target: Target, others: Target[], smallest: number): boolean {
	const radius = smallest / 2;
	const [cx, cy] = middle(target);
	return others.every((other) => {
		if (
			other === target ||
			(other.x === target.x &&
				other.y === target.y &&
				other.width === target.width &&
				other.height === target.height)
		)
			return true;
		// The nearest point of the other box to the middle of the circle.
		const nx = Math.min(Math.max(cx, other.x), other.x + other.width);
		const ny = Math.min(Math.max(cy, other.y), other.y + other.height);
		if (Math.hypot(cx - nx, cy - ny) < radius) return false;
		if (other.width >= smallest && other.height >= smallest) return true;
		const [ox, oy] = middle(other);
		return Math.hypot(cx - ox, cy - oy) >= smallest;
	});
}

/** The report of one project's measurements, as Markdown. */
export function report(project: string, measured: Measured[]): string {
	const phone = project.includes('phone');
	const smallest = phone ? SMALLEST.phone : SMALLEST.pointer;
	const small = new Map<string, Set<string>>();
	const roomy = new Set<string>();
	for (const { picture, targets } of measured) {
		for (const target of targets) {
			const { name, width, height } = target;
			if (width >= smallest && height >= smallest) continue;
			const key = `${width}×${height} ${name}`;
			// A finger needs the size itself; a pointer is helped by room around.
			if (!phone && spaced(target, targets, SMALLEST.pointer)) {
				roomy.add(key);
				continue;
			}
			small.set(key, (small.get(key) ?? new Set()).add(picture.replace(/\.png$/, '')));
		}
	}
	const lines = measured.flatMap(({ picture, lines }) =>
		lines.map((length) => ({ picture, length }))
	);
	const long = lines.filter(({ length }) => length > LONGEST_LINE);
	const out = [
		`## ${project}`,
		'',
		`${measured.length} pictures. Targets smaller than ${smallest}×${smallest}` +
			(phone
				? `: ${small.size}.`
				: ` without the room around them that WCAG 2.5.8 asks for: ${small.size}` +
					` (with it: ${[...roomy].filter((key) => !small.has(key)).length}).`)
	];
	if (small.size) {
		out.push('', '| Size and name | Seen in |', '| --- | --- |');
		for (const [key, pictures] of [...small].sort(([a], [b]) => a.localeCompare(b))) {
			const seen = [...pictures];
			const shown =
				seen.slice(0, 3).join(', ') + (seen.length > 3 ? ` and ${seen.length - 3} more` : '');
			// Backslashes too, so a name cannot undo the escape of a bar.
			out.push(`| ${key.replace(/[\\|]/g, '\\$&')} | ${shown} |`);
		}
	}
	if (lines.length) {
		const most = Math.max(...lines.map(({ length }) => length));
		out.push(
			'',
			`Article text: at most ${most} characters to the line in ${lines.length} paragraphs` +
				(long.length ? `, ${long.length} over ${LONGEST_LINE}.` : '.')
		);
	}
	return out.join('\n');
}

/** Reads what each project's folder holds. */
function read(folder: string): Measured[] {
	const file = join(OUT, folder, 'measure.jsonl');
	if (!existsSync(file)) return [];
	return readFileSync(file, 'utf8')
		.split('\n')
		.filter(Boolean)
		.map((line) => JSON.parse(line) as Measured);
}

if (import.meta.main) {
	const folders = existsSync(OUT)
		? readdirSync(OUT, { withFileTypes: true })
				.filter((entry) => entry.isDirectory())
				.map((entry) => entry.name)
				.sort()
		: [];
	const parts = folders.map((folder) => report(folder, read(folder)));
	writeFileSync(join(OUT, 'measure.md'), ['# Measured', ...parts].join('\n\n') + '\n');
	console.log(`screenshots/out/measure.md: ${folders.length} projects`);
}
