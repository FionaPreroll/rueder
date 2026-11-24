// The entries the windows over the page make in the browser's history.
import { describe, expect, it, vi } from 'vitest';
import { backCloses, layerOf } from './layers.ts';

/** Until the history has done what it was asked, and the windows what they wait for. */
const settled = () => new Promise((done) => setTimeout(done, 20));

describe('backCloses', () => {
	it('closes the topmost window on back, and the one under it on the next', async () => {
		history.replaceState({ rueder: 1, open: '5' }, '');
		const length = history.length;
		const dialog = vi.fn();
		const form = vi.fn();
		const leaveDialog = backCloses(dialog);
		const leaveForm = backCloses(form);
		expect(history.length).toBe(length + 2);
		// The entries keep what the reader's way holds.
		expect(history.state).toEqual({ rueder: 1, open: '5', layer: 2 });

		history.back();
		await settled();
		expect(form).toHaveBeenCalledOnce();
		expect(dialog).not.toHaveBeenCalled();
		leaveForm();
		await settled();
		expect(layerOf(history.state)).toBe(1);

		history.back();
		await settled();
		expect(dialog).toHaveBeenCalledOnce();
		leaveDialog();
		await settled();
		expect(history.state).toEqual({ rueder: 1, open: '5' });
	});

	it('takes the entries of windows closed some other way with them', async () => {
		history.replaceState(null, '');
		const dialog = vi.fn();
		const leaveDialog = backCloses(dialog);
		const leaveForm = backCloses(() => {});
		// The dialog closes with the form still open in it.
		leaveForm();
		leaveDialog();
		await settled();
		await settled();
		expect(layerOf(history.state)).toBe(0);
		expect(dialog).not.toHaveBeenCalled();
	});

	it('leaves the entry of another list that was made while the window closed', async () => {
		history.replaceState({ rueder: 1, stream: 'all' }, '');
		const leave = backCloses(() => {});
		leave();
		history.pushState({ rueder: 1, stream: 'feed' }, '');
		await settled();
		expect(history.state).toEqual({ rueder: 1, stream: 'feed' });
		history.back();
		await settled();
	});

	it('stays open on back while it holds something', async () => {
		history.replaceState(null, '');
		const close = vi.fn();
		let holds = true;
		const leave = backCloses(close, () => holds);
		history.back();
		await settled();
		expect(close).not.toHaveBeenCalled();
		expect(layerOf(history.state)).toBe(1);
		holds = false;
		history.back();
		await settled();
		expect(close).toHaveBeenCalledOnce();
		leave();
	});

	it('counts only whole numbers as windows', () => {
		expect(layerOf(null)).toBe(0);
		expect(layerOf({ layer: '2' })).toBe(0);
		expect(layerOf({ layer: 1.5 })).toBe(0);
		expect(layerOf({ layer: -1 })).toBe(0);
		expect(layerOf({ layer: 3 })).toBe(3);
	});
});
