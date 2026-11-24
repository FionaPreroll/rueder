import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/svelte';
import { afterEach } from 'vitest';
import { layerOf } from '#lib/layers.ts';

// Transitions end at once. happy-dom's own animations reject a promise nobody
// waits for when Svelte cancels them, which Vitest reports as an error.
Element.prototype.animate = function () {
	const animation = {
		playState: 'finished',
		effect: null,
		currentTime: 0,
		cancel() {},
		finish() {},
		set onfinish(done: () => void) {
			queueMicrotask(done);
		}
	};
	return animation as unknown as Animation;
};

// happy-dom's `:checked` misses selected options, which Svelte's `bind:value` on a
// <select> looks for.
const querySelector = HTMLSelectElement.prototype.querySelector;
HTMLSelectElement.prototype.querySelector = function (this: HTMLSelectElement, selector: string) {
	return selector === ':checked'
		? (this.selectedOptions[0] ?? null)
		: querySelector.call(this, selector);
} as typeof querySelector;

// Windows over the page make entries in the history of their own (`#lib/layers.ts`),
// and take them back once they close. Every test leaves the history as it found it,
// so that a step back left over from one does not close the windows of the next.
afterEach(async () => {
	cleanup();
	const settled = () => new Promise((done) => setTimeout(done, 20));
	await settled();
	while (layerOf(history.state) > 0) {
		history.back();
		await settled();
	}
});
