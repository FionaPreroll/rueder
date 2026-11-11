// Reports of errors in this browser, for readers who agreed to send them: errors
// nothing caught, promises that failed, parts of the page that failed to draw, and
// requests the server failed. They go to rueder's own server and nowhere else,
// gathered and sent at most every 30 seconds, or when the page is left. Before
// anything leaves the browser, addresses and numbers that name something are taken
// out, and of a stack only the lines in rueder's own files are kept. The server
// does the same once more.

import type { ClientReport, ReportKind } from './api/types.ts';

export type Report = Required<ClientReport>;

/** The longest message, in characters; the server keeps no more. */
export const MAX_MESSAGE = 500;
/** How many reports wait at most; what comes after is dropped until they are sent. */
export const MAX_WAITING = 20;
/** How often what waits is sent, in milliseconds. */
export const SEND_EVERY = 30_000;
/** How many lines of a stack are kept. */
const MAX_FRAMES = 10;

const ADDRESS = /[A-Za-z][A-Za-z0-9+.-]*:\/\/[^\s'")>\]]+|[^\s@'"]+@[^\s@'"]+\.[A-Za-z]{2,}/g;
const NUMBER = /([/=#])\d+|\d{4,}/g;

/** A text as it may leave the browser: one line, without addresses and numbers that name something. */
export function scrub(text: string, atMost = MAX_MESSAGE) {
	const line = text
		.replace(/[\p{Cc}]/gu, ' ')
		.split(/\s+/)
		.filter(Boolean)
		.join(' ')
		.replace(ADDRESS, '<address>')
		.replace(NUMBER, (_, before: string | undefined) => (before ? `${before}<n>` : '<n>'));
	return [...line].slice(0, atMost).join('');
}

/**
 * The lines of a stack in rueder's own files, as "_app/immutable/chunks/x.js:12:34".
 * Chrome writes "at f (https://host/_app/…:12:34)", Firefox and Safari "f@https://host/_app/…:12:34";
 * lines of other hosts, such as extensions, are left out.
 */
export function ownFrames(stack: string, host: string) {
	const frames: string[] = [];
	const frame = /(?:([a-z][a-z0-9+.-]*):\/\/([^/\s]+))?\/(_app\/[A-Za-z0-9._/-]+\.js):(\d+):(\d+)/i;
	for (const line of stack.split('\n')) {
		const found = frame.exec(line);
		if (!found) continue;
		const [, scheme, at, file, row, column] = found;
		if (scheme && (!/^https?$/i.test(scheme) || at !== host)) continue;
		frames.push(`${file}:${row}:${column}`);
		if (frames.length === MAX_FRAMES) break;
	}
	return frames.join('\n');
}

/** The browser and its major version, such as "Firefox 140"; nothing more of the user agent. */
export function browserOf(userAgent: string) {
	const named: [string, RegExp][] = [
		['Edge', /Edg\/(\d+)/],
		['Opera', /OPR\/(\d+)/],
		['Firefox', /Firefox\/(\d+)/],
		['Chrome', /Chrome\/(\d+)/],
		['Safari', /Version\/(\d+)[.\d]* .*Safari\//]
	];
	for (const [name, pattern] of named) {
		const found = pattern.exec(userAgent);
		if (found) return `${name} ${found[1]}`;
	}
	return 'Other';
}

/** What an error says of itself. */
function messageOf(error: unknown) {
	if (error instanceof Error) return `${error.name}: ${error.message}`;
	if (typeof error === 'string') return error;
	try {
		return JSON.stringify(error) ?? String(error);
	} catch {
		return String(error);
	}
}

export interface ReporterOptions {
	/** Sends reports to the server; a failure is dropped, so it reports nothing itself. */
	send: (reports: Report[]) => Promise<unknown>;
	/** Sends them as the page is left; false if the browser would not. */
	beacon?: (reports: Report[]) => boolean;
	version: string;
	browser: string;
	/** The host of rueder, whose files the stack may name. */
	host: string;
}

/** Gathers the reports of a browser and sends them now and then. */
export class Reporter {
	private waiting: Report[] = [];
	private options: ReporterOptions;

	constructor(options: ReporterOptions) {
		this.options = options;
	}

	/** How many reports wait to be sent. */
	get pending() {
		return this.waiting.length;
	}

	/** Notes an error. One that waits already is not noted twice. */
	note(kind: ReportKind, error: unknown, stack?: string) {
		if (this.waiting.length >= MAX_WAITING) return;
		const report: Report = {
			kind,
			message: scrub(messageOf(error)),
			stack: ownFrames(
				stack ?? (error instanceof Error ? (error.stack ?? '') : ''),
				this.options.host
			),
			version: this.options.version,
			browser: this.options.browser
		};
		const first = (stack: string) => stack.split('\n')[0];
		const known = this.waiting.some(
			(waiting) =>
				waiting.kind === report.kind &&
				waiting.message === report.message &&
				first(waiting.stack) === first(report.stack)
		);
		if (!known) this.waiting.push(report);
	}

	/** Sends what waits. */
	flush() {
		if (this.waiting.length === 0) return;
		const reports = this.waiting;
		this.waiting = [];
		this.options.send(reports).catch(() => {});
	}

	/** Sends what waits as the page goes, if the browser lets it. */
	private leave() {
		if (this.waiting.length === 0) return;
		if (this.options.beacon?.(this.waiting)) this.waiting = [];
		else this.flush();
	}

	/** Listens for errors on the page and sends them every 30 seconds; returns how to stop. */
	start(page: Window) {
		const error = (event: ErrorEvent) => this.note('error', event.error ?? event.message);
		const rejection = (event: PromiseRejectionEvent) => this.note('rejection', event.reason);
		const hidden = () => {
			if (page.document.visibilityState === 'hidden') this.leave();
		};
		const gone = () => this.leave();
		page.addEventListener('error', error);
		page.addEventListener('unhandledrejection', rejection);
		page.addEventListener('pagehide', gone);
		page.document.addEventListener('visibilitychange', hidden);
		const every = setInterval(() => this.flush(), SEND_EVERY);
		return () => {
			clearInterval(every);
			page.removeEventListener('error', error);
			page.removeEventListener('unhandledrejection', rejection);
			page.removeEventListener('pagehide', gone);
			page.document.removeEventListener('visibilitychange', hidden);
			this.flush();
		};
	}
}
