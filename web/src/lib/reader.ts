// What the page needs from whatever holds the feeds, the articles and what was
// read. The components get their data from a `Reader` and nowhere else. There are
// two: the one that talks to the server (api/reader.svelte.ts) and the one with
// made-up data that lives in the browser (demo/reader.svelte.ts), for tests and for
// the build that runs without a server.

import type { FilterRule } from './filters.ts';
import type { Article, ArticleVersion, Feed, Stream, TreeNode } from './model.ts';
import type { DropPosition, NudgeDirection } from './tree.ts';

/** A feed with something unread, for the start page. */
export interface Fresh {
	feed: Feed;
	/** How many of its articles are unread. */
	count: number;
	/** Its newest unread articles, newest first. */
	top: Article[];
}

/** Who is logged in. */
export interface Account {
	name: string;
	admin: boolean;
}

/** What went wrong, for a line the reader can close. The page has the words. */
export type Notice =
	/** The server did not answer, or not with what was asked for. */
	| 'unreachable'
	/** The server refused a change; it was taken back. */
	| 'notSaved';

/** What a reader can do. The page leaves out the controls for the rest. */
export interface Abilities {
	/** Lists can start with the oldest article. */
	oldestFirst: boolean;
	/** "Mark as read" can stop at articles younger than a day or a week. */
	markOlder: boolean;
	/** Feeds can be found and subscribed to. */
	subscribe: boolean;
	/** Subscriptions can be taken in from an OPML file and given out as one. */
	opml: boolean;
}

/** A feed found at an address. */
export interface FeedCandidate {
	/** The address of the feed itself, which is what is subscribed to. */
	url: string;
	/** Its name, as far as the feed or the page says; it may be empty. */
	title: string;
}

/** Why looking for feeds, subscribing or an import did not work. The page has the words. */
export type Refusal =
	/**
	 * The server said no: not an address, nothing to fetch there, not a feed, not
	 * an OPML file. `reason` is what the server said, in its own words.
	 */
	| { problem: 'refused'; reason: string }
	/** Too many lookups in a short time; wait `retryAfter` seconds, if the server said. */
	| { problem: 'busy'; retryAfter: number | null }
	/** This server does not fetch feeds. */
	| { problem: 'noFetching' }
	/** The file is larger than the server takes. */
	| { problem: 'tooLarge' }
	/** The server did not answer, or not with what was asked for. */
	| { problem: 'unreachable' };

/** The feeds at an address: the feed itself, or what a web page names. */
export type Found = { feeds: FeedCandidate[] } | Refusal;

/** A subscription that holds now; `known` if it held before. */
export type Subscribed = { feedId: string; known: boolean } | Refusal;

/** What an import of an OPML file did. */
export type Imported =
	| {
			/** Feeds read now that were not before. */
			feeds: number;
			/** Folders that were made. */
			folders: number;
			/** Feeds of the file that were read already. */
			known: number;
			/** Entries of the file that were left out. */
			skipped: number;
	  }
	| Refusal;

/** The largest OPML file a server takes, in bytes. */
export const MAX_OPML = 1024 * 1024;

/**
 * What a rule would catch if it were saved now, or why nobody can say: its pattern
 * cannot be used, or the server did not answer.
 */
export type RulePreview =
	| { matches: number; of: number }
	/** The pattern is not one, a broken regular expression for one. */
	| 'invalid'
	| 'unreachable';

/** The filter rules of the account and what they catch. */
export interface RuleBook {
	readonly rules: FilterRule[];
	saveRule(rule: FilterRule): void;
	deleteRule(id: string): void;
	newRuleId(): string;
	/**
	 * Tries a rule that is being written. A reader that has to ask its server
	 * answers later; this is also how the page learns whether a pattern is one, as
	 * the server's regular expressions are not the browser's.
	 */
	preview(rule: FilterRule): RulePreview | Promise<RulePreview>;
	/** How many articles a saved rule finds; null as long as nobody has counted. */
	hitsOf(rule: FilterRule): number | null;
	/** Counts what the saved rules find, where that has to be asked for. */
	countHits(): void;
	/** The first enabled rule that hides the article, to say why it is hidden. */
	hidingRule(article: Article): FilterRule | null;
}

