// The account's own settings, with the server: its password and address, the
// second factor with passkeys, app passwords, invitations and deleting it (see
// #lib/account.ts). Each asks the API and says in the page's words why the
// server refused. What changes of the account itself, the reader's `account`
// takes, so the page shows it.

import type {
	AccountSettings,
	AddedPasskey,
	AppPassword,
	AppPasswordRefusal,
	AppPasswords,
	EmailRefusal,
	NewAppPassword,
	PasskeyItem,
	PasskeyList,
	PasskeyProof,
	PasskeyRefusal,
	PasswordRefusal,
	RemoveRefusal,
	SecondFactorRefusal,
	SecondFactorSetup,
	SecondFactorState
} from '#lib/account.ts';
import { language } from '#lib/i18n/locale.svelte.ts';
import { createPasskey, passkeysWork, usePasskey } from '#lib/webauthn.ts';
import { api as theApi, ApiError, type Api } from './client.ts';
import type { ServerReader, SessionEnds } from './reader.svelte.ts';

/** A passkey as the page keeps it. */
function passkeyItem(stored: {
	id: number;
	name: string;
	created_at: string;
	used_at: string | null;
}): PasskeyItem {
	return {
		id: stored.id,
		name: stored.name,
		createdAt: Date.parse(stored.created_at),
		usedAt: stored.used_at === null ? null : Date.parse(stored.used_at)
	};
}

function appPassword(stored: {
	id: number;
	name: string;
	created_at: string;
	used_at: string | null;
}): AppPassword {
	return {
		id: stored.id,
		name: stored.name,
		createdAt: Date.parse(stored.created_at),
		usedAt: stored.used_at === null ? null : Date.parse(stored.used_at)
	};
}

export class ServerAccount implements AccountSettings {
	readonly can: { email: boolean; passkeys: boolean };
	private reader: ServerReader;
	private session: SessionEnds;
	private api: Api;

	constructor(reader: ServerReader, session: SessionEnds, api: Api = theApi) {
		this.reader = reader;
		this.session = session;
		this.api = api;
		const account = reader.account;
		this.can = {
			email: (account.emailMode ?? 'off') !== 'off',
			passkeys: account.passkeys === true && passkeysWork()
		};
	}

	get appsVersion() {
		return this.reader.appsVersion;
	}

	/** A session that is gone is told to the page, which logs out. */
	private signedOut(error: unknown) {
		if (error instanceof ApiError && error.signedOut) this.session.expired();
	}

	// The second factor

	/** What a refusal of a change to the second factor means here. */
	private refusalOf(error: unknown, wrong: 'wrongPassword' | 'wrongCode'): SecondFactorRefusal {
		if (!(error instanceof ApiError)) return 'unreachable';
		if (error.signedOut) this.session.expired();
		if (error.status === 429) return 'tooMany';
		if (error.status === 403) return wrong;
		return 'unreachable';
	}

	// App passwords for apps of other makers

	async appPasswords(): Promise<AppPasswords | null> {
		try {
			const answer = await this.api.appPasswords();
			return { list: answer.app_passwords.map(appPassword), max: answer.max };
		} catch (error) {
			if (error instanceof ApiError && error.signedOut) this.session.expired();
			return null;
		}
	}

	async createAppPassword(
		name: string,
		password: string,
		code: string,
		passkey?: PasskeyProof
	): Promise<NewAppPassword | AppPasswordRefusal> {
		try {
			const made = await this.api.createAppPassword({
				name: name.trim(),
				password,
				code: code.trim(),
				passkey
			});
			return { ...appPassword(made), secret: made.secret };
		} catch (error) {
			if (!(error instanceof ApiError)) return 'unreachable';
			if (error.signedOut) this.session.expired();
			const said = error.message.trim();
			if (error.status === 429) return 'tooMany';
			if (error.status === 400) return 'tooManyApps';
			if (error.status === 403 && said === 'second factor needed') return 'codeNeeded';
			if (error.status === 403 && said === 'log in again first') return 'loginAgain';
			if (error.status === 403 && said === 'the code is wrong') return 'wrongCode';
			if (error.status === 403 && said === 'the passkey is wrong') return 'wrongCode';
			if (error.status === 403) return 'wrongPassword';
			return 'unreachable';
		}
	}

	async revokeAppPassword(id: number): Promise<boolean> {
		try {
			await this.api.revokeAppPassword(id);
			return true;
		} catch (error) {
			if (error instanceof ApiError && error.signedOut) this.session.expired();
			return false;
		}
	}

