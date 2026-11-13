// What the page needs from whatever holds the feeds, the articles and what was
// read. The components get their data from a `Reader` and nowhere else. There are
// two: the one that talks to the server (api/reader.svelte.ts) and the one with
// made-up data that lives in the browser (demo/reader.svelte.ts), for tests and for
// the build that runs without a server.

import type { FilterRule } from './filters.ts';
import type { Settings } from './settings.svelte.ts';
import type { Article, ArticleVersion, Feed, Label, Stream, TreeNode } from './model.ts';
import type { DropPosition, NudgeDirection } from './tree.ts';
import type { Invites, ReportKind, ServerStatus } from './api/types.ts';

export type { Invites, ServerStatus };

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
	/**
	 * Whether it logs in with a password; one that logs in only through single
	 * sign-on has none. Left out, it has one.
	 */
	password?: boolean;
	/** An admin whose right waits for a login with a second factor, as the server wants. */
	adminNeedsSecondFactor?: boolean;
	/** The version of the terms of use the account has yet to agree to, if any. */
	termsToAccept?: string | null;
	/** The server takes reports of feeds and articles from the account. */
	contentReports?: boolean;
}

/** Why a feed or an article is reported, as the server knows them. */
export const reportReasons = ['illegal', 'copyright', 'malicious', 'spam', 'other'] as const;
export type ReportReason = (typeof reportReasons)[number];
/** How a report went. */
export type ReportOutcome = 'sent' | 'tooMany' | 'unreachable';
/** The longest note of a report, in characters, as the server takes it. */
export const MAX_REPORT_NOTE = 500;

/** Where the operator's legal texts are; what is null, the operator did not name. */
export interface LegalLinks {
	imprint: string | null;
	privacy: string | null;
	terms: string | null;
}

/** Why the account was not deleted. The page has the words. */
export type RemoveRefusal =
	/** The password, or the name for an account without one, or the code was not right. */
	| { problem: 'wrong' }
	/** The password was right; the account has a second factor, whose code is missing. */
	| { problem: 'code' }
	/** Too many wrong tries; wait so many minutes. */
	| { problem: 'busy'; minutes: number }
	| { problem: 'unreachable' };

/** Whether the account has a second factor, and what the server wants of it. */
export interface SecondFactorState {
	on: boolean;
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
export type SecondFactorRefusal = 'wrongPassword' | 'wrongCode' | 'tooMany' | 'unreachable';

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
	/**
	 * The whole account can go out as a file and come in again, for moving to
	 * another rueder: settings, folders, feeds, what was read, stars, labels, rules.
	 */
	move: boolean;
	/** The status of the server can be shown: for admins. */
	status: boolean;
	/** Others can be invited with a link that lets them make an account. */
	invites: boolean;
	/** Errors of the browser can be reported to the server, if the reader agrees. */
	reports: boolean;
	/** The account can have a second factor: codes of an authenticator app. */
	secondFactor: boolean;
	/** The account can delete itself, with everything that is its own. */
	remove: boolean;
	/** Feeds and articles can be reported to the admins. */
	reportContent: boolean;
}

/** A feed found at an address. */
export interface FeedCandidate {
	/** The address of the feed itself, which is what is subscribed to. */
	url: string;
	/** Its name, as far as the feed or the page says; it may be empty. */
	title: string;
	/** The feed in the tree, if it is subscribed to already. */
	feedId: string | null;
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
export type Found =
	| {
			feeds: FeedCandidate[];
			/** Where the site sent the request on to, if it did; often to its login. */
			movedTo: string | null;
	  }
	| Refusal;

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

/** What bringing in the file of an account did. */
export type Moved =
	| {
			/** Feeds read now that were not before. */
			feeds: number;
			/** Feeds of the file that were read already. */
			known: number;
			/** Folders that were made. */
			folders: number;
			/** Articles of the file that were found on this server. */
			articles: number;
			/** Articles that were not found, so what was read, starred or labelled of them is lost. */
			missing: number;
			/** Labels that were made. */
			labels: number;
			/** Filter rules that were added. */
			rules: number;
			/** Entries of the file that were left out. */
			skipped: number;
			/** Whether the settings came with it. */
			settings: boolean;
	  }
	| Refusal;

/** The largest file of an account a server takes, in bytes. */
export const MAX_ACCOUNT_FILE = 32 * 1024 * 1024;

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

/** What the main pane showed before the page was loaded anew, to show it again. */
export interface Resumed {
	stream: Stream;
	unreadOnly: boolean;
	newestFirst: boolean;
	cursor: string | null;
	open: string | null;
	/** Articles the list should hold again, such as the one that was at its top. */
	wanted: string[];
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
	/** The operator's legal texts, where the reader has a server that names them. */
	readonly legal: LegalLinks | null;
	/** Reports a feed, or one of its articles, to the admins. */
	reportContent(
		target: { feedId: string; seq?: number },
		reason: ReportReason,
		note: string
	): Promise<ReportOutcome>;
	/** Agrees to the version of the terms the account was asked about; false if that did not work. */
	acceptTerms(): Promise<boolean>;

