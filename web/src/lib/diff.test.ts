import { describe, expect, it } from 'vitest';
import { diffMedia, diffWords } from '#lib/diff.ts';

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

describe('diffMedia', () => {
	const picture = (n: number, alt: string) => `<img src="/img/sig/image/${n}" alt="${alt}">`;

	it('finds a picture swapped for another, and a new text for a picture', () => {
		const before = `<p>Bond</p>${picture(118, 'Garfitt/AP')}${picture(7, 'Map')}`;
		const after = `<p>Bond</p>${picture(122, 'Buitendijk/PA')}${picture(7, 'Map of London')}`;
		expect(diffMedia(before, after).pictures).toEqual({
			removed: [
				{ url: '/img/sig/image/118', title: 'Garfitt/AP' },
				{ url: '/img/sig/image/7', title: 'Map' }
			],
			added: [
				{ url: '/img/sig/image/122', title: 'Buitendijk/PA' },
				{ url: '/img/sig/image/7', title: 'Map of London' }
			]
		});
	});

	it('finds links taken out and put in, and counts repeats', () => {
		const link = (to: string) => `<a href="${to}">more</a>`;
		const before = `<p>${link('https://a.example/')} ${link('https://b.example/?x=1&amp;y=2')}</p>`;
		const after = `<p>${link('https://a.example/')} ${link('https://a.example/')}</p>`;
		expect(diffMedia(before, after).links).toEqual({
			removed: ['https://b.example/?x=1&y=2'],
			added: ['https://a.example/']
		});
	});

	it('finds nothing where only the layout changed', () => {
		const changes = diffMedia(
			`<p>One ${picture(1, 'A')}</p><p><a href="https://a.example/">two</a></p>`,
			`<div><p>One</p>${picture(1, 'A')}<a href="https://a.example/"><em>two</em></a></div>`
		);
		expect(changes).toEqual({
			pictures: { removed: [], added: [] },
			links: { removed: [], added: [] }
		});
	});
});
