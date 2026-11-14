// The account's own settings: its password and address, the second factor
// with passkeys, app passwords, invitations and deleting it. Only a server has
// them, so the page gets them apart from the `Reader`, which stays with
// reading: `AccountSettings` from the server (api/account.svelte.ts), and app
// passwords alone from the page with made-up data (demo/apps.svelte.ts), to
// show how an app is set up. The dialog (AccountDialog.svelte) shows a tab for
// each part there is, and the menu of the account offers it only where there
// is at least one.

import type { Invites } from './api/types.ts';
import type { Reader } from './reader.ts';

export type { Invites };

/** How many hours the link that confirms an address is good for (`email::LINK_HOURS`). */
export const EMAIL_LINK_HOURS = 24;

/** How many minutes a link that sets a new password is good for (`recovery::LINK_MINUTES`). */
export const RESET_LINK_MINUTES = 30;

/** A passkey, as the list shows it. */
export interface PasskeyItem {
	id: number;
	name: string;
	/** When it was made, and when it last logged in or confirmed something, in ms. */
	createdAt: number;
	usedAt: number | null;
}

/** The account's passkeys, and how many it can have. */
export interface PasskeyList {
	list: PasskeyItem[];
	max: number;
}

/** A new passkey, with the recovery codes where it turned the second factor on. */
export interface AddedPasskey {
	passkey: PasskeyItem;
	recoveryCodes: string[] | null;
}

/**
 * A passkey's answer that the owner of the account is at the device, for one
 * change that asks for it (from `provePasskey`). The page passes it on as it is.
 */
export interface PasskeyProof {
	readonly ceremony: string;
	readonly credential: unknown;
}

/** Why a passkey was not added, removed or used. The page has the words. */
export type PasskeyRefusal =
	/** The reader closed the browser's window, or it timed out. */
	| 'cancelled'
	/** This device has a passkey for the account already. */
	| 'known'
	/** The device or the browser could not make or use one. */
	| 'failed'
	/** The passkey was not the account's, or the server did not take its answer. */
	| 'wrong'
	| 'wrongPassword'
	/** The password was right; the second factor is on, and its code is missing. */
	| 'codeNeeded'
	| 'wrongCode'
	/** The account has as many passkeys as it can have. */
	| 'tooManyPasskeys'
	/** Without a password and a second factor, only a fresh login adds the first. */
	| 'loginAgain'
	| 'tooMany'
	| 'unreachable';

/** An app password, as the list shows it: never the password itself. */
export interface AppPassword {
	id: number;
	name: string;
	/** When it was made, and when an app last used it (to the hour), in ms. */
	createdAt: number;
	usedAt: number | null;
}

/** The account's app passwords, and how many it can have. */
export interface AppPasswords {
	list: AppPassword[];
	max: number;
}

/** A new app password, with the password itself, shown this once. */
export interface NewAppPassword extends AppPassword {
	secret: string;
}

/** Why no app password was made. */
export type AppPasswordRefusal =
	| 'wrongPassword'
	| 'codeNeeded'
	| 'wrongCode'
	/** The account has neither a password nor a second factor, and the session is not young. */
	| 'loginAgain'
	| 'tooManyApps'
	| 'tooMany'
	| 'unreachable';

/** Why the account was not deleted. The page has the words. */
export type RemoveRefusal =
	/** The password, or the name for an account without one, or the code was not right. */
	| { problem: 'wrong' }
	/** The password was right; the account has a second factor, whose code is missing. */
	| { problem: 'code' }
	/** Too many wrong tries; wait so many minutes. */
	| { problem: 'busy'; minutes: number }
	| { problem: 'unreachable' };

