import { describe, expect, it } from 'vitest';
import { MAX_FOLDER_DEPTH, type TreeNode } from '#lib/model.ts';
import {
	addFeed,
	addFolder,
	ancestorIds,
	canGroup,
	canMove,
	feedIds,
	findNode,
	folderList,
	groupFeeds,
	moveNode,
	nudge,
	removeFeed,
	removeFolder,
	renameFolder
} from '#lib/tree.ts';

const feed = (id: string): TreeNode => ({ kind: 'feed', id });
const folder = (id: string, ...children: TreeNode[]): TreeNode => ({
	kind: 'folder',
	id,
	title: id.toUpperCase(),
	children
});

/** The tree as text: "tech[rust[a b] c] d". */
const shape = (nodes: readonly TreeNode[]): string =>
	nodes
		.map((node) => (node.kind === 'feed' ? node.id : `${node.id}[${shape(node.children)}]`))
		.join(' ');

const tree = [folder('tech', folder('rust', feed('a'), feed('b')), feed('c')), feed('d')];

/** Folders inside each other, `levels` deep, with one feed at the bottom. */
const nested = (levels: number, name = 'n'): TreeNode =>
	levels === 0 ? feed(`${name}-feed`) : folder(`${name}${levels}`, nested(levels - 1, name));

describe('looking things up', () => {
	it('finds nodes and what is around them', () => {
		expect(findNode(tree, 'rust')?.kind).toBe('folder');
		expect(findNode(tree, 'nope')).toBeNull();
		expect(ancestorIds(tree, 'a')).toEqual(['tech', 'rust']);
		expect(ancestorIds(tree, 'd')).toEqual([]);
		expect(feedIds(tree)).toEqual(['a', 'b', 'c', 'd']);
	});
});

describe('moving', () => {
	it('drops before, after and into', () => {
		expect(shape(moveNode(tree, 'd', 'a', 'before'))).toBe('tech[rust[d a b] c]');
		expect(shape(moveNode(tree, 'a', 'd', 'after'))).toBe('tech[rust[b] c] d a');
		expect(shape(moveNode(tree, 'c', 'rust', 'inside'))).toBe('tech[rust[a b c]] d');
		expect(shape(moveNode(tree, 'rust', 'tech', 'before'))).toBe('rust[a b] tech[c] d');
	});

	it('sorts among siblings', () => {
		expect(shape(moveNode(tree, 'b', 'a', 'before'))).toBe('tech[rust[b a] c] d');
		expect(shape(moveNode(tree, 'a', 'b', 'after'))).toBe('tech[rust[b a] c] d');
	});

	it('leaves the tree it was given alone', () => {
		const before = shape(tree);
		moveNode(tree, 'd', 'rust', 'inside');
		expect(shape(tree)).toBe(before);
	});

	it('refuses what makes no sense, and returns the same tree', () => {
		expect(canMove(tree, 'a', 'a', 'before')).toBe(false);
		expect(canMove(tree, 'a', 'b', 'inside')).toBe(false);
		expect(canMove(tree, 'tech', 'rust', 'inside')).toBe(false);
		expect(canMove(tree, 'tech', 'a', 'after')).toBe(false);
		expect(canMove(tree, 'nope', 'a', 'after')).toBe(false);
		expect(moveNode(tree, 'tech', 'rust', 'inside')).toBe(tree);
	});

	it('nests folders only as deep as the limit', () => {
		const deep = [nested(MAX_FOLDER_DEPTH - 1), folder('lone'), feed('x')];
		const innermost = 'n1';
		// A feed fits into the deepest folder, a folder beside it too, but not inside it.
		expect(canMove(deep, 'x', innermost, 'inside')).toBe(true);
		expect(canMove(deep, 'lone', innermost, 'after')).toBe(true);
		expect(canMove(deep, 'lone', innermost, 'inside')).toBe(true);
		const full = [nested(MAX_FOLDER_DEPTH), folder('lone'), feed('x')];
		expect(canMove(full, 'lone', 'n1', 'inside')).toBe(false);
		expect(canMove(full, 'x', 'n1', 'inside')).toBe(true);
		// A folder with folders in it counts with all its levels.
		const two = [nested(MAX_FOLDER_DEPTH - 1), nested(2, 'm')];
		expect(canMove(two, 'm2', 'n1', 'inside')).toBe(false);
		expect(canMove(two, 'm2', 'n2', 'inside')).toBe(true);
		expect(canMove(two, 'm1', 'n1', 'inside')).toBe(true);
	});
});

