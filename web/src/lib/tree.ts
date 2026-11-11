// The tree of folders and feeds in the sidebar, and moving things in it by drag and
// drop. The functions return a new tree and never change the one they are given.

import { MAX_FOLDER_DEPTH, type Folder, type TreeNode } from './model.ts';

/** Where a dragged node lands, seen from the node it is dropped on. */
export type DropPosition = 'before' | 'after' | 'inside';

const clone = (nodes: readonly TreeNode[]): TreeNode[] =>
	nodes.map((node) =>
		node.kind === 'folder' ? { ...node, children: clone(node.children) } : { ...node }
	);

interface Found {
	node: TreeNode;
	/** The list the node is in: the tree itself or a folder's children. */
	siblings: TreeNode[];
	index: number;
	/** Folders above the node, outermost first. */
	path: Folder[];
}

function find(nodes: TreeNode[], id: string, path: Folder[] = []): Found | null {
	for (const [index, node] of nodes.entries()) {
		if (node.id === id) return { node, siblings: nodes, index, path };
		if (node.kind === 'folder') {
			const found = find(node.children, id, [...path, node]);
			if (found) return found;
		}
	}
	return null;
}

export const findNode = (tree: readonly TreeNode[], id: string) =>
	find(tree as TreeNode[], id)?.node ?? null;

/** Ids of the folders around a node, outermost first. */
export const ancestorIds = (tree: readonly TreeNode[], id: string) =>
	find(tree as TreeNode[], id)?.path.map((folder) => folder.id) ?? [];

/** Ids of all feeds in these nodes, in the order of the tree. */
export const feedIds = (nodes: readonly TreeNode[]): string[] =>
	nodes.flatMap((node) => (node.kind === 'feed' ? [node.id] : feedIds(node.children)));

/** Levels of folders in a node: 0 for a feed, 1 for a folder without folders in it. */
const folderLevels = (node: TreeNode): number =>
	node.kind === 'feed' ? 0 : 1 + Math.max(0, ...node.children.map(folderLevels));

/** Whether `id` may be dropped there: not into itself, and not deeper than the limit. */
export function canMove(
	tree: readonly TreeNode[],
	id: string,
	targetId: string,
	position: DropPosition
) {
	if (id === targetId) return false;
	const moved = find(tree as TreeNode[], id);
	const target = find(tree as TreeNode[], targetId);
	if (!moved || !target) return false;
	if (position === 'inside' && target.node.kind !== 'folder') return false;
	// The folders the node would be in afterwards.
	const around = position === 'inside' ? [...target.path, target.node as Folder] : target.path;
	if (around.some((folder) => folder.id === id)) return false;
	return around.length + folderLevels(moved.node) <= MAX_FOLDER_DEPTH;
}

/** The tree with `id` moved before, after or into `targetId`; the same tree when that is not allowed. */
export function moveNode(
	tree: readonly TreeNode[],
	id: string,
	targetId: string,
	position: DropPosition
): TreeNode[] {
	if (!canMove(tree, id, targetId, position)) return tree as TreeNode[];
	const next = clone(tree);
	const moved = find(next, id)!;
	moved.siblings.splice(moved.index, 1);
	// Looked up after the removal, as the target's index may have changed with it.
	const target = find(next, targetId)!;
	if (position === 'inside') (target.node as Folder).children.push(moved.node);
	else target.siblings.splice(target.index + (position === 'after' ? 1 : 0), 0, moved.node);
	return next;
}

/** The tree with a new, empty folder at its end. */
export const addFolder = (tree: readonly TreeNode[], id: string, title: string): TreeNode[] => [
	...clone(tree),
	{ kind: 'folder', id, title, children: [] }
];

/** The tree with a folder renamed. */
export function renameFolder(tree: readonly TreeNode[], id: string, title: string): TreeNode[] {
	const next = clone(tree);
	const found = find(next, id);
	if (found?.node.kind === 'folder') found.node.title = title;
	return next;
}

/** The tree with a folder taken out; what was in it moves to where the folder was. */
export function removeFolder(tree: readonly TreeNode[], id: string): TreeNode[] {
	const next = clone(tree);
	const found = find(next, id);
	if (found?.node.kind === 'folder') found.siblings.splice(found.index, 1, ...found.node.children);
	return next;
}

/** The tree with a new feed at the end of a folder, or of the top level for `null`. */
export function addFeed(
	tree: readonly TreeNode[],
	id: string,
	folderId: string | null
): TreeNode[] {
	const next = clone(tree);
	const folder = folderId === null ? null : find(next, folderId)?.node;
	const level = folder?.kind === 'folder' ? folder.children : next;
	level.push({ kind: 'feed', id });
	return next;
}

/** The tree without a feed. */
export function removeFeed(tree: readonly TreeNode[], id: string): TreeNode[] {
	const next = clone(tree);
	const found = find(next, id);
	if (found?.node.kind === 'feed') found.siblings.splice(found.index, 1);
	return next;
}

/** A folder as a list of all of them names it: how deep it lies, 0 at the top. */
export interface ListedFolder {
	id: string;
	title: string;
	depth: number;
}

/** All folders in the order of the tree, each before what is in it. */
export const folderList = (nodes: readonly TreeNode[], depth = 0): ListedFolder[] =>
	nodes.flatMap((node) =>
		node.kind === 'folder'
			? [{ id: node.id, title: node.title, depth }, ...folderList(node.children, depth + 1)]
			: []
	);

/** A step with the keyboard: up or down among its siblings, into the folder above it, or out of its folder. */
export type NudgeDirection = 'up' | 'down' | 'in' | 'out';

/** Where a keyboard step takes a node, as a drop; null when there is nowhere to go. */
export function nudge(
	tree: readonly TreeNode[],
	id: string,
	direction: NudgeDirection
): { targetId: string; position: DropPosition } | null {
	const found = find(tree as TreeNode[], id);
	if (!found) return null;
	const { siblings, index, path } = found;
	const parent = path.at(-1);
	const before = siblings[index - 1];
	const after = siblings[index + 1];
	switch (direction) {
		case 'up':
			if (before) return { targetId: before.id, position: 'before' };
			return parent ? { targetId: parent.id, position: 'before' } : null;
		case 'down':
			if (after) return { targetId: after.id, position: 'after' };
			return parent ? { targetId: parent.id, position: 'after' } : null;
		case 'in':
			return before?.kind === 'folder' ? { targetId: before.id, position: 'inside' } : null;
		case 'out':
			return parent ? { targetId: parent.id, position: 'after' } : null;
	}
}
