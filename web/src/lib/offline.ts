// What the service worker (src/service-worker.ts) does with a request. The app's
// own files come from its cache, so the page starts at once and without a
// network; what is the reader's, the API and the images, always comes from the
// server and is never kept.

/** Whatever the service worker keeps starts with this, followed by the version. */
export const CACHE_PREFIX = 'rueder ';

/** The page every address of the app is, as SvelteKit's SPA falls back to it. */
export const SHELL = '/';

/** How a request is answered: the kept page, a kept file, or the server. */
export type Way = 'shell' | 'file' | 'network';

export function way(request: { method: string; url: string; mode: string }, origin: string): Way {
	if (request.method !== 'GET') return 'network';
	const url = new URL(request.url);
	if (url.origin !== origin) return 'network';
	// The reader's data and images, and the worker itself, which the browser
	// fetches to look for a new version.
	if (/^\/(api|img)(\/|$)/.test(url.pathname) || url.pathname === '/service-worker.js') {
		return 'network';
	}
	if (request.mode === 'navigate') return 'shell';
	return 'file';
}

/** The files of a build as the cache keeps them: from the top of the site, the page first. */
export function filesToKeep(paths: readonly string[]): string[] {
	const files = paths.map((path) => (path.startsWith('/') ? path : `/${path}`));
	return [SHELL, ...new Set(files.filter((file) => file !== SHELL))];
}
