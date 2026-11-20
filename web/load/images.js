// Load of the image proxy (work package 5.36). A made-up site serves pictures
// under HOSTS names out of 127.0.0.0/8, which on Linux all lead to this
// machine. First every address is asked for once, so that the proxy fetches
// the picture, makes it smaller where its size asks for it and keeps it; then
// workers ask for random addresses it keeps, as random readers. Says pictures
// a second, the times by size and what the server and PostgreSQL took, for the
// first pass and for each number of workers at once.
//
//   RUEDER_PID=<pid> TOKENS=<file> IMAGE_PROXY_KEY=<key> node load/images.js [workers, e.g. 8,32,64] [seconds] [pictures]
//
// The server runs with the same made-up IMAGE_PROXY_KEY, so that this makes
// the addresses as `images::Signer::address` does, and with
// FETCH_ALLOW_NETS=127.0.0.0/8 for the site. See "Measuring" in
// server/README.md for the whole run.

import { createHmac } from 'node:crypto';
import http from 'node:http';
import { deflateSync } from 'node:zlib';
import {
	RUEDER,
	agent,
	cpu,
	headers,
	now,
	percentile,
	postgresCpu,
	readers,
	round,
	serverPid
} from './common.js';

const [workersArg = '8,32,64', secondsArg = '30', picturesArg = '3000'] = process.argv.slice(2);
const PORT = Number(process.env.PICTURES_PORT ?? 3298);
const HOSTS = Number(process.env.HOSTS ?? 50);
const key = process.env.IMAGE_PROXY_KEY;
if (!key || key.length < 32)
	throw new Error('IMAGE_PROXY_KEY, 32 characters or more, as the server has it');
const pid = serverPid();
const all = readers();

// ------------------------------------------------------------------ the site

const crcTable = Array.from({ length: 256 }, (_, n) => {
	let c = n;
	for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
	return c >>> 0;
});

function crc(bytes) {
	let c = 0xffffffff;
	for (const byte of bytes) c = crcTable[(c ^ byte) & 0xff] ^ (c >>> 8);
	return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, body) {
	const typed = Buffer.concat([Buffer.from(type), body]);
	const head = Buffer.alloc(4);
	head.writeUInt32BE(body.length);
	const tail = Buffer.alloc(4);
	tail.writeUInt32BE(crc(typed));
	return Buffer.concat([head, typed, tail]);
}

/** A PNG of a gradient with a disc on it, different for each `seed`; a photo of 1600 by 1000 is about 360 KB. */
function png(width, height, seed) {
	const header = Buffer.alloc(13);
	header.writeUInt32BE(width, 0);
	header.writeUInt32BE(height, 4);
	header.set([8, 2, 0, 0, 0], 8);
	const pixels = Buffer.alloc(height * (1 + width * 3));
	const [cx, cy, r] = [width * (0.3 + (seed % 5) / 10), height / 2, height / 3];
	for (let y = 0; y < height; y++) {
		const row = y * (1 + width * 3);
		for (let x = 0; x < width; x++) {
			const disc = (x - cx) ** 2 + (y - cy) ** 2 <= r * r;
			const at = row + 1 + x * 3;
			pixels[at] = disc ? 200 : (x * 255) / width;
			pixels[at + 1] = disc ? 80 + seed * 20 : (y * 255) / height;
			pixels[at + 2] = (seed * 37 + ((x + y) >> 4)) & 255;
		}
	}
	return Buffer.concat([
		Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
		chunk('IHDR', header),
		chunk('IDAT', deflateSync(pixels)),
		chunk('IEND', Buffer.alloc(0))
	]);
}

// A few pictures, each under many addresses: the proxy fetches and makes
// smaller every address on its own. A photo is larger than a thumbnail may
// be, an icon fits as it is.
const photos = Array.from({ length: 8 }, (_, seed) => png(1600, 1000, seed));
const icons = Array.from({ length: 8 }, (_, seed) => png(64, 64, seed));

