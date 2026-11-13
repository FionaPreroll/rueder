// A made-up status of a server, for the status page without one: as a small
// installation might look after a day, with a feed search that is slow now and
// then and a few feeds that fail.

import type { Durations, ServerStatus } from '#lib/api/types.ts';

const took = (count: number, p50: number, p95: number, max: number): Durations => ({
	count,
	mean_ms: count ? (p50 * 3 + p95) / 4 : null,
	p50_ms: count ? p50 : null,
	p95_ms: count ? p95 : null,
	max_ms: count ? max : null
});

/** The status as it would be at `now` (milliseconds since 1970). */
export function demoStatus(now: number): ServerStatus {
	const ago = (seconds: number) => new Date(now - seconds * 1000).toISOString();
	const uptime = 26 * 60 * 60 + 17 * 60;
	return {
		version: '0.1.0',
		build: {
			version: '0.1.0',
			commit: '9f1c2d4e5a6b7c8d9e0f1a2b3c4d5e6f70819293',
			branch: 'main',
			tag: 'v0.1.0',
			dirty: false,
			built_at: new Date(now - 3 * 24 * 60 * 60 * 1000).toISOString().replace(/\.\d+Z$/, 'Z'),
			target: 'x86_64-unknown-linux-gnu',
			profile: 'release',
			opt_level: '3',
			ci_run: 'https://github.com/FionaPreroll/rueder/actions/runs/1234567890',
			container: true
		},
		started_at: ago(uptime),
		uptime_s: uptime,
		memory_bytes: 74 * 1024 * 1024,
		hints: [
			{ kind: 'slow_route', route: 'POST /api/feeds/find', p95_ms: 2380 },
			{ kind: 'feeds_failing', failing: 7, subscribed: 31 },
			{ kind: 'errors', count: 1 },
			{ kind: 'browser_errors', errors: 1 },
			{ kind: 'reported_feeds', feeds: 1 }
		],
		counts: {
			accounts: 3,
			feeds: 34,
			subscribed_feeds: 31,
			articles: 48_212,
			live_connections: 2
		},
		requests: {
			all: took(1843, 3.2, 41, 2900),
			answers: [0, 1790, 12, 39, 2],
			limited: 0,
			routes: [
				{ route: 'POST /api/feeds/find', took: took(6, 820, 2380, 2900), errors: 0 },
				{ route: 'POST /api/feeds', took: took(2, 640, 1150, 1150), errors: 0 },
				{ route: 'GET /api/feeds/{id}/articles', took: took(212, 6.4, 18, 52), errors: 0 },
				{ route: 'GET /api/articles', took: took(96, 7.1, 16, 31), errors: 0 },
				{ route: 'GET /api/tree', took: took(118, 4.3, 12, 27), errors: 0 },
				{ route: 'GET /api/feeds/{id}/articles/{seq}', took: took(371, 2.6, 8.1, 19), errors: 2 },
				{
					route: 'PUT /api/feeds/{id}/articles/{seq}/state',
					took: took(402, 1.9, 5.2, 14),
					errors: 0
				},
				{
					route: 'GET /img/{signature}/{size}/{address}',
					took: took(588, 0.4, 3.3, 640),
					errors: 0
				}
			]
		},
		database: {
			pool_size: 4,
			pool_idle: 3,
			pool_max: 10,
			connections: 6,
			size_bytes: 412 * 1024 * 1024,
			cache_hit_ratio: 0.9987,
			deadlocks: 0,
			longest_query_s: 0.004,
			tables: [
				{ name: 'article_texts', bytes: 251 * 1024 * 1024, rows: 48_212 },
				{ name: 'article_versions', bytes: 118 * 1024 * 1024, rows: 61_530 },
				{ name: 'articles', bytes: 29 * 1024 * 1024, rows: 48_212 },
				{ name: 'read_exceptions', bytes: 2 * 1024 * 1024, rows: 3_411 },
				{ name: 'feeds', bytes: 160 * 1024, rows: 34 }
			]
		},
		fetcher: {
			running: true,
			pace: { round: 100, at_once: 8, every_s: 30, per_hour: 12_000 },
			per_hour: 84,
			behind_s: 0,
			last_round_at: ago(14),
			last_round_feeds: 3,
			last_round_s: 1.8,
			took: took(84, 410, 1900, 6100),
			fetched: 22,
			unchanged: 55,
			failed: 7,
			broken: 0,
			new_articles: 137,
			failing: [
				{ failure: 'status', status: 403, feeds: 3 },
				{ failure: 'timeout', status: null, feeds: 2 },
				{ failure: 'unreadable', status: null, feeds: 1 },
				{ failure: 'dns', status: null, feeds: 1 }
			],
			overdue: 0,
			most_overdue_s: null,
			waiting: 1
		},
		finder: {
			on: true,
			took: took(6, 820, 2380, 2900),
			found: 4,
			none: 1,
			failed: 1,
			limited: 0
		},
		images: {
			on: true,
			cache_bytes: 183 * 1024 * 1024,
			cache_max_bytes: 1024 * 1024 * 1024,
			failing_sources: 4,
			took: took(141, 180, 720, 2400),
			cached: 431,
			fetched: 137,
			not_found: 0,
			source_failed: 4,
			not_image: 1,
			too_large: 0
		},
		problems: {
			errors_last_hour: 1,
			warnings_last_hour: 2,
			recent: [
				{
					error: true,
					target: 'rueder',
					text: 'error returned from database: canceling statement due to statement timeout',
					count: 1,
					first: ago(1260),
					last: ago(1260)
				},
				{
					error: false,
					target: 'rueder::oidc',
					text: 'OpenID Connect: the provider did not answer',
					count: 2,
					first: ago(5400),
					last: ago(2100)
				}
			]
		},
		browsers: {
			available: true,
			consenting: 2,
			errors: [
				{
					kind: 'error',
					message: "TypeError: Cannot read properties of undefined (reading 'feedId')",
					stack: '_app/immutable/chunks/reader.js:812:31\n_app/immutable/nodes/2.js:44:9',
					count: 3,
					accounts: 2,
					versions: ['1762899600000'],
					browsers: ['Chrome 141', 'Firefox 140'],
					first: ago(7 * 3600),
					last: ago(2 * 3600)
				},
				{
					kind: 'request',
					message: 'GET /feeds/<n>/articles: 502',
					stack: '',
					count: 1,
					accounts: 1,
					versions: ['1762899600000'],
					browsers: ['Safari 18'],
					first: ago(3 * 86_400),
					last: ago(3 * 86_400)
				}
			]
		},
		reported: [
			{
				feed_id: 31,
				url: 'https://deals.example/feed.xml',
				title: 'Daily Deals',
				reports: 3,
				reasons: [
					{ reason: 'spam', reports: 2 },
					{ reason: 'malicious', reports: 1 }
				],
				last_at: ago(5 * 3600),
				blocked: false
			}
		]
	};
}
