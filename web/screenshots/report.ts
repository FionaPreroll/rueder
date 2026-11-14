// Adds up what the walk measured next to its pictures (harness.ts, measure)
// into screenshots/out/measure.md: for each project the targets too small to
// press and the length of the lines of article text, against the measures of
// the design review (docs/design_review.md): 44 by 44 pixels for a finger on a
// phone, 24 by 24 for a pointer, at most 80 characters to the line.
//
//   node screenshots/report.ts

import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import type { Measured } from './harness.ts';

/** Where the pictures are, as in harness.ts. */
const OUT = resolve(import.meta.dirname, 'out');

/** The smallest a target may be, in CSS pixels, on a phone and elsewhere. */
export const SMALLEST = { phone: 44, pointer: 24 };
/** The most characters to a line of article text. */
export const LONGEST_LINE = 80;

/** The report of one project's measurements, as Markdown. */
export function report(project: string, measured: Measured[]): string {
	const smallest = project.includes('phone') ? SMALLEST.phone : SMALLEST.pointer;
	const small = new Map<string, Set<string>>();
	for (const { picture, targets } of measured) {
		for (const { name, width, height } of targets) {
			if (width >= smallest && height >= smallest) continue;
			const key = `${width}×${height} ${name}`;
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
		`${measured.length} pictures. Targets smaller than ${smallest}×${smallest}: ${small.size}.`
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
