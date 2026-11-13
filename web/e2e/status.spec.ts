// An admin counts the status of the server over a longer time than the last
// hour, as far as the operator keeps the counts.

import { expect, test } from '@playwright/test';
import type { ServerStatus } from '../src/lib/api/types.ts';
import { logIn, rueder } from './harness/helpers.ts';

test('the status page counts over a day, from what the server saved', async ({ page }) => {
	await logIn(page, 'login');
	await page.getByRole('button', { name: 'Logged in as login' }).click();
	const hour = page.waitForResponse((response) => response.url().endsWith('/api/status'));
	await page.getByRole('menuitem', { name: 'Server status' }).click();
	const dialog = page.getByRole('dialog', { name: 'Server status' });
	await expect(
		dialog.getByText(/keeps its counts for 7 days, also across a restart/)
	).toBeVisible();
	await expect(dialog.getByText(/^Over the last hour, \d+ errors? and/)).toBeVisible();
	const inHour: ServerStatus = await (await hour).json();

	const period = dialog.getByRole('combobox', { name: 'Counted over' });
	await expect(period.getByRole('option')).toHaveText([
		'the last hour',
		'the last 24 hours',
		'the last 7 days'
	]);
	const day = page.waitForResponse((response) => response.url().endsWith('/api/status?hours=24'));
	await period.selectOption({ label: 'the last 24 hours' });
	const answer = await day;
	expect(answer.status()).toBe(200);
	await expect(dialog.getByText(/^Over the last 24 hours, \d+ errors? and/)).toBeVisible();
	// What the server saved counts: at least all of the hour before, and the
	// request for the status of the hour since.
	const inDay: ServerStatus = await answer.json();
	expect([inDay.hours, inDay.kept_days]).toEqual([24, 7]);
	expect(inDay.requests.all.count).toBeGreaterThan(inHour.requests.all.count);

	expect(rueder('retention', 'show')).toContain('kept for 7 days');
});
