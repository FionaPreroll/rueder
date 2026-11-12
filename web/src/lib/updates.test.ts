// Looking for a new version, and letting it in when the reader chooses.

import { afterEach, describe, expect, it, vi } from 'vitest';
import { registerServiceWorker, update } from './updates.svelte.ts';

/** A service worker of the new version, installed and waiting. */
class Worker extends EventTarget {
	state = 'installing';
	postMessage = vi.fn();
}

/** What the browser does with service workers, as far as the page uses it. */
function browser(controlled: boolean) {
	const registration = Object.assign(new EventTarget(), {
		waiting: null as Worker | null,
		installing: null as Worker | null,
		update: vi.fn(async () => {})
	});
	const container = Object.assign(new EventTarget(), {
		controller: controlled ? {} : null,
		register: vi.fn(async () => registration)
	});
	return { registration, container: container as unknown as ServiceWorkerContainer };
}

afterEach(() => {
	update.apply = null;
	vi.useRealTimers();
});

describe('registerServiceWorker', () => {
	it('offers a new version once it is installed, and reloads into it', async () => {
		const { registration, container } = browser(true);
		const reload = vi.fn();
		const stop = await registerServiceWorker(container, reload, document);
		expect(container.register).toHaveBeenCalledWith('/service-worker.js');
		expect(update.apply).toBeNull();

		const worker = new Worker();
		registration.installing = worker;
		registration.dispatchEvent(new Event('updatefound'));
		worker.state = 'installed';
		registration.waiting = worker;
		worker.dispatchEvent(new Event('statechange'));
		expect(update.apply).not.toBeNull();

		update.apply?.();
		expect(worker.postMessage).toHaveBeenCalledWith('skipWaiting');
		expect(reload).not.toHaveBeenCalled();
		container.dispatchEvent(new Event('controllerchange'));
		expect(reload).toHaveBeenCalledTimes(1);
		stop();
	});

	it('takes the first worker without asking, and finds one that waits from before', async () => {
		const first = browser(false);
		first.registration.waiting = new Worker();
		await registerServiceWorker(first.container, vi.fn(), document);
		expect(update.apply).toBeNull();

		const again = browser(true);
		again.registration.waiting = new Worker();
		await registerServiceWorker(again.container, vi.fn(), document);
		expect(update.apply).not.toBeNull();
	});

	it('looks again every hour, and when the page comes back after a while', async () => {
		vi.useFakeTimers();
		let now = 0;
		const { registration, container } = browser(true);
		const stop = await registerServiceWorker(container, vi.fn(), document, () => now);
		const visible = () => document.dispatchEvent(new Event('visibilitychange'));
		now = 60_000;
		visible();
		expect(registration.update).not.toHaveBeenCalled();
		now = 6 * 60_000;
		visible();
		expect(registration.update).toHaveBeenCalledTimes(1);
		await vi.advanceTimersByTimeAsync(60 * 60_000);
		expect(registration.update).toHaveBeenCalledTimes(2);
		stop();
		await vi.advanceTimersByTimeAsync(60 * 60_000);
		expect(registration.update).toHaveBeenCalledTimes(2);
	});

	it('does without, where there are no service workers or none is allowed', async () => {
		expect(await registerServiceWorker(undefined, vi.fn(), document)).toBeTypeOf('function');
		const { container } = browser(true);
		vi.mocked(container.register).mockRejectedValueOnce(new Error('denied'));
		const stop = await registerServiceWorker(container, vi.fn(), document);
		stop();
		expect(update.apply).toBeNull();
	});
});
