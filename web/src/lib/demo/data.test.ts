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

describe('the English corpus', () => {
	const de = demoData(now);
	const en = demoData(now, 'en');
	/** Every text a reader sees of a corpus: names, headlines, teasers, authors, tags, labels. */
	const texts = (data: typeof de) => {
		const folders: string[] = [];
		const walk = (nodes: TreeNode[]) =>
			nodes.forEach((node) => {
				if (node.kind === 'folder') {
					folders.push(node.title);
					walk(node.children);
				}
			});
		walk(data.tree);
		return [
			...folders,
			...Object.values(data.feeds).map((feed) => feed.title),
			...Object.values(data.articles)
				.flat()
				.flatMap((article) => [
					article.title,
					article.teaser ?? '',
					article.author ?? '',
					...article.tags
				]),
			...data.labels.map((label) => label.title)
		];
	};

	it('has the same feeds, folders and articles as the German one', () => {
		expect(Object.keys(en.feeds)).toEqual(Object.keys(de.feeds));
		for (const id of Object.keys(de.feeds)) {
			const { title: _de, siteUrl: _deSite, ...rest } = de.feeds[id];
			const { title: _en, siteUrl: _enSite, ...same } = en.feeds[id];
			expect(same, id).toEqual(rest);
			expect(en.articles[id].map((article) => article.seq)).toEqual(
				de.articles[id].map((article) => article.seq)
			);
		}
		const ids = (nodes: TreeNode[]): unknown[] =>
			nodes.map((node) => (node.kind === 'folder' ? [node.id, ids(node.children)] : node.id));
		expect(ids(en.tree)).toEqual(ids(de.tree));
		expect(en.pace).toEqual(de.pace);
		expect(en.labels.map((label) => label.id)).toEqual(de.labels.map((label) => label.id));
		expect(en.rules.map((rule) => rule.id)).toEqual(de.rules.map((rule) => rule.id));
	});

	it('is all in English', () => {
		const german = /[äöüßÄÖÜ]|\b(der|die|das|und|nicht|mit|für|Anzeige|Redaktion|Nachrichten)\b/;
		const found = texts(en).filter((text) => german.test(text));
		expect(found).toEqual([]);
		// The German corpus stays as it was.
		expect(de.feeds['feed-tagesbote'].title).toBe('Tagesbote');
		expect(texts(de).some((text) => german.test(text))).toBe(true);
	});

	it('has rules that still catch what they are for', () => {
		const titles = Object.values(en.articles)
			.flat()
			.map((article) => article.title);
		const [hide, sponsor, news] = en.rules;
		expect(
			titles.filter((title) => title.split(/\W+/).includes(hide.pattern)).length
		).toBeGreaterThan(0);
		expect(
			Object.values(en.articles)
				.flat()
				.some((article) => article.author?.includes(sponsor.pattern))
		).toBe(true);
		const readOnSight = new RegExp(news.pattern);
		expect(en.articles['feed-tagesbote'].some((article) => readOnSight.test(article.title))).toBe(
			true
		);
	});
});
