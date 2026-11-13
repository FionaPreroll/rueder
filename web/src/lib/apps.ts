// The apps of other makers that work with rueder, for the settings' "Apps"
// tab. Only apps that were checked are here: their way of logging in, syncing
// and changing is reproduced in server/tests/greader.rs from their source, or
// they were set up on a device. Each link goes to the app's own page.

import { asset } from '$app/paths';
import type { AssetPath } from '$app/types';

export type Platform = 'apple' | 'android';

/** A picture of a step in an app: its size in pixels. */
export interface Shot {
	width: number;
	height: number;
}

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
	 * The app loads the icons of the feeds itself, from their websites, so
	 * that those see the reader's address (NetNewsWire does this); the setup
	 * says so.
	 */
	ownIcons: boolean;
	/**
	 * The pictures that show the way in the app, step by step: screenshots in
	 * `static/apps/<id>-<n>.webp`, from 1 on, each with its caption in the
	 * texts. None where there are none yet. They are made at twice the size
	 * they are shown at.
	 */
	shots: Shot[];
}

/**
 * Where the picture of a step of an app is, from 1 on: below the address the
 * page is served from, which is not always the top of the site (the demo on
 * GitHub Pages is at /rueder/demo/).
 */
export const shot = (app: KnownApp, n: number) => asset(`apps/${app.id}-${n}.webp` as AssetPath);

export const knownApps: KnownApp[] = [
	{
		id: 'netnewswire',
		name: 'NetNewsWire',
		platforms: ['apple'],
		url: 'https://netnewswire.com/',
		slash: false,
		// It reads no `iconUrl` and looks for each feed's icon on its home
		// page (FaviconDownloader.swift).
		ownIcons: true,
		// On the Mac.
		shots: [
			{ width: 640, height: 585 },
			{ width: 640, height: 775 },
			{ width: 640, height: 406 }
		]
	},
	{
		id: 'readyou',
		name: 'Read You',
		platforms: ['android'],
		url: 'https://f-droid.org/packages/me.ash.reader/',
		slash: true,
		ownIcons: false,
		shots: Array.from({ length: 5 }, () => ({ width: 360, height: 718 }))
	}
];

/** The address an app is given: this server's, with the API's path. */
export function serverAddress(origin: string, slash: boolean): string {
	return `${origin}/greader${slash ? '/' : ''}`;
}
