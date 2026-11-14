// The server's API as functions: one per endpoint the page uses. The shapes of
// what the server sends and reads come from types.ts, which the server writes
// out of its Rust types (see server/src/typescript.rs).
//
// The session is a cookie the browser keeps and scripts cannot read, so nothing
// here knows a token. Every address is rueder's own.

import type { Stream } from '#lib/model.ts';
import type { DropPosition } from '#lib/tree.ts';
import type {
	AccountImported,
	AppPasswords as AppPasswordsJson,
	Awaiting,
	ConfirmEmail,
	Confirmed,
	EmailProof,
	EmailState,
	NewEmail,
	Pending,
	Resend,
	CreatedAppPassword,
	NewAppPassword as NewAppPasswordJson,
	Article,
	ArticleVersion,
	ClientReport,
	ClientReports,
	Created,
	ContentReport,
	Credentials,
	Farewell,
	NewPassword,
	ForgotPassword,
	ResetPassword,
	Feed,
	FindFeeds,
	Folder,
	FoundFeeds,
	Invites,
	Label,
	LabelChange,
	List,
	ListItem,
	LoginWays,
	MadeInvite,
	Me,
	Move,
	NewFolder,
	NewInvite,
	NewLabel,
	NewRegistration,
	NewSubscription,
	NodeRef,
	OpmlImported,
	NewPasskey,
	PasskeyAnswer,
	PasskeyChallenge,
	PasskeyRegistration,
	Passkeys as PasskeysJson,
	RegisteredPasskey,
	RemovePasskey,
	Registering,
	Rename,
	ReportsConsent,
	ReportsConsentChange,
	Rule,
	RuleOutcome,
	RulePreview,
	RuleScope,
	RulesHits,
	RecoveryCodes,
	SearchHits,
	SecondFactor,
	SecondFactorCode,
	SecondFactorPassword,
	SecondFactorSwitchOff,
	SecondFactorSetup,
	TermsAgreement,
	ServerStatus,
	StateChange,
	Subscribed,
	Tree,
	Upto,
	Versions
} from './types.ts';

/** A request the server refused, or one it did not answer (`status` 0). */
export class ApiError extends Error {
	constructor(
		readonly status: number,
		message: string,
		/** Seconds to wait, if the server said so (429). */
		readonly retryAfter: number | null = null,
		/** What was asked, as "GET /feeds/12/articles", for a report of the error. */
		readonly request = ''
	) {
		super(message);
		this.name = 'ApiError';
	}

	/** The session is over, or there never was one. */
	get signedOut() {
		return this.status === 401;
	}
}

// The shapes of what the server sends, under the names the page knows them by.
// They are the server's own: types.ts is written out of its Rust types.
export type ApiAccount = Me;
export type ApiLoginWays = LoginWays;
export type ApiFolder = Folder;
export type ApiFeed = Feed;
export type ApiTree = Tree;
export type ApiListItem = ListItem;
export type ApiArticle = Article;
export type ApiVersion = ArticleVersion;
export type ApiRuleScope = RuleScope;
export type ApiRule = Rule;
/** A new rule is sent without its id. */
export type ApiNewRule = Omit<Rule, 'id'>;
export type ApiRulePreview = RulePreview;
export type ApiNode = NodeRef;
export type ApiLabel = Label;

/** Which part of a list is asked for. */
export interface PageQuery {
	/** Only what arrived before the article of this id. */
	before?: number;
	/** Only what arrived after the article of this id, the oldest first; 0 starts at the oldest. */
	after?: number;
	limit: number;
	/** Leave out what was read. */
	unread?: boolean;
}

/** The lists of articles: a feed's, a folder's, everything, the starred and the hidden. */
export type ListStream = Exclude<Stream, { kind: 'home' }>;

/** What a search on the server goes through: everything, a feed or a folder. */
export type SearchScope = Extract<Stream, { kind: 'all' | 'feed' | 'folder' }>;

