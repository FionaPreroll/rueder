import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { fill, footer, origin, repoUrl, sitePath } from '../site.js';

const commit = '0123456789abcdef0123456789abcdef01234567';
const env = {
	GITHUB_REPOSITORY: 'someone/rueder',
	GITHUB_SHA: commit,
	RUEDER_DOCS_DAY: '2026-11-13'
};

describe('origin', () => {
	it('takes repository, commit and day from the run', () => {
		assert.deepEqual(origin(env), { repo: 'someone/rueder', commit, day: '2026-11-13' });
	});

	it('refuses what is not a repository, a commit or a day', () => {
		assert.throws(() => origin({ ...env, GITHUB_REPOSITORY: 'a/b/c' }), /not a repository/);
		assert.throws(() => origin({ ...env, GITHUB_REPOSITORY: 'a/<b>' }), /not a repository/);
		assert.throws(() => origin({ ...env, GITHUB_SHA: 'main' }), /not a commit/);
		assert.throws(() => origin({ ...env, RUEDER_DOCS_DAY: 'today' }), /not a day/);
	});

	it('asks git and the clock without a run', () => {
		const here = origin({});
		assert.equal(here.repo, 'FionaPreroll/rueder');
		assert.match(here.commit, /^[0-9a-f]{40}$/);
		assert.match(here.day, /^\d{4}-\d{2}-\d{2}$/);
	});
});

describe('the site', () => {
	const from = origin(env);

	it('is served under the repository’s name', () => {
		assert.equal(sitePath(from), '/rueder/');
		assert.equal(repoUrl(from), 'https://github.com/someone/rueder');
	});

	it('says on every page what it was built from', () => {
		const line = footer(from);
		assert.match(line, /^Built on 2026-11-13 from commit /);
		assert.ok(
			line.includes(`href="https://github.com/someone/rueder/commit/${commit}">0123456</a>`)
		);
	});
});

describe('fill', () => {
	it('puts the values into their places', () => {
		assert.equal(
			fill('<p>{{a}} and {{b}}, {{a}}</p>', { a: '1', b: '<i>2</i>' }),
			'<p>1 and <i>2</i>, 1</p>'
		);
	});

	it('refuses a place without a value and a value without a place', () => {
		assert.throws(() => fill('{{a}} {{b}}', { a: '1' }), /no value for \{\{b\}\}/);
		assert.throws(() => fill('{{a}}', { a: '1', b: '2' }), /no place for \{\{b\}\}/);
	});
});
