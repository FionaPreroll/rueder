// A new version of rueder: the service worker of the new build is installed and
// waits until the reader chooses to reload, so it never swaps the page out under
// them. While the page stays open, it looks for one every hour, and whenever the
// page comes back into view after a while.

/** The worker SvelteKit builds from src/service-worker.ts, at the top of the site. */
const WORKER = '/service-worker.js';
const HOUR = 60 * 60 * 1000;
/** Coming back into view looks again only if the last look is this long ago. */
const AGAIN_AFTER = 5 * 60 * 1000;

/** What the page knows of a new version. */
export const update = $state<{
	/** Lets the new version in and reloads the page; null while there is none. */
	apply: (() => void) | null;
}>({ apply: null });

/**
 * Registers the service worker and looks for new versions. Returns how to stop
 * looking. Without service workers, as over plain http, it does nothing.
 */
export async function registerServiceWorker(
	container: ServiceWorkerContainer | undefined = navigator.serviceWorker,
	reload: () => void = () => location.reload(),
	page: Document = document,
	now: () => number = Date.now
): Promise<() => void> {
	if (!container) return () => {};
	let registration: ServiceWorkerRegistration;
	try {
		// One file without imports: a classic worker, which every browser takes.
		registration = await container.register(WORKER);
	} catch {
		// Not allowed here, as in a private window of some browsers: the page
		// works as before, only without starting offline.
		return () => {};
	}
	// A worker that waits while another serves the page is a new version.
	const offer = () => {
		const waiting = registration.waiting;
		if (!waiting || !container.controller) return;
		update.apply = () => {
			container.addEventListener('controllerchange', () => reload(), { once: true });
			waiting.postMessage('skipWaiting');
		};
	};
	offer();
	registration.addEventListener('updatefound', () => {
		const installing = registration.installing;
		installing?.addEventListener('statechange', () => {
			if (installing.state === 'installed') offer();
		});
	});
	let looked = now();
	const look = () => {
		looked = now();
		// Offline or unreachable: the next look will do.
		registration.update().catch(() => {});
	};
	const every = setInterval(look, HOUR);
	const shown = () => {
		if (page.visibilityState === 'visible' && now() - looked >= AGAIN_AFTER) look();
	};
	page.addEventListener('visibilitychange', shown);
	return () => {
		clearInterval(every);
		page.removeEventListener('visibilitychange', shown);
	};
}