describe('folders', () => {
	it('adds, renames and removes', () => {
		const added = addFolder(tree, 'new', 'New');
		expect(shape(added)).toBe('tech[rust[a b] c] d new[]');
		const renamed = renameFolder(added, 'new', 'Later');
		expect(findNode(renamed, 'new')).toMatchObject({ title: 'Later' });
		expect(findNode(added, 'new')).toMatchObject({ title: 'New' });
	});

	it('keeps what was in a removed folder, where the folder was', () => {
		expect(shape(removeFolder(tree, 'rust'))).toBe('tech[a b c] d');
		expect(shape(removeFolder(tree, 'tech'))).toBe('rust[a b] c d');
		expect(shape(removeFolder(tree, 'a'))).toBe(shape(tree));
	});
});

describe('feeds', () => {
	it('are added at the end of a folder or of the top level', () => {
		expect(shape(addFeed(tree, 'e', 'rust'))).toBe('tech[rust[a b e] c] d');
		expect(shape(addFeed(tree, 'e', null))).toBe('tech[rust[a b] c] d e');
		// A folder that is gone is the top level, and a feed is no folder.
		expect(shape(addFeed(tree, 'e', 'gone'))).toBe('tech[rust[a b] c] d e');
		expect(shape(addFeed(tree, 'e', 'a'))).toBe('tech[rust[a b] c] d e');
		expect(shape(tree)).toBe('tech[rust[a b] c] d');
	});

	it('are removed, and nothing else is', () => {
		expect(shape(removeFeed(tree, 'b'))).toBe('tech[rust[a] c] d');
		expect(shape(removeFeed(tree, 'd'))).toBe('tech[rust[a b] c]');
		expect(shape(removeFeed(tree, 'rust'))).toBe(shape(tree));
		expect(shape(removeFeed(tree, 'gone'))).toBe(shape(tree));
	});

	it('go into folders that are listed in the order of the tree', () => {
		expect(folderList(tree)).toEqual([
			{ id: 'tech', title: 'TECH', depth: 0 },
			{ id: 'rust', title: 'RUST', depth: 1 }
		]);
		expect(folderList([nested(3), folder('x')]).map((f) => `${f.id}:${f.depth}`)).toEqual([
			'n3:0',
			'n2:1',
			'n1:2',
			'x:0'
		]);
		expect(folderList([feed('a')])).toEqual([]);
	});
});

describe('putting two feeds into a new folder', () => {
	it('puts the new folder where the feed dropped on was', () => {
		expect(shape(groupFeeds(tree, 'd', 'a', 'new', 'New'))).toBe('tech[rust[new[a d] b] c]');
		expect(shape(groupFeeds(tree, 'a', 'd', 'new', 'New'))).toBe('tech[rust[b] c] new[d a]');
		expect(shape(groupFeeds(tree, 'b', 'a', 'new', 'New'))).toBe('tech[rust[new[a b]] c] d');
		expect(findNode(groupFeeds(tree, 'a', 'd', 'new', 'New'), 'new')).toMatchObject({
			title: 'New'
		});
		expect(shape(tree)).toBe('tech[rust[a b] c] d');
	});

	it('only with two feeds, and not deeper than folders go', () => {
		for (const [id, target] of [
			['a', 'a'],
			['rust', 'd'],
			['d', 'rust'],
			['gone', 'd'],
			['d', 'gone']
		]) {
			expect(canGroup(tree, id, target)).toBe(false);
			expect(groupFeeds(tree, id, target, 'new', 'New')).toBe(tree);
		}
		// A feed at the bottom of the deepest folders has no room for one more.
		const deep = [nested(MAX_FOLDER_DEPTH), feed('x')];
		expect(canGroup(deep, 'x', 'n-feed')).toBe(false);
		const shallower = [nested(MAX_FOLDER_DEPTH - 1), feed('x')];
		expect(canGroup(shallower, 'x', 'n-feed')).toBe(true);
	});
});

describe('steps with the keyboard', () => {
	it('goes up and down among siblings, and out at the ends', () => {
		expect(nudge(tree, 'b', 'up')).toEqual({ targetId: 'a', position: 'before' });
		expect(nudge(tree, 'a', 'down')).toEqual({ targetId: 'b', position: 'after' });
		expect(nudge(tree, 'a', 'up')).toEqual({ targetId: 'rust', position: 'before' });
		expect(nudge(tree, 'b', 'down')).toEqual({ targetId: 'rust', position: 'after' });
		expect(nudge(tree, 'tech', 'up')).toBeNull();
		expect(nudge(tree, 'd', 'down')).toBeNull();
	});

	it('goes into the folder above and out of its own', () => {
		expect(nudge(tree, 'c', 'in')).toEqual({ targetId: 'rust', position: 'inside' });
		expect(nudge(tree, 'd', 'in')).toEqual({ targetId: 'tech', position: 'inside' });
		expect(nudge(tree, 'b', 'in')).toBeNull();
		expect(nudge(tree, 'a', 'out')).toEqual({ targetId: 'rust', position: 'after' });
		expect(nudge(tree, 'd', 'out')).toBeNull();
		expect(nudge(tree, 'nope', 'up')).toBeNull();
	});
});
