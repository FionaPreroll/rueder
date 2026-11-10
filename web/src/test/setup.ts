import '@testing-library/jest-dom/vitest';

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
