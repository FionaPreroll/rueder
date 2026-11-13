// The frontend's documentation (`pnpm build:docs`, for GitHub Pages, see
// pages/README.md): the TypeScript in src/lib, starting from README.md. The
// components are .svelte files, which TypeDoc does not read; the demo shows
// them. RUEDER_DOCS_FOOTER puts the commit and the day of the build at the
// foot of every page.

/** @type {Partial<import('typedoc').TypeDocOptions>} */
const config = {
	name: 'rueder frontend',
	entryPoints: ['src/lib'],
	entryPointStrategy: 'expand',
	exclude: ['**/*.test.ts', '**/test/**', 'src/lib/components/**', 'src/lib/index.ts'],
	out: 'build-docs',
	readme: 'README.md',
	// Type errors are svelte-check's to find (`pnpm check`); TypeDoc's own
	// compiler does not know .svelte files.
	skipErrorChecking: true,
	treatWarningsAsErrors: true,
	...(process.env.RUEDER_DOCS_FOOTER ? { customFooterHtml: process.env.RUEDER_DOCS_FOOTER } : {})
};

export default config;
