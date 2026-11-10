// What changed between two versions of an article, word by word.

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
