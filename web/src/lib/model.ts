// What the reader shows: feeds in a tree of folders, articles with their versions.
// The draft fills these from made-up data (demo/); later they come from the server.

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
	/** Number of the feed's newest article; articles are numbered from 1 as they arrive. */
	lastSeq: number;
	/** Why the last fetch failed, or null when it worked. */
	error: string | null;
}

export interface ArticleVersion {
	rev: number;
	fetchedAt: number;
	title: string;
	html: string;
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
	/** The cleaned text of the current version. */
	html: string;
	/** Earlier versions, oldest first; empty for an article that never changed. */
	versions: ArticleVersion[];
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
