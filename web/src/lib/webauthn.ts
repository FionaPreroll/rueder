// Between rueder's server and the browser's WebAuthn: the server sends the
// options of a ceremony as JSON, with bytes in base64url (as webauthn-rs writes
// them), and reads the device's answer the same way. This turns the one into
// what `navigator.credentials.create()` and `.get()` take, and what they give
// back into JSON. It checks the shape of what the server sends rather than
// trusting it, and passes on only what WebAuthn knows.

/** Why the device gave no answer. */
export type DeviceRefusal =
	/** The reader closed the browser's window, or it timed out. */
	| 'cancelled'
	/** The device has a passkey for this account already. */
	| 'known'
	/** Anything else: no device, a browser without WebAuthn, options it refused. */
	| 'failed';

/** The longest base64url text taken: a key's id has at most 1023 bytes. */
const MAX_B64 = 2048;

/** Whether this browser can use passkeys at all. */
export function passkeysWork(): boolean {
	return (
		typeof PublicKeyCredential === 'function' &&
		typeof navigator !== 'undefined' &&
		typeof navigator.credentials?.create === 'function' &&
		typeof navigator.credentials?.get === 'function'
	);
}

/** Bytes as base64url without padding, as the server reads them. */
export function toB64url(bytes: ArrayBuffer | ArrayBufferView): string {
	const view =
		bytes instanceof ArrayBuffer
			? new Uint8Array(bytes)
			: new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength);
	let binary = '';
	for (const byte of view) binary += String.fromCharCode(byte);
	return btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '');
}

/** The bytes of a base64url text, or an error if it is none. */
export function fromB64url(text: unknown): Uint8Array<ArrayBuffer> {
	// A length of one more than a multiple of four is no whole byte.
	if (
		typeof text !== 'string' ||
		text.length > MAX_B64 ||
		text.length % 4 === 1 ||
		!/^[A-Za-z0-9_-]*$/.test(text)
	) {
		throw new TypeError('not base64url');
	}
	const binary = atob(text.replaceAll('-', '+').replaceAll('_', '/'));
	const bytes = new Uint8Array(new ArrayBuffer(binary.length));
	for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
	return bytes;
}

type Json = Record<string, unknown>;

function object(value: unknown, what: string): Json {
	if (typeof value !== 'object' || value === null || Array.isArray(value)) {
		throw new TypeError(`${what} is no object`);
	}
	return value as Json;
}

function text(value: unknown, what: string): string {
	if (typeof value !== 'string' || value.length > 256) throw new TypeError(`${what} is no text`);
	return value;
}

const optionalText = (value: unknown, what: string) =>
	value === undefined || value === null ? undefined : text(value, what);

function number(value: unknown): number | undefined {
	return typeof value === 'number' && Number.isFinite(value) && value > 0 ? value : undefined;
}

/** The keys a device is told about, with their ids as bytes. */
function descriptors(value: unknown): PublicKeyCredentialDescriptor[] {
	if (value === undefined || value === null) return [];
	if (!Array.isArray(value) || value.length > 100) throw new TypeError('no list of keys');
	return value.map((entry) => {
		const key = object(entry, 'a key');
		const transports = Array.isArray(key.transports)
			? key.transports.filter((t): t is AuthenticatorTransport => typeof t === 'string')
			: undefined;
		return {
			type: 'public-key',
			id: fromB64url(key.id),
			...(transports ? { transports } : {})
		};
	});
}

/** The selection of the device, as the server asks for it. */
function selection(value: unknown): AuthenticatorSelectionCriteria | undefined {
	if (value === undefined || value === null) return undefined;
	const asked = object(value, 'the selection');
	const out: AuthenticatorSelectionCriteria = {};
	const attachment = optionalText(asked.authenticatorAttachment, 'the attachment');
	if (attachment) out.authenticatorAttachment = attachment as AuthenticatorAttachment;
	const residentKey = optionalText(asked.residentKey, 'the resident key');
	if (residentKey) out.residentKey = residentKey as ResidentKeyRequirement;
	if (typeof asked.requireResidentKey === 'boolean') {
		out.requireResidentKey = asked.requireResidentKey;
	}
	const verification = optionalText(asked.userVerification, 'the user verification');
	if (verification) out.userVerification = verification as UserVerificationRequirement;
	return out;
}

