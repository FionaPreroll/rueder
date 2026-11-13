// What the start page links to of the operator's legal texts: the addresses the
// server names in its answer about logging in (`GET /api/login/ways`). Only
// whole http and https addresses are taken; anything else in the answer is
// left alone.

/** The longest answer and the longest address that are read. */
export const MAX_WAYS = 16 * 1024;
export const MAX_URL = 2000;

/** The texts in the order the page shows them. */
const KINDS = ['imprint', 'privacy', 'terms'];

/** The links in an answer of the server: `[{ kind, href }]`, empty for none. */
export function legalLinks(body) {
	if (typeof body !== 'string' || body.length > MAX_WAYS) return [];
	let ways;
	try {
		ways = JSON.parse(body);
	} catch {
		return [];
	}
	const legal = ways && typeof ways === 'object' ? ways.legal : null;
	if (!legal || typeof legal !== 'object') return [];
	return KINDS.flatMap((kind) => {
		const href = legal[kind];
		if (typeof href !== 'string' || href.length > MAX_URL) return [];
		let url;
		try {
			url = new URL(href);
		} catch {
			return [];
		}
		if (url.protocol !== 'https:' && url.protocol !== 'http:') return [];
		return [{ kind, href: url.href }];
	});
}
