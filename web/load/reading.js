// Load of reading (work package 5.30): workers that visit random readers
// again and again, a visit being what a reader does on the page: the tree with
// its unread counts, the first page of unread articles of all feeds, three of
// them opened and marked read, and a page of one feed. Says requests a second
// and the times by kind, for each number of workers at once.
//
//   RUEDER_PID=<pid> TOKENS=<file> node load/reading.js [workers, e.g. 8,32,64] [seconds]
//
// See "Measuring" in server/README.md for the whole run.

import { call, cpu, now, percentile, postgresCpu, readers, round, serverPid } from './common.js';

const [workersArg = '8,32,64', secondsArg = '30'] = process.argv.slice(2);
const pid = serverPid();
const all = readers();

const times = {};
const statuses = {};
function note(kind, result) {
	(times[kind] ??= []).push(result.ms);
	statuses[result.status] = (statuses[result.status] ?? 0) + 1;
	return result;
}

async function visit() {
	const reader = all[Math.floor(Math.random() * all.length)];
	const tree = note('tree', await call('GET', '/api/tree', reader));
	const unread = note(
		'unread page',
		await call('GET', '/api/articles?limit=50&unread=true', reader)
	);
	for (const item of (unread.json?.articles ?? []).slice(0, 3)) {
		const path = `/api/feeds/${item.feed_id}/articles/${item.seq}`;
		note('article', await call('GET', path, reader));
		note('mark read', await call('PUT', `${path}/state`, reader, { read: true }));
	}
	const feeds = tree.json?.feeds ?? [];
	if (feeds.length > 0) {
		const feed = feeds[Math.floor(Math.random() * feeds.length)];
		note('feed page', await call('GET', `/api/feeds/${feed.id}/articles?limit=50`, reader));
	}
}

for (const workers of workersArg.split(',').map(Number)) {
	for (const key of Object.keys(times)) delete times[key];
	for (const key of Object.keys(statuses)) delete statuses[key];
	const until = now() + Number(secondsArg) * 1000;
	const [serverBefore, postgresBefore, started] = [cpu(pid), postgresCpu(), now()];
	let visits = 0;
	await Promise.all(
		Array.from({ length: workers }, async () => {
			while (now() < until) {
				await visit();
				visits++;
			}
		})
	);
	const seconds = (now() - started) / 1000;
	const requests = Object.values(times).reduce((n, list) => n + list.length, 0);
	console.log(
		JSON.stringify({
			workers,
			seconds: round(seconds),
			visitsPerSecond: round(visits / seconds),
			requestsPerSecond: round(requests / seconds),
			serverCores: round((cpu(pid) - serverBefore) / seconds),
			postgresCores: round((postgresCpu() - postgresBefore) / seconds),
			statuses,
			milliseconds: Object.fromEntries(
				Object.entries(times).map(([kind, list]) => [
					kind,
					{
						p50: round(percentile(list, 50)),
						p95: round(percentile(list, 95)),
						p99: round(percentile(list, 99))
					}
				])
			)
		})
	);
}
process.exit(0);
