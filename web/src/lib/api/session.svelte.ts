// Who is logged in. The session itself is a cookie only the browser and the
// server know; this asks the server whose it is, logs in and out, and holds the
// reader of the account that is logged in.

import { api as theApi, ApiError, type Api, type ApiAccount, type ApiLoginWays } from './client.ts';
import type { EmailMode, LegalLinks } from '#lib/reader.ts';
import { language } from '#lib/i18n/locale.svelte.ts';
import { ServerReader } from './reader.svelte.ts';
import { passkeysWork, usePasskey } from '#lib/webauthn.ts';
import { forgetView } from '#lib/view.ts';

/** Why a login did not work. */
export type LoginFailure =
	| { kind: 'wrong' }
	/** Name and password were right; the account wants a code of its second factor. */
	| { kind: 'codeNeeded' }
	| { kind: 'wrongCode' }
	/** Too many wrong tries; the server says how long to wait. */
	| { kind: 'tooMany'; minutes: number }
	/** The password was right, but the operator disabled the account. */
	| { kind: 'disabled' }
	/** The password was right, but the account waits for its address to be confirmed. */
	| { kind: 'unconfirmed' }
	/** The reader closed the browser's window for the passkey, or it timed out. */
	| { kind: 'passkeyCancelled' }
	/** The browser or the device could not use a passkey. */
	| { kind: 'passkeyFailed' }
	/** The server took the passkey for none of its accounts. */
	| { kind: 'passkeyWrong' }
	| { kind: 'unreachable' };

/** Why nobody was registered. */
export type RegisterFailure =
	/** Registration is not open, and there was no invitation. */
	| { kind: 'closed' }
	/** The invitation cannot be used any more. */
	| { kind: 'invite' }
	/** There is no room for another account, for good or for today. */
	| { kind: 'full' }
	| { kind: 'today' }
	/** The terms of use were not agreed to. */
	| { kind: 'terms' }
	| { kind: 'taken' }
	/** The name or the password cannot be one; the server says why. */
	| { kind: 'invalid'; reason: string }
	/** The address is none mail can go to, or the server asks for one and there was none. */
	| { kind: 'email' }
	/** The link could not be mailed, so no account was made. */
	| { kind: 'unsent' }
	/** Too many registrations or mails; wait so many minutes. */
	| { kind: 'tooMany'; minutes: number }
	| { kind: 'unreachable' };

/** How sending the link again went. */
export type ResendOutcome =
	| { kind: 'sent' }
	| { kind: 'wrong' }
	| { kind: 'tooMany'; minutes: number }
	| { kind: 'unsent' }
	| { kind: 'unreachable' };

/** What the link from a mail did, opened in this page. */
export type Confirmation =
	/** The address is the account's now; `activated` if the account waited for it to log in. */
	| { kind: 'confirmed'; name: string; email: string; activated: boolean }
	/** No such link, or not any more: used, replaced by a newer one, or more than a day old. */
	| { kind: 'gone' }
	/** Another account has the address meanwhile. */
	| { kind: 'taken' }
	| { kind: 'unreachable' };

/** How asking for a link to set a new password went. */
export type ForgotOutcome =
	/** Asked: if there is such an account with a confirmed address, the link is on its way. */
	{ kind: 'asked' } | { kind: 'tooMany'; minutes: number } | { kind: 'unreachable' };

/** Why the new password from the link was not set. */
export type ResetFailure =
	/** No such link, or not any more: used, replaced, more than half an hour old. */
	| { kind: 'gone' }
	/** The server does not take the password; it says why. */
	| { kind: 'invalid'; reason: string }
	| { kind: 'unreachable' };

/** The token of a link to set a new password in the page's address: `/?reset=<token>`. */
function resetToken(): string | null {
	if (typeof location === 'undefined') return null;
	return new URLSearchParams(location.search).get('reset') || null;
}

/** An account that was made and waits for its address, which the login page says. */
export interface Awaiting {
	name: string;
	address: string;
}

