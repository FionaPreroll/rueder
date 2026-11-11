// What the page needs from whatever holds the feeds, the articles and what was
// read. The components get their data from a `Reader` and nowhere else. There are
// two: the one that talks to the server (api/reader.svelte.ts) and the one with
// made-up data that lives in the browser (demo/reader.svelte.ts), for tests and for
// the build that runs without a server.

import type { FilterRule, RuleScope } from './filters.ts';
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
	/** Feeds can be subscribed to. */
	subscribe: boolean;
}

/** The filter rules of the account and what they catch. */
export interface RuleBook {
	readonly rules: FilterRule[];
	saveRule(rule: FilterRule): void;
	deleteRule(id: string): void;
	newRuleId(): string;
	/** How many of the articles in its scope a rule finds, whether it is on or not. */
	countMatches(rule: FilterRule): number;
	/** How many articles there are in a rule's scope. */
	countIn(scope: RuleScope): number;
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
	/** The filter rules, if this reader has them. */
	readonly filters: RuleBook | null;
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
	subscribe(address: string): void;
}
