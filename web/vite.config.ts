/// <reference types="vitest/config" />
import tailwindcss from '@tailwindcss/vite';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { svelteTesting } from '@testing-library/svelte/vite';
import { defineConfig } from 'vite';

if (process.env.VITEST) {
	// The same time zone and language on every machine, so dates and numbers in
	// tests come out the same. Set here, so the test workers start with them.
	process.env.TZ = 'Europe/Berlin';
	process.env.LC_ALL = 'en_US.UTF-8';
}

// `pnpm build:single` packs the whole app into one index.html that runs from any
// address, or straight from a file: for handing the draft of the interface around
// while there is no server to serve it.
// It finds its page by the part of the address after the #, and takes its routes from
// src/single: the same page without `ssr = false`, which SvelteKit refuses there.
const singleFile = !!process.env.RUEDER_SINGLE_FILE;

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},
			// SPA build: the Rust server will serve build/ and fall back to index.html.
			adapter: adapter(singleFile ? {} : { fallback: 'index.html' }),
			...(singleFile
				? {
						output: { bundleStrategy: 'inline' as const },
						router: { type: 'hash' as const },
						files: { routes: 'src/single' }
					}
				: {})
		}),
		// Only under Vitest: Svelte's browser build and a clean DOM after each test.
		svelteTesting()
	],
	build: singleFile ? { assetsInlineLimit: Infinity } : {},
	test: {
		environment: 'happy-dom',
		include: ['src/**/*.test.ts'],
		setupFiles: ['src/test/setup.ts'],
		coverage: {
			provider: 'v8',
			include: ['src/**/*.{ts,svelte}'],
			exclude: ['src/**/*.test.ts', 'src/**/test/**', 'src/single/**', 'src/app.d.ts'],
			// Paths in lcov.info from the repository root, as Codecov sees them.
			reporter: ['text-summary', ['lcov', { projectRoot: '..' }]]
		}
	}
});