/** The token of a link from a mail in the page's address: `/?confirm=<token>`. */
function confirmToken(): string | null {
	if (typeof location === 'undefined') return null;
	return new URLSearchParams(location.search).get('confirm') || null;
}

const minutes = (error: ApiError) => Math.max(1, Math.ceil((error.retryAfter ?? 60) / 60));

/** A server that does not say: no other ways, no legal texts. */
const NO_WAYS: ApiLoginWays = {
	oidc: false,
	registration: false,
	passkeys: false,
	password_reset: false,
	legal: { imprint: null, privacy: null, terms: null, terms_version: null }
};

/** The texts to link to, or null where the operator names none. */
function legalLinks(ways: ApiLoginWays): LegalLinks | null {
	const { imprint, privacy, terms } = ways.legal;
	return imprint || privacy || terms ? { imprint, privacy, terms } : null;
}

/** The invitation the page was opened with, and whether it can be used. */
export interface Invitation {
	token: string;
	valid: boolean;
}

/** The token of the invitation in the page's address: `/?invite=<token>`. */
function invitedBy(): string | null {
	if (typeof location === 'undefined') return null;
	return new URLSearchParams(location.search).get('invite') || null;
}

export class Session {
	/**
	 * `checking` until the server has said whether there is a session, `unreachable`
	 * if it did not say.
	 */
	state = $state<'checking' | 'unreachable' | 'out' | 'in'>('checking');
	/** The reader of the account that is logged in. */
	reader = $state.raw<ServerReader | null>(null);
	/** The ways to log in besides name and password. */
	ways = $state.raw<ApiLoginWays>(NO_WAYS);
	/** The invitation the page was opened with, once the server said whether it can be used. */
	invitation = $state.raw<Invitation | null>(null);
	/** The account was deleted here a moment ago, which the login page says. */
	farewell = $state(false);
	/** Whether registration asks for an address, and whether it must be one. */
	emailMode = $state<EmailMode>('off');
	/** An account made here a moment ago that waits for the link mailed to it. */
	awaiting = $state.raw<Awaiting | null>(null);
	/** What the link the page was opened with did, until the reader closes the note. */
	confirmation = $state.raw<Confirmation | null>(null);
	/** The token of the link to set a new password the page was opened with, until it is used. */
	resetting = $state<string | null>(null);
	/** A new password was set here a moment ago, which the login page says. */
	passwordSet = $state(false);

	private api: Api;

	constructor(api: Api = theApi) {
		this.api = api;
	}

	/** Asks the server whether the browser has a session. */
	async check() {
		this.state = 'checking';
		try {
			this.enter(await this.api.me());
		} catch (error) {
			if (error instanceof ApiError && error.signedOut) this.leave();
			else this.state = 'unreachable';
		}
	}

	private enter(account: ApiAccount) {
		this.reader = new ServerReader(
			{
				name: account.username,
				admin: account.is_admin,
				password: account.has_password,
				adminNeedsSecondFactor: account.admin_needs_second_factor,
				termsToAccept: account.terms_to_accept,
				contentReports: account.content_reports,
				appApi: account.app_api,
				passkeys: account.passkeys,
				emailMode: account.email_mode,
				email: account.email,
				emailPending: account.email_pending
			},
			{
				signOut: () => void this.signOut(),
				expired: () => this.leave(),
				removed: () => this.removed()
			},
			this.api
		);
		this.farewell = false;
		this.awaiting = null;
		this.reader.legal = legalLinks(this.ways);
		this.state = 'in';
		// The account menu links to the operator's texts, which come with the ways.
		void this.askWays();
	}

	/** The account is gone, and with it the session; where it was goes too. */
	private removed() {
		forgetView();
		this.farewell = true;
		this.leave();
	}

	private leave() {
		this.reader = null;
		this.state = 'out';
		void this.askWays();
	}

