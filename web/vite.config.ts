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
// address, or straight from a file: for handing the interface around without a
// server to serve it.
// It finds its page by the part of the address after the #, and takes its routes from
// src/single: the reader with made-up data, without `ssr = false`, which SvelteKit
// refuses there.
const singleFile = !!process.env.RUEDER_SINGLE_FILE;

// `pnpm dev:demo` runs the same reader with made-up data in the dev server, to see
// the interface without a rueder server next to it.
const demo = singleFile || !!process.env.RUEDER_DEMO;

// `pnpm dev` passes what is the server's on to a rueder server on this machine
// (`cargo run` in server/), so the page and the API share one address as they do
// in production. RUEDER_SERVER names another one.
const server = process.env.RUEDER_SERVER ?? 'http://127.0.0.1:3000';

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
			// The page registers the service worker itself, to offer new versions
			// (src/lib/updates.svelte.ts). The builds with made-up data have none.
			serviceWorker: { register: false },
			...(demo ? { files: { routes: 'src/single', serviceWorker: 'src/single/no-worker' } } : {}),
			...(singleFile
				? { output: { bundleStrategy: 'inline' as const }, router: { type: 'hash' as const } }
				: {})
		}),
		// Only under Vitest: Svelte's browser build and a clean DOM after each test.
		svelteTesting()
	],
	build: singleFile ? { assetsInlineLimit: Infinity } : {},
	server: { proxy: { '/api': server, '/img': server } },
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
