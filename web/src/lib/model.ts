// What the reader shows: feeds in a tree of folders, articles with their versions.
// They come from the server (api/), or from made-up data (demo/) in tests and
// in the build that runs without a server.

/**
 * What was different when an article last changed. A feed that only rewrites a
 * headline leaves nothing to see in the text, and the list says so.
 */
export type Changed = 'title' | 'text' | 'both';

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
	/** The operator blocked the feed: nobody sees its articles, and it is not fetched. */
	blocked?: boolean;
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
	/**
	 * The picture a list shows the article by, as lists come: from the feed or the
	 * first in the text. Without a server's lists, as in tests, it may be missing.
	 */
	thumbnail?: string | null;
	/** The rule that hides the article, where the server names it; it comes with the text. */
	hiddenBy?: string | null;
	/**
	 * The server keeps the article's line but no longer its text, as its
	 * operator chose for old articles; it comes with the text.
	 */
	textDropped?: boolean;
	/** How many earlier versions there are; 0 for an article that never changed. */
	changes: number;
	/**
	 * What was different when it last changed, where the server noted it: a feed
	 * that only rewrites a headline leaves nothing to see in the text. Null for a
	 * change from before rueder noted it.
	 */
	changed?: Changed | null;
	/** When the current version replaced the one before, or null. */
	updatedAt: number | null;
}

/** A reader's own mark for articles, with a name and a color. */
export interface Label {
	id: string;
	title: string;
	/** As `#rrggbb`, one of `labelColors`. */
	color: string;
}

/** The colors a label can have; the server takes any `#rrggbb`. */
export const labelColors = [
	'#d93025',
	'#e8710a',
	'#f9ab00',
	'#1e8e3e',
	'#12b5cb',
	'#1a73e8',
	'#9334e6',
	'#5f6368'
] as const;

/** The color a new label takes, the next in turn after those there are. */
export const nextLabelColor = (labels: number) => labelColors[labels % labelColors.length];

/** What the main pane shows. */
export type Stream =
	| { kind: 'home' }
	| { kind: 'all' }
	/** All that is unread; an article read here goes once the reader leaves it. */
	| { kind: 'unread' }
	| { kind: 'starred' }
	| { kind: 'hidden' }
	| { kind: 'folder'; id: string }
	| { kind: 'feed'; id: string }
	| { kind: 'label'; id: string };

export const sameStream = (a: Stream, b: Stream) =>
	a.kind === b.kind && ('id' in a ? a.id : '') === ('id' in b ? b.id : '');