/** Why an address was not taken, or not taken away. The page has the words. */
export type EmailRefusal =
	/** The password, or the code of the second factor, was not right. */
	| { problem: 'wrong' }
	/** The password was right; the account has a second factor, whose code is missing. */
	| { problem: 'code' }
	/** The account has neither a password nor a second factor, and the session is not young. */
	| { problem: 'loginAgain' }
	/** No address mail can go to. */
	| { problem: 'invalid' }
	/** It is the address the account has. */
	| { problem: 'same' }
	/** Every account here needs an address, so it cannot go. */
	| { problem: 'required' }
	/** Too many wrong tries or too many mails; wait so many minutes. */
	| { problem: 'busy'; minutes: number }
	/** The server could not hand the mail on; it may later. */
	| { problem: 'unsent' }
	| { problem: 'unreachable' };

/** Why a new password was not taken. The page has the words. */
export type PasswordRefusal =
	/** The password so far, or the code of the second factor, is not right. */
	| { problem: 'wrong' }
	/** The password was right; the account has a second factor, whose code is missing. */
	| { problem: 'code' }
	/** The account has neither a password nor a second factor, and the session is not young. */
	| { problem: 'loginAgain' }
	/** The server does not take the new one: too short, or too long. */
	| { problem: 'invalid' }
	/** Too many wrong tries; wait so many minutes. */
	| { problem: 'busy'; minutes: number }
	| { problem: 'unreachable' };

/** Whether the account has a second factor, and what the server wants of it. */
export interface SecondFactorState {
	/** Whether the second factor is on: an authenticator app or a passkey. */
	on: boolean;
	/** Whether the authenticator app is set up. */
	app: boolean;
	/** How many passkeys the account has. */
	passkeys: number;
	recoveryCodesLeft: number;
	/** Admins are admins only in a session that began with it. */
	requiredForAdmins: boolean;
}

/** A new secret for an authenticator app, waiting for a code to confirm it. */
export interface SecondFactorSetup {
	/** In base32, to type in by hand. */
	secret: string;
	/** The QR code of the `otpauth://` address, as SVG. */
	qrSvg: string;
}

/** Why a change to the second factor did not work. The page has the words. */
export type SecondFactorRefusal =
	| 'wrongPassword'
	/** Passkeys have the second factor on: setting up the app takes a code too. */
	| 'codeNeeded'
	| 'wrongCode'
	/** The account has neither a password nor a second factor, and the session is not young. */
	| 'loginAgain'
	| 'tooMany'
	| 'unreachable';

/** App passwords, for apps of other makers. */
export interface AppPasswordKeeper {
	/**
	 * Counts up when app passwords change or an app logs in for the first
	 * time, in another tab or from the app itself.
	 */
	readonly appsVersion: number;
	/** The account's app passwords; null if the server does not answer. */
	appPasswords(): Promise<AppPasswords | null>;
	/**
	 * Makes an app password, with the account's password and, if it has one, a
	 * code; or with a passkey's proof instead of both.
	 */
	createAppPassword(
		name: string,
		password: string,
		code: string,
		passkey?: PasskeyProof
	): Promise<NewAppPassword | AppPasswordRefusal>;
	/** Revokes an app password; false if that did not work. */
	revokeAppPassword(id: number): Promise<boolean>;
}

/** What an account sets about itself, with a server. */
export interface AccountSettings extends AppPasswordKeeper {
	/** What the server has: addresses at all, and passkeys this browser can use too. */
	readonly can: { email: boolean; passkeys: boolean };

	/**
	 * Gives the account another password, with the one so far; an account that
	 * logs in through single sign-on alone has none and sends nothing. Where
	 * the second factor is on, a code goes with it, or a passkey stands for
	 * both. Null when it worked: every other session of the account ends, this
	 * one goes on.
	 */
	changePassword(
		current: string,
		fresh: string,
		code?: string,
		passkey?: PasskeyProof
	): Promise<PasswordRefusal | null>;

