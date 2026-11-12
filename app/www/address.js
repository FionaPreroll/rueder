// The address of a rueder as the app keeps it: the origin of an https page, as
// rueder logs in only over a secure connection and runs at the top of its host.
// The native side checks it once more (Settings.java).

/** The longest address taken, in characters. */
export const MAX_ADDRESS = 2000;

/**
 * What a reader typed, as the address to open, or why it is none: "empty",
 * "invalid" or "insecure".
 */
export function serverAddress(typed) {
	const text = String(typed ?? '').trim();
	if (!text) return { problem: 'empty' };
	if (text.length > MAX_ADDRESS || /\s/.test(text)) return { problem: 'invalid' };
	let url;
	try {
		url = new URL(/^[a-z][a-z0-9+.-]*:\/\//i.test(text) ? text : `https://${text}`);
	} catch {
		return { problem: 'invalid' };
	}
	if (url.protocol === 'http:') return { problem: 'insecure' };
	if (url.protocol !== 'https:' || !url.hostname || url.username || url.password) {
		return { problem: 'invalid' };
	}
	return { address: url.origin };
}
