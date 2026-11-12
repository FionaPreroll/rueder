// Keeps the screen on while the page is in sight, where the reader asked for it
// (the setting `wakeLock`) and the browser can, through the Screen Wake Lock API.
// The browser lets go of the lock whenever the page is hidden, so it is asked for
// again each time the page comes back.

/** Whether this browser can keep the screen on. */
export const canKeepAwake = (nav: Navigator | undefined = globalThis.navigator) =>
	!!nav && 'wakeLock' in nav && !!nav.wakeLock;

/** Keeps the screen on until the function it returns is called. */
export function keepAwake(
	doc: Document = document,
	locks: WakeLock | undefined = canKeepAwake() ? navigator.wakeLock : undefined
): () => void {
	if (!locks) return () => {};
	const wake = locks;
	let lock: WakeLockSentinel | null = null;
	let asking = false;
	let stopped = false;

	async function take() {
		if (stopped || asking || doc.visibilityState !== 'visible' || (lock && !lock.released)) return;
		asking = true;
		try {
			const got = await wake.request('screen');
			if (stopped) void got.release().catch(() => {});
			else lock = got;
		} catch {
			// Refused, as with a low battery: the screen goes off as it would anyway.
		} finally {
			asking = false;
		}
	}

	void take();
	doc.addEventListener('visibilitychange', take);
	return () => {
		stopped = true;
		doc.removeEventListener('visibilitychange', take);
		void lock?.release().catch(() => {});
		lock = null;
	};
}
