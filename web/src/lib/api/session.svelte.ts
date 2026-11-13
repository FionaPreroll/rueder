// Who is logged in. The session itself is a cookie only the browser and the
// server know; this asks the server whose it is, logs in and out, and holds the
// reader of the account that is logged in.

import { api as theApi, ApiError, type Api, type ApiAccount, type ApiLoginWays } from './client.ts';
import type { LegalLinks } from '#lib/reader.ts';
import { ServerReader } from './reader.svelte.ts';
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
	| { kind: 'unreachable' };

/** A server that does not say: no other ways, no legal texts. */
const NO_WAYS: ApiLoginWays = {
	oidc: false,
	registration: false,
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
				contentReports: account.content_reports
			},
			{
				signOut: () => void this.signOut(),
				expired: () => this.leave(),
				removed: () => this.removed()
			},
			this.api
		);
		this.farewell = false;
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
		if (!token) return;
		try {
			this.invitation = { token, valid: (await this.api.registering(token)).invite === true };
		} catch {
			this.invitation = { token, valid: false };
		}
	}

	/**
	 * Makes an account and logs it in; null if it worked, else why it did not.
	 * `agreed` says the reader agreed to the terms of use the server names.
	 */
	async register(name: string, password: string, agreed = false): Promise<RegisterFailure | null> {
		try {
			const terms = agreed ? (this.ways.legal.terms_version ?? undefined) : undefined;
			this.enter(await this.api.register(name, password, this.invitation?.token, terms));
			this.invitation = null;
			return null;
		} catch (error) {
			if (!(error instanceof ApiError)) throw error;
			const word = error.message.trim();
			if (error.status === 403 && ['closed', 'invite', 'full', 'today', 'terms'].includes(word)) {
				return { kind: word as 'closed' | 'invite' | 'full' | 'today' | 'terms' };
			}
			if (error.status === 409) return { kind: 'taken' };
			if (error.status === 400 || error.status === 422) {
				return { kind: 'invalid', reason: error.message };
			}
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
			if (error.status === 429) {
				return { kind: 'tooMany', minutes: Math.max(1, Math.ceil((error.retryAfter ?? 60) / 60)) };
			}
			if (error.status === 401 && error.message.trim() === 'second factor needed') {
				return { kind: code ? 'wrongCode' : 'codeNeeded' };
			}
			if (error.status === 401 && error.message.trim() === 'wrong code') {
				return { kind: 'wrongCode' };
			}
			if (error.status === 403 && error.message.trim() === 'disabled') return { kind: 'disabled' };
			// A name or a password the server cannot even read is a wrong one too.
			if (error.status >= 400 && error.status < 500) return { kind: 'wrong' };
			return { kind: 'unreachable' };
		}
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
