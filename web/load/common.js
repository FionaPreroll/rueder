// What the load tests share: the tokens from `rueder bench readers`, an
// address of their own for each reader, requests to the server, and how much
// processor time a process used. Linux only, as that is read from /proc.

import http from 'node:http';
import { readFileSync, readdirSync } from 'node:fs';

/** The server under load; it is started with CLIENT_IP_HEADER=x-forwarded-for. */
export const RUEDER = new URL(process.env.RUEDER ?? 'http://127.0.0.1:3290');

/** The readers and their tokens, as `rueder bench readers --tokens` wrote them. */
export function readers(path = process.env.TOKENS) {
	if (!path) throw new Error('TOKENS names the file of `rueder bench readers --tokens`');
	return readFileSync(path, 'utf8')
		.split('\n')
		.filter(Boolean)
		.map((line) => {
			const [id, token] = line.split(' ');
			return { id: Number(id), token };
		});
}

/**
 * Each reader comes from an address of its own, as readers do, so that the
 * limits per sender count per reader and not for all of them at once.
 */
export const address = (id) => `10.${(id >> 16) & 255}.${(id >> 8) & 255}.${id & 255}`;

export const agent = new http.Agent({ keepAlive: true, maxSockets: 1024 });

export const headers = (reader, more = {}) => ({
	Cookie: `__Host-rueder=${reader.token}`,
	'X-Forwarded-For': address(reader.id),
	...more
});

export const now = () => performance.now();
export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** A request to the server: its status, how long it took, and its JSON if any. */
export function call(method, path, reader, body) {
	return new Promise((resolve) => {
		const started = now();
		const data = body === undefined ? undefined : JSON.stringify(body);
		const request = http.request(
			{
				host: RUEDER.hostname,
				port: RUEDER.port,
				path,
				method,
				agent,
				headers: headers(reader, data ? { 'Content-Type': 'application/json' } : {})
			},
			(response) => {
				const chunks = [];
				response.on('data', (chunk) => chunks.push(chunk));
				response.on('end', () => {
					let json = null;
					try {
						json = JSON.parse(Buffer.concat(chunks).toString('utf8'));
					} catch {
						json = null;
					}
					resolve({ status: response.statusCode, ms: now() - started, json });
				});
			}
		);
		request.on('error', () => resolve({ status: 0, ms: now() - started, json: null }));
		if (data) request.write(data);
		request.end();
	});
}

/** The processor time a process used so far, in seconds, from /proc. */
export function cpu(pid) {
	const fields = readFileSync(`/proc/${pid}/stat`, 'utf8').split(') ')[1].split(' ');
	return (Number(fields[11]) + Number(fields[12])) / 100;
}

/** The memory the process holds, in MB. */
export function memory(pid) {
	const status = readFileSync(`/proc/${pid}/status`, 'utf8');
	return Number(/VmRSS:\s+(\d+)/.exec(status)?.[1] ?? 0) / 1024;
}

/** The processor time of all PostgreSQL processes on this machine together. */
export function postgresCpu() {
	let total = 0;
	for (const pid of readdirSync('/proc')) {
		if (!/^\d+$/.test(pid)) continue;
		try {
			const [name, rest] = readFileSync(`/proc/${pid}/stat`, 'utf8').split(') ');
			if (!name.endsWith('(postgres')) continue;
			const fields = rest.split(' ');
			total += (Number(fields[11]) + Number(fields[12])) / 100;
		} catch {
			// Gone meanwhile.
		}
	}
	return total;
}

/** The value below which `p` percent of the values lie. */
export function percentile(values, p) {
	if (values.length === 0) return null;
	const sorted = [...values].sort((a, b) => a - b);
	return sorted[Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length))];
}

export const round = (x) => (x === null ? null : Math.round(x * 10) / 10);

/** The id of the rueder process, which the tests need to see what it costs. */
export function serverPid() {
	const pid = Number(process.env.RUEDER_PID);
	if (!pid) throw new Error('RUEDER_PID names the process of `rueder serve`');
	return pid;
}
