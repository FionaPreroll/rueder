// rueder as an installed app on a desktop: the number of unread articles on the
// app's icon (Badging API), and installing the page as an app from the account
// menu, where the browser offers it (`beforeinstallprompt`, in browsers built on
// Chromium) or where it says how (Safari). The page runs as an app once it is
// installed; then there is nothing left to offer.

import { inAndroidApp } from './app.ts';

/** What Chromium hands the page when it would install it, held back until the reader asks. */
interface InstallPrompt extends Event {
	prompt(): Promise<unknown>;
}

/** What the account menu offers for installing. */
export const install = $state<{
	/** Shows the browser's own dialog for installing; null while the browser offers none. */
	offer: (() => Promise<void>) | null;
	/** Safari installs only from its own menus; then the account menu says where. */
	safari: boolean;
}>({ offer: null, safari: false });

/** Whether the page runs as an installed app, in a window of its own. */
export function runsAsApp(win: Window = window): boolean {
	const nav = win.navigator as Navigator & { standalone?: boolean };
	if (nav.standalone === true) return true;
	if (typeof win.matchMedia !== 'function') return false;
	return ['standalone', 'window-controls-overlay', 'minimal-ui', 'fullscreen'].some(
		(mode) => win.matchMedia(`(display-mode: ${mode})`).matches
	);
}

/** Safari, and every browser on an iPhone or iPad, which are Safari inside. */
const looksLikeSafari = (nav: Navigator) =>
	nav.vendor === 'Apple Computer, Inc.' && !/\b(Chrome|Chromium|Edg)\//.test(nav.userAgent);

/**
 * Listens for the browser offering to install the page, and tells the account
 * menu (`install`). Returns how to stop. Inside the Android app and in a page
 * that already runs as an app, there is nothing to offer.
 */
export function watchInstall(win: Window = window): () => void {
	if (runsAsApp(win) || inAndroidApp(win.navigator.userAgent)) return () => {};
	install.safari = looksLikeSafari(win.navigator);
	const offered = (event: Event) => {
		// The browser's own hint stays; the menu offers the same, when asked.
		event.preventDefault();
		const prompt = event as InstallPrompt;
		install.safari = false;
		install.offer = async () => {
			// The browser shows its dialog only once for each offer.
			install.offer = null;
			try {
				await prompt.prompt();
			} catch {
				// Shown already, or refused by the browser: it offers anew when it can.
			}
		};
	};
	const installed = () => {
		install.offer = null;
		install.safari = false;
	};
	win.addEventListener('beforeinstallprompt', offered);
	win.addEventListener('appinstalled', installed);
	return () => {
		win.removeEventListener('beforeinstallprompt', offered);
		win.removeEventListener('appinstalled', installed);
		installed();
	};
}

/** What the Badging API adds to the navigator, where the browser has it. */
export type Badges = Pick<Navigator, 'setAppBadge' | 'clearAppBadge'>;

/**
 * Puts the number of unread articles on the app's icon, or takes it off at zero.
 * Where the browser cannot, or the page is no installed app, nothing shows, and
 * the title of the window still has the number.
 */
export function showOnIcon(
	unread: number,
	nav: Partial<Badges> | undefined = globalThis.navigator
): void {
	if (!nav?.setAppBadge || !nav.clearAppBadge) return;
	const done = unread > 0 ? nav.setAppBadge(unread) : nav.clearAppBadge();
	// Refused, as where the browser asks for leave to show notifications first.
	void Promise.resolve(done).catch(() => {});
}
