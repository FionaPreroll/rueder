import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { DemoReader } from '#lib/demo/reader.svelte.ts';
import { layerOf } from '#lib/layers.ts';
import ArticleBody from './ArticleBody.svelte';
import ImageViewer from './ImageViewer.svelte';

const now = Date.parse('2026-11-10T14:30:00Z');

const comic =
	'<p><img src="/comic.png" alt="Ladder" title="The ladder was never the problem."></p>' +
	'<p><a href="https://comics.example/"><img src="/linked.png" alt="Site"></a></p>';

function article() {
	const reader = new DemoReader(() => now);
	reader.show({ kind: 'feed', id: 'feed-tagesbote' });
	const article = { ...reader.list[0]!, html: comic, image: null };
	render(ArticleBody, { reader, article, now, onversions: vi.fn() });
	return reader;
}

describe('an image in an article', () => {
	it('opens large with its title and alternative text, and closes by its button', async () => {
		article();
		// Only the image that is no link opens large.
		expect(screen.queryByRole('button', { name: 'Site' })).toBeNull();
		const image = screen.getByRole('button', { name: 'Ladder' });
		expect(image).toHaveAttribute('aria-haspopup', 'dialog');

		const length = history.length;
		await userEvent.click(image);
		const dialog = screen.getByRole('dialog', { name: 'The ladder was never the problem.' });
		expect(within(dialog).getByRole('img', { name: 'Ladder' })).toHaveAttribute(
			'src',
			expect.stringContaining('/comic.png')
		);
		expect(dialog).toHaveTextContent('The ladder was never the problem.');
		expect(dialog).toHaveTextContent('Ladder');
		expect(history.length).toBe(length + 1);

		await userEvent.click(within(dialog).getByRole('button', { name: 'Close' }));
		await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
		await waitFor(() => expect(layerOf(history.state)).toBe(0));
	});

	it('closes when the browser goes back', async () => {
		article();
		await userEvent.click(screen.getByRole('button', { name: 'Ladder' }));
		expect(screen.getByRole('dialog')).toBeInTheDocument();
		history.back();
		await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
	});

	it('opens with Enter, and keeps the reader’s shortcuts from the keys while open', async () => {
		article();
		const keys = vi.fn();
		window.addEventListener('keydown', keys);
		screen.getByRole('button', { name: 'Ladder' }).focus();
		await userEvent.keyboard('{Enter}');
		const dialog = screen.getByRole('dialog');
		expect(dialog).toHaveFocus();
		await userEvent.keyboard('j');
		await userEvent.keyboard('{Escape}');
		await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
		expect(keys).not.toHaveBeenCalledWith(expect.objectContaining({ key: 'j' }));
		window.removeEventListener('keydown', keys);
		expect(screen.getByRole('button', { name: 'Ladder' })).toHaveFocus();
	});
});

describe('the large view', () => {
	const open = (props: Partial<{ alt: string; title: string }> = {}) => {
		const onclose = vi.fn();
		const { unmount } = render(ImageViewer, {
			src: '/comic.png',
			alt: 'Ladder',
			title: '',
			onclose,
			...props
		});
		const dialog = screen.getByRole('dialog');
		const image = within(dialog).getByRole('img');
		return { onclose, dialog, image, unmount };
	};

	it('shows a text the feed gave twice only once, and is named without one', () => {
		const { dialog, unmount } = open({ title: 'Ladder' });
		expect(dialog.querySelectorAll('p')).toHaveLength(1);
		unmount();
		render(ImageViewer, { src: '/x.png', alt: '', title: '', onclose: vi.fn() });
		expect(screen.getByRole('dialog', { name: 'Image' }).querySelector('p')).toBeNull();
	});

	it('zooms by the buttons, the keys and the wheel, and back with 0', async () => {
		const { image, dialog } = open();
		const scale = () => Number(/scale\(([\d.]+)\)/.exec(image.style.transform)?.[1]);
		expect(scale()).toBe(1);
		await userEvent.click(within(dialog).getByRole('button', { name: 'Zoom in' }));
		expect(scale()).toBe(2);
		await userEvent.click(within(dialog).getByRole('button', { name: 'Zoom out' }));
		expect(scale()).toBe(1);
		await userEvent.keyboard('+');
		expect(scale()).toBe(2);
		await userEvent.keyboard('0');
		expect(scale()).toBe(1);
		await fireEvent.wheel(image.parentElement!, { deltaY: -200 });
		expect(scale()).toBeGreaterThan(1);
		expect(screen.getByRole('dialog')).toBeInTheDocument();
	});

	it('closes on a tap beside the image, but not after a drag', async () => {
		const { onclose, image } = open();
		const stage = image.parentElement!;
		const tap = async (to: number) => {
			await fireEvent.pointerDown(stage, { pointerId: 1, button: 0, clientX: 5, clientY: 5 });
			await fireEvent.pointerMove(stage, { pointerId: 1, clientX: to, clientY: 5 });
			await fireEvent.pointerUp(stage, { pointerId: 1, button: 0, clientX: to, clientY: 5 });
		};
		await tap(80);
		await new Promise((done) => setTimeout(done, 10));
		expect(onclose).not.toHaveBeenCalled();
		await tap(6);
		await waitFor(() => expect(onclose).toHaveBeenCalled());
	});
});