/** Which part of what a search found is asked for. */
export interface SearchPage {
	/** Where to go on: `next` of the page before. */
	before?: number;
	limit: number;
	/** Leave out what was read. */
	unread?: boolean;
}

/** The number in an id of the page: 12 for "feed-12", which is feed 12 of the server. */
export const serverId = (id: string) => Number(id.slice(id.lastIndexOf('-') + 1));

const listPath = (stream: ListStream) => {
	switch (stream.kind) {
		case 'all':
		case 'unread':
			return '/articles';
		case 'starred':
			return '/starred';
		case 'hidden':
			return '/hidden';
		case 'feed':
			return `/feeds/${serverId(stream.id)}/articles`;
		case 'folder':
			return `/folders/${serverId(stream.id)}/articles`;
		case 'label':
			return `/labels/${serverId(stream.id)}/articles`;
	}
};

/** Marking as read goes by feed, by folder or for everything. */
const readPath = (stream: ListStream) => {
	switch (stream.kind) {
		case 'feed':
			return `/feeds/${serverId(stream.id)}/read`;
		case 'folder':
			return `/folders/${serverId(stream.id)}/read`;
		default:
			return '/read';
	}
};

/** How much of a refusal's text is kept. */
const MAX_REASON = 300;

/** An OPML file as the server takes it. */
const OPML = 'text/x-opml';

/**
 * This tab's name, which every request carries in `X-Rueder-Tab`. The server
 * names it again in the event a change brings (`by`), so the tab knows its own
 * changes when they come back.
 */
export const TAB = Array.from(crypto.getRandomValues(new Uint8Array(8)), (byte) =>
	byte.toString(16).padStart(2, '0')
).join('');

/** What the page needs of an `EventSource`; the tests put a made-up one in its place. */
export interface EventStream {
	addEventListener(type: string, listener: (event: MessageEvent<string>) => void): void;
	close(): void;
	readonly readyState: number;
	onerror: ((event: Event) => void) | null;
}

/** The `readyState` of a stream that will not come back by itself. */
export const CLOSED = 2;

/**
 * Sends a request and gives back the answer, if the server took it. A body is
 * sent as JSON, a `Blob` as it is, as an OPML file unless `kind` says otherwise.
 */
async function send(
	method: string,
	path: string,
	body?: unknown,
	/** The request may outlive the page, as one sent while it is left. */
	keepalive = false,
	/** What a `Blob` is said to be. */
	kind = OPML
): Promise<Response> {
	const file = body instanceof Blob;
	let response: Response;
	try {
		response = await fetch(`/api${path}`, {
			method,
			credentials: 'same-origin',
			keepalive,
			headers: {
				'X-Rueder-Tab': TAB,
				...(body === undefined ? {} : { 'Content-Type': file ? kind : 'application/json' })
			},
			body: body === undefined ? undefined : file ? body : JSON.stringify(body)
		});
	} catch {
		throw new ApiError(0, 'the server did not answer', null, asked(method, path));
	}
	if (!response.ok) {
		const reason = (await response.text().catch(() => '')).slice(0, MAX_REASON);
		const wait = Number(response.headers.get('Retry-After'));
		throw new ApiError(response.status, reason, wait > 0 ? wait : null, asked(method, path));
	}
	return response;
}

/** A request as a report names it: without what follows the `?`. */
const asked = (method: string, path: string) => `${method} ${path.split('?')[0]}`;

async function call<T>(method: string, path: string, body?: unknown, kind?: string): Promise<T> {
	const response = await send(method, path, body, false, kind);
	if (response.status === 204) return undefined as T;
	try {
		return (await response.json()) as T;
	} catch {
		// Something else answered in the server's place, a proxy's error page for one.
		throw new ApiError(0, 'the answer was not what the server sends', null, asked(method, path));
	}
}

