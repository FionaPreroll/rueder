// The pictures in the README at the top of the repository: the reader with
// made-up data, as the demo on GitHub Pages is, in English, on a desktop and
// on a phone, light and dark. The page is the single-file build, served to
// the browser by the test itself (showcase/showcase.spec.ts), so nothing
// else has to run. The pictures land in showcase/out/; the workflow Pages
// takes them for every pull request and publishes them with the site, so they
// are never committed.
//
//   pnpm build:single && pnpm showcase

import { defineConfig, devices, type Project } from '@playwright/test';
import { DEMO, looks } from './showcase/looks.ts';

const projects: Project[] = looks.map(({ name, phone, theme }) => ({
	name,
	testMatch: 'showcase.spec.ts',
	// Each picture is meant for a desktop or for a phone.
	grep: phone ? /@phone/ : /@desktop/,
	use: {
		...(phone
			? devices['Pixel 7']
			: { ...devices['Desktop Chrome'], viewport: { width: 1280, height: 800 } }),
		// Sharp on screens with more pixels to the point, as most are now.
		deviceScaleFactor: phone ? devices['Pixel 7'].deviceScaleFactor : 2,
		colorScheme: theme
	}
}));

export default defineConfig({
	testDir: 'showcase',
	fullyParallel: true,
	forbidOnly: !!process.env.CI,
	retries: 0,
	reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
	expect: { timeout: 10_000 },
	use: {
		actionTimeout: 10_000,
		baseURL: DEMO,
		locale: 'en-US',
		timezoneId: 'Europe/Berlin',
		trace: 'retain-on-failure'
	},
	projects
});
