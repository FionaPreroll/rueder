import { afterEach, describe, expect, it } from 'vitest';
import { formatAgo, formatBytes, formatFull, formatWhen } from '#lib/format.ts';
import { useLanguage } from '#lib/i18n/locale.svelte.ts';

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

// The tests run in Europe/Berlin (see vite.config.ts): this is 15:30 there.
const now = Date.parse('2026-11-10T14:30:00Z');

afterEach(() => useLanguage(null));

describe('formatWhen', () => {
	it('is the time for today and the day before that', () => {
		expect(formatWhen(now - 3 * HOUR, now)).toBe('12:30 PM');
		expect(formatWhen(now - 2 * DAY, now)).toBe('Nov 8');
		expect(formatWhen(Date.parse('2025-12-24T10:00:00Z'), now)).toBe('Dec 24, 2025');
	});

	it('follows the language', () => {
		useLanguage('de');
		expect(formatWhen(now - 3 * HOUR, now)).toBe('12:30');
		expect(formatWhen(now - 2 * DAY, now)).toBe('8. Nov.');
	});
});

describe('formatAgo', () => {
	it('picks the unit by how long it has been', () => {
		expect(formatAgo(now - 20_000, now)).toBe('this minute');
		expect(formatAgo(now - 12 * MINUTE, now)).toBe('12 minutes ago');
		expect(formatAgo(now - 8 * HOUR, now)).toBe('8 hours ago');
		expect(formatAgo(now - DAY, now)).toBe('yesterday');
		expect(formatAgo(now - 5 * DAY, now)).toBe('5 days ago');
		expect(formatAgo(now - 21 * DAY, now)).toBe('3 weeks ago');
		expect(formatAgo(now - 90 * DAY, now)).toBe('3 months ago');
		expect(formatAgo(now - 800 * DAY, now)).toBe('2 years ago');
	});

	it('never looks into the future', () => {
		expect(formatAgo(now + 5 * MINUTE, now)).toBe('this minute');
	});

	it('follows the language', () => {
		useLanguage('de');
		expect(formatAgo(now - 8 * HOUR, now)).toBe('vor 8 Stunden');
		expect(formatAgo(now - DAY, now)).toBe('gestern');
	});
});

describe('formatFull', () => {
	it('is the day and the time', () => {
		expect(formatFull(now)).toBe('Tuesday, November 10, 2026 at 3:30 PM');
		useLanguage('de');
		expect(formatFull(now)).toBe('Dienstag, 10. November 2026 um 15:30');
	});
});

describe('formatBytes', () => {
	it('is the size of a file in the unit that fits', () => {
		expect(formatBytes(0)).toBe('0 B');
		expect(formatBytes(999)).toBe('999 B');
		expect(formatBytes(1500)).toBe('1.5 kB');
		expect(formatBytes(34_560_000)).toBe('34.6 MB');
		expect(formatBytes(5e12)).toBe('5,000 GB');
		useLanguage('de');
		expect(formatBytes(1500)).toBe('1,5 kB');
	});
});