/** An answer that is a text, not JSON. */
async function text(method: string, path: string): Promise<string> {
	const response = await send(method, path);
	try {
		return await response.text();
	} catch {
		throw new ApiError(0, 'the answer was cut short');
	}
}

export const api = {
	/** This tab's name, as the requests carry it. */
	tab: TAB,
	/**
	 * The live updates: one connection that stays open and comes back by itself
	 * after a break, saying the last event it got. Null where the browser has none.
	 */
	events: (): EventStream | null =>
		typeof EventSource === 'undefined' ? null : new EventSource('/api/events'),

	me: () => call<ApiAccount>('GET', '/me'),
	loginWays: () => call<ApiLoginWays>('GET', '/login/ways'),
	login: (username: string, password: string, code?: string) =>
		call<ApiAccount>('POST', '/login', { username, password, code } satisfies Credentials),
	logout: () => call<void>('POST', '/logout'),
	/** Deletes the account that is logged in, for good; the password, or the name of one without. */
	removeAccount: (proof: Farewell) => call<void>('DELETE', '/account', proof),
	/**
	 * Gives the account another password. The server ends every other session of
	 * it and answers with a fresh cookie for this browser.
	 */
	changePassword: (current: string, fresh: string) =>
		call<void>('PUT', '/password', { current, new: fresh } satisfies NewPassword),

	/**
	 * Asks for a link that sets a new password, by the account's name or its
	 * address. The server answers the same, with nothing to read, whoever it is.
	 */
	forgotPassword: async (name: string) => {
		await send('POST', '/password/forgot', { name } satisfies ForgotPassword);
	},
	/** Sets a new password with the token from that link; it does not log in. */
	resetPassword: (token: string, fresh: string) =>
		call<void>('POST', '/password/reset', { token, new: fresh } satisfies ResetPassword),

	/** Whether anybody can register, and whether an invitation can be used; for anybody. */
	registering: (invite?: string) =>
		call<Registering>(
			'GET',
			invite ? `/register?invite=${encodeURIComponent(invite)}` : '/register'
		),
	/**
	 * Makes an account and logs it in; where every account needs a confirmed
	 * address, it is made and waits for its link instead (`Awaiting`). A refusal
	 * is 403 with one word, see `NewRegistration`.
	 */
	register: (
		username: string,
		password: string,
		invite?: string,
		acceptedTerms?: string,
		email?: string,
		language?: string
	) =>
		call<ApiAccount | Awaiting>('POST', '/register', {
			username,
			password,
			...(invite ? { invite } : {}),
			...(acceptedTerms ? { accepted_terms: acceptedTerms } : {}),
			...(email ? { email } : {}),
			...(language ? { language } : {})
		} satisfies NewRegistration),

	/** The account's address and the one that waits for its link. */
	emailState: () => call<EmailState>('GET', '/email'),
	/** Mails a link to a new address, which counts once it is opened. */
	changeEmail: (body: NewEmail) => call<Pending>('PUT', '/email', body),
	/** Takes the address from the account, where it is optional. */
	removeEmail: (proof: EmailProof) => call<void>('DELETE', '/email', proof),
	/** Opens the link from a mail, with or without a session. */
	confirmEmail: (token: string) =>
		call<Confirmed>('POST', '/email/confirm', { token } satisfies ConfirmEmail),
	/** Sends the link again to an account that waits for its address. */
	resendEmail: (username: string, password: string) =>
		call<Pending>('POST', '/email/resend', { username, password } satisfies Resend),

	invites: () => call<Invites>('GET', '/invites'),
	invite: (uses: number) => call<MadeInvite>('POST', '/invites', { uses } satisfies NewInvite),
	revokeInvite: (id: number) => call<void>('DELETE', `/invites/${id}`),

	tree: () => call<ApiTree>('GET', '/tree'),

	list: async (stream: ListStream, page: PageQuery) => {
		const query = new URLSearchParams({ limit: String(page.limit) });
		if (page.before !== undefined) query.set('before', String(page.before));
		if (page.after !== undefined) query.set('after', String(page.after));
		if (page.unread) query.set('unread', 'true');
		const list = await call<List>('GET', `${listPath(stream)}?${query}`);
		return list.articles;
	},

	search: (words: string, scope: SearchScope, page: SearchPage) => {
		const query = new URLSearchParams({ q: words, limit: String(page.limit) });
		if (scope.kind !== 'all') query.set(scope.kind, String(serverId(scope.id)));
		if (page.before !== undefined) query.set('before', String(page.before));
		if (page.unread) query.set('unread', 'true');
		return call<SearchHits>('GET', `/search?${query}`);
	},

	article: (feedId: number, seq: number) =>
		call<ApiArticle>('GET', `/feeds/${feedId}/articles/${seq}`),

	versions: async (feedId: number, seq: number) => {
		const kept = await call<Versions>('GET', `/feeds/${feedId}/articles/${seq}/versions`);
		return kept.versions;
	},

	setState: (feedId: number, seq: number, change: { read?: boolean; starred?: boolean }) =>
		call<void>('PUT', `/feeds/${feedId}/articles/${seq}/state`, change satisfies StateChange),

	/**
	 * Marks as read what arrived in the stream up to the article of this id; with
	 * `olderThan`, only what was published more than that many seconds ago.
	 */
	readUpto: (stream: ListStream, uptoId: number, olderThan?: number) =>
		call<void>(
			'POST',
			readPath(stream),
			(olderThan === undefined
				? { upto_id: uptoId }
				: { upto_id: uptoId, older_than: olderThan }) satisfies Upto
		),

	move: (node: ApiNode, target: ApiNode, position: DropPosition) =>
		call<void>('POST', '/tree/move', { node, target, position } satisfies Move),
	addFolder: (title: string, parentId: number | null = null) =>
		call<Created>('POST', '/folders', { title, parent_id: parentId } satisfies NewFolder),
	renameFolder: (id: number, title: string) =>
		call<void>('PATCH', `/folders/${id}`, { title } satisfies Rename),
	removeFolder: (id: number) => call<void>('DELETE', `/folders/${id}`),

	/** The feeds at an address: the feed itself, or those a web page names. */
	findFeeds: (url: string) => call<FoundFeeds>('POST', '/feeds/find', { url } satisfies FindFeeds),
	/** Subscribes to the feed at an address and gives back its number. */
	subscribe: async (url: string, folderId: number | null) => {
		const body: NewSubscription = { url, folder_id: folderId };
		return (await call<Subscribed>('POST', '/feeds', body)).id;
	},
	/** An empty title gives the feed its own name back. */
	renameFeed: (id: number, title: string) =>
		call<void>('PATCH', `/feeds/${id}`, { title } satisfies Rename),
	unsubscribe: (id: number) => call<void>('DELETE', `/feeds/${id}`),
	importOpml: (file: Blob) => call<OpmlImported>('POST', '/opml', file),
	exportOpml: () => text('GET', '/opml'),
	/** The whole account as a file, for another rueder. */
	exportAccount: () => text('GET', '/export'),
	/** A file of `exportAccount`, from this rueder or another, sent as it is. */
	importAccount: (file: Blob) => call<AccountImported>('POST', '/import', file, 'application/json'),

	/** The settings as the account keeps them; what is in them is the page's business. */
	settings: () => call<unknown>('GET', '/settings'),
	saveSettings: (settings: unknown) => call<void>('PUT', '/settings', settings),

	labels: () => call<ApiLabel[]>('GET', '/labels'),
	addLabel: (title: string, color: string) =>
		call<Created>('POST', '/labels', { title, color } satisfies NewLabel),
	changeLabel: (id: number, change: LabelChange) => call<void>('PATCH', `/labels/${id}`, change),
	removeLabel: (id: number) => call<void>('DELETE', `/labels/${id}`),
	/** Puts a label on an article, or takes it off. */
	setLabel: (feedId: number, seq: number, labelId: number, on: boolean) =>
		call<void>(on ? 'PUT' : 'DELETE', `/feeds/${feedId}/articles/${seq}/labels/${labelId}`),

	rules: () => call<ApiRule[]>('GET', '/rules'),
	addRule: (rule: ApiNewRule) => call<RuleOutcome>('POST', '/rules', rule),
	changeRule: (id: number, rule: ApiNewRule) => call<RuleOutcome>('PUT', `/rules/${id}`, rule),
	removeRule: (id: number) => call<RuleOutcome>('DELETE', `/rules/${id}`),
	/** Tries a rule without storing it; a pattern that cannot be used is refused with 400. */
	previewRule: (rule: ApiNewRule) => call<ApiRulePreview>('POST', '/rules/preview', rule),
	/** What each of the stored rules finds. */
	ruleHits: async () => (await call<RulesHits>('GET', '/rules/hits')).hits,

	/** For admins only: counted over the last `hours`, as far as the server keeps its counts. */
	status: (hours = 1) =>
		call<ServerStatus>('GET', hours > 1 ? `/status?hours=${hours}` : '/status'),

	reportContent: (feedId: number, seq: number | undefined, report: ContentReport) =>
		call<void>(
			'POST',
			seq === undefined ? `/feeds/${feedId}/report` : `/feeds/${feedId}/articles/${seq}/report`,
			report
		),
	acceptTerms: (version: string) =>
		call<void>('PUT', '/terms', { version } satisfies TermsAgreement),

	appPasswords: () => call<AppPasswordsJson>('GET', '/app-passwords'),
	createAppPassword: (body: NewAppPasswordJson) =>
		call<CreatedAppPassword>('POST', '/app-passwords', body),
	revokeAppPassword: (id: number) => call<void>('DELETE', `/app-passwords/${id}`),
	secondFactor: () => call<SecondFactor>('GET', '/second-factor'),
	setUpSecondFactor: (body: SecondFactorPassword) =>
		call<SecondFactorSetup>('POST', '/second-factor/setup', body),
	confirmSecondFactor: (code: string) =>
		call<RecoveryCodes>('POST', '/second-factor/confirm', { code } satisfies SecondFactorCode),
	switchOffSecondFactor: (body: SecondFactorSwitchOff) =>
		call<void>('DELETE', '/second-factor', body),

	passkeys: () => call<PasskeysJson>('GET', '/passkeys'),
	/** Starts making a passkey: what to ask the device. */
	startPasskey: (body: NewPasskey) => call<PasskeyChallenge>('POST', '/passkeys/register', body),
	finishPasskey: (body: PasskeyRegistration) => call<RegisteredPasskey>('POST', '/passkeys', body),
	removePasskey: (id: number, body: RemovePasskey) => call<void>('DELETE', `/passkeys/${id}`, body),
	/** Starts asking a passkey of the account that its owner is there. */
	startPasskeyProof: () => call<PasskeyChallenge>('POST', '/passkeys/proof'),
	/** Starts logging in with a passkey: what to ask the device. */
	startPasskeyLogin: () => call<PasskeyChallenge>('POST', '/login/passkey'),
	finishPasskeyLogin: (body: PasskeyAnswer) =>
		call<ApiAccount>('POST', '/login/passkey/finish', body),

	reportsConsent: () => call<ReportsConsent>('GET', '/reports/consent'),
	setReportsConsent: (on: boolean) =>
		call<void>('PUT', '/reports/consent', { on } satisfies ReportsConsentChange),
	sendReports: (reports: ClientReport[]) =>
		call<void>('POST', '/reports', { reports } satisfies ClientReports),
	/** Sends reports as the page is left: the request outlives the page. */
	beaconReports: (reports: ClientReport[]) => {
		send('POST', '/reports', { reports } satisfies ClientReports, true).catch(() => {});
		return true;
	}
};

export type Api = typeof api;
