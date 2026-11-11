// Who is logged in. The session itself is a cookie only the browser and the
// server know; this asks the server whose it is, logs in and out, and holds the
// reader of the account that is logged in.

import { api as theApi, ApiError, type Api, type ApiAccount, type ApiLoginWays } from './client.ts';
import { ServerReader } from './reader.svelte.ts';

/** Why a login did not work. */
export type LoginFailure =
	| { kind: 'wrong' }
	/** Too many wrong tries; the server says how long to wait. */
	| { kind: 'tooMany'; minutes: number }
	| { kind: 'unreachable' };

export class Session {
	/**
	 * `checking` until the server has said whether there is a session, `unreachable`
	 * if it did not say.
	 */
	state = $state<'checking' | 'unreachable' | 'out' | 'in'>('checking');
	/** The reader of the account that is logged in. */
	reader = $state.raw<ServerReader | null>(null);
	/** The ways to log in besides name and password. */
	ways = $state.raw<ApiLoginWays>({ oidc: false });

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
			{ name: account.username, admin: account.is_admin },
			{ signOut: () => void this.signOut(), expired: () => this.leave() },
			this.api
		);
		this.state = 'in';
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
			this.ways = { oidc: false };
		}
	}

	/** Logs in; null if it worked, else why it did not. */
	async login(name: string, password: string): Promise<LoginFailure | null> {
		try {
			this.enter(await this.api.login(name, password));
			return null;
		} catch (error) {
			if (!(error instanceof ApiError)) throw error;
			if (error.status === 429) {
				return { kind: 'tooMany', minutes: Math.max(1, Math.ceil((error.retryAfter ?? 60) / 60)) };
			}
			// A name or a password the server cannot even read is a wrong one too.
			if (error.status >= 400 && error.status < 500) return { kind: 'wrong' };
			return { kind: 'unreachable' };
		}
	}

	/** Ends the session on the server, and here whether or not the server heard it. */
	async signOut() {
		try {
			await this.api.logout();
		} catch {
			// The cookie may live on; the page shows the login all the same.
		}
		this.leave();
	}
}
