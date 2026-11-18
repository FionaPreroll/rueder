// The bar that says how wide a pane is.

import { fireEvent, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import Resizer from './Resizer.svelte';

const setup = (width = 300) => {
	const onwidth = vi.fn();
	render(Resizer, { width, min: 200, max: 500, fallback: 288, label: 'How wide', onwidth });
	return { bar: screen.getByRole('separator', { name: 'How wide' }), onwidth };
};

/** What the bar would be told after being dragged this far. */
async function drag(bar: HTMLElement, by: number) {
	await fireEvent.pointerDown(bar, { button: 0, pointerId: 1, clientX: 1000 });
	await fireEvent.pointerMove(bar, { pointerId: 1, clientX: 1000 + by });
	await fireEvent.pointerUp(bar, { pointerId: 1, clientX: 1000 + by });
}

describe('Resizer', () => {
	it('says what it is for and how wide the pane is', () => {
		const { bar } = setup();
		expect(bar).toHaveAttribute('aria-orientation', 'vertical');
		expect(bar).toHaveAttribute('aria-valuenow', '300');
		expect(bar).toHaveAttribute('aria-valuemin', '200');
		expect(bar).toHaveAttribute('aria-valuemax', '500');
		expect(bar).toHaveAttribute('tabindex', '0');
	});

	it('follows the pointer, and stops at what fits', async () => {
		const { bar, onwidth } = setup();
		await drag(bar, 60);
		expect(onwidth).toHaveBeenLastCalledWith(360);
		await drag(bar, -400);
		expect(onwidth).toHaveBeenLastCalledWith(200);
		await drag(bar, 1000);
		expect(onwidth).toHaveBeenLastCalledWith(500);
	});

	it('moves nothing before the pointer went down, or after it came up', async () => {
		const { bar, onwidth } = setup();
		await fireEvent.pointerMove(bar, { pointerId: 1, clientX: 1200 });
		expect(onwidth).not.toHaveBeenCalled();
		await drag(bar, 10);
		onwidth.mockClear();
		await fireEvent.pointerMove(bar, { pointerId: 1, clientX: 1200 });
		expect(onwidth).not.toHaveBeenCalled();
		// Nor does any button but the first one.
		await fireEvent.pointerDown(bar, { button: 2, pointerId: 1, clientX: 1000 });
		await fireEvent.pointerMove(bar, { pointerId: 1, clientX: 1100 });
		expect(onwidth).not.toHaveBeenCalled();
	});

	it('takes the arrow keys, and the ends with Home and End', async () => {
		const { bar, onwidth } = setup();
		bar.focus();
		await userEvent.keyboard('{ArrowRight}');
		expect(onwidth).toHaveBeenLastCalledWith(316);
		await userEvent.keyboard('{ArrowLeft}');
		expect(onwidth).toHaveBeenLastCalledWith(284);
		await userEvent.keyboard('{Shift>}{ArrowRight}{/Shift}');
		expect(onwidth).toHaveBeenLastCalledWith(364);
		await userEvent.keyboard('{Home}');
		expect(onwidth).toHaveBeenLastCalledWith(200);
		await userEvent.keyboard('{End}');
		expect(onwidth).toHaveBeenLastCalledWith(500);
		onwidth.mockClear();
		await userEvent.keyboard('{ArrowUp}');
		expect(onwidth).not.toHaveBeenCalled();
	});

	it('grows the pane towards the bar when the pane lies on the right', async () => {
		// The sidebar at the end: the pane comes after the bar.
		const onwidth = vi.fn();
		render(Resizer, {
			width: 300,
			min: 200,
			max: 500,
			fallback: 288,
			label: 'After',
			after: true,
			onwidth
		});
		const after = screen.getByRole('separator', { name: 'After' });
		await drag(after, 60);
		expect(onwidth).toHaveBeenLastCalledWith(240);
		after.focus();
		await userEvent.keyboard('{ArrowLeft}');
		expect(onwidth).toHaveBeenLastCalledWith(316);
		// In a page written from right to left the pane before the bar lies on the right,
		// and one after it on the left.
		document.documentElement.dir = 'rtl';
		try {
			const { bar, onwidth: before } = setup();
			await drag(bar, 60);
			expect(before).toHaveBeenLastCalledWith(240);
			await drag(after, 60);
			expect(onwidth).toHaveBeenLastCalledWith(360);
		} finally {
			document.documentElement.dir = 'ltr';
		}
	});

	it('puts the width back on a double click', async () => {
		const { bar, onwidth } = setup(500);
		await userEvent.dblClick(bar);
		expect(onwidth).toHaveBeenLastCalledWith(288);
	});
});
