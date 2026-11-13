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
	/**
	 * How many pictures show the way in the app, step by step: screenshots in
	 * `static/apps/<id>-<n>.webp`, from 1 on, each with its caption in the
	 * texts. None where there are none yet.
	 */
	shots: number;
}

/** Where the picture of a step of an app is, from 1 on. */
export const shot = (app: KnownApp, n: number) => `/apps/${app.id}-${n}.webp`;

export const knownApps: KnownApp[] = [
	{
		id: 'netnewswire',
		name: 'NetNewsWire',
		platforms: ['apple'],
		url: 'https://netnewswire.com/',
		slash: false,
		shots: 0
	},
	{
		id: 'readyou',
		name: 'Read You',
		platforms: ['android'],
		url: 'https://f-droid.org/packages/me.ash.reader/',
		slash: true,
		shots: 5
	}
];

/** The address an app is given: this server's, with the API's path. */
export function serverAddress(origin: string, slash: boolean): string {
	return `${origin}/greader${slash ? '/' : ''}`;
}