	/** A server that does not say is one without other ways. */
	private async askWays() {
		try {
			this.ways = await this.api.loginWays();
		} catch {
			this.ways = NO_WAYS;
		}
		if (this.reader) this.reader.legal = legalLinks(this.ways);
		const token = invitedBy();
		// Whether the form asks for an address, where there is a form.
		if (!token && !this.ways.registration) return;
		try {
			const asked = await this.api.registering(token ?? undefined);
			this.emailMode = asked.email;
			if (token) this.invitation = { token, valid: asked.invite === true };
		} catch {
			if (token) this.invitation = { token, valid: false };
		}
	}

	/**
	 * Opens the link from a mail the page was opened with, if it was, and takes
	 * it from the address, so that a reload does not open it again.
	 */
	async confirmFromAddress() {
		const token = confirmToken();
		if (!token) return;
		history.replaceState(history.state, '', location.pathname);
		try {
			const done = await this.api.confirmEmail(token);
			this.confirmation = {
				kind: 'confirmed',
				name: done.username,
				email: done.email,
				activated: done.activated
			};
			if (this.awaiting?.name.toLowerCase() === done.username.toLowerCase()) this.awaiting = null;
		} catch (error) {
			if (error instanceof ApiError && error.status === 404) this.confirmation = { kind: 'gone' };
			else if (error instanceof ApiError && error.status === 409)
				this.confirmation = { kind: 'taken' };
			else this.confirmation = { kind: 'unreachable' };
		}
	}

	/**
	 * Takes the token of a link to set a new password from the page's address,
	 * so that it is not left in the history or shown, and keeps it for the form.
	 */
	takeResetLink() {
		const token = resetToken();
		if (!token) return;
		history.replaceState(history.state, '', location.pathname);
		this.resetting = token;
		this.passwordSet = false;
	}

	/** Asks for a link that sets a new password, by name or by address. */
	async forgot(name: string): Promise<ForgotOutcome> {
		try {
			await this.api.forgotPassword(name.trim());
			return { kind: 'asked' };
		} catch (error) {
			if (error instanceof ApiError && error.status === 429) {
				return { kind: 'tooMany', minutes: minutes(error) };
			}
			return { kind: 'unreachable' };
		}
	}

	/** Sets the new password with the link's token; null if it is set. It does not log in. */
	async resetPassword(fresh: string): Promise<ResetFailure | null> {
		const token = this.resetting;
		if (!token) return { kind: 'gone' };
		try {
			await this.api.resetPassword(token, fresh);
		} catch (error) {
			if (error instanceof ApiError && error.status === 404) {
				this.resetting = null;
				return { kind: 'gone' };
			}
			if (error instanceof ApiError && error.status === 400) {
				return { kind: 'invalid', reason: error.message };
			}
			return { kind: 'unreachable' };
		}
		this.resetting = null;
		this.passwordSet = true;
		// Every session of the account ended, this browser's too if it had one.
		await this.check();
		return null;
	}

	/** Sends the link again to an account that waits for its address. */
	async resend(name: string, password: string): Promise<ResendOutcome> {
		try {
			await this.api.resendEmail(name, password);
			return { kind: 'sent' };
		} catch (error) {
			if (!(error instanceof ApiError)) throw error;
			if (error.status === 429) return { kind: 'tooMany', minutes: minutes(error) };
			if (error.status === 503) return { kind: 'unsent' };
			if (error.status === 401 || error.status === 409) return { kind: 'wrong' };
			return { kind: 'unreachable' };
		}
	}

