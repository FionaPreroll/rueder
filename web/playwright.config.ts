// End-to-end tests: the built page in a real browser against the real server,
// with a database of its own and a made-up website to fetch feeds from (e2e/).
//
//   pnpm e2e

import { defineConfig, devices } from '@playwright/test';
import { RUEDER, SITE } from './e2e/harness/ports.js';

export default defineConfig({
	testDir: 'e2e',
	// One server and one database for the whole run: the specs take turns.
	workers: 1,
	fullyParallel: false,
	forbidOnly: !!process.env.CI,
	retries: 0,
	timeout: 60_000,
	reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
	use: {
		baseURL: RUEDER,
		locale: 'en-US',
		timezoneId: 'Europe/Berlin',
		trace: 'retain-on-failure',
		screenshot: 'only-on-failure'
	},
	projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
	webServer: [
		{ command: 'node e2e/harness/site.js', url: `${SITE}/harbour/`, reuseExistingServer: false },
		{
			command: 'node e2e/harness/server.js',
			url: `${RUEDER}/api/health`,
			reuseExistingServer: false,
			timeout: 120_000
		}
	]
});
