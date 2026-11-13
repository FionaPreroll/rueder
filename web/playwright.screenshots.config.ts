// Screenshots of every view, for looking at the design: the built page in a
// real browser against the real server, with the made-up feeds of the draft
// (src/lib/demo/data.ts) served by a made-up website (screenshots/site.ts).
// Each language, device and theme is a project of its own with an account of
// its own; the pictures land in screenshots/out/.
//
//   pnpm screenshots
//   SCREENSHOT_LANGUAGES=de pnpm screenshots
//
// It needs what `pnpm e2e` needs (README.md).

import { defineConfig, devices, type Project } from '@playwright/test';
import { RUEDER, SITE } from './e2e/harness/ports.js';
import { TERMS_VERSION, projects as shots } from './screenshots/projects.ts';

const projects: Project[] = [
	{ name: 'seed', testMatch: 'seed.setup.ts' },
	...shots(process.env.SCREENSHOT_LANGUAGES).map(({ name, language, phone, theme }) => ({
		name,
		testMatch: 'views.spec.ts',
		dependencies: ['seed'],
		use: {
			...(phone
				? devices['Pixel 7']
				: { ...devices['Desktop Chrome'], viewport: { width: 1440, height: 900 } }),
			// Sharp on screens with more pixels to the point, as reviewers look at them.
			deviceScaleFactor: phone ? devices['Pixel 7'].deviceScaleFactor : 2,
			locale: language === 'de' ? 'de-DE' : 'en-US',
			colorScheme: theme
		}
	}))
];

export default defineConfig({
	testDir: 'screenshots',
	// One server and one database; every project has its own account.
	workers: 4,
	fullyParallel: false,
	forbidOnly: !!process.env.CI,
	retries: 0,
	// A project walks through every view in one test.
	timeout: 15 * 60_000,
	reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
	// A view that cannot be found is given up soon, and the walk goes on.
	expect: { timeout: 10_000 },
	use: {
		actionTimeout: 10_000,
		baseURL: RUEDER,
		timezoneId: 'Europe/Berlin',
		trace: 'retain-on-failure'
	},
	projects,
	webServer: [
		{ command: 'node screenshots/site.ts', url: `${SITE}/feeds.opml`, reuseExistingServer: false },
		{
			command: 'node e2e/harness/server.js',
			url: `${RUEDER}/api/health`,
			reuseExistingServer: false,
			timeout: 120_000,
			env: {
				E2E_ACCOUNTS: [
					'seed',
					...shots(process.env.SCREENSHOT_LANGUAGES).map((p) => p.account)
				].join(','),
				// The operator's texts, so the login and the menu show their links.
				LEGAL_IMPRINT_URL: 'https://rueder.example/imprint',
				LEGAL_PRIVACY_URL: 'https://rueder.example/privacy',
				LEGAL_TERMS_URL: 'https://rueder.example/terms',
				TERMS_VERSION,
				// A round of fetching every five seconds, not every 30, for the
				// many rounds the made-up articles arrive in (seed.setup.ts).
				FETCH_EVERY: '5'
			}
		}
	]
});