	/**
	 * Makes an account and logs it in; null if it worked, else why it did not.
	 * `agreed` says the reader agreed to the terms of use the server names.
	 */
	async register(
		name: string,
		password: string,
		agreed = false,
		email = ''
	): Promise<RegisterFailure | null> {
		try {
			const terms = agreed ? (this.ways.legal.terms_version ?? undefined) : undefined;
			const address = this.emailMode === 'off' ? undefined : email.trim() || undefined;
			const made = await this.api.register(
				name,
				password,
				this.invitation?.token,
				terms,
				address,
				language()
			);
			this.invitation = null;
			// Where every account needs its address, the account waits for its link.
			if ('awaiting_email' in made) this.awaiting = { name, address: made.awaiting_email };
			else this.enter(made);
			return null;
		} catch (error) {
			if (!(error instanceof ApiError)) throw error;
			const word = error.message.trim();
			if (error.status === 403 && ['closed', 'invite', 'full', 'today', 'terms'].includes(word)) {
				return { kind: word as 'closed' | 'invite' | 'full' | 'today' | 'terms' };
			}
			if (error.status === 409) return { kind: 'taken' };
			if (error.status === 400 && word === 'that is no address mail can go to') {
				return { kind: 'email' };
			}
			if (error.status === 400 || error.status === 422) {
				return { kind: 'invalid', reason: error.message };
			}
			if (error.status === 429) return { kind: 'tooMany', minutes: minutes(error) };
			if (error.status === 503) return { kind: 'unsent' };
			return { kind: 'unreachable' };
		}
	}

	/** Logs in; null if it worked, else why it did not. */
	async login(name: string, password: string, code?: string): Promise<LoginFailure | null> {
		try {
			this.enter(await this.api.login(name, password, code?.trim() || undefined));
			return null;
		} catch (error) {
			if (!(error instanceof ApiError)) throw error;
			if (error.status === 429) return { kind: 'tooMany', minutes: minutes(error) };
			if (error.status === 401 && error.message.trim() === 'second factor needed') {
				return { kind: code ? 'wrongCode' : 'codeNeeded' };
			}
			if (error.status === 401 && error.message.trim() === 'wrong code') {
				return { kind: 'wrongCode' };
			}
			if (error.status === 403 && error.message.trim() === 'disabled') return { kind: 'disabled' };
			if (error.status === 403 && error.message.trim() === 'unconfirmed') {
				return { kind: 'unconfirmed' };
			}
			// A name or a password the server cannot even read is a wrong one too.
			if (error.status >= 400 && error.status < 500) return { kind: 'wrong' };
			return { kind: 'unreachable' };
		}
	}

	/** Whether the login page offers a passkey: the server has them, and so does the browser. */
	get passkeys(): boolean {
		return this.ways.passkeys && passkeysWork();
	}

	/** Logs in with a passkey, whoever it belongs to; null if it worked, else why it did not. */
	async loginWithPasskey(): Promise<LoginFailure | null> {
		let challenge;
		try {
			challenge = await this.api.startPasskeyLogin();
		} catch (error) {
			return this.passkeyFailure(error);
		}
		const credential = await usePasskey(challenge.options);
		if (credential === 'cancelled') return { kind: 'passkeyCancelled' };
		if (typeof credential === 'string') return { kind: 'passkeyFailed' };
		try {
			this.enter(await this.api.finishPasskeyLogin({ ceremony: challenge.ceremony, credential }));
			return null;
		} catch (error) {
			return this.passkeyFailure(error);
		}
	}

	private passkeyFailure(error: unknown): LoginFailure {
		if (!(error instanceof ApiError)) return { kind: 'unreachable' };
		if (error.status === 429) {
			return { kind: 'tooMany', minutes: Math.max(1, Math.ceil((error.retryAfter ?? 60) / 60)) };
		}
		if (error.status === 403 && error.message.trim() === 'disabled') return { kind: 'disabled' };
		if (error.status >= 400 && error.status < 500) return { kind: 'passkeyWrong' };
		return { kind: 'unreachable' };
	}

	/** Ends the session on the server, and here whether or not the server heard it. */
	async signOut() {
		// Where the reader was is nobody else's business.
		forgetView();
		try {
			await this.api.logout();
		} catch {
			// The cookie may live on; the page shows the login all the same.
		}
		this.leave();
	}
}
