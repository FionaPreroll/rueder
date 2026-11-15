// The captcha of open registration: the server hides a number in a hash, and
// the page counts up to it (`server/src/captcha.rs`).

import type { CaptchaChallenge, CaptchaSolution } from '#lib/api/types.ts';

/**
 * Finds the number hidden in the challenge by trying each from 0 on, up to
 * its highest; null if none fits, which no challenge of the server does.
 */
export async function solve(captcha: CaptchaChallenge): Promise<CaptchaSolution | null> {
	const encoder = new TextEncoder();
	const wanted = Uint8Array.from(captcha.challenge.match(/../g) ?? [], (pair) =>
		parseInt(pair, 16)
	);
	for (let number = 0; number <= captcha.max; number++) {
		const data = encoder.encode(`${captcha.salt}${number}`);
		const hash = new Uint8Array(await crypto.subtle.digest('SHA-256', data));
		if (hash.every((byte, at) => byte === wanted[at])) {
			const { salt, challenge, expires, signature } = captcha;
			return { salt, challenge, expires, signature, number };
		}
	}
	return null;
}