	async secondFactor(): Promise<SecondFactorState | null> {
		try {
			const state = await this.api.secondFactor();
			return {
				on: state.on,
				app: state.app,
				passkeys: state.passkeys,
				recoveryCodesLeft: state.recovery_codes_left,
				requiredForAdmins: state.required_for_admins
			};
		} catch (error) {
			this.refusalOf(error, 'wrongPassword');
			return null;
		}
	}

	async setUpSecondFactor(
		password: string,
		code = '',
		passkey?: PasskeyProof
	): Promise<SecondFactorSetup | SecondFactorRefusal> {
		try {
			const setup = await this.api.setUpSecondFactor({ password, code: code.trim(), passkey });
			return { secret: setup.secret, qrSvg: setup.qr_svg };
		} catch (error) {
			const said = error instanceof ApiError ? error.message.trim() : '';
			if (said === 'second factor needed') return 'codeNeeded';
			if (said === 'log in again first') return 'loginAgain';
			const codeWrong = said === 'the code is wrong' || said === 'the passkey is wrong';
			return this.refusalOf(error, codeWrong ? 'wrongCode' : 'wrongPassword');
		}
	}

	async confirmSecondFactor(code: string): Promise<string[] | SecondFactorRefusal> {
		try {
			return (await this.api.confirmSecondFactor(code.trim())).codes;
		} catch (error) {
			return this.refusalOf(error, 'wrongCode');
		}
	}

	async switchOffSecondFactor(
		password: string,
		code: string,
		passkey?: PasskeyProof
	): Promise<SecondFactorRefusal | null> {
		try {
			await this.api.switchOffSecondFactor({ password, code: code.trim(), passkey });
			return null;
		} catch (error) {
			const said = error instanceof ApiError ? error.message.trim() : '';
			const codeWrong = said === 'the code is wrong' || said === 'the passkey is wrong';
			return this.refusalOf(error, codeWrong ? 'wrongCode' : 'wrongPassword');
		}
	}

	// Passkeys

	async passkeys(): Promise<PasskeyList | null> {
		try {
			const answer = await this.api.passkeys();
			return { list: answer.passkeys.map(passkeyItem), max: answer.max };
		} catch (error) {
			if (error instanceof ApiError && error.signedOut) this.session.expired();
			return null;
		}
	}

	/** Why the server refused something about passkeys. */
	private passkeyRefusal(error: unknown): PasskeyRefusal {
		if (!(error instanceof ApiError)) return 'unreachable';
		if (error.signedOut) this.session.expired();
		const said = error.message.trim();
		if (error.status === 429) return 'tooMany';
		if (error.status === 400 && said.startsWith('an account has at most')) {
			return 'tooManyPasskeys';
		}
		if (error.status === 400) return 'failed';
		if (error.status === 409 && said === 'this passkey is known already') return 'known';
		if (error.status === 409) return 'wrong';
		if (error.status === 403) {
			if (said === 'second factor needed') return 'codeNeeded';
			if (said === 'the code is wrong') return 'wrongCode';
			if (said === 'the passkey is wrong') return 'wrong';
			if (said === 'log in again first') return 'loginAgain';
			return 'wrongPassword';
		}
		return 'unreachable';
	}

	async provePasskey(): Promise<PasskeyProof | PasskeyRefusal> {
		let challenge;
		try {
			challenge = await this.api.startPasskeyProof();
		} catch (error) {
			return this.passkeyRefusal(error);
		}
		const credential = await usePasskey(challenge.options);
		if (typeof credential === 'string') return credential;
		return { ceremony: challenge.ceremony, credential };
	}

	async addPasskey(
		name: string,
		password: string,
		code: string,
		passkey?: PasskeyProof
	): Promise<AddedPasskey | PasskeyRefusal> {
		let challenge;
		try {
			challenge = await this.api.startPasskey({
				name: name.trim(),
				password,
				code: code.trim(),
				passkey
			});
		} catch (error) {
			return this.passkeyRefusal(error);
		}
		const credential = await createPasskey(challenge.options);
		if (typeof credential === 'string') return credential;
		try {
			const added = await this.api.finishPasskey({ ceremony: challenge.ceremony, credential });
			return { passkey: passkeyItem(added.passkey), recoveryCodes: added.recovery_codes ?? null };
		} catch (error) {
			return this.passkeyRefusal(error);
		}
	}

	async removePasskey(
		id: number,
		password: string,
		code: string,
		passkey?: PasskeyProof
	): Promise<PasskeyRefusal | null> {
		try {
			await this.api.removePasskey(id, { password, code: code.trim(), passkey });
			return null;
		} catch (error) {
			return this.passkeyRefusal(error);
		}
	}

