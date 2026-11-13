// The apps of other makers that work with rueder, for the settings' "Apps"
// tab. Only apps that were checked are here: their way of logging in, syncing
// and changing is reproduced in server/tests/greader.rs from their source, or
// they were set up on a device. Each link goes to the app's own page.

export type Platform = 'apple' | 'android';

export interface KnownApp {
	id: 'netnewswire' | 'readyou';
	name: string;
	platforms: Platform[];
	url: string;
	/**
	 * The app puts the paths of the API right after the server address, so the
	 * address it is given ends with a slash (Read You does this).
	 */
	slash: boolean;
}

export const knownApps: KnownApp[] = [
	{
		id: 'netnewswire',
		name: 'NetNewsWire',
		platforms: ['apple'],
		url: 'https://netnewswire.com/',
		slash: false
	},
	{
		id: 'readyou',
		name: 'Read You',
		platforms: ['android'],
		url: 'https://github.com/ReadYouApp/ReadYou/releases',
		slash: true
	}
];

/** The address an app is given: this server's, with the API's path. */
export function serverAddress(origin: string, slash: boolean): string {
	return `${origin}/greader${slash ? '/' : ''}`;
}
