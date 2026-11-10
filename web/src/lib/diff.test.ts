import { describe, expect, it } from 'vitest';
import { diffWords } from '#lib/diff.ts';

const joined = (parts: ReturnType<typeof diffWords>, without: 'added' | 'removed') =>
	parts
		.filter((part) => part.kind !== without)
		.map((part) => part.text)
		.join('');

describe('diffWords', () => {
	it('is one part for the same text', () => {
		expect(diffWords('one two three', 'one two three')).toEqual([
			{ kind: 'same', text: 'one two three' }
		]);
		expect(diffWords('', '')).toEqual([]);
	});

	it('shows a changed word as removed and added', () => {
		expect(diffWords('costs 4 million euros', 'costs 6 million euros')).toEqual([
			{ kind: 'same', text: 'costs ' },
			{ kind: 'removed', text: '4 ' },
			{ kind: 'added', text: '6 ' },
			{ kind: 'same', text: 'million euros' }
		]);
	});

	it('shows text put at the end or taken from the start', () => {
		expect(diffWords('one two', 'one two three four')).toEqual([
			{ kind: 'same', text: 'one two ' },
			{ kind: 'added', text: 'three four' }
		]);
		expect(diffWords('zero one two', 'one two')).toEqual([
			{ kind: 'removed', text: 'zero ' },
			{ kind: 'same', text: 'one two' }
		]);
		expect(diffWords('', 'new')).toEqual([{ kind: 'added', text: 'new' }]);
		expect(diffWords('old', '')).toEqual([{ kind: 'removed', text: 'old' }]);
	});

	it('gives both texts back', () => {
		const before = 'The council voted on Monday and the budget passed without changes.';
		const after = 'The council voted late on Monday and the revised budget passed.';
		const parts = diffWords(before, after);
		expect(joined(parts, 'added').replace(/\s+/g, ' ').trim()).toBe(before);
		expect(joined(parts, 'removed').replace(/\s+/g, ' ').trim()).toBe(after);
	});
});
