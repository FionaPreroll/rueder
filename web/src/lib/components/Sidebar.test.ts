import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
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

	it('asks to subscribe', async () => {
		const { onsubscribe } = setup();
		await userEvent.click(screen.getByRole('button', { name: 'Subscribe' }));
		expect(onsubscribe).toHaveBeenCalledOnce();
	});
});
