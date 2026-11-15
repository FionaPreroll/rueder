// The feed every reader of the live load test reads (work package 5.30): it
// serves twelve articles at http://127.0.0.1:<SHARED_PORT>/feed.xml and one more
// with every POST /publish. Start it before the server, so that its first
// fetch finds it.
//
//   SHARED_PORT=3299 node load/feed.js

import http from 'node:http';

const port = Number(process.env.SHARED_PORT ?? 3299);
let articles = 12;

http
	.createServer((request, response) => {
		if (request.method === 'POST' && request.url === '/publish') {
			articles++;
			response.writeHead(204).end();
			return;
		}
		const items = Array.from({ length: articles }, (_, i) => {
			const n = i + 1;
			return `<item><guid>shared-${n}</guid><title>Shared ${n}</title>
<link>http://127.0.0.1:${port}/${n}</link><description>Article ${n}.</description></item>`;
		}).join('\n');
		response.writeHead(200, { 'Content-Type': 'application/rss+xml' });
		response.end(`<?xml version="1.0"?><rss version="2.0"><channel><title>Shared</title>
<link>http://127.0.0.1:${port}/</link>${items}</channel></rss>`);
	})
	.listen(port, '127.0.0.1', () =>
		console.log(`the shared feed is at http://127.0.0.1:${port}/feed.xml`)
	);
