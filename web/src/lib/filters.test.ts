import { describe, expect, it } from 'vitest';
import {
	builder,
	compile,
	MAX_PATTERN_LENGTH,
	ruleMatches,
	textOf,
	tidy,
	wanting,
	type FilterRule
} from '#lib/filters.ts';
import type { Article } from '#lib/model.ts';

const article: Article = {
	id: 'a1',
	feedId: 'feed-1',
	seq: 1,
	title: 'Anzeige: Der neue Lötkolben',
	teaser: '',
	author: 'Werbeteam',
	tags: ['werbung', 'hardware'],
	link: 'https://blog.example/1',
	publishedAt: 0,
	html: '<p>Jetzt <b>zugreifen</b>, nur diese Woche.</p>',
	enclosures: [],
	image: null,
	changes: 0,
	updatedAt: null
};

const rule = (over: Partial<FilterRule>): FilterRule => ({
	id: 'r',
	enabled: true,
	scope: { kind: 'all' },
	field: 'title',
	match: 'contains',
	pattern: '',
	action: 'hide',
	...over
});

describe('textOf', () => {
	it('drops the tags and folds the blanks', () => {
		expect(textOf('<p>Jetzt <b>zugreifen</b>,\n nur</p><p>heute</p>')).toBe(
			'Jetzt zugreifen , nur heute'
		);
	});
});

describe('compile', () => {
	it('is null for an empty, too long or broken pattern', () => {
		expect(compile({ match: 'contains', pattern: '  ' })).toBeNull();
		expect(compile({ match: 'contains', pattern: 'x'.repeat(MAX_PATTERN_LENGTH + 1) })).toBeNull();
		expect(compile({ match: 'regex', pattern: '(' })).toBeNull();
	});

	it('takes the text literally unless it is a regular expression', () => {
		expect(compile({ match: 'contains', pattern: 'a.c' })?.test('abc')).toBe(false);
		expect(compile({ match: 'contains', pattern: 'a.c' })?.test('A.C')).toBe(true);
		expect(compile({ match: 'regex', pattern: 'a.c' })?.test('abc')).toBe(true);
	});

	it('finds whole words only, with umlauts as letters', () => {
		const word = compile({ match: 'word', pattern: 'ende' });
		expect(word?.test('Am Ende.')).toBe(true);
		expect(word?.test('Wochenende')).toBe(false);
		expect(word?.test('endet')).toBe(false);
		expect(compile({ match: 'word', pattern: 'öl' })?.test('Möl')).toBe(false);
	});

	it('finds a start or an end of the whole text, taken literally', () => {
		expect(compile({ match: 'starts', pattern: 'anzeige' })?.test('Anzeige: Lötkolben')).toBe(true);
		expect(compile({ match: 'starts', pattern: 'lötkolben' })?.test('Anzeige: Lötkolben')).toBe(
			false
		);
		expect(compile({ match: 'ends', pattern: 'Lötkolben' })?.test('Anzeige: lötkolben')).toBe(true);
		expect(compile({ match: 'ends', pattern: 'anzeige' })?.test('Anzeige: Lötkolben')).toBe(false);
		expect(compile({ match: 'starts', pattern: 'a.' })?.test('ab')).toBe(false);
		expect(compile({ match: 'ends', pattern: '(1)' })?.test('Teil (1)')).toBe(true);
	});
});

describe('ruleMatches', () => {
	it('looks at the field the rule names', () => {
		expect(ruleMatches(rule({ pattern: 'anzeige' }), article)).toBe(true);
		expect(ruleMatches(rule({ pattern: 'zugreifen' }), article)).toBe(false);
		expect(ruleMatches(rule({ field: 'content', pattern: 'zugreifen' }), article)).toBe(true);
		expect(ruleMatches(rule({ field: 'author', pattern: 'werbe' }), article)).toBe(true);
		expect(ruleMatches(rule({ field: 'tag', match: 'word', pattern: 'hardware' }), article)).toBe(
			true
		);
		expect(ruleMatches(rule({ field: 'tag', pattern: 'software' }), article)).toBe(false);
	});

	it('takes a start or an end of each tag and of the text', () => {
		expect(ruleMatches(rule({ field: 'tag', match: 'starts', pattern: 'hard' }), article)).toBe(
			true
		);
		expect(
			ruleMatches(rule({ field: 'content', match: 'ends', pattern: 'diese woche.' }), article)
		).toBe(true);
	});

	it('matches every article for a rule for all articles, which has no pattern', () => {
		const all = rule({ field: 'tag', match: 'all', pattern: ' x ' });
		expect(ruleMatches(all, { ...article, title: '', tags: [], author: '', html: '' })).toBe(true);
		expect(tidy(all)).toEqual({ ...all, field: 'any', pattern: '' });
		expect(tidy(rule({ pattern: ' ad ' })).pattern).toBe('ad');
		expect(wanting(all)).toBe(false);
		expect(wanting(rule({ pattern: '  ' }))).toBe(true);
	});

	it('sends the pattern to the builder as the server reads it', () => {
		const at = new URL(builder(' a&b+ '));
		expect(at.origin).toBe('https://regex101.com');
		expect(at.searchParams.get('flavor')).toBe('rust');
		expect(at.searchParams.get('regex')).toBe('a&b+');
	});

	it('looks everywhere for "any"', () => {
		for (const pattern of ['Lötkolben', 'diese Woche', 'Werbeteam', 'werbung']) {
			expect(ruleMatches(rule({ field: 'any', pattern }), article)).toBe(true);
		}
		expect(ruleMatches(rule({ field: 'any', pattern: 'Teleskop' }), article)).toBe(false);
	});

	it('matches nothing with a broken pattern', () => {
		expect(ruleMatches(rule({ match: 'regex', pattern: '[' }), article)).toBe(false);
	});
});
