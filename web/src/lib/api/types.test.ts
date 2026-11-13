// The generated types hold what the server sends: the answers below are the
// ones the server's own tests expect (server/tests/), held against the types.
// svelte-check fails on this file when a type and its answer part ways.
import { describe, expect, it } from 'vitest';
import type { Article, List, Rule, ServerEvent, Tree } from './types';

/** What a tab does on an event; every event has to be named here. */
function reaction(event: ServerEvent): string {
	switch (event.event) {
		case 'hello':
			return 'load';
		case 'resync':
			return 'load everything anew';
		case 'articles':
			return `${event.data.new} new in feed ${event.data.feed_id}`;
		case 'updated':
			return `${event.data.count} changed in feed ${event.data.feed_id}`;
		case 'feed':
			return event.data.error ?? 'works again';
		case 'reading':
			return event.data.feed ? `${event.data.feed.unread} unread` : 'counts anew';
		case 'tree':
		case 'labels':
		case 'settings':
		case 'rules':
		case 'session':
		case 'apps':
			return `${event.event} by ${event.data.by ?? 'somebody else'}`;
		default: {
			const unknown: never = event;
			return unknown;
		}
	}
}

describe('the types of the API', () => {
	it('tell the events apart by their name', () => {
		const events: ServerEvent[] = [
			{ event: 'hello', data: {} },
			{ event: 'articles', data: { feed_id: 7, last_seq: 12, new: 2 } },
			{ event: 'feed', data: { feed_id: 7, error: null } },
			{
				event: 'reading',
				data: { feed: { feed_id: 7, last_seq: 12, read_upto: 10, unread: 2 }, seq: 11, by: 'tab-1' }
			},
			{ event: 'tree', data: { by: null } }
		];
		expect(events.map(reaction)).toEqual([
			'load',
			'2 new in feed 7',
			'works again',
			'2 unread',
			'tree by somebody else'
		]);
	});

	it('hold the answers of the server', () => {
		const tree: Tree = {
			hidden: 2,
			folders: [{ id: 1, parent_id: null, title: 'News', position: 1 }],
			feeds: [
				{
					id: 7,
					folder_id: 1,
					position: 1,
					title: 'Blog',
					feed_title: 'A blog',
					feed_url: 'https://blog.example/feed.xml',
					site_url: null,
					icon_url: null,
					fetch_agent: 'rueder',
					fetched: true,
					blocked: false,
					error: null,
					last_seq: 12,
					read_upto: 10,
					unread: 2
				}
			]
		};
		const list: List = {
			articles: [
				{
					feed_id: 7,
					seq: 12,
					id: 4711,
					published_at: '2026-03-01T12:12:00Z',
					updated_at: null,
					updated_what: null,
					version: 1,
					title: 'Article 12',
					teaser: 'Text of article 12.',
					author: 'Ada',
					url: 'https://blog.example/12',
					thumbnail_url: null,
					read: false,
					starred: true,
					labels: [3]
				}
			]
		};
		const article: Article = {
			feed_id: 7,
			seq: 12,
			id: 4711,
			published_at: '2026-03-01T12:12:00Z',
			updated_at: '2026-03-02T08:00:00Z',
			updated_what: 'both',
			version: 2,
			title: 'Article 12',
			author: 'Ada',
			url: null,
			html: '<p>Text of article 12.</p>',
			tags: ['news'],
			image_url: null,
			image_title: null,
			enclosures: [
				{ url: 'https://a.example/1.mp3', type: 'audio/mpeg', length: 123, inline: false }
			],
			labels: [],
			hidden_by: null
		};
		const rule: Rule = {
			id: 1,
			enabled: true,
			scope: { kind: 'folder', id: 1 },
			field: 'title',
			match: 'word',
			pattern: 'advert',
			action: 'hide'
		};
		expect(tree.feeds[0].folder_id).toBe(tree.folders[0].id);
		expect(list.articles[0].seq).toBe(article.seq);
		expect(rule.scope.kind === 'folder' && rule.scope.id).toBe(1);
	});
});
