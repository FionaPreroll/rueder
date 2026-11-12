import { describe, expect, it, vi } from 'vitest';
import { canKeepAwake, keepAwake } from './wakelock.ts';

/** A stand-in for the browser's locks, which lets go of them as a hidden page does. */
function fakeLocks(refuse = false) {
	const held: { released: boolean; release: () => Promise<void> }[] = [];
	const locks = {
		request: vi.fn(async () => {
			if (refuse) throw new DOMException('Low battery', 'NotAllowedError');
			const lock = {
				released: false,
				release: vi.fn(async () => {
					lock.released = true;
				})
			};
			held.push(lock);
			return lock;
		})
	};
	return { locks: locks as unknown as WakeLock, request: locks.request, held };
}

function setVisibility(state: DocumentVisibilityState) {
	Object.defineProperty(document, 'visibilityState', { value: state, configurable: true });
	document.dispatchEvent(new Event('visibilitychange'));
}

const settle = () => new Promise((resolve) => setTimeout(resolve));

describe('keeping the screen on', () => {
	it('holds a lock while the page is in sight, and takes a new one when it comes back', async () => {
		setVisibility('visible');
		const { locks, request, held } = fakeLocks();
		const stop = keepAwake(document, locks);
		await settle();
		expect(request).toHaveBeenCalledTimes(1);
		expect(held[0].released).toBe(false);

		// Hiding the page lets go of the lock, as browsers do.
		held[0].released = true;
		setVisibility('hidden');
		await settle();
		expect(request).toHaveBeenCalledTimes(1);

		setVisibility('visible');
		await settle();
		expect(request).toHaveBeenCalledTimes(2);

		// Coming back while the lock is still held asks for nothing more.
		setVisibility('visible');
		await settle();
		expect(request).toHaveBeenCalledTimes(2);

		stop();
		expect(held[1].released).toBe(true);
		setVisibility('visible');
		await settle();
		expect(request).toHaveBeenCalledTimes(2);
	});

	it('lets go of a lock that comes after it was stopped', async () => {
		setVisibility('visible');
		const { locks, held } = fakeLocks();
		const stop = keepAwake(document, locks);
		stop();
		await settle();
		expect(held).toHaveLength(1);
		expect(held[0].released).toBe(true);
	});

	it('asks for nothing while the page is hidden, and lives with a refusal', async () => {
		setVisibility('hidden');
		const hidden = fakeLocks();
		keepAwake(document, hidden.locks)();
		await settle();
		expect(hidden.request).not.toHaveBeenCalled();

		setVisibility('visible');
		const refused = fakeLocks(true);
		const stop = keepAwake(document, refused.locks);
		await settle();
		expect(refused.request).toHaveBeenCalledTimes(1);
		stop();
	});

	it('does nothing where the browser cannot', () => {
		expect(canKeepAwake({} as Navigator)).toBe(false);
		expect(canKeepAwake({ wakeLock: {} } as Navigator)).toBe(true);
		expect(keepAwake(document, undefined)).toBeTypeOf('function');
	});
});
