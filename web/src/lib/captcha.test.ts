import { describe, expect, it } from 'vitest';
import { solve } from './captcha.ts';

const captcha = {
	salt: '0123456789abcdef01234567',
	// The same example as in `server/src/captcha.rs`, so that the page and the
	// server hash alike: the SHA-256 of the salt followed by 4242.
	challenge: 'db254f5032fc9fed67d5407565e4b275e98ee8d74751fd5829f0e7b9158a2517',
	max: 10_000,
	expires: 1_800_000_300,
	signature: 'signed'
};

describe('solve', () => {
	it('finds the number hidden in the challenge', async () => {
		expect(await solve(captcha)).toEqual({
			salt: captcha.salt,
			challenge: captcha.challenge,
			expires: captcha.expires,
			signature: 'signed',
			number: 4242
		});
	});

	it('looks no further than the highest number', async () => {
		expect(await solve({ ...captcha, max: 4241 })).toBeNull();
	});
});