	async invites() {
		try {
			return await this.api.invites();
		} catch (error) {
			this.signedOut(error);
			return null;
		}
	}

	async removeAccount(
		proof: string,
		code?: string,
		passkey?: PasskeyProof
	): Promise<RemoveRefusal | null> {
		const farewell = this.reader.account.password === false ? { name: proof } : { password: proof };
		try {
			await this.api.removeAccount(passkey ? { passkey } : code ? { ...farewell, code } : farewell);
		} catch (error) {
			if (error instanceof ApiError && error.status === 403) {
				const missing = error.message.trim() === 'second factor needed';
				return { problem: missing && !code && !passkey ? 'code' : 'wrong' };
			}
			if (error instanceof ApiError && error.status === 429) {
				return { problem: 'busy', minutes: Math.max(1, Math.ceil((error.retryAfter ?? 60) / 60)) };
			}
			if (error instanceof ApiError && error.signedOut) this.session.expired();
			return { problem: 'unreachable' };
		}
		if (this.session.removed) this.session.removed();
		else this.session.expired();
		return null;
	}

	async changePassword(
		current: string,
		fresh: string,
		code?: string,
		passkey?: PasskeyProof
	): Promise<PasswordRefusal | null> {
		try {
			const proof = passkey ? { passkey } : { current, code: code?.trim() };
			await this.api.changePassword({ new: fresh, ...proof });
		} catch (error) {
			if (error instanceof ApiError && error.status === 403) {
				const said = error.message.trim();
				if (said === 'log in again first') return { problem: 'loginAgain' };
				const missing = said === 'second factor needed' && !code && !passkey;
				return { problem: missing ? 'code' : 'wrong' };
			}
			if (error instanceof ApiError && error.status === 400) return { problem: 'invalid' };
			if (error instanceof ApiError && error.status === 429) {
				return { problem: 'busy', minutes: Math.max(1, Math.ceil((error.retryAfter ?? 60) / 60)) };
			}
			if (error instanceof ApiError && error.signedOut) this.session.expired();
			return { problem: 'unreachable' };
		}
		// An account that had none logs in with its name from now on. The server
		// gave this browser a new session; the others are gone.
		this.reader.account = { ...this.reader.account, password: true };
		return null;
	}

	async changeEmail(
		address: string,
		password: string,
		code?: string,
		passkey?: PasskeyProof
	): Promise<EmailRefusal | null> {
		try {
			const proof = passkey ? { passkey } : { password, code: code?.trim() };
			const { pending } = await this.api.changeEmail({
				email: address.trim(),
				language: language(),
				...proof
			});
			this.reader.account = { ...this.reader.account, emailPending: pending };
			return null;
		} catch (error) {
			return this.emailRefusal(error, code, passkey);
		}
	}

	async removeEmail(
		password: string,
		code?: string,
		passkey?: PasskeyProof
	): Promise<EmailRefusal | null> {
		try {
			await this.api.removeEmail(passkey ? { passkey } : { password, code: code?.trim() });
			this.reader.account = { ...this.reader.account, email: null, emailPending: null };
			return null;
		} catch (error) {
			return this.emailRefusal(error, code, passkey);
		}
	}

	private emailRefusal(error: unknown, code?: string, passkey?: PasskeyProof): EmailRefusal {
		if (!(error instanceof ApiError)) return { problem: 'unreachable' };
		const said = error.message.trim();
		if (error.status === 403) {
			if (said === 'log in again first') return { problem: 'loginAgain' };
			const missing = said === 'second factor needed';
			return { problem: missing && !code && !passkey ? 'code' : 'wrong' };
		}
		if (error.status === 400) return { problem: 'invalid' };
		if (error.status === 409) {
			return { problem: said === 'that is the address already' ? 'same' : 'required' };
		}
		if (error.status === 429) {
			return { problem: 'busy', minutes: Math.max(1, Math.ceil((error.retryAfter ?? 60) / 60)) };
		}
		if (error.status === 503) return { problem: 'unsent' };
		if (error.signedOut) this.session.expired();
		return { problem: 'unreachable' };
	}

	async invite(uses: number) {
		try {
			const made = await this.api.invite(uses);
			// The link leads to this rueder's page, which reads the token from it.
			return { link: `${location.origin}/#invite=${made.token}` };
		} catch (error) {
			if (error instanceof ApiError && error.status === 403) return 'none-left' as const;
			this.signedOut(error);
			return null;
		}
	}

	async revokeInvite(id: number) {
		try {
			await this.api.revokeInvite(id);
			return true;
		} catch (error) {
			this.signedOut(error);
			return false;
		}
	}
}
