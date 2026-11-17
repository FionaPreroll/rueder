// Filter rules of a user: hide articles, or mark them read, by words in the title or
// text, by author or by tag. The draft applies them in the browser; the server will
// apply them when an article arrives, with a regex engine that runs in linear time.

import type { Article } from './model.ts';

export type RuleField = 'any' | 'title' | 'content' | 'author' | 'tag';
export type RuleMatch = 'contains' | 'word' | 'starts' | 'ends' | 'regex' | 'all';
export type RuleAction = 'hide' | 'read';
export type RuleScope =
	{ kind: 'all' } | { kind: 'folder'; id: string } | { kind: 'feed'; id: string };

export interface FilterRule {
	id: string;
	enabled: boolean;
	scope: RuleScope;
	field: RuleField;
	match: RuleMatch;
	pattern: string;
	action: RuleAction;
}

export const MAX_PATTERN_LENGTH = 200;

/** Whether the rule still lacks the text it looks for. */
export const wanting = (rule: Pick<FilterRule, 'match' | 'pattern'>) =>
	rule.match !== 'all' && rule.pattern.trim() === '';

/** The rule as it is kept: a rule for all articles looks at nothing in particular. */
export const tidy = (rule: FilterRule): FilterRule =>
	rule.match === 'all'
		? { ...rule, field: 'any', pattern: '' }
		: { ...rule, pattern: rule.pattern.trim() };

/**
 * Where to build and try out a regular expression: regex101, with the pattern so far,
 * read as the server's engine reads it.
 */
export const builder = (pattern: string) =>
	`https://regex101.com/?flavor=rust&flags=i&regex=${encodeURIComponent(pattern.trim())}`;

const escape = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/**
 * The test a rule's pattern stands for, or null for an empty or broken pattern. A rule
 * for all articles has none, and its test is found in every text.
 */
export function compile(rule: Pick<FilterRule, 'match' | 'pattern'>): RegExp | null {
	if (rule.match === 'all') return /(?:)/u;
	const pattern = rule.pattern.trim();
	if (!pattern || pattern.length > MAX_PATTERN_LENGTH) return null;
	try {
		switch (rule.match) {
			case 'contains':
				return new RegExp(escape(pattern), 'iu');
			case 'word':
				// A whole word: no letter or digit right before or after.
				return new RegExp(`(?<![\\p{L}\\p{N}])${escape(pattern)}(?![\\p{L}\\p{N}])`, 'iu');
			case 'starts':
				return new RegExp(`^${escape(pattern)}`, 'iu');
			case 'ends':
				return new RegExp(`${escape(pattern)}$`, 'iu');
			case 'regex':
				return new RegExp(pattern, 'iu');
		}
	} catch {
		return null;
	}
}

/** The words of a piece of HTML, without its tags. */
export const textOf = (html: string) =>
	html
		.replace(/<[^>]*>/g, ' ')
		.replace(/\s+/g, ' ')
		.trim();

function fields(article: Article, field: RuleField): string[] {
	switch (field) {
		case 'title':
			return [article.title];
		case 'content':
			return [textOf(article.html ?? '')];
		case 'author':
			return [article.author];
		case 'tag':
			return article.tags;
		case 'any':
			return [article.title, textOf(article.html ?? ''), article.author, ...article.tags];
	}
}

/** Whether the rule's pattern is found in the article. Says nothing about the scope. */
export function ruleMatches(rule: FilterRule, article: Article) {
	if (rule.match === 'all') return true;
	const test = compile(rule);
	return test !== null && fields(article, rule.field).some((text) => test.test(text));
}
