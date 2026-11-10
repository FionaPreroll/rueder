// A filter rule in words, in the page's language.

import type { Reader } from './demo/reader.svelte.ts';
import type { FilterRule, RuleScope } from './filters.ts';
import { m } from './i18n/index.ts';

/** "All feeds", "Folder: Technik" or "Feed: Tagesbote". */
export function scopeText(reader: Reader, scope: RuleScope) {
	if (scope.kind === 'all') return m.rules.scopeAll;
	const title = reader.title(scope) ?? '';
	return scope.kind === 'folder' ? m.rules.scopeFolder(title) : m.rules.scopeFeed(title);
}

/** "Title has the word “Anzeige”: hide the article". */
export const ruleText = (rule: FilterRule) =>
	m.rules.summary(
		m.rules.fields[rule.field],
		m.rules.matches[rule.match],
		rule.pattern,
		m.rules.actions[rule.action]
	);
