// A made-up passkey device for the tests: it stands in for the browser's
// `navigator.credentials` and `PublicKeyCredential`, answers `create()` and
// `get()` with a key of the id it was given, and remembers what it was asked.
// Or it refuses, as a reader who closes the browser's window does.

import { vi } from 'vitest';

const bytes = (text: string) => new TextEncoder().encode(text).buffer as ArrayBuffer;

/** What the browser gives the page as a credential. */
export class FakeCredential {
	readonly type = 'public-key';
	readonly rawId: ArrayBuffer;
	constructor(
		readonly id: string,
		readonly response: object
	) {
		this.rawId = bytes(id);
	}
	getClientExtensionResults() {
		return {};
	}
}

export interface FakeDevice {
	create: ReturnType<typeof vi.fn>;
	get: ReturnType<typeof vi.fn>;
	/** Makes the next call, or the next of `create` or `get`, refuse with a DOMException of this name. */
	refuse(name: string, call?: 'create' | 'get'): void;
}

/**
 * Puts a device with the key `key` in the browser's place; a passkey it makes
 * gets the id `made`, which is `key` unless said otherwise.
 */
export function fakeDevice(key = 'key-1', made = key): FakeDevice {
	let refusal: { name: string; call?: 'create' | 'get' } | null = null;
	const refusing = (call: 'create' | 'get') => {
		if (!refusal || (refusal.call && refusal.call !== call)) return null;
		const error = new DOMException('refused', refusal.name);
		refusal = null;
		return error;
	};
	const create = vi.fn(async () => {
		const error = refusing('create');
		if (error) throw error;
		return new FakeCredential(made, {
			attestationObject: bytes('attestation'),
			clientDataJSON: bytes('{"type":"webauthn.create"}'),
			getTransports: () => ['internal']
		});
	});
	const get = vi.fn(async () => {
		const error = refusing('get');
		if (error) throw error;
		return new FakeCredential(key, {
			authenticatorData: bytes('authenticator'),
			clientDataJSON: bytes('{"type":"webauthn.get"}'),
			signature: bytes('signature'),
			userHandle: bytes('user')
		});
	});
	vi.stubGlobal('PublicKeyCredential', FakeCredential);
	Object.defineProperty(navigator, 'credentials', {
		value: { create, get },
		configurable: true
	});
	return { create, get, refuse: (name, call) => (refusal = { name, call }) };
}

/** Takes the device away again. */
export function noDevice() {
	Object.defineProperty(navigator, 'credentials', { value: undefined, configurable: true });
}
