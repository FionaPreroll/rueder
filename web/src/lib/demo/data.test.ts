import { describe, expect, it } from 'vitest';
import { MAX_FOLDER_DEPTH, type TreeNode } from '#lib/model.ts';
import { feedIds } from '#lib/tree.ts';
import { demoData, random } from './data.ts';

const now = Date.parse('2026-11-10T14:30:00Z');

describe('random', () => {
	it('gives the same numbers for the same seed', () => {
		const a = random(7);
		const b = random(7);
		const first = [a(), a(), a()];
		expect([b(), b(), b()]).toEqual(first);
		expect(first.every((value) => value >= 0 && value < 1)).toBe(true);
		expect(random(8)()).not.toBe(first[0]);
	});
});

describe('demoData', () => {
	const data = demoData(now);

	it('is the same every time', () => {
		expect(demoData(now).articles).toEqual(data.articles);
	});

	it('has every feed in the tree once, with its articles', () => {
		const inTree = feedIds(data.tree);
		expect([...inTree].sort()).toEqual(Object.keys(data.feeds).sort());
		expect(new Set(inTree).size).toBe(inTree.length);
		for (const feed of Object.values(data.feeds)) {
			const articles = data.articles[feed.id];
			expect(articles.map((article) => article.seq)).toEqual(
				Array.from({ length: feed.lastSeq }, (_, index) => index + 1)
			);
			expect(articles.every((article) => article.feedId === feed.id)).toBe(true);
		}
	});

	it('numbers articles as they came: none from the future, the oldest first', () => {
		for (const articles of Object.values(data.articles)) {
			const times = articles.map((article) => article.publishedAt);
			expect(times).toEqual([...times].sort((a, b) => a - b));
			expect(times.at(-1)).toBeLessThanOrEqual(now);
		}
	});

	it('uses made-up addresses only', () => {
		for (const feed of Object.values(data.feeds)) {
			expect(new URL(feed.siteUrl).hostname).toMatch(/\.example$/);
		}
		for (const article of Object.values(data.articles).flat()) {
			expect(new URL(article.link).hostname).toMatch(/\.example$/);
			expect(article.html).not.toMatch(/https?:\/\/(?![\w.-]+\.example\b)/);
		}
	});

	it('nests its folders within the limit', () => {
		const depth = (nodes: readonly TreeNode[]): number =>
			Math.max(0, ...nodes.map((node) => (node.kind === 'folder' ? 1 + depth(node.children) : 0)));
		expect(depth(data.tree)).toBeGreaterThan(1);
		expect(depth(data.tree)).toBeLessThanOrEqual(MAX_FOLDER_DEPTH);
	});

	it('has articles with earlier versions, advertisements and a failing feed', () => {
		const all = Object.values(data.articles).flat();
		const revised = all.filter((article) => article.versions.length > 0);
		expect(revised.length).toBeGreaterThan(5);
		expect(revised.every((article) => article.updatedAt !== null)).toBe(true);
		expect(all.some((article) => /^(Anzeige|Sponsored): /.test(article.title))).toBe(true);
		expect(Object.values(data.feeds).filter((feed) => feed.error)).toHaveLength(1);
	});

	it('makes the next article of a feed', () => {
		const feed = data.feeds['feed-tagesbote'];
		const next = data.nextArticle(feed.id, feed.lastSeq + 1, now);
		expect(next).toMatchObject({ feedId: feed.id, seq: feed.lastSeq + 1, publishedAt: now });
		expect(data.articles[feed.id].some((article) => article.id === next.id)).toBe(false);
	});
});
