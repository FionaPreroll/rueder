// The keyboard shortcuts and the keys a reader chose for them.

import { describe, expect, it } from 'vitest';
import {
	actions,
	canBeKey,
	defaultKeys,
	keyMap,
	keyName,
	keyOf,
	takenBy,
	understoodKeys
} from './keys.ts';

describe('the keys of the shortcuts', () => {
	it('are each used once, as rueder has them', () => {
		const keys = actions.map((action) => defaultKeys[action]);
		expect(new Set(keys).size).toBe(keys.length);
		const map = keyMap({});
		expect(map.single.get('j')).toBe('next');
		expect(map.single.get('A')).toBe('markAll');
		expect(map.afterG.get('h')).toBe('goHome');
		expect(map.single.has('g h')).toBe(false);
	});

	it('take the keys a reader chose instead', () => {
		const map = keyMap({ next: 'l', goHome: 'g x' });
		expect([map.single.get('l'), map.single.get('j')]).toEqual(['next', undefined]);
		expect([map.afterG.get('x'), map.afterG.get('h')]).toEqual(['goHome', undefined]);
		expect(keyOf('previous', { next: 'l' })).toBe('k');
	});

	it('say which shortcut has a key already', () => {
		expect(takenBy('next', 'k', {})).toBe('previous');
		expect(takenBy('next', 'j', {})).toBeNull();
		expect(takenBy('next', 'k', { previous: 'x' })).toBeNull();
		expect(takenBy('goHome', 'g a', {})).toBe('goAll');
	});

	it('keep the fixed keys and the second key after g', () => {
		for (const fixed of ['Enter', 'Escape', 'g', 'Tab', ' ', '', 'two keys', 'x'.repeat(17)]) {
			expect(canBeKey('next', fixed), fixed).toBe(false);
		}
		expect(canBeKey('next', 'ArrowDown')).toBe(true);
		expect(canBeKey('goHome', 'g z')).toBe(true);
		expect(canBeKey('goHome', 'z')).toBe(false);
		expect(canBeKey('goHome', 'g ')).toBe(false);
	});

	it('take from storage only what is a key for a shortcut', () => {
		for (const nonsense of [null, 'j', 3, ['j'], { next: 3 }, { nothing: 'x' }]) {
			expect(understoodKeys(nonsense)).toEqual({});
		}
		expect(
			understoodKeys({ next: 'l', previous: 'Enter', goAll: 'x', goHome: 'g y', star: 'S' })
		).toEqual({ next: 'l', goHome: 'g y', star: 'S' });
	});

	it('are written for people', () => {
		expect([keyName('A'), keyName('j'), keyName(' '), keyName('?')]).toEqual([
			'Shift+A',
			'j',
			'Space',
			'?'
		]);
	});
});
