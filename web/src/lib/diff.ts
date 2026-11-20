// What changed between two versions of an article: its words, its pictures and its links.

import type { Picture } from './model.ts';

export interface DiffPart {
	kind: 'same' | 'added' | 'removed';
	text: string;
}

/** Words with the blanks after them, so that joining the parts gives the text back. */
const words = (text: string) => text.match(/\S+\s*/g) ?? [];

/** The parts of `after` and `before` in reading order: kept, added or removed. */
export function diffWords(before: string, after: string): DiffPart[] {
	const a = words(before);
	const b = words(after);
	// Length of the longest common run of words from each pair of positions on.
	const common = Array.from({ length: a.length + 1 }, () => new Uint16Array(b.length + 1));
	for (let i = a.length - 1; i >= 0; i--) {
		for (let j = b.length - 1; j >= 0; j--) {
			common[i][j] =
				a[i].trim() === b[j].trim()
					? common[i + 1][j + 1] + 1
					: Math.max(common[i + 1][j], common[i][j + 1]);
		}
	}
	const parts: DiffPart[] = [];
	const push = (kind: DiffPart['kind'], text: string) => {
		const last = parts.at(-1);
		if (last?.kind === kind) last.text += text;
		else parts.push({ kind, text });
	};
	let i = 0;
	let j = 0;
	while (i < a.length && j < b.length) {
		if (a[i].trim() === b[j].trim()) {
			push('same', b[j]);
			i++;
			j++;
		} else if (common[i + 1][j] >= common[i][j + 1]) {
			push('removed', a[i++]);
		} else {
			push('added', b[j++]);
		}
	}
	while (i < a.length) push('removed', a[i++]);
	while (j < b.length) push('added', b[j++]);
	return parts;
}

/** The pictures and links one version has and the other lacks. */
export interface MediaChanges {
	pictures: { removed: Picture[]; added: Picture[] };
	links: { removed: string[]; added: string[] };
}

/** What `before` has and `after` lacks, and the other way round, repeats counted. */
function difference<T>(before: T[], after: T[], key: (item: T) => string) {
	const added = [...after];
	const removed = before.filter((item) => {
		const at = added.findIndex((other) => key(other) === key(item));
		if (at >= 0) added.splice(at, 1);
		return at < 0;
	});
	return { removed, added };
}

/** The pictures, with their texts, and the link addresses of a cleaned text. */
function media(html: string) {
	const body = new DOMParser().parseFromString(html, 'text/html').body;
	return {
		pictures: [...body.querySelectorAll('img')].map((img) => ({
			url: img.getAttribute('src') ?? '',
			title: img.getAttribute('alt') ?? ''
		})),
		links: [...body.querySelectorAll('a[href]')].map((a) => a.getAttribute('href') ?? '')
	};
}

/**
 * The pictures and links that changed between two versions, which a diff of
 * the words does not show: a feed may swap a picture and keep every word.
 */
export function diffMedia(before: string, after: string): MediaChanges {
	const [was, is] = [media(before), media(after)];
	return {
		pictures: difference(was.pictures, is.pictures, (picture) => `${picture.url} ${picture.title}`),
		links: difference(was.links, is.links, (link) => link)
	};
}