function site(request, response) {
	const match = /^\/(photo|icon)\/(\d+)\.png$/.exec(request.url ?? '');
	if (!match) {
		response.writeHead(404).end();
		return;
	}
	const body = (match[1] === 'photo' ? photos : icons)[Number(match[2]) % 8];
	response.writeHead(200, { 'Content-Type': 'image/png', 'Content-Length': body.length });
	response.end(body);
}

const host = (n) => `127.0.0.${2 + (n % HOSTS)}`;
const servers = await Promise.all(
	Array.from(
		{ length: HOSTS },
		(_, n) =>
			new Promise((resolve, reject) => {
				const server = http.createServer(site);
				server.on('error', reject);
				server.listen(PORT, host(n), () => resolve(server));
			})
	)
);

// ------------------------------------------------------------- the addresses

/** The proxy's address of `url` at `size`, as `images::Signer::address` makes it. */
function address(size, url) {
	const signature = createHmac('sha256', key)
		.update(`${size}\n${url}`)
		.digest()
		.subarray(0, 16)
		.toString('base64url');
	return `/img/${signature}/${size}/${Buffer.from(url).toString('base64url')}`;
}

// As a page shows them: a thumbnail for every article in the list, an icon
// for every second, the picture of every fourth opened.
const addresses = [];
for (let n = 0; n < Number(picturesArg); n++) {
	const photo = `http://${host(n)}:${PORT}/photo/${n}.png`;
	addresses.push({ size: 'thumbnail', path: address('thumbnail', photo) });
	if (n % 2 === 0) {
		const icon = `http://${host(n)}:${PORT}/icon/${n}.png`;
		addresses.push({ size: 'icon', path: address('icon', icon) });
	}
	if (n % 4 === 0) addresses.push({ size: 'image', path: address('image', photo) });
}

/** Asks for a picture as a random reader: its status, how long it took and its bytes. */
function picture(path) {
	const reader = all[Math.floor(Math.random() * all.length)];
	return new Promise((resolve) => {
		const started = now();
		const request = http.get(
			{ host: RUEDER.hostname, port: RUEDER.port, path, agent, headers: headers(reader) },
			(response) => {
				let bytes = 0;
				response.on('data', (chunk) => (bytes += chunk.length));
				response.on('end', () =>
					resolve({ status: response.statusCode, ms: now() - started, bytes })
				);
			}
		);
		request.on('error', () => resolve({ status: 0, ms: now() - started, bytes: 0 }));
	});
}

// ------------------------------------------------------------------ the runs

/** Lets `workers` ask for the next of `next()` until it says none, and prints what it took. */
async function run(name, workers, next) {
	const times = {};
	const statuses = {};
	let bytes = 0;
	const [serverBefore, postgresBefore, started] = [cpu(pid), postgresCpu(), now()];
	await Promise.all(
		Array.from({ length: workers }, async () => {
			for (let item = next(); item; item = next()) {
				const result = await picture(item.path);
				(times[item.size] ??= []).push(result.ms);
				statuses[result.status] = (statuses[result.status] ?? 0) + 1;
				bytes += result.bytes;
			}
		})
	);
	const seconds = (now() - started) / 1000;
	const count = Object.values(times).reduce((n, list) => n + list.length, 0);
	console.log(
		JSON.stringify({
			run: name,
			workers,
			seconds: round(seconds),
			picturesPerSecond: round(count / seconds),
			megabytesPerSecond: round(bytes / seconds / 1e6),
			serverCores: round((cpu(pid) - serverBefore) / seconds),
			postgresCores: round((postgresCpu() - postgresBefore) / seconds),
			statuses,
			milliseconds: Object.fromEntries(
				Object.entries(times).map(([size, list]) => [
					size,
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

// Each address once: the proxy fetches, makes smaller and keeps them.
let first = 0;
await run('first', 32, () => addresses[first++]);

for (const workers of workersArg.split(',').map(Number)) {
	const until = now() + Number(secondsArg) * 1000;
	await run('kept', workers, () =>
		now() < until ? addresses[Math.floor(Math.random() * addresses.length)] : undefined
	);
}

for (const server of servers) server.close();
process.exit(0);
