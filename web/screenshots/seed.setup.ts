// Before the pictures: the made-up feeds become feeds of the server. What is
// older than two days comes with the first fetch, the rest in steps of two
// hours, so that the newest articles are listed in the order they were
// published and not feed after feed. A last fetch brings the articles that
// changed in the made-up data as they are now, so that they have an earlier
// version, and the feed with an error its error. Registration is opened, so
// the login shows the way to make an account.

import { test } from '@playwright/test';
import { SITE, fetchSoon, logIn, rueder } from '../e2e/harness/helpers.ts';
import { api, importFeeds, psql } from './harness.ts';
import { TERMS_VERSION } from './projects.ts';

/** Makes every feed due and waits until the server fetched them all. */
async function fetchAll() {
	const since = psql('SELECT now()');
	fetchSoon();
	for (;;) {
		const left = psql(
			`SELECT count(*) FROM feeds WHERE fetched_at IS NULL OR fetched_at <= '${since}'`
		);
		if (left === '0') return;
		await new Promise((resolve) => setTimeout(resolve, 1000));
	}
}

/** Tells the made-up site what to serve. */
async function site(control: string) {
	const answer = await fetch(`${SITE}/_control/${control}`, { method: 'POST' });
	if (!answer.ok) throw new Error(`the made-up site answered ${answer.status} to ${control}`);
}

test('the made-up feeds are fetched, then fetched again as they are now', async ({ page }) => {
	test.setTimeout(10 * 60_000);
	rueder('registration', 'set', '--open', 'true');
	await logIn(page, 'seed');
	await api(page, 'PUT', '/terms', { version: TERMS_VERSION });
	await importFeeds(page);

	const HOUR = 3_600_000;
	const now = Date.now();
	for (let at = now - 48 * HOUR; at < now + 2 * HOUR; at += 2 * HOUR) {
		await site(`until?at=${at}`);
		await fetchAll();
	}
	await site('current');
	await fetchAll();
});
