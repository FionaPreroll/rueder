// What the reader shows: feeds in a tree of folders, articles with their versions.
// They come from the server (api/), or from made-up data (demo/) in tests and
// in the build that runs without a server.

/** Folders nest this deep at most; feeds may sit in a folder of the last level. */
export const MAX_FOLDER_DEPTH = 5;

export interface Folder {
	kind: 'folder';
	id: string;
	title: string;
	children: TreeNode[];
}

/** A feed's place in the tree; the feed itself is in `Feed`. */
export interface Subscription {
	kind: 'feed';
	id: string;
}

export type TreeNode = Folder | Subscription;

export interface Feed {
	id: string;
	title: string;
	siteUrl: string;
	/** Hue of the stand-in for the feed's icon, 0 to 360. */
	hue: number;
	/** Where the feed's icon is, at the server's image proxy; without one the stand-in shows. */
	icon?: string | null;
	/** Number of the feed's newest article; articles are numbered from 1 as they arrive. */
	lastSeq: number;
	/** Why the last fetch failed, or null when it worked. */
	error: string | null;
	/** The feed waits for its first fetch, as one does that came with an OPML file. */
	waiting?: boolean;
}

export interface ArticleVersion {
	rev: number;
	fetchedAt: number;
	title: string;
	html: string;
}

/** A file that comes with an article, like the episode of a podcast. */
export interface Enclosure {
	url: string;
	/** Its media type, such as `audio/mpeg`, if the feed says. */
	type: string | null;
	/** Its size in bytes, if the feed says. */
	length: number | null;
}

/** The picture a feed gives for an article, apart from its text. */
export interface Picture {
	url: string;
	title: string;
}

export interface Article {
	id: string;
	feedId: string;
	/** The article's number in its feed. */
	seq: number;
	title: string;
	teaser: string;
	author: string;
	tags: string[];
	link: string;
	publishedAt: number;
	/**
	 * The cleaned text of the current version. Lists come without it: null until the
	 * article is opened and `Reader.load` has fetched it. The tags come with the text.
	 */
	html: string | null;
	/** The files and the picture that come with the article. Like the text, they are there once it is opened. */
	enclosures: Enclosure[];
	image: Picture | null;
	/** The rule that hides the article, where the server names it; it comes with the text. */
	hiddenBy?: string | null;
	/** How many earlier versions there are; 0 for an article that never changed. */
	changes: number;
	/** When the current version replaced the one before, or null. */
	updatedAt: number | null;
}

/** What the main pane shows. */
export type Stream =
	| { kind: 'home' }
	| { kind: 'all' }
	| { kind: 'starred' }
	| { kind: 'hidden' }
	| { kind: 'folder'; id: string }
	| { kind: 'feed'; id: string };

export const sameStream = (a: Stream, b: Stream) =>
	a.kind === b.kind && ('id' in a ? a.id : '') === ('id' in b ? b.id : '');