	/** The account's second factor, where the reader has one (`can.secondFactor`). */
	secondFactor(): Promise<SecondFactorState | null>;
	/** Starts setting it up, with the account's password. */
	setUpSecondFactor(password: string): Promise<SecondFactorSetup | SecondFactorRefusal>;
	/** Confirms it with a code of the app: the recovery codes, shown this once. */
	confirmSecondFactor(code: string): Promise<string[] | SecondFactorRefusal>;
	/** Switches it off, with the account's password; null when it worked. */
	switchOffSecondFactor(password: string): Promise<SecondFactorRefusal | null>;
	/** The filter rules. */
	readonly filters: RuleBook;
	/** Only the reader with made-up data has these. */
	readonly demo: DemoSwitches | null;

	/**
	 * Keeps the settings with the account, where there is one; called with them
	 * whenever they change. The reader puts what the account has into `settings`.
	 */
	saveSettings(settings: Settings): void;

	/** Starts what goes on in the background; returns how to stop it. */
	start(): () => void;

	unreadIn(node: TreeNode): number;
	isRead(article: Article): boolean;
	isStarred(article: Article): boolean;
	isHidden(article: Article): boolean;
	/** The name of a feed or a folder; null for the fixed lists. */
	title(stream?: Stream): string | null;

	show(stream: Stream): void;
	/**
	 * Shows a stream as it was left, with its filters and its open article, before
	 * `start`. The list is loaded far enough to hold the wanted articles and the
	 * open one, as far as they are still in it.
	 */
	resume(view: Resumed): void;
	/** Whether `resume` is still looking for the articles it wants. */
	readonly resuming: boolean;
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
	/**
	 * Whether a folder of the tree is folded away in the sidebar. The sidebar's own
	 * sections use a name instead of a folder's id, `labels` for one.
	 */
	isClosed(id: string): boolean;
	setClosed(id: string, closed: boolean): void;
	addFolder(title: string): void;
	/** Whether dropping the feed `id` on the feed `targetId` can put both into a new folder. */
	canGroup(id: string, targetId: string): boolean;
	/** Puts the feed `targetId` and then the feed `id` into a new folder where the target was. */
	group(id: string, targetId: string, title: string): void;
	renameFolder(id: string, title: string): void;
	removeFolder(id: string): void;

	/** The account's labels, in their order. */
	readonly labels: Label[];
	/** The labels an article has, by their ids, in the order of the labels. */
	labelsOf(article: Article): string[];
	setLabel(article: Article, labelId: string, on: boolean): void;
	/** Makes a label, and with `article` puts it on that article too. */
	addLabel(title: string, color: string, article?: Article): void;
	changeLabel(id: string, change: { title?: string; color?: string }): void;
	/** Takes a label off every article and deletes it. */
	removeLabel(id: string): void;

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
	/** The whole account as a file for another rueder (`can.move`); null if there was no answer. */
	exportAccount(): Promise<string | null>;
	/**
	 * Brings in the file of an account, from this rueder or another: adds what it
	 * has and replaces the settings, then shows it all anew.
	 */
	importAccount(file: Blob): Promise<Moved>;

	/** The reader's invitations and how many more they can make (`can.invites`); null if there was no answer. */
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
	 * Deletes the account for good (`can.remove`), with the password, or the
	 * account's name where it has none, and the code of its second factor where
	 * it has one. Null when it is gone: then the page is logged out.
	 */
	removeAccount(proof: string, code?: string): Promise<RemoveRefusal | null>;

	/** How the server fares, for an admin (`can.status`); null if there was no answer. */
	status(): Promise<ServerStatus | null>;

	/**
	 * Whether the server takes reports of errors from browsers and whether this
	 * account sends them (`can.reports`); null until known.
	 */
	readonly reportsConsent: { available: boolean; on: boolean } | null;
	/** Agrees to send reports of errors, or no longer; no longer deletes what was sent. */
	setReportsConsent(on: boolean): void;
	/** Notes an error of the page, if the account sends reports. */
	noteError(kind: ReportKind, error: unknown): void;
}
