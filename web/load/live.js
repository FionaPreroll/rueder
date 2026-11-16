// Load of the live updates (work package 5.30): every reader holds a stream
// of /api/events open; then a new article arrives in the feed all of them
// read (the fan-out); then fifty readers change something at a rate, each
// change an event for its own reader alone. Says memory per stream, how fast
// the fan-out reaches everybody, and what the changes cost.
//
//   DATABASE_URL=<url> RUEDER_PID=<pid> TOKENS=<file> SHARED_PORT=3299 node load/live.js [rates, e.g. 50,200,800]
//
// The feed all readers read comes from load/feed.js on SHARED_PORT and must
// be the one `rueder bench readers --shared` named:
// http://127.0.0.1:<SHARED_PORT>/feed.xml. The server fetches it, so it runs
// with FETCH_ALLOW_NETS=127.0.0.0/8 and FETCH_EVERY=5. See "Measuring" in
// server/README.md for the whole run.

import http from 'node:http';
import { execFileSync } from 'node:child_process';
import {
	RUEDER,
	call,
	cpu,
	headers,
	memory,
	now,
	percentile,
	readers,
	round,
	serverPid,
	sleep
} from './common.js';

const [ratesArg = '50,200,800'] = process.argv.slice(2);
/** The database of the server under load, for making the shared feed due. */
const database = () => {
	if (!process.env.DATABASE_URL) throw new Error('DATABASE_URL names the database of the server');
	return process.env.DATABASE_URL;
};
const pid = serverPid();
const all = readers();
const port = Number(process.env.SHARED_PORT ?? 3299);
const log = (what) => console.log(JSON.stringify(what));

// The shared feed comes from load/feed.js; the server's first fetch brings
// its twelve articles before the streams open.
const feedAt = `http://127.0.0.1:${port}/feed.xml`;
let feedId;
for (let i = 0; i < 120 && !feedId; i++) {
	const tree = await call('GET', '/api/tree', all[0]);
	const shared = tree.json?.feeds?.find((f) => f.feed_url === feedAt);
	if (!shared) throw new Error('the readers do not read the shared feed; see --shared');
	if (shared.fetched && !shared.error) feedId = shared.id;
	else await sleep(1000);
}
if (!feedId) throw new Error('the server did not fetch the shared feed; is load/feed.js running?');

const streams = [];
function open(reader) {
	return new Promise((resolve) => {
		const s = { reader, hello: false, events: 0, resyncs: 0, articleAt: null, buffer: '' };
		streams.push(s);
		const request = http.get(
			{
				host: RUEDER.hostname,
				port: RUEDER.port,
				path: '/api/events',
				headers: headers(reader, { Accept: 'text/event-stream' })
			},
			(response) => {
				if (response.statusCode !== 200) {
					s.failed = response.statusCode;
					resolve();
					return;
				}
				response.setEncoding('utf8');
				response.on('data', (chunk) => {
					s.buffer += chunk;
					let end;
					while ((end = s.buffer.indexOf('\n\n')) !== -1) {
						const kind = /^event: (.*)$/m.exec(s.buffer.slice(0, end))?.[1];
						s.buffer = s.buffer.slice(end + 2);
						if (!kind) continue;
						s.events++;
						if (kind === 'hello' && !s.hello) {
							s.hello = true;
							resolve();
						}
						if (kind === 'resync') s.resyncs++;
						if (kind === 'articles' && s.articleAt === null) s.articleAt = now();
					}
				});
			}
		);
		request.on('error', () => {
			s.failed = 'error';
			resolve();
		});
	});
}

const before = memory(pid);
const opening = now();
// In batches, as browsers come one after the other.
for (let first = 0; first < all.length; first += 200) {
	await Promise.all(all.slice(first, first + 200).map(open));
}
const opened = streams.filter((s) => s.hello).length;
await sleep(5000);
log({
	phase: 'open',
	streams: all.length,
	opened,
	failed: [...new Set(streams.filter((s) => s.failed).map((s) => s.failed))],
	seconds: round((now() - opening) / 1000),
	serverMb: round(memory(pid)),
	kbPerStream: round(((memory(pid) - before) * 1024) / Math.max(1, opened))
});

// Fan-out: one more article in the feed everybody reads; the server finds it
// within FETCH_EVERY.
await fetch(`http://127.0.0.1:${port}/publish`, { method: 'POST' });
const published = now();
// Nothing a reader can do makes the server fetch, so the test says it to the
// database, as the end-to-end tests do.
execFileSync('psql', [
	database(),
	'--quiet',
	'-c',
	`UPDATE feeds SET next_fetch_at = now() WHERE feed_url = '${feedAt}'`
]);
const cpuBefore = cpu(pid);
for (let i = 0; i < 240 && streams.filter((s) => s.articleAt !== null).length < opened; i++) {
	await sleep(250);
}
const arrivals = streams.filter((s) => s.articleAt !== null).map((s) => s.articleAt);
const first = Math.min(...arrivals);
log({
	phase: 'fan-out',
	reached: arrivals.length,
	of: opened,
	firstAfterSeconds: round((first - published) / 1000),
	spreadMilliseconds: {
		p50: round(
			percentile(
				arrivals.map((at) => at - first),
				50
			)
		),
		p99: round(
			percentile(
				arrivals.map((at) => at - first),
				99
			)
		),
		max: round(Math.max(...arrivals) - first)
	},
	serverCpuSeconds: round(cpu(pid) - cpuBefore)
});

// Changes at a rate: fifty readers mark an article of the shared feed read and
// unread again by turns, an event for each of them alone.
const writers = all.slice(0, 50);
for (const rate of ratesArg.split(',').map(Number)) {
	const resyncs = streams.reduce((n, s) => n + s.resyncs, 0);
	const events = streams.reduce((n, s) => n + s.events, 0);
	const cpuStart = cpu(pid);
	const seconds = 20;
	const results = [];
	const pending = [];
	const started = now();
	for (let i = 0; i < rate * seconds; i++) {
		const wait = started + (i * 1000) / rate - now();
		if (wait > 1) await sleep(wait);
		const writer = writers[i % writers.length];
		const seq = 1 + (Math.floor(i / writers.length) % 12);
		const read = Math.floor(i / (writers.length * 12)) % 2 === 0;
		const path = `/api/feeds/${feedId}/articles/${seq}/state`;
		pending.push(call('PUT', path, writer, { read }).then((result) => results.push(result)));
	}
	await Promise.all(pending);
	const elapsed = (now() - started) / 1000;
	await sleep(2000);
	const ms = results.map((r) => r.ms);
	log({
		phase: 'changes',
		ratePerSecond: rate,
		sentPerSecond: round(results.length / elapsed),
		statuses: [...new Set(results.map((r) => r.status))],
		milliseconds: { p50: round(percentile(ms, 50)), p99: round(percentile(ms, 99)) },
		serverCores: round((cpu(pid) - cpuStart) / (elapsed + 2)),
		resyncs: streams.reduce((n, s) => n + s.resyncs, 0) - resyncs,
		eventsToStreams: streams.reduce((n, s) => n + s.events, 0) - events,
		serverMb: round(memory(pid))
	});
}
process.exit(0);
