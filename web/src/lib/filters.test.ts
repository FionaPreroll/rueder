import { describe, expect, it } from 'vitest';
import { compile, MAX_PATTERN_LENGTH, ruleMatches, textOf, type FilterRule } from '#lib/filters.ts';
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
