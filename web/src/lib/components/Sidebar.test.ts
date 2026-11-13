import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DemoReader } from '#lib/demo/reader.svelte.ts';
import { ancestorIds } from '#lib/tree.ts';
import { defaults, settings } from '#lib/settings.svelte.ts';
import Sidebar from './Sidebar.svelte';

const now = Date.parse('2026-11-10T14:30:00Z');

function setup() {
	const reader = new DemoReader(() => now);
	const onsubscribe = vi.fn();
	const onpick = vi.fn();
	const { container } = render(Sidebar, { reader, onsubscribe, onpick });
	const row = (id: string) => container.querySelector<HTMLElement>(`[data-node="${id}"]`)!;
	return { reader, onsubscribe, onpick, row };
}

afterEach(() => Object.assign(settings, defaults));

describe('Sidebar', () => {
	it('shows the lists and the tree with their unread counts', () => {
		const { reader, row } = setup();
		const nav = screen.getByRole('navigation', { name: 'Subscriptions' });
		expect(within(nav).getByRole('button', { name: /^All items/ })).toHaveTextContent(
			`(${reader.totalUnread})`
		);
		expect(within(nav).getByRole('button', { name: /^Hidden by filters/ })).toHaveTextContent(
			String(reader.hiddenCount)
		);
		expect(row('feed-tagesbote')).toHaveTextContent(
			`Tagesbote (${reader.unread('feed-tagesbote')})`
		);
		// A feed with nothing unread has no count, and is not bold.
		expect(reader.unread('feed-borrow')).toBe(0);
		expect(row('feed-borrow')).toHaveTextContent(/^Borrow Checker Diaries$/);
		expect(row('feed-wort')).toHaveTextContent('The last fetch failed: 404 Not Found');
		expect(within(nav).getByRole('button', { name: /^Home/ })).toHaveAttribute(
			'aria-current',
			'page'
		);
	});

	it('picks what the list shows', async () => {
		const { reader, onpick, row } = setup();
		await userEvent.click(row('feed-tagesbote'));
		expect(reader.stream).toEqual({ kind: 'feed', id: 'feed-tagesbote' });
		expect(row('feed-tagesbote')).toHaveAttribute('aria-current', 'page');
		await userEvent.click(row('folder-news'));
		expect(reader.stream).toEqual({ kind: 'folder', id: 'folder-news' });
		await userEvent.click(screen.getByRole('button', { name: /^Starred items/ }));
		expect(reader.stream).toEqual({ kind: 'starred' });
		expect(onpick).toHaveBeenCalledTimes(3);
	});

	it('counts down as articles are read', async () => {
		const { reader, row } = setup();
		const before = reader.unread('feed-tagesbote');
		reader.show({ kind: 'feed', id: 'feed-tagesbote' });
		reader.step(1);
		await vi.waitFor(() =>
			expect(row('feed-tagesbote')).toHaveTextContent(`Tagesbote (${before - 1})`)
		);
		reader.markAllRead();
		await vi.waitFor(() => expect(row('feed-tagesbote')).toHaveTextContent(/^Tagesbote$/));
	});

	it('closes and opens folders', async () => {
		const { row } = setup();
		const news = row('folder-news').parentElement!;
		await userEvent.click(within(news).getByRole('button', { name: 'Close folder' }));
		expect(row('feed-tagesbote')).toBeNull();
		await userEvent.click(within(news).getByRole('button', { name: 'Open folder' }));
		expect(row('feed-tagesbote')).not.toBeNull();
	});

	it('sorts by dragging: a feed into a folder', async () => {
		const { reader, row } = setup();
		const dragged = row('feed-nebenan').parentElement!;
		const target = row('folder-news').parentElement!;
		await fireEvent.dragStart(dragged);
		expect(dragged).toHaveClass('opacity-40');
		await fireEvent.dragOver(target);
		expect(target).toHaveClass('ring-2');
		await fireEvent.drop(target);
		expect(ancestorIds(reader.tree, 'feed-nebenan')).toEqual(['folder-news']);
		expect(target).not.toHaveClass('ring-2');
	});

	it('makes a new folder of a feed dropped on the middle of another', async () => {
		const { reader, row } = setup();
		const dragged = row('feed-nebenan').parentElement!;
		const target = row('feed-harbor').parentElement!;
		target.getBoundingClientRect = () => DOMRect.fromRect({ x: 0, y: 100, width: 200, height: 20 });
		// The test's browser has no DragEvent with a place; a mouse event carries one.
		const over = (clientY: number) =>
			fireEvent(target, new MouseEvent('dragover', { clientY, bubbles: true, cancelable: true }));
		await fireEvent.dragStart(dragged);
		// The edges of a feed still put the dragged one before or after it.
		await over(102);
		expect(target).not.toHaveClass('ring-2');
		await over(110);
		expect(target).toHaveClass('ring-2');
		await fireEvent.drop(target);
		const [folderId] = ancestorIds(reader.tree, 'feed-harbor').slice(-1);
		expect(ancestorIds(reader.tree, 'feed-harbor')).toEqual(['folder-news', folderId]);
		expect(ancestorIds(reader.tree, 'feed-nebenan')).toEqual(['folder-news', folderId]);
		expect(row(folderId)).toHaveTextContent('New folder');
	});

	it('does not take a drop where the thing cannot go', async () => {
		const { reader, row } = setup();
		const tech = row('folder-tech').parentElement!;
		const rust = row('folder-rust').parentElement!;
		// A folder cannot go into a folder inside it.
		await fireEvent.dragStart(tech);
		await fireEvent.dragOver(rust);
		expect(rust).not.toHaveClass('ring-2');
		await fireEvent.drop(rust);
		await fireEvent.dragEnd(tech);
		expect(ancestorIds(reader.tree, 'folder-rust')).toEqual(['folder-tech', 'folder-software']);
		expect(tech).not.toHaveClass('opacity-40');
	});

	it('sorts with Alt and the arrow keys, and keeps the focus on the row', async () => {
		const { reader, row } = setup();
		row('feed-nebenan').focus();
		await userEvent.keyboard('{Alt>}{ArrowUp}{/Alt}');
		// One up from below the last folder: now before it.
		const order = reader.tree.map((node) => node.id);
		expect(order.indexOf('feed-nebenan')).toBe(order.indexOf('folder-fun') - 1);
		await vi.waitFor(() => expect(row('feed-nebenan')).toHaveFocus());
		await userEvent.keyboard('{Alt>}{ArrowDown}{ArrowRight}{/Alt}');
		expect(ancestorIds(reader.tree, 'feed-nebenan')).toEqual(['folder-fun']);
		await userEvent.keyboard('{Alt>}{ArrowLeft}{/Alt}');
		expect(ancestorIds(reader.tree, 'feed-nebenan')).toEqual([]);
		// Without Alt the arrows are left alone.
		const before = reader.tree;
		await userEvent.keyboard('{ArrowUp}');
		expect(reader.tree).toBe(before);
	});

	it('makes, renames and removes folders', async () => {
		const { reader, row } = setup();
		await userEvent.click(screen.getByRole('button', { name: 'New folder' }));
		await userEvent.type(screen.getByLabelText('Name of the folder'), 'Later{Enter}');
		expect(reader.tree.at(-1)).toMatchObject({ kind: 'folder', title: 'Later' });
		expect(screen.queryByLabelText('Name of the folder')).toBeNull();

		await userEvent.click(screen.getByRole('button', { name: 'Actions for the folder Later' }));
		await userEvent.click(screen.getByRole('menuitem', { name: 'Rename' }));
		const name = screen.getByLabelText('Name of the folder');
		expect(name).toHaveValue('Later');
		await userEvent.clear(name);
		await userEvent.type(name, 'Someday{Enter}');
		expect(reader.tree.at(-1)).toMatchObject({ title: 'Someday' });

		await userEvent.click(
			screen.getByRole('button', { name: 'Actions for the folder Nachrichten' })
		);
		await userEvent.click(screen.getByRole('menuitem', { name: /^Remove the folder/ }));
		expect(row('folder-news')).toBeNull();
		expect(ancestorIds(reader.tree, 'feed-tagesbote')).toEqual([]);
		expect(row('feed-tagesbote')).not.toBeNull();
	});

	it('gives up naming a folder on Escape', async () => {
		const { reader } = setup();
		const folders = reader.tree.length;
		await userEvent.click(screen.getByRole('button', { name: 'New folder' }));
		await userEvent.type(screen.getByLabelText('Name of the folder'), 'Never{Escape}');
		expect(screen.queryByLabelText('Name of the folder')).toBeNull();
		expect(reader.tree).toHaveLength(folders);
	});

	it('renames a feed, and gives it its own name back', async () => {
		const { reader, row } = setup();
		await userEvent.click(screen.getByRole('button', { name: 'Actions for the feed Tagesbote' }));
		await userEvent.click(screen.getByRole('menuitem', { name: 'Rename' }));
		const name = screen.getByLabelText('Name of the feed');
		expect(name).toHaveValue('Tagesbote');
		expect(name).toHaveAttribute('placeholder', 'Empty for the feed’s own name');
		await userEvent.clear(name);
		await userEvent.type(name, 'Morning paper{Enter}');
		expect(reader.feeds['feed-tagesbote'].title).toBe('Morning paper');
		expect(row('feed-tagesbote')).toHaveTextContent(/^Morning paper/);

		await userEvent.click(
			screen.getByRole('button', { name: 'Actions for the feed Morning paper' })
		);
		await userEvent.click(screen.getByRole('menuitem', { name: 'Rename' }));
		await userEvent.clear(screen.getByLabelText('Name of the feed'));
		await userEvent.keyboard('{Enter}');
		expect(reader.feeds['feed-tagesbote'].title).toBe('Tagesbote');
	});

	it('unsubscribes from a feed once that is confirmed', async () => {
		const { reader, row } = setup();
		const open = async () => {
			await userEvent.click(screen.getByRole('button', { name: 'Actions for the feed Tagesbote' }));
			await userEvent.click(screen.getByRole('menuitem', { name: 'Unsubscribe' }));
			return screen.getByRole('dialog', { name: 'Unsubscribe?' });
		};
		let dialog = await open();
		expect(dialog).toHaveTextContent('Unsubscribe from “Tagesbote”? What you read and starred');
		await userEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
		expect(screen.queryByRole('dialog')).toBeNull();
		expect(reader.feeds['feed-tagesbote']).toBeDefined();

		dialog = await open();
		await userEvent.click(within(dialog).getByRole('button', { name: 'Unsubscribe' }));
		expect(screen.queryByRole('dialog')).toBeNull();
		expect(reader.feeds['feed-tagesbote']).toBeUndefined();
		expect(row('feed-tagesbote')).toBeNull();
	});

	it('reports a feed where the server takes reports, and marks a blocked one', async () => {
		const reader = new DemoReader(() => now);
		const feedMenu = () =>
			userEvent.click(screen.getByRole('button', { name: 'Actions for the feed Tagesbote' }));
		const { unmount } = render(Sidebar, { reader, onsubscribe: vi.fn(), onpick: vi.fn() });
		await feedMenu();
		expect(screen.queryByRole('menuitem', { name: 'Report the feed …' })).toBeNull();
		unmount();

		Object.assign(reader.can, { reportContent: true });
		const reportContent = vi.spyOn(reader, 'reportContent').mockResolvedValue('sent');
		render(Sidebar, { reader, onsubscribe: vi.fn(), onpick: vi.fn() });
		await feedMenu();
		await userEvent.click(screen.getByRole('menuitem', { name: 'Report the feed …' }));
		const dialog = screen.getByRole('dialog', { name: 'Report the feed' });
		expect(dialog).toHaveTextContent('“Tagesbote” was reported');
		await userEvent.click(within(dialog).getByRole('radio', { name: 'Against the law' }));
		await userEvent.click(within(dialog).getByRole('button', { name: 'Send the report' }));
		expect(reportContent).toHaveBeenCalledWith({ feedId: 'feed-tagesbote' }, 'illegal', '');
		await userEvent.click(within(dialog).getAllByRole('button', { name: 'Close' })[1]!);
		expect(screen.queryByRole('dialog')).toBeNull();

		reader.feeds = {
			...reader.feeds,
			'feed-tagesbote': { ...reader.feeds['feed-tagesbote']!, blocked: true }
		};
		flushSync();
		expect(
			within(screen.getByRole('button', { name: /^Tagesbote/ })).getByText(
				'Blocked by the operator of this rueder'
			)
		).toBeInTheDocument();
		await feedMenu();
		expect(screen.queryByRole('menuitem', { name: 'Report the feed …' })).toBeNull();
	});

	it('lists the labels, makes, renames, recolors and deletes them', async () => {
		const { reader, onpick } = setup();
		const nav = screen.getByRole('navigation', { name: 'Subscriptions' });
		await userEvent.click(within(nav).getByRole('button', { name: 'Arbeit' }));
		expect(reader.stream).toEqual({ kind: 'label', id: 'label-work' });
		expect(onpick).toHaveBeenCalled();

		await userEvent.click(within(nav).getByRole('button', { name: 'New label' }));
		await userEvent.type(screen.getByLabelText('Name of the label'), 'Recipes{Enter}');
		expect(reader.labels.at(-1)).toMatchObject({ title: 'Recipes', color: '#f9ab00' });

		await userEvent.click(screen.getByRole('button', { name: 'Actions for the label Recipes' }));
		await userEvent.click(screen.getByRole('menuitem', { name: 'Rename' }));
		const name = screen.getByLabelText('Name of the label');
		await userEvent.clear(name);
		await userEvent.type(name, 'Cooking{Enter}');
		expect(reader.labels.at(-1)?.title).toBe('Cooking');

		await userEvent.click(screen.getByRole('button', { name: 'Actions for the label Cooking' }));
		await userEvent.click(screen.getByRole('menuitemradio', { name: 'Purple' }));
		expect(reader.labels.at(-1)?.color).toBe('#9334e6');

		await userEvent.click(screen.getByRole('button', { name: 'Actions for the label Cooking' }));
		await userEvent.click(screen.getByRole('menuitem', { name: 'Delete the label' }));
		expect(reader.labels.map((label) => label.title)).toEqual(['Später lesen', 'Arbeit']);
	});

	it('folds the labels away and back, and the + brings them out again', async () => {
		const { reader } = setup();
		const nav = screen.getByRole('navigation', { name: 'Subscriptions' });
		expect(within(nav).getByRole('button', { name: 'Arbeit' })).toBeInTheDocument();

		await userEvent.click(within(nav).getByRole('button', { name: 'Hide the labels' }));
		expect(within(nav).queryByRole('button', { name: 'Arbeit' })).toBeNull();
		// Folded away, the heading says how many there are.
		const heading = within(nav).getByRole('button', { name: 'Show the labels' });
		expect(heading).toHaveTextContent(`Labels (${reader.labels.length})`);
		expect(heading).toHaveAttribute('aria-expanded', 'false');
		// It stays folded away for the next time, next to the folders.
		expect(reader.isClosed('labels')).toBe(true);

		await userEvent.click(heading);
		expect(within(nav).getByRole('button', { name: 'Arbeit' })).toBeInTheDocument();

		// Making one brings them out, so the new one is not made out of sight.
		await userEvent.click(within(nav).getByRole('button', { name: 'Hide the labels' }));
		await userEvent.click(within(nav).getByRole('button', { name: 'New label' }));
		expect(within(nav).getByRole('button', { name: 'Arbeit' })).toBeInTheDocument();
		await userEvent.type(screen.getByLabelText('Name of the label'), 'Recipes{Enter}');
		expect(reader.labels.at(-1)?.title).toBe('Recipes');
	});

	it('asks to subscribe', async () => {
		const { onsubscribe } = setup();
		await userEvent.click(screen.getByRole('button', { name: 'Subscribe' }));
		expect(onsubscribe).toHaveBeenCalledOnce();
	});
});