/** The switches of the reader with made-up data, to try what a server would do. */
export interface DemoSwitches {
	/** Whether made-up articles keep arriving. */
	live: boolean;
	arrive(): unknown;
	revise(article: Article, note: string): unknown;
}

export interface Reader {
	readonly tree: TreeNode[];
	readonly feeds: Record<string, Feed>;

	/** What the main pane shows. */
	readonly stream: Stream;
	unreadOnly: boolean;
	newestFirst: boolean;
	/** The words the list is searched for. */
	query: string;
	/** The article the cursor is on, and the one that is open in the list view. */
	readonly cursor: string | null;
	open: string | null;
	/** Counts the steps of the cursor, so the list can scroll to where it went. */
	readonly moved: number;
	/** The part of the list that is on the page, in reading order. */
	readonly visible: Article[];
	/** Whether the list goes on after what is on the page. */
	readonly hasMore: boolean;
	/** Whether what the main pane shows is still on its way. */
	readonly loading: boolean;
	/** Articles that arrived since the list was drawn. */
	readonly pending: number;
	/** The article the cursor is on. */
	readonly current: Article | null;

	readonly totalUnread: number;
	readonly hiddenCount: number;
	/** For the start page: feeds with something unread, and the latest stars. */
	readonly fresh: Fresh[];
	readonly latestStarred: Article[];

	/** What went wrong last, until it is closed. */
	readonly notice: Notice | null;
	dismissNotice(): void;

	readonly can: Abilities;
	/** Who is logged in; null where there is nobody to log in. */
	readonly account: Account | null;
	signOut(): void;
	/** The filter rules. */
	readonly filters: RuleBook;
	/** Only the reader with made-up data has these. */
	readonly demo: DemoSwitches | null;

	/** Starts what goes on in the background; returns how to stop it. */
	start(): () => void;

	unreadIn(node: TreeNode): number;
	isRead(article: Article): boolean;
	isStarred(article: Article): boolean;
	isHidden(article: Article): boolean;
	/** The name of a feed or a folder; null for the fixed lists. */
	title(stream?: Stream): string | null;

	show(stream: Stream): void;
	/** Brings in the articles that arrived since the list was drawn. */
	showNew(): void;
	refresh(): void;
	showMore(): void;

	/** Fetches the article's text if it is not there yet. */
	load(article: Article): void;
	/** The article's earlier versions, oldest first. */
	versions(article: Article): Promise<ArticleVersion[]>;

	setRead(article: Article, read: boolean): void;
	/** The article scrolled out at the top of the expanded view. */
	scrolledPast(article: Article): void;
	toggleStar(article: Article): void;
	/** Puts the cursor on an article; opening it marks it as read. */
	select(article: Article, open?: boolean): void;
	toggleOpen(article: Article): void;
	/** Moves the cursor by one article, down the list for +1. */
	step(by: 1 | -1, open?: boolean): void;
	/** Marks the stream's articles as read; with `olderThan`, only those older than that many milliseconds. */
	markAllRead(olderThan?: number): void;

	canMove(id: string, targetId: string, position: DropPosition): boolean;
	move(id: string, targetId: string, position: DropPosition): void;
	/** Moves a node one step with the keyboard; false when there is nowhere to go. */
	nudge(id: string, direction: NudgeDirection): boolean;
	isClosed(folderId: string): boolean;
	setClosed(folderId: string, closed: boolean): void;
	addFolder(title: string): void;
	renameFolder(id: string, title: string): void;
	removeFolder(id: string): void;

	/** Looks at an address, which may lack its `https://`, for feeds. */
	findFeeds(address: string): Promise<Found>;
	/**
	 * Subscribes to a feed found by `findFeeds`, at the end of a folder or of the top
	 * level, and shows it.
	 */
	subscribe(url: string, folderId: string | null): Promise<Subscribed>;
	/** Gives a feed a name of the reader's own; an empty one goes back to the feed's. */
	renameFeed(id: string, title: string): void;
	/** Ends a subscription, and with it what was read and starred there. */
	unsubscribe(id: string): void;
	/** Takes the subscriptions of an OPML file in; folders are found by their name or made. */
	importOpml(file: Blob): Promise<Imported>;
	/** All subscriptions as an OPML file; null if there was no answer. */
	exportOpml(): Promise<string | null>;
}
