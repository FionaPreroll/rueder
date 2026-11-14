// The words of a search in a text, so a title can mark what was found. The
// server finds a word at the start of a word, and from three letters on also
// as its beginning; the marks follow it, ignoring case.

/** A piece of the text, and whether it is a word of the search. */
export interface Piece {
	text: string;
	marked: boolean;
}

/** At most so many words are marked, as a long query marks little more. */
const MOST_WORDS = 8;

/** The text cut into pieces at the words of `query`, in order; one piece without a search. */
export function marks(text: string, query: string): Piece[] {
	const words = query
		.split(/\s+/)
		.filter((word) => word.length > 0)
		.slice(0, MOST_WORDS)
		.map((word) => word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'))
		// Longer words first, so that a word wins over its own beginning.
		.sort((a, b) => b.length - a.length);
	if (words.length === 0 || !text) return [{ text, marked: false }];
	const found = new RegExp(`(?<![\\p{L}\\p{N}])(?:${words.join('|')})`, 'giu');
	const pieces: Piece[] = [];
	let at = 0;
	for (const match of text.matchAll(found)) {
		const start = match.index ?? 0;
		if (start > at) pieces.push({ text: text.slice(at, start), marked: false });
		pieces.push({ text: match[0], marked: true });
		at = start + match[0].length;
	}
	if (at < text.length) pieces.push({ text: text.slice(at), marked: false });
	return pieces;
}
