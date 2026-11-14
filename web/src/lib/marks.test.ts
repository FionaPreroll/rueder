import { describe, expect, it } from 'vitest';
import { marks } from './marks.ts';

const shown = (text: string, query: string) =>
	marks(text, query)
		.map((piece) => (piece.marked ? `[${piece.text}]` : piece.text))
		.join('');

describe('the marks of a search', () => {
	it('marks the words at the start of a word, whatever their case', () => {
		expect(shown('Borrow Checker Diaries', 'borrow')).toBe('[Borrow] Checker Diaries');
		expect(shown('Der Stadtrat wird teurer', 'stadt TEU')).toBe('Der [Stadt]rat wird [teu]rer');
		expect(shown('Überall Übungen', 'über')).toBe('[Über]all Übungen');
	});

	it('leaves a word inside another alone', () => {
		expect(shown('Unborrowed', 'borrow')).toBe('Unborrowed');
	});

	it('marks the longer of two words that start alike', () => {
		expect(shown('Apfelbaum', 'apfel apfelbaum')).toBe('[Apfelbaum]');
	});

	it('gives the text whole without a search, and takes the words as words, not as a pattern', () => {
		expect(marks('A title', '  ')).toEqual([{ text: 'A title', marked: false }]);
		expect(shown('C++ (and more)', 'c++ (and')).toBe('[C++] [(and] more)');
		expect(shown('', 'word')).toBe('');
	});
});
