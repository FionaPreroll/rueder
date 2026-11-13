// What every page of the site shares: where it comes from (repository, commit,
// day) and the footer that says so, and filling a template.

import { execFileSync } from 'node:child_process';

/**
 * The repository, commit and day the site is built from. In GitHub Actions they
 * come from the run; elsewhere from git and the clock. A fork names its own
 * repository, and so gets its own address on GitHub Pages.
 */
export function origin(env = process.env) {
	const repo = env.GITHUB_REPOSITORY || 'FionaPreroll/rueder';
	const commit =
		env.GITHUB_SHA || execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
	if (!/^[\w.-]+\/[\w.-]+$/.test(repo)) throw new Error(`not a repository: ${repo}`);
	if (!/^[0-9a-f]{40}$/.test(commit)) throw new Error(`not a commit: ${commit}`);
	const day = env.RUEDER_DOCS_DAY || new Date().toISOString().slice(0, 10);
	if (!/^\d{4}-\d{2}-\d{2}$/.test(day)) throw new Error(`not a day: ${day}`);
	return { repo, commit, day };
}

/** The path the site is served under on GitHub Pages: /<repository>/. */
export const sitePath = ({ repo }) => `/${repo.split('/')[1]}/`;

/** The repository on GitHub. */
export const repoUrl = ({ repo }) => `https://github.com/${repo}`;

/** One line for the foot of every page: the commit and the day it was built from. */
export function footer(from) {
	const short = from.commit.slice(0, 7);
	return (
		`Built on ${from.day} from commit ` +
		`<a href="${repoUrl(from)}/commit/${from.commit}">${short}</a> of ` +
		`<a href="${repoUrl(from)}">${from.repo}</a>.`
	);
}

/**
 * Puts the values into the template's {{name}} places. The values are HTML;
 * a place without a value, or a value without a place, is a mistake.
 */
export function fill(template, values) {
	const used = new Set();
	const html = template.replace(/\{\{(\w+)\}\}/g, (_, name) => {
		if (!(name in values)) throw new Error(`no value for {{${name}}}`);
		used.add(name);
		return values[name];
	});
	for (const name of Object.keys(values)) {
		if (!used.has(name)) throw new Error(`no place for {{${name}}}`);
	}
	return html;
}
