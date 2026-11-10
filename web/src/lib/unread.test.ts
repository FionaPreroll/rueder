import { describe, expect, it } from 'vitest';
import {
	emptyState,
	isRead,
	isStarred,
	markAllRead,
	markRead,
	markUnread,
	setHidden,
	setStarred,
	unreadCount
} from '#lib/unread.ts';

describe('the unread count', () => {
	it('is the number of articles for a feed nobody has read', () => {
		expect(unreadCount(emptyState(), 0)).toBe(0);
		expect(unreadCount(emptyState(), 12)).toBe(12);
	});

	it('follows from the mark and the newest number', () => {
		const state = markAllRead(emptyState(), 10);
		expect(unreadCount(state, 10)).toBe(0);
		// Three more arrive: nothing was counted up, and still it is three.
		expect(unreadCount(state, 13)).toBe(3);
	});

	it('counts the exceptions above and below the mark', () => {
		let state = markAllRead(emptyState(), 10);
		state = markUnread(state, 4);
		state = markRead(state, 12);
		expect(unreadCount(state, 13)).toBe(3);
		expect(isRead(state, 4)).toBe(false);
		expect(isRead(state, 12)).toBe(true);
		expect(isRead(state, 11)).toBe(false);
	});

	it('leaves hidden articles out, read or not', () => {
		let state = markAllRead(emptyState(), 10);
		state = markUnread(state, 4);
		state = markRead(state, 12);
		state = setHidden(state, [4, 11, 12]);
		expect(unreadCount(state, 13)).toBe(1);
	});

	it('ignores what is beyond the newest number', () => {
		const state = markRead(setHidden(emptyState(), [9]), 8);
		expect(unreadCount(state, 5)).toBe(5);
	});
});

describe('marking', () => {
	it('does nothing when nothing changes', () => {
		const state = markAllRead(emptyState(), 5);
		expect(markRead(state, 3)).toBe(state);
		expect(markUnread(state, 8)).toBe(state);
		expect(setStarred(state, 2, false)).toBe(state);
	});

	it('moves the mark over what is read right above it', () => {
		let state = markAllRead(emptyState(), 5);
		state = markRead(state, 7);
		expect(state.readUpto).toBe(5);
		state = markRead(state, 6);
		expect(state.readUpto).toBe(7);
		expect(state.read.size).toBe(0);
	});

	it('reads a kept article again and sets a read one back', () => {
		let state = markAllRead(emptyState(), 5);
		state = markUnread(state, 2);
		expect(state.keptUnread.has(2)).toBe(true);
		state = markRead(state, 2);
		expect(state.keptUnread.size).toBe(0);
		state = markUnread(markRead(state, 9), 9);
		expect(state.read.size).toBe(0);
	});

	it('marks all up to a number, and keeps what is newer', () => {
		let state = markAllRead(emptyState(), 10);
		state = markUnread(state, 3);
		state = markUnread(state, 9);
		state = markRead(state, 14);
		// Up to 5 only: 9 stays unread, 14 stays read.
		state = markAllRead(state, 5);
		expect(state.readUpto).toBe(10);
		expect([...state.keptUnread]).toEqual([9]);
		expect(unreadCount(state, 15)).toBe(5);
		state = markAllRead(state, 15);
		expect(unreadCount(state, 15)).toBe(0);
		expect(state.read.size).toBe(0);
	});

	it('never moves the mark back', () => {
		const state = markAllRead(markAllRead(emptyState(), 10), 4);
		expect(state.readUpto).toBe(10);
	});

	it('stars apart from reading', () => {
		let state = setStarred(emptyState(), 3, true);
		expect(isStarred(state, 3)).toBe(true);
		expect(isRead(state, 3)).toBe(false);
		state = setStarred(state, 3, false);
		expect(isStarred(state, 3)).toBe(false);
	});
});