/** The options of a new passkey, for `navigator.credentials.create()`. */
export function creationOptions(json: unknown): CredentialCreationOptions {
	const options = object(object(json, 'the options').publicKey, 'publicKey');
	const rp = object(options.rp, 'rp');
	const user = object(options.user, 'user');
	const params = options.pubKeyCredParams;
	if (!Array.isArray(params) || params.length === 0 || params.length > 20) {
		throw new TypeError('no algorithms');
	}
	return {
		publicKey: {
			rp: { id: optionalText(rp.id, 'rp.id'), name: text(rp.name, 'rp.name') },
			user: {
				id: fromB64url(user.id),
				name: text(user.name, 'user.name'),
				displayName: text(user.displayName, 'user.displayName')
			},
			challenge: fromB64url(options.challenge),
			pubKeyCredParams: params.map((entry) => {
				const param = object(entry, 'an algorithm');
				if (typeof param.alg !== 'number') throw new TypeError('no algorithm');
				return { type: 'public-key', alg: param.alg };
			}),
			timeout: number(options.timeout),
			excludeCredentials: descriptors(options.excludeCredentials),
			authenticatorSelection: selection(options.authenticatorSelection),
			attestation: (optionalText(options.attestation, 'attestation') ??
				'none') as AttestationConveyancePreference,
			extensions: extensions(options.extensions)
		}
	};
}

/**
 * The extensions the server asks for when a passkey is made, as far as they
 * are its own: that the device uses the key only after the PIN or biometrics
 * (credProtect), and that it says whether it keeps the key.
 */
function extensions(value: unknown): AuthenticationExtensionsClientInputs | undefined {
	if (value === undefined || value === null) return undefined;
	const asked = object(value, 'the extensions');
	const out: Record<string, unknown> = {};
	const policy = optionalText(asked.credentialProtectionPolicy, 'the protection policy');
	if (policy) out.credentialProtectionPolicy = policy;
	for (const flag of ['enforceCredentialProtectionPolicy', 'credProps', 'uvm']) {
		if (typeof asked[flag] === 'boolean') out[flag] = asked[flag];
	}
	return out as AuthenticationExtensionsClientInputs;
}

/** The options of a login or a proof, for `navigator.credentials.get()`. */
export function requestOptions(json: unknown): CredentialRequestOptions {
	const options = object(object(json, 'the options').publicKey, 'publicKey');
	return {
		publicKey: {
			challenge: fromB64url(options.challenge),
			rpId: optionalText(options.rpId, 'rpId'),
			timeout: number(options.timeout),
			allowCredentials: descriptors(options.allowCredentials),
			userVerification: (optionalText(options.userVerification, 'userVerification') ??
				'required') as UserVerificationRequirement
		}
	};
}

/** What a device answered to `create()`, as the server reads it. */
export function registrationJson(credential: PublicKeyCredential): Json {
	const response = credential.response as AuthenticatorAttestationResponse;
	const transports =
		typeof response.getTransports === 'function' ? response.getTransports() : undefined;
	return {
		id: credential.id,
		rawId: toB64url(credential.rawId),
		type: credential.type,
		response: {
			attestationObject: toB64url(response.attestationObject),
			clientDataJSON: toB64url(response.clientDataJSON),
			...(transports ? { transports } : {})
		},
		extensions: {}
	};
}

/** What a device answered to `get()`, as the server reads it. */
export function assertionJson(credential: PublicKeyCredential): Json {
	const response = credential.response as AuthenticatorAssertionResponse;
	return {
		id: credential.id,
		rawId: toB64url(credential.rawId),
		type: credential.type,
		response: {
			authenticatorData: toB64url(response.authenticatorData),
			clientDataJSON: toB64url(response.clientDataJSON),
			signature: toB64url(response.signature),
			userHandle: response.userHandle ? toB64url(response.userHandle) : null
		},
		extensions: {}
	};
}

/** Why the browser gave no answer, from what it threw. */
function refusalOf(error: unknown): DeviceRefusal {
	if (error instanceof DOMException) {
		if (error.name === 'NotAllowedError' || error.name === 'AbortError') return 'cancelled';
		if (error.name === 'InvalidStateError') return 'known';
	}
	return 'failed';
}

/** Asks the device for a new passkey; its answer as JSON, or why there is none. */
export async function createPasskey(options: unknown): Promise<Json | DeviceRefusal> {
	if (!passkeysWork()) return 'failed';
	try {
		const credential = await navigator.credentials.create(creationOptions(options));
		if (!(credential instanceof PublicKeyCredential)) return 'failed';
		return registrationJson(credential);
	} catch (error) {
		return refusalOf(error);
	}
}

/** Asks the device to sign in with a passkey; its answer as JSON, or why there is none. */
export async function usePasskey(options: unknown): Promise<Json | DeviceRefusal> {
	if (!passkeysWork()) return 'failed';
	try {
		const credential = await navigator.credentials.get(requestOptions(options));
		if (!(credential instanceof PublicKeyCredential)) return 'failed';
		return assertionJson(credential);
	} catch (error) {
		return refusalOf(error);
	}
}
