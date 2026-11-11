// Times as the list and the articles show them, in the page's language.

import { localized } from './i18n/intl.ts';
import { language } from './i18n/locale.svelte.ts';

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

const timeOfDay = localized(
	(locale) => new Intl.DateTimeFormat(locale, { hour: 'numeric', minute: '2-digit' })
);
const dayOfYear = localized(
	(locale) => new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'short' })
);
const dayWithYear = localized(
	(locale) => new Intl.DateTimeFormat(locale, { day: 'numeric', month: 'short', year: 'numeric' })
);
const whole = localized(
	(locale) => new Intl.DateTimeFormat(locale, { dateStyle: 'full', timeStyle: 'short' })
);
const relative = localized(() => new Intl.RelativeTimeFormat(language(), { numeric: 'auto' }));

const sameDay = (a: Date, b: Date) =>
	a.getFullYear() === b.getFullYear() &&
	a.getMonth() === b.getMonth() &&
	a.getDate() === b.getDate();

/** Short, for a row of the list: the time for today, else the day, with the year when it is another one. */
export function formatWhen(at: number, now: number) {
	const date = new Date(at);
	const today = new Date(now);
	if (sameDay(date, today)) return timeOfDay().format(date);
	return (date.getFullYear() === today.getFullYear() ? dayOfYear() : dayWithYear()).format(date);
}

/** "8 hours ago", "yesterday", "3 weeks ago". */
export function formatAgo(at: number, now: number) {
	const passed = now - at;
	if (passed < HOUR) return relative().format(-Math.max(0, Math.round(passed / MINUTE)), 'minute');
	if (passed < DAY) return relative().format(-Math.round(passed / HOUR), 'hour');
	if (passed < 14 * DAY) return relative().format(-Math.round(passed / DAY), 'day');
	if (passed < 60 * DAY) return relative().format(-Math.round(passed / (7 * DAY)), 'week');
	if (passed < 365 * DAY) return relative().format(-Math.round(passed / (30 * DAY)), 'month');
	return relative().format(-Math.round(passed / (365 * DAY)), 'year');
}

const amount = localized((locale) => new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }));

/** The size of a file: "512 B", "3.4 MB". */
export function formatBytes(bytes: number) {
	const units = ['B', 'kB', 'MB', 'GB'];
	let size = Math.max(0, bytes);
	let unit = 0;
	while (size >= 1000 && unit < units.length - 1) {
		size /= 1000;
		unit++;
	}
	return `${amount().format(unit === 0 ? Math.round(size) : size)} ${units[unit]}`;
}

/** Day and time in full, for the head of an open article and for tooltips. */
export const formatFull = (at: number) => whole().format(new Date(at));

/** Formats of numbers with a unit, made once per unit and number of digits. */
const units = localized((locale) => {
	const made = new Map<string, Intl.NumberFormat>();
	return (unit: string, digits: number) => {
		const key = `${unit}/${digits}`;
		let format = made.get(key);
		if (!format) {
			format = new Intl.NumberFormat(locale, {
				style: 'unit',
				unit,
				unitDisplay: 'short',
				maximumFractionDigits: digits
			});
			made.set(key, format);
		}
		return format;
	};
});

/** How long something took: "0.4 ms", "12 ms", "2.4 s", "12 min", "5 hr", "3 days". */
export function formatDuration(ms: number) {
	const format = (value: number, unit: string, digits = 0) => units()(unit, digits).format(value);
	const span = Math.max(0, ms);
	if (span < 10) return format(span, 'millisecond', 1);
	if (span < 1000) return format(span, 'millisecond');
	if (span < 60_000) return format(span / 1000, 'second', 1);
	if (span < HOUR) return format(span / MINUTE, 'minute');
	if (span < 2 * DAY) return format(span / HOUR, 'hour');
	return format(span / DAY, 'day');
}

const percent = localized(
	(locale) => new Intl.NumberFormat(locale, { style: 'percent', maximumFractionDigits: 1 })
);

/** A share as a percentage: 0.987 is "98.7%". */
export const formatPercent = (share: number) => percent().format(share);
