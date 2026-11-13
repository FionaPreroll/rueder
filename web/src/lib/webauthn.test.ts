// Between the server's JSON and the browser's WebAuthn: what the options become,
// what is refused, and what goes back.

import { afterEach, describe, expect, it, vi } from 'vitest';
import { fakeDevice, noDevice } from '#lib/api/test/fake-device.ts';
import {
	createPasskey,
	creationOptions,
	fromB64url,
	passkeysWork,
	requestOptions,
	toB64url,
	usePasskey
} from './webauthn.ts';

afterEach(() => {
	vi.unstubAllGlobals();
	noDevice();
});

const created = {
	publicKey: {
		rp: { id: 'reader.example', name: 'rueder' },
		user: { id: 'AQIDBA', name: 'ada', displayName: 'ada' },
		challenge: 'Y2hhbGxlbmdl',
		pubKeyCredParams: [
			{ type: 'public-key', alg: -7 },
			{ type: 'public-key', alg: -257 }
		],
		timeout: 300000,
		excludeCredentials: [{ type: 'public-key', id: 'a2V5', transports: ['internal'] }],
		authenticatorSelection: {
			residentKey: 'required',
			requireResidentKey: true,
			userVerification: 'required'
		},
		attestation: 'none',
		extensions: { credentialProtectionPolicy: 'userVerificationRequired', credProps: true, evil: 1 }
	}
};

describe('base64url', () => {
	it('goes there and back, without padding', () => {
		for (const text of ['', 'f', 'fo', 'foo', 'foob', 'fooba', 'foobar', 'ÿþ']) {
			const bytes = Uint8Array.from(text, (c) => c.charCodeAt(0));
			const encoded = toB64url(bytes);
			expect(encoded).not.toMatch(/[=+/]/);
			expect([...fromB64url(encoded)]).toEqual([...bytes]);
		}
		expect(toB64url(new Uint8Array([0xfb, 0xff]))).toBe('-_8');
	});

	it('takes nothing else', () => {
		for (const bad of [null, 1, 'a+b', 'a/b', 'ab==', 'a b', 'abcde', 'x'.repeat(5000)]) {
			expect(() => fromB64url(bad), String(bad)).toThrow(TypeError);
		}
	});
});

describe('the options', () => {
	it('of a new passkey are what WebAuthn knows, with bytes where it wants them', () => {
		const options = creationOptions(created).publicKey!;
		expect([...new Uint8Array(options.user.id as ArrayBuffer)]).toEqual([1, 2, 3, 4]);
		expect(new TextDecoder().decode(options.challenge as ArrayBuffer)).toBe('challenge');
		expect(options.rp).toEqual({ id: 'reader.example', name: 'rueder' });
		expect(options.pubKeyCredParams.map((p) => p.alg)).toEqual([-7, -257]);
		expect(new TextDecoder().decode(options.excludeCredentials![0].id as ArrayBuffer)).toBe('key');
		expect(options.authenticatorSelection).toEqual({
			residentKey: 'required',
			requireResidentKey: true,
			userVerification: 'required'
		});
		// Only the extensions the server means; nothing else gets through.
		expect(options.extensions).toEqual({
			credentialProtectionPolicy: 'userVerificationRequired',
			credProps: true
		});
	});

	it('of a login ask for the user to be verified', () => {
		const options = requestOptions({
			publicKey: { challenge: 'Y2hhbGxlbmdl', rpId: 'reader.example', allowCredentials: [] }
		}).publicKey!;
		expect(options.userVerification).toBe('required');
		expect(options.rpId).toBe('reader.example');
		expect(options.allowCredentials).toEqual([]);
	});

	it('that are not what the server sends are refused', () => {
		for (const broken of [
			null,
			{},
			{ publicKey: [] },
			{ publicKey: { ...created.publicKey, challenge: 'not base64!' } },
			{
				publicKey: {
					...created.publicKey,
					user: { id: 'AQ', name: 'x'.repeat(300), displayName: 'x' }
				}
			},
			{ publicKey: { ...created.publicKey, pubKeyCredParams: [] } },
			{ publicKey: { ...created.publicKey, excludeCredentials: 'all' } }
		]) {
			expect(() => creationOptions(broken), JSON.stringify(broken)).toThrow(TypeError);
		}
		expect(() => requestOptions({ publicKey: {} })).toThrow(TypeError);
	});
});

describe('the device', () => {
	it('is not there without WebAuthn', async () => {
		noDevice();
		expect(passkeysWork()).toBe(false);
		expect(await createPasskey(created)).toBe('failed');
	});

	it('answers in the JSON the server reads', async () => {
		const device = fakeDevice('key-1');
		expect(passkeysWork()).toBe(true);
		const made = (await createPasskey(created)) as Record<string, Record<string, unknown>>;
		expect(made.id).toBe('key-1');
		expect(made.rawId).toBe(toB64url(new TextEncoder().encode('key-1')));
		expect(made.response.transports).toEqual(['internal']);
		expect(typeof made.response.attestationObject).toBe('string');
		const asked = device.create.mock.calls[0][0] as CredentialCreationOptions;
		expect(asked.publicKey!.authenticatorSelection!.residentKey).toBe('required');

		const used = (await usePasskey({ publicKey: { challenge: 'AA' } })) as Record<
			string,
			Record<string, unknown>
		>;
		expect(Object.keys(used.response).sort()).toEqual([
			'authenticatorData',
			'clientDataJSON',
			'signature',
			'userHandle'
		]);
		expect(used.response.userHandle).toBe(toB64url(new TextEncoder().encode('user')));
	});

	it('that is closed, knows the key already, or fails, says which', async () => {
		const device = fakeDevice();
		device.refuse('NotAllowedError');
		expect(await usePasskey({ publicKey: { challenge: 'AA' } })).toBe('cancelled');
		device.refuse('InvalidStateError');
		expect(await createPasskey(created)).toBe('known');
		device.refuse('SecurityError');
		expect(await createPasskey(created)).toBe('failed');
		// Options the server would not send never reach the device.
		expect(await createPasskey({ publicKey: {} })).toBe('failed');
		expect(device.create).toHaveBeenCalledTimes(2);
	});
});