	/**
	 * Mails a link to a new address for the account (`can.email`), with the
	 * password (none for an account that has none) and the code of its second
	 * factor where it has one, or a passkey's proof instead. The address counts
	 * once the link is opened; until then `account.emailPending` of the reader
	 * names it. Null when the link went.
	 */
	changeEmail(
		address: string,
		password: string,
		code?: string,
		passkey?: PasskeyProof
	): Promise<EmailRefusal | null>;
	/** Takes the address from the account, where it is optional; proved as above. */
	removeEmail(
		password: string,
		code?: string,
		passkey?: PasskeyProof
	): Promise<EmailRefusal | null>;

	/** The account's second factor; null if the server does not answer. */
	secondFactor(): Promise<SecondFactorState | null>;
	/**
	 * Starts setting the app up, with the account's password and, where passkeys
	 * have the second factor on already, a recovery code; or a passkey's proof.
	 */
	setUpSecondFactor(
		password: string,
		code?: string,
		passkey?: PasskeyProof
	): Promise<SecondFactorSetup | SecondFactorRefusal>;
	/** Confirms it with a code of the app: the recovery codes, shown this once. */
	confirmSecondFactor(code: string): Promise<string[] | SecondFactorRefusal>;
	/**
	 * Switches the app off, with the account's password and a code of the app or
	 * a recovery code, or a passkey's proof; null when it worked.
	 */
	switchOffSecondFactor(
		password: string,
		code: string,
		passkey?: PasskeyProof
	): Promise<SecondFactorRefusal | null>;

	/** The account's passkeys (`can.passkeys`); null if the server does not answer. */
	passkeys(): Promise<PasskeyList | null>;
	/** Asks a passkey of the account that its owner is there, for one change. */
	provePasskey(): Promise<PasskeyProof | PasskeyRefusal>;
	/**
	 * Makes a passkey on this device, with the account's password and, where the
	 * second factor is on, a code; or a passkey's proof instead of both.
	 */
	addPasskey(
		name: string,
		password: string,
		code: string,
		passkey?: PasskeyProof
	): Promise<AddedPasskey | PasskeyRefusal>;
	/** Removes a passkey, with the same proof as adding one; null when it worked. */
	removePasskey(
		id: number,
		password: string,
		code: string,
		passkey?: PasskeyProof
	): Promise<PasskeyRefusal | null>;

	/** The reader's invitations and how many more they can make; null if there was no answer. */
	invites(): Promise<Invites | null>;
	/**
	 * Makes an invitation for so many accounts. The link is shown once: nobody
	 * can ask for it again. `none-left` if the reader's invitations are used up,
	 * null if there was no answer.
	 */
	invite(uses: number): Promise<{ link: string } | 'none-left' | null>;
	/** Takes an invitation back; false if that did not work. */
	revokeInvite(id: number): Promise<boolean>;

	/**
	 * Deletes the account for good, with the password, or the account's name
	 * where it has none, and the code of its second factor where it has one; or
	 * with a passkey's proof instead of all of that. Null when it is gone: then
	 * the page is logged out.
	 */
	removeAccount(
		proof: string,
		code?: string,
		passkey?: PasskeyProof
	): Promise<RemoveRefusal | null>;
}

/** The tabs of the account settings, in the order they are shown. */
export const accountTabs = [
	'password',
	'email',
	'security',
	'apps',
	'invites',
	'move',
	'privacy',
	'remove'
] as const;

export type AccountTab = (typeof accountTabs)[number];

/** The ones there are, with these settings and these app passwords. */
export function tabsOf(
	reader: Reader,
	settings: AccountSettings | null,
	apps: AppPasswordKeeper | null
): AccountTab[] {
	return accountTabs.filter((name) => {
		switch (name) {
			case 'password':
			case 'security':
			case 'invites':
			case 'remove':
				return settings !== null;
			case 'email':
				return settings?.can.email === true;
			case 'apps':
				return apps !== null;
			case 'move':
				return reader.can.move;
			// Privacy only says something where the server takes reports of errors.
			case 'privacy':
				return reader.can.reports && reader.reportsConsent?.available === true;
		}
	});
}
