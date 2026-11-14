// Which of the account's own settings a reader has. The dialog
// (AccountDialog.svelte) shows a tab per entry, and the menu of the account
// offers the dialog only where there is at least one.

import type { Reader } from './reader.ts';

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

/** The ones this reader can show. */
export function tabsOf(reader: Reader): AccountTab[] {
	return accountTabs.filter((name) => {
		switch (name) {
			case 'password':
				return reader.can.password && reader.account !== null;
			case 'email':
				return reader.can.email && reader.account !== null;
			case 'security':
				return reader.can.secondFactor;
			case 'apps':
				return reader.can.apps;
			case 'invites':
				return reader.can.invites;
			case 'move':
				return reader.can.move;
			// Privacy only says something where the server takes reports of errors.
			case 'privacy':
				return reader.can.reports && reader.reportsConsent?.available === true;
			case 'remove':
				return reader.can.remove && reader.account !== null;
		}
	});
}
