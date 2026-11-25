// The Japanese texts, in the shape of the English catalog (en.ts), in the polite
// form (です・ます). Japanese has no plural, so counts take a counter instead;
// as in the style of Apple and Google, a half-width space separates numbers and
// Latin words from the Japanese around them.

import type { Messages } from './en.ts';
import { formatNumber as n } from './intl.ts';

const articles = (count: number) => `${n(count)} 件の記事`;

const minutes = (count: number) => `${n(count)} 分`;

const ja: Messages = {
	languageName: '日本語',

	articles,

	common: {
		close: '閉じる',
		cancel: 'キャンセル',
		save: '保存',
		delete: '削除',
		edit: '編集'
	},

	nav: {
		subscribe: '購読',
		home: 'ホーム',
		all: 'すべての記事',
		unreadItems: '未読の記事',
		starred: 'スター付きの記事',
		hidden: 'フィルタで非表示',
		subscriptions: '購読中',
		newFolder: '新しいフォルダ',
		groupedFolder: '新しいフォルダ',
		folderName: 'フォルダ名',
		renameFolder: '名前を変更',
		removeFolder: 'フォルダを削除し、中身は残す',
		folderActions: (title) => `フォルダ「${title}」の操作`,
		feedName: 'フィード名',
		feedNameHint: '空欄にするとフィード自身の名前になります',
		renameFeed: '名前を変更',
		unsubscribe: '購読を解除',
		reportFeed: 'フィードを報告…',
		feedInfo: 'フィードの情報…',
		feedActions: (title) => `フィード「${title}」の操作`,
		unsubscribeTitle: '購読を解除しますか？',
		unsubscribeText: (title) =>
			`「${title}」の購読を解除しますか？このフィードで既読にしたものやスターを付けたものは失われます。`,
		openFolder: 'フォルダを開く',
		closeFolder: 'フォルダを閉じる',
		openSidebar: 'サイドバーを表示',
		closeSidebar: 'サイドバーを隠す',
		search: '検索',
		searchPlaceholder: 'これらの記事を検索',
		clearSearch: '検索をクリア',
		settings: '設定',
		accountSettings: 'アカウント設定',
		shortcuts: 'キーボードショートカット',
		live: 'ライブ',
		liveOn: '新しい記事が自動で届きます。クリックすると一時停止します。',
		liveOff: '一時停止中です。クリックすると新しい記事がまた届くようになります。',
		paused: '一時停止中',
		account: (name) => `${name} としてログイン中`,
		logout: 'ログアウト',
		appSettings: 'サーバとアプリのアップデート',
		install: 'アプリとしてインストール',
		installSafari:
			'rueder をアプリとしてインストールするには、Mac では「ファイル › Dock に追加」、iPhone や iPad では「共有 › ホーム画面に追加」を選びます。',
		unread: (count) => `未読 ${n(count)} 件`,
		hiddenCount: (count) => `フィルタで非表示 ${n(count)} 件`,
		fetchFailed: (reason) => `前回の取得に失敗しました: ${reason}`,
		waiting: 'まだ取得されていません',
		blocked: 'この rueder の運営者がブロックしました',
		moveHint: 'ドラッグで並べ替え。Alt キーと矢印キーでも動かせます',
		resizeSidebar: 'サイドバーの幅: ドラッグするか矢印キーを使います',
		tooDeep: (levels) => `フォルダの入れ子は最大 ${n(levels)} 階層までです。`
	},

	toolbar: {
		refresh: '更新',
		showAll: 'すべての記事',
		showUnread: '未読のみ',
		show: '表示',
		newestFirst: '新しい順',
		oldestFirst: '古い順',
		markAllRead: 'すべて既読にする',
		markOptions: 'ほかの既読にする方法',
		phoneMenu: (shown) => `表示: ${shown}`,
		markMenu: '既読にする…',
		markEverything: 'すべての記事',
		markOlderDay: '1 日より前の記事',
		markOlderWeek: '1 週間より前の記事',
		listView: 'リスト表示',
		expandedView: '展開表示',
		splitView: '3 ペイン: ツリー、リスト、記事',
		magazineView: 'マガジン',
		previous: '前の記事',
		next: '次の記事',
		newArticles: (count) => `新しい記事 ${n(count)} 件`
	},

	list: {
		empty: 'ここには記事がありません。',
		emptyUnread: 'ここには未読がありません。',
		emptySearch: '検索に一致する記事はありません。',
		found: (count, more, words) => `「${words}」を含む記事 ${n(count)}${more ? '+' : ''} 件`,
		searchAll: 'すべてのフィードを検索',
		feedFailing:
			'rueder はあとでもう一度試します。失敗するたび、またはサイトの求めに応じて、待つ時間を長くします。失敗が続く場合は、フィードが移転したのかもしれません。',
		hiddenNote:
			'フィルタルールによってこれらの記事は非表示になりました。記事を開くと、どのルールによるものかがわかります。',
		toRules: 'フィルタルール',
		emptyFeed: 'このフィードにはまだ記事がありません。',
		waitingFeed:
			'このフィードはまだ取得されていません。記事は次の取得の巡回で、数分以内に届きます。',
		showAll: 'すべての記事を表示',
		more: 'さらに表示',
		star: 'スターを付ける',
		unstar: 'スターを外す',
		keepUnread: '未読のままにする',
		openOriginal: '元の記事を開く',
		showFeed: 'このフィードのすべての記事を rueder で表示',
		from: '出典',
		by: (author) => `著者: ${author}`,
		updated: '更新あり',
		updatedHeadline: '見出しを変更',
		updatedTitle: (count) => `最初の取得以降に変更されました。以前の版 ${n(count)} 件`,
		updatedHeadlineTitle: (count) =>
			`見出しが書き換えられ、本文は変わっていません。以前の版 ${n(count)} 件`,
		compare: '版を比較',
		hiddenBy: (rule) => `次のルールで非表示: ${rule}`,
		textDropped: 'この記事は古いため、このサーバはもう本文を保存していません。',
		textDroppedOriginal: '元のサイトで読む',
		loading: '本文を読み込み中…',
		loadingList: '読み込み中…',
		toTop: '先頭に戻る',
		pick: '記事を選ぶと、ここで読めます。',
		resizeList: 'リストの幅: ドラッグするか矢印キーを使います',
		closeArticle: '記事を閉じる',
		blockedFeed:
			'この rueder の運営者がこのフィードをブロックしました。取得されず、記事も表示されません。',
		articleActions: '記事のその他の操作',
		articleInfo: '記事の情報…',
		articleSource: 'ソース…',
		reportArticle: '記事を報告…'
	},

	report: {
		feedTitle: 'フィードを報告',
		articleTitle: '記事を報告',
		text: (title) =>
			`この rueder の運営者には、「${title}」が報告されたこと、その理由、あなたが書いた内容が伝わりますが、誰が報告したかは伝わりません。全員に対してブロックするかどうかは運営者が決めます。`,
		reason: '理由',
		reasons: {
			illegal: '法律に違反している',
			copyright: '他人の著作物',
			malicious: 'マルウェア、フィッシング、詐欺',
			spam: 'スパム',
			other: 'その他'
		},
		note: '補足（任意）',
		left: (count) => `残り ${n(count)} 文字`,
		send: '報告を送信',
		sending: '送信中…',
		sent: 'ありがとうございます。報告はこの rueder の運営者に届きました。',
		tooMany: '最近、多くの報告を送信しています。しばらくしてからもう一度お試しください。',
		unreachable: '報告はサーバに届きませんでした。しばらくしてからもう一度お試しください。'
	},

	feedInfo: {
		title: 'フィードの情報',
		text: (title) =>
			`この rueder が「${title}」について知っていることと、取得の方法です。フィードが機能しない場合は、運営者へのメッセージにこれをコピーしてください。`,
		number: '番号',
		name: '名前',
		address: 'フィードのアドレス',
		site: 'ウェブサイト',
		noSite: 'なし',
		agent: '取得時の名乗り',
		agents: { rueder: 'rueder', browser: 'ブラウザ' },
		every: '取得の間隔',
		fetched: '前回の取得',
		succeeded: '前回成功した取得',
		failed: '最後のエラー',
		next: '次回の取得',
		blocked: 'なし（運営者がフィードをブロックしました）',
		articles: 'これまでの記事',
		never: 'なし',
		fetchAs: '取得時の名乗り',
		fetchAsHint:
			'このフィードを読む全員に適用されます。rueder を拒むサイトには「ブラウザ」を使います。すぐに取得し直します。',
		notSwitched: 'うまくいきませんでした。しばらくしてからもう一度お試しください。',
		atMost: '最短の取得間隔',
		noLimit: '制限なし',
		atMostHint:
			'このフィードを読む全員に適用されます。リクエストが多すぎると応答するサイト (429) 向けです。どれだけ更新されても、これより頻繁には取得しません。',
		copy: 'コピー',
		copied: 'コピーしました',
		loading: '読み込み中…',
		unreachable: 'サーバが応答しませんでした。しばらくしてからもう一度お試しください。'
	},
	articleInfo: {
		title: '記事の情報',
		text: (title) =>
			`この rueder が「${title}」とその版について知っていることです。おかしな点がある場合は、運営者へのメッセージにこれをコピーしてください。`,
		feed: 'フィード',
		feedNumber: 'フィードの番号',
		number: 'フィード内の番号',
		name: 'タイトル',
		id: 'フィード内の ID',
		link: 'リンク',
		noLink: 'なし',
		published: '公開',
		fetched: '最初の取得',
		changed: '最後の変更',
		never: 'なし',
		changedAt: (when, what) => `${when}（${what}）`,
		what: { title: 'タイトル', text: '本文', both: 'タイトルと本文' },
		version: '版',
		kept: '保存されている版',
		textLabel: '本文',
		textDropped: '古いため、保存されていません',
		copy: 'コピー',
		copied: 'コピーしました',
		loading: '読み込み中…',
		unreachable: 'サーバが応答しませんでした。しばらくしてからもう一度お試しください。'
	},
	articleSource: {
		title: '記事のソース',
		text: 'この記事の HTML です。この rueder がクリーニング後に保存しているもの、またはフィードが送ったままのものを表示します。リンクと画像はここでは元のアドレスのままなので、残っているトラッキングパラメータがわかります。',
		show: '表示',
		kinds: { cleaned: 'クリーニング後', original: 'フィードが送ったまま' },
		noText: 'このサーバはこの記事の本文をもう保存していません。',
		noOriginal: 'このサーバはフィードが送った内容をもう保存していません。',
		copy: 'コピー',
		copied: 'コピーしました',
		loading: '読み込み中…',
		unreachable: 'サーバが応答しませんでした。しばらくしてからもう一度お試しください。'
	},

	media: {
		title: '添付ファイル',
		play: '再生',
		image: '画像',
		zoomIn: '拡大',
		zoomOut: '縮小'
	},

	connection: {
		reconnecting: '再接続中…',
		unreachable: (since) => `${since} からサーバが応答していません。`,
		offline: (since) => `${since} からネットワークに接続されていません。`,
		retry: '今すぐ再試行',
		unsent: (count) => `${n(count)} 件の変更が送信を待っています。`
	},

	notice: {
		unreachable: 'サーバが応答しません。表示されている内容は最新ではない可能性があります。',
		notSaved: 'サーバが最後の変更を受け付けませんでした。ここでは元に戻しました。',
		tooManyUnsent: 'サーバへの送信を待つ変更が多すぎるため、この変更は取り消されました。'
	},

	login: {
		title: 'rueder にログイン',
		about:
			'フィードリーダーです。フォローしているサイトが公開したものを取得し、まだ読んでいないものを表示します。',
		name: '名前',
		password: 'パスワード',
		submit: 'ログイン',
		checking: 'お待ちください…',
		wrong: '名前またはパスワードが正しくありません。',
		codeNeeded:
			'このアカウントには 2 段階認証が設定されています。アプリに表示されたコードか復旧コードを入力するか、パスキーでログインしてください。',
		wrongCode: 'そのコードは正しくありません。コードは 30 秒ごとに変わります。',
		code: 'コード',
		codeNote: '認証アプリに表示される 6 桁の数字、または復旧コードのひとつ。',
		tooMany: (count) => `間違いが多すぎます。${minutes(count)}後にもう一度お試しください。`,
		unreachable: 'サーバが応答しません。少し待ってからもう一度お試しください。',
		retry: '再試行',
		or: 'または',
		oidc: 'シングルサインオンでログイン',
		passkey: 'パスキーでログイン',
		passkeyCancelled:
			'パスキーは使われませんでした。もう一度試すか、名前とパスワードでログインしてください。',
		passkeyFailed: 'このブラウザまたはデバイスでは、ここでパスキーを使えません。',
		passkeyWrong: 'このパスキーはここのどのアカウントのものでもありません。',
		oidcFailed: 'シングルサインオンでのログインはうまくいきませんでした。もう一度お試しください。',
		oidcDenied: 'シングルサインオンでのログインはキャンセルされました。',
		oidcUnknown:
			'そのログインに対応するアカウントはここにありません。この rueder の運営者に連携を依頼してください。',
		disabled: 'このアカウントは無効になっています。理由はこの rueder の運営者にお尋ねください。',
		unconfirmed:
			'このアカウントはメールアドレスの確認を待っています。メールのリンクを開くか、もう一度送信してください。',
		resend: 'リンクを再送信',
		resent: 'リンクを再送信しました。受信トレイと迷惑メールフォルダをご確認ください。',
		resendWrong: 'リンクを再送信するには、アカウントの名前とパスワードを入力してください。',
		forgot: 'パスワードをお忘れですか？',
		removed: 'あなたのアカウントは、あなたのものすべてと一緒に削除されました。',
		note: '自分で登録することはできません。アカウントはこの rueder の運営者が作成します。'
	},

	legal: {
		imprint: '運営者情報',
		privacy: 'プライバシーポリシー',
		terms: '利用規約',
		agree: { before: '内容を確認し、', link: '利用規約', after: 'に同意します。' },
		changed: '利用規約が変更されました',
		changedIntro: '新しい利用規約をお読みください。rueder を引き続き使うには、同意が必要です。',
		read: '利用規約を読む',
		accept: '同意する',
		decline: 'ログアウト',
		notSaved: 'うまくいきませんでした。少し待ってからもう一度お試しください。'
	},

	register: {
		title: 'アカウントを作成',
		invited: 'この rueder に招待されています。名前とパスワードを決めてください。',
		again: 'パスワード（確認）',
		passwordRule: (characters) => `${n(characters)} 文字以上。`,
		mismatch: '2 つのパスワードが一致しません。',
		submit: 'アカウントを作成',
		solving: 'このブラウザを確認中…',
		toRegister: 'アカウントを作成',
		toLogin: 'お持ちのアカウントでログイン',
		closed: 'ここでは招待なしに登録することはできません。',
		inviteGone: 'この招待はもう使えません。新しい招待を依頼してください。',
		full: 'ここにはこれ以上アカウントを作る余裕がありません。',
		today: '今日はここで作成できるアカウントの数に達しました。明日もう一度お試しください。',
		captcha: 'このブラウザの確認が完了しませんでした。もう一度お試しください。',
		taken: 'この名前のアカウントはすでにあります。',
		terms: 'アカウントを作成するには、利用規約に同意してください。',
		invalid: (reason) => `アカウントとして使えません。サーバの応答: ${reason}`,
		email: 'メールアドレス',
		emailOptional: 'メールアドレス（任意）',
		emailNote: '確認用のリンクが届きます。',
		emailRequiredNote:
			'確認用のリンクが届きます。リンクを開くと、アカウントでログインできるようになります。',
		emailInvalid: 'メールを受け取れるメールアドレスを入力してください。',
		unsent:
			'リンクをメールで送れなかったため、アカウントは作成されませんでした。しばらくしてからもう一度お試しください。',
		tooMany: (count) => `試行が多すぎます。${minutes(count)}後にもう一度お試しください。`,
		awaitingTitle: 'メールをご確認ください',
		awaiting: (address, hours) =>
			`アカウントを作成しました。${address} にリンクを送りました。${n(hours)} 時間以内に開いてから、ログインしてください。`
	},

	forgot: {
		title: 'パスワードをお忘れですか？',
		intro:
			'名前かメールアドレスを入力してください。新しいパスワードを設定するためのリンクが、確認済みのアドレスに届きます。',
		field: '名前またはメールアドレス',
		submit: 'リンクを送信',
		asked: (count) =>
			`該当するアカウントに確認済みのアドレスがあれば、そこにリンクを送りました。リンクは ${minutes(count)}間有効です。迷惑メールフォルダもご確認ください。`,
		tooMany: (count) => `リクエストが多すぎます。${minutes(count)}後にもう一度お試しください。`,
		back: 'ログインに戻る'
	},

	reset: {
		title: '新しいパスワードを設定',
		intro: 'アカウントのすべてのブラウザとデバイスがログアウトされます。',
		fresh: '新しいパスワード',
		again: '新しいパスワード（確認）',
		submit: 'パスワードを設定',
		mismatch: '2 つが一致しません。',
		gone: 'このリンクはもう使えません。新しいリンクを依頼してください。',
		invalid: (reason) => `サーバがそのパスワードを受け付けませんでした。サーバの応答: ${reason}`,
		done: 'パスワードを設定しました。新しいパスワードでログインしてください。',
		cancel: 'キャンセル'
	},

	email: {
		title: 'メールアドレス',
		intro: 'rueder からのメールは、あなたのアカウントに関するものだけです。',
		current: 'あなたのアドレス',
		none: 'このアカウントにはアドレスがありません。',
		pending: (address, hours) =>
			`${address} にリンクを送りました。${n(hours)} 時間以内にリンクが開かれると、そのアドレスが有効になります。それまでは前のアドレスのままです。`,
		address: '新しいアドレス',
		password: 'パスワード',
		passwordNote:
			'このブラウザを使える他の人が、あなたのメールの宛先を変えられないようにするためです。',
		code: '2 段階認証のコード',
		submit: 'リンクを送信',
		sent: 'リンクを送りました。受信トレイと迷惑メールフォルダをご確認ください。',
		remove: 'アドレスを削除',
		removeIntro: 'アドレスがないと、メールは届きません。それ以外は、アカウントはそのままです。',
		removed: 'アドレスを削除しました。',
		wrong: 'パスワードまたはコードが正しくありません。',
		codeNeeded: 'このアカウントには 2 段階認証が設定されています。そのコードも入力してください。',
		loginAgain:
			'あなたのアカウントにはまだパスワードも 2 段階認証もありません。もう一度ログインしてから、10 分以内にアドレスを入力してください。',
		invalid: 'メールを受け取れるメールアドレスを入力してください。',
		same: 'それはこのアカウントのアドレスです。',
		required:
			'この rueder のアカウントにはアドレスが必要です。変更はできますが、削除はできません。',
		busy: (count) => `試行またはメールが多すぎます。${minutes(count)}後にもう一度お試しください。`,
		unsent: 'メールを送信できませんでした。しばらくしてからもう一度お試しください。',
		unreachable: 'サーバが応答しません。何も変更されていません。',
		askTitle: 'メールアドレスを追加',
		askIntro:
			'この rueder のアカウントには、今後メールアドレスが必要です。アドレスを入力し、届いたリンクを開いてください。',
		askWaiting:
			'リンクを開くと、この表示は自動で消えます。何も届きませんか？もう一度送信するか、別のアドレスに送ってください。',
		logOut: 'ログアウト',
		confirmed: (address) => `アドレス ${address} を確認しました。`,
		confirmedLogIn: 'アカウントの準備ができました。ログインしてください。',
		gone: 'このリンクはもう使えません。すでに使われたか、新しいリンクが送られたか、1 日以上経っています。',
		taken: 'その間に、別のアカウントがこのアドレスを使うようになりました。',
		confirmUnreachable: 'サーバが応答しません。少し待ってからもう一度リンクを開いてください。',
		close: '閉じる'
	},

	invites: {
		intro:
			'リンクを使うと、この rueder でアカウントを作成できます。リンクは作成したときに、ここで一度だけ表示されます。',
		left: (count) =>
			count === 0 ? '渡せる招待はありません。' : `あと ${n(count)} 人を招待できます。`,
		admin: '管理者はいつでも招待できます。',
		uses: '何アカウント分か',
		make: 'リンクを作成',
		noneLeft: 'その数の招待は残っていません。',
		failed: 'うまくいきませんでした。少し待ってからもう一度お試しください。',
		link: '今すぐ渡すリンク',
		copy: 'コピー',
		copied: 'コピーしました',
		goodFor: (days) => `リンクの有効期間は ${n(days)} 日です。`,
		yours: 'あなたのリンク',
		none: 'まだリンクを作成していません。',
		used: (uses, of) => `${n(of)} 件中 ${n(uses)} 件使用`,
		until: (when) => `${when} まで有効`,
		ran: (when) => `${when} に期限切れ`,
		usedUp: '使い切り',
		revoked: '取り消し済み',
		revoke: '取り消す',
		revokeLink: (made) => `${made} に作成したリンクを取り消す`
	},

	home: {
		newIn: '購読中のフィードの新着',
		allRead: 'すべて読みました。新しい記事は届きしだいここに表示されます。',
		recentlyStarred: '最近スターを付けたもの',
		noStars: 'まだスター付きのものはありません。記事で s を押してください。',
		problems: '問題のあるフィード',
		tips: 'ヒント',
		tipKeys:
			'? を押すとすべてのキーボードショートカットが表示されます。j と k で記事を移動します。',
		tipDrag: 'サイドバーのフィードやフォルダはドラッグで並べ替えられます。',
		tipFilters: '設定のフィルタルールで、キーワードや著者によって記事を非表示にできます。',
		showFeed: (title) => `${title} を表示`
	},

	versions: {
		title: 'この記事の版',
		current: '現在の版',
		version: (rev) => `第 ${n(rev)} 版`,
		fetched: (when) => `${when} に取得`,
		compareWith: '現在の版と比較する版',
		legendAdded: '追加',
		legendRemoved: '削除',
		titleWas: '以前のタイトル:',
		same: 'この版の本文は現在の版と同じです。',
		sameWords: '言葉は同じです。画像かリンクが変わりました。',
		markup: '言葉、画像、リンクは同じです。本文の構成だけが変わりました。',
		pictures: '変わった画像',
		noPicture: 'なし',
		address: '画像のアドレス',
		links: '変わったリンク'
	},

	account: {
		title: 'アカウント設定',
		tabs: {
			password: 'パスワード',
			email: 'メール',
			security: 'セキュリティ',
			apps: 'アプリ',
			invites: '招待',
			move: '引っ越し',
			privacy: 'プライバシー',
			remove: 'アカウントを削除'
		},
		password: {
			title: 'パスワードを変更',
			titleFirst: 'パスワードを設定',
			intro:
				'新しいパスワードにすると、ほかのすべてのブラウザとデバイスがログアウトされ、すべてのアプリパスワードが取り消されます。',
			introFirst:
				'パスワードがあれば、このアカウントはシングルサインオンだけでなく、名前でもログインできます。',
			current: '現在のパスワード',
			fresh: '新しいパスワード',
			again: '新しいパスワード（確認）',
			submit: 'パスワードを変更',
			submitFirst: 'パスワードを設定',
			done: 'パスワードを変更しました。ほかのすべてのブラウザとデバイスはログアウトされ、アプリパスワードは取り消されました。',
			doneFirst: 'パスワードを設定しました。',
			mismatch: '2 つが一致しません。',
			code: '2 段階認証のコード',
			wrong: '現在のパスワードまたはコードが正しくありません。',
			codeNeeded:
				'このアカウントには 2 段階認証が設定されています。認証アプリのコードか復旧コードも入力するか、パスキーで確認してください。',
			loginAgain:
				'あなたのアカウントにはまだパスワードも 2 段階認証もありません。もう一度ログインしてから、10 分以内にパスワードを設定してください。',
			invalid: (characters) =>
				`サーバがそのパスワードを受け付けませんでした。${n(characters)} 文字以上にしてください。`,
			busy: (count) => `間違いが多すぎます。${minutes(count)}後にもう一度お試しください。`,
			unreachable: 'サーバが応答しません。パスワードは変更されていません。'
		},
		settingsNote: '表示、閲覧、フィルタルール、キーボードは「設定」にあります。'
	},

	settings: {
		title: '設定',
		tabs: {
			look: '表示',
			reading: '閲覧',
			rules: 'フィルタルール',
			keys: 'キーボード'
		},
		accountNote:
			'パスワード、2 段階認証、アプリ、招待は、アカウントのメニューにある「アカウント設定」にあります。',
		theme: 'テーマ',
		themes: { system: 'システムに合わせる', light: 'ライト', dark: 'ダーク' },
		accent: 'アクセントカラー',
		accents: {
			red: '赤',
			blue: '青',
			green: '緑',
			orange: 'オレンジ',
			purple: '紫',
			graphite: 'グラファイト'
		},
		density: 'リストの密度',
		densities: { compact: 'コンパクト', comfortable: 'ゆったり' },
		sidebarSide: 'サイドバー',
		sidebarSides: { start: '左', end: '右' },
		textSize: '記事の文字サイズ（このデバイス）',
		textSizes: { small: '小', medium: '中', large: '大' },
		readingFont: '記事のフォント',
		readingFonts: { sans: 'ゴシック体', serif: '明朝体' },
		counters: '未読数',
		counterStyles: { parens: 'かっこ内', badge: 'バッジ' },
		language: '言語',
		languageAuto: 'ブラウザに合わせる',
		view: '表示形式',
		views: { list: 'リスト', expanded: '展開', split: '3 ペイン', magazine: 'マガジン' },
		markOnScroll: '展開表示でスクロールして通り過ぎた記事を既読にする',
		teasers: 'リストでタイトルの後に本文の冒頭を表示する',
		wakeLock: 'rueder を開いている間は画面をオンのままにする',
		wakeLockMissing: 'このブラウザは画面をオンのままにできません。',
		note: '変更はすぐに反映され、アカウントに保存されるので、どのデバイスでも同じになります。',
		noteDemo: '変更はすぐに反映され、このブラウザに残ります。',
		apps: {
			intro:
				'ほかのフィードリーダーアプリも、Google Reader API を通じてあなたのアカウントを使えます。アプリごとに専用のアプリパスワードが発行され、いつでも取り消せます。',
			none: 'まだアプリは設定されていません。',
			made: (when) => `${when} に設定`,
			used: (when) => `最終使用 ${when}`,
			unused: 'まだ使われていません',
			revoke: '取り消す',
			revokeName: (name) => `アプリパスワード「${name}」を取り消す`,
			add: 'アプリを設定',
			full: (max) =>
				`アプリパスワードは 1 アカウントにつき最大 ${n(max)} 個です。別のアプリを設定するには、1 つ取り消してください。`,
			choose: 'どのアプリを使いますか？',
			website: (app) => `${app} のウェブサイト`,
			other: 'ほかのアプリ',
			otherHint: 'Google Reader API または FreshRSS に対応したアプリなら使えます。',
			platforms: {
				apple: 'Mac、iPhone、iPad',
				android: 'Android'
			},
			name: 'アプリパスワードの名前',
			password: 'パスワード',
			code: '認証アプリのコード、または復旧コード',
			create: 'アプリパスワードを作成',
			creating: 'お待ちください…',
			back: '戻る',
			how: {
				netnewswire:
					'NetNewsWire で「設定」を開き、「アカウント」でアカウントを追加して（Mac では +）、FreshRSS を選びます。下の名前を Login（iPhone と iPad では Username or Email）に、アプリパスワードを Password に、サーバのアドレスを API URL に入力します。',
				readyou:
					'Read You で「設定」、「アカウント」、「アカウントを追加」を開き、Google Reader を選びます。下のサーバのアドレスをサーバ URL に、名前をユーザ名に、アプリパスワードをパスワードに入力します。',
				other:
					'お使いのアプリで、「Google Reader API」または（なければ）「FreshRSS」の種類のアカウントを追加し、下のサーバのアドレス、名前、アプリパスワードを入力します。'
			},
			ownIcons: (app) =>
				`プライバシー: ${app} はフィードのアイコンを各フィードのウェブサイトから自分で読み込むため、そのサイトにはあなたの IP アドレスが伝わります。記事とその画像は rueder を通してのみ届きます。`,
			shotsTitle: '画像付きの手順',
			shots: {
				readyou: [
					'左上の歯車をタップします。',
					'「アカウント」を開きます。',
					'「アカウントを追加」をタップします。',
					'Google Reader を選びます。',
					'サーバのアドレスをサーバ URL に、名前をユーザ名に、アプリパスワードをパスワードに入力し、「追加」をタップします。'
				],
				netnewswire: [
					'Mac で「設定」（⌘,）を開き、「アカウント」で左下の + をクリックします。',
					'FreshRSS を選び、「続ける」をクリックします。',
					'名前を Login に、アプリパスワードを Password に、サーバのアドレスを API URL に入力し、「作成」をクリックします。'
				]
			},
			shotAlt: (app, step) => `${app}、手順 ${n(step)}`,
			server: 'サーバのアドレス',
			user: '名前',
			secret: 'アプリパスワード',
			copy: (what) => `${what}をコピー`,
			copied: 'コピーしました',
			once: 'アプリパスワードは今だけ表示されます。なくした場合は、取り消して新しく作成してください。',
			waiting: 'アプリのログインを待っています…',
			connected: '接続しました。アプリがログインしました。',
			done: '完了',
			refusals: {
				wrongPassword: 'パスワードが正しくありません。',
				codeNeeded: '認証アプリのコードか復旧コードも入力するか、パスキーで確認してください。',
				wrongCode: 'そのコードは正しくありません。コードは 30 秒ごとに変わります。',
				loginAgain:
					'あなたのアカウントにはまだパスワードも 2 段階認証もありません。もう一度ログインしてから、10 分以内にアプリパスワードを作成してください。',
				tooManyApps: 'アプリパスワードは 1 アカウントにつき最大 20 個です。',
				tooMany: '間違いが多すぎます。数分後にもう一度お試しください。',
				unreachable: 'サーバが応答しません。少し待ってからもう一度お試しください。'
			}
		},
		secondFactor: {
			title: '2 段階認証',
			intro: '2 段階認証を設定すると、ログインには認証アプリのコードかパスキーも必要になります。',
			app: '認証アプリ',
			on: 'オン。',
			left: (count) => `復旧コードは残り ${n(count)} 個です。`,
			off: 'オフ。',
			byPasskeys: '未設定です。パスキーがあるため、2 段階認証はオンのままです。',
			adminNeeds:
				'このサーバでは、2 段階認証を設定した管理者だけが管理者として扱われます。サーバの状態を見るには、設定するか、そのコードでもう一度ログインしてください。',
			password: 'パスワード',
			start: '設定',
			scan: '認証アプリでこのコードを読み取るか、下のキーを入力してください。',
			qr: '認証アプリ用の QR コード',
			larger: 'QR コードを大きく表示',
			smaller: 'QR コードを小さく表示',
			key: 'キー',
			copyKey: 'キーをコピー',
			copied: 'コピーしました',
			code: 'アプリに今表示されているコード',
			codeOrRecovery: 'アプリのコード、または復旧コード',
			confirm: '確認',
			cancel: 'キャンセル',
			codesTitle: '復旧コード',
			codesIntro:
				'どれも、スマートフォンなしで一度だけログインできます。大切に保管してください。表示されるのは今だけです。',
			copyCodes: 'コードをコピー',
			codesDone: '保管しました',
			switchOff: 'オフにする',
			switchOffNote: 'オフにすると、ログインはまたパスワードだけでできるようになります。',
			switchOffNoteKeys: 'パスキーがあるため、2 段階認証はオンのままです。',
			startWithPasskey: 'パスキーで設定',
			switchOffWithPasskey: 'パスキーでオフにする',
			refusals: {
				wrongPassword: 'パスワードが正しくありません。',
				codeNeeded: 'パスキーにより 2 段階認証がオンになっています。復旧コードも入力してください。',
				wrongCode: 'そのコードは正しくありません。コードは 30 秒ごとに変わります。',
				loginAgain:
					'あなたのアカウントにはまだパスワードも 2 段階認証もありません。もう一度ログインしてから、10 分以内にアプリを設定してください。',
				tooMany: '間違いが多すぎます。数分後にもう一度お試しください。',
				unreachable: 'サーバが応答しません。少し待ってからもう一度お試しください。'
			}
		},
		passkeys: {
			title: 'パスキー',
			intro:
				'パスキーを使うと、名前とパスワードなしで、デバイスが求める PIN、指紋、顔でログインできます。2 段階認証としても数えられます。',
			once: 'パスキーがあると、パスワードだけではアカウントを開けなくなります。',
			none: 'まだパスキーはありません。',
			made: (when) => `${when} に追加`,
			used: (when) => `最終使用 ${when}`,
			unused: 'まだ使われていません',
			add: 'パスキーを追加',
			name: '名前（デバイス名など）',
			defaultName: 'パスキー',
			password: 'パスワード',
			code: '認証アプリのコード、または復旧コード',
			create: 'パスキーを作成',
			working: 'ブラウザの案内に従ってください…',
			cancel: 'キャンセル',
			remove: '削除',
			removeName: (name) => `パスキー「${name}」を削除`,
			confirmRemove: 'パスキーを削除',
			full: (max) => `パスキーは 1 アカウントにつき最大 ${n(max)} 個です。`,
			or: 'または',
			confirmWith: 'パスキーで確認',
			codesTitle: '復旧コード',
			codesIntro:
				'最初のパスキーで 2 段階認証がオンになりました。デバイスをなくしたときは、どのコードでも一度だけログインできます。大切に保管してください。表示されるのは今だけです。',
			copyCodes: 'コードをコピー',
			copied: 'コピーしました',
			codesDone: '保管しました',
			refusals: {
				cancelled: 'パスキーが使われる前に、ブラウザのウィンドウが閉じられました。',
				known: 'このデバイスにはすでにあなたのアカウントのパスキーがあります。',
				failed:
					'このブラウザまたはデバイスでは、パスキーを作成することも使うこともできませんでした。',
				wrong: 'パスキーは受け付けられませんでした。もう一度お試しください。',
				wrongPassword: 'パスワードが正しくありません。',
				codeNeeded:
					'あなたのアカウントには 2 段階認証が設定されています。アプリのコードか復旧コードも入力するか、パスキーで確認してください。',
				wrongCode: 'そのコードは正しくありません。コードは 30 秒ごとに変わります。',
				tooManyPasskeys: 'パスキーは 1 アカウントにつき最大 10 個です。',
				loginAgain:
					'あなたのアカウントにはまだパスワードも 2 段階認証もありません。もう一度ログインしてから、10 分以内に最初のパスキーを追加してください。',
				tooMany: '間違いが多すぎます。数分後にもう一度お試しください。',
				unreachable: 'サーバが応答しません。少し待ってからもう一度お試しください。'
			}
		},
		reports: {
			send: 'このブラウザで起きたエラーの報告をこのサーバに送る',
			intro: 'ページで問題が起きると、このサーバの管理者に伝わります。',
			what: '報告に含まれるもの:',
			holds: [
				'エラーのメッセージ（アドレスや、何かを特定する数字は除く）',
				'rueder 自身のファイルのどこで起きたか',
				'rueder のバージョンと、ブラウザとそのメジャーバージョン'
			],
			never: '読んでいるもの、フィード、ラベル、ルールは含まれません。報告は 30 日後に消えます。',
			off: 'オフにすると、送信した報告が削除されます。'
		}
	},

	rules: {
		intro:
			'ルールは、届いた記事を非表示にしたり既読にしたりします。非表示の記事は未読に数えられず、「フィルタで非表示」に残ります。',
		none: 'まだルールはありません。',
		add: 'ルールを追加',
		new: '新しいルール',
		change: 'ルールを変更',
		on: 'オン',
		scope: '対象',
		scopeAll: 'すべてのフィード',
		scopeFolder: (title) => `フォルダ: ${title}`,
		scopeFeed: (title) => `フィード: ${title}`,
		field: '調べる場所',
		fields: { any: 'すべて', title: 'タイトル', content: '本文', author: '著者', tag: 'タグ' },
		match: '比較',
		matches: {
			contains: 'を含む',
			word: 'という単語を含む',
			starts: 'で始まる',
			ends: 'で終わる',
			regex: 'という正規表現に一致する',
			all: 'すべての記事（内容を問わず）'
		},
		pattern: '探すテキスト',
		action: '処理',
		actions: { hide: '記事を非表示にする', read: '既読にする' },
		preview: (count, total) => `現在、ここの記事 ${n(total)} 件中 ${n(count)} 件に当てはまります。`,
		regexTip: '正規表現は Rust と同じように読まれ、大文字と小文字は区別されません。',
		regexBuilder: 'regex101 で作ってテストする',
		invalid: '有効な正規表現ではありません。',
		empty: '探すテキストを入力してください。',
		checking: '当てはまる記事を調べています…',
		hits: (count) => `${n(count)} 件一致`,
		summary: (field, match, pattern, action) => `${field}が「${pattern}」${match}: ${action}`,
		summaryAll: (action) => `すべての記事: ${action}`
	},

	keys: {
		title: 'キーボードショートカット',
		note: 'キーの割り当ては設定と一緒に保存されます。Enter、Esc、g は変わりません。',
		change: (what) => `次のキーを変更: ${what}`,
		changeShort: '変更',
		press: 'キーを押してください…',
		taken: (key, other) => `${key} はすでに次のキーです: ${other}`,
		fixed: (key) => `${key} はショートカットに割り当てられません。`,
		reset: 'rueder のキーに戻す',
		groups: { articles: '記事', lists: 'リスト', go: '移動', app: 'どこでも' },
		next: '次の記事を開く',
		previous: '前の記事を開く',
		nextCursor: '次の記事へ（開かない）',
		previousCursor: '前の記事へ（開かない）',
		toggle: '記事を開く・閉じる',
		star: 'スターを付ける・外す',
		read: '既読・未読にする',
		original: '元の記事を新しいタブで開く',
		markAll: 'すべて既読にする',
		refresh: 'リストを更新',
		unreadOnly: '未読のみ、またはすべての記事',
		listView: 'リスト表示',
		expandedView: '展開表示',
		splitView: '3 ペイン',
		magazineView: 'マガジン',
		goHome: 'ホーム',
		goAll: 'すべての記事',
		goUnread: '未読の記事',
		goStarred: 'スター付きの記事',
		search: '検索',
		sidebar: 'サイドバーを表示・非表示',
		help: 'この一覧',
		then: '続けて'
	},

	labels: {
		title: 'ラベル',
		add: '新しいラベル',
		name: 'ラベル名',
		none: 'まだラベルはありません。',
		actions: (title) => `ラベル「${title}」の操作`,
		rename: '名前を変更',
		color: '色',
		colors: ['赤', 'オレンジ', '黄', '緑', '青緑', '青', '紫', 'グレー'],
		remove: 'ラベルを削除',
		show: 'ラベルを表示',
		hide: 'ラベルを隠す',
		forArticle: 'ラベル',
		set: (title) => `ラベル: ${title}`
	},

	subscribe: {
		title: 'フィードを購読',
		address: 'フィードまたはウェブサイトのアドレス',
		find: 'フィードを探す',
		finding: '探しています…',
		note: '下書きでは何も取得できないため、フィードは空のままです。',
		found: (count) =>
			count === 1 ? 'フィードが 1 件あります:' : `フィードが ${n(count)} 件あります:`,
		none: 'このアドレスにはフィードがありません。',
		movedTo: (url) => `サイトはリクエストを ${url} に転送しました。`,
		loginWall:
			'ログインしていない人をログインページに送るサイトがあります。その場合、フィードは読めません。',
		subscribed: '購読しました',
		show: '表示',
		untitled: '名前なし',
		page: 'フィードのないウェブサイトです。rueder がタイトルと要約だけのフィードを作ります。',
		folder: '追加先のフォルダ',
		topLevel: 'フォルダなし',
		add: '購読',
		adding: '購読中…',
		refused: 'うまくいきませんでした。',
		serverSays: (reason) => `サーバの応答: ${reason}`,
		busy: (seconds) =>
			seconds
				? `同時に調べる数が多すぎます。${n(seconds)} 秒後にもう一度お試しください。`
				: '同時に調べる数が多すぎます。少し待ってからもう一度お試しください。',
		noFetching: 'このサーバはフィードを取得しません。',
		tooLarge: (megabytes) => `ファイルが ${n(megabytes)} MB を超えています。`,
		unreachable: 'サーバが応答しません。少し待ってからもう一度お試しください。'
	},

	opml: {
		title: 'インポートとエクスポート',
		intro: '別のフィードリーダーの購読を OPML ファイルで持ち込んだり、持ち出したりできます。',
		import: 'OPML ファイルをインポート',
		importing: 'インポート中…',
		imported: (feeds, folders) =>
			`新しいフィード ${n(feeds)} 件と新しいフォルダ ${n(folders)} 件。`,
		known: (count) => `${n(count)} 件のフィードはすでに購読していました。`,
		skipped: (count) => `読めるフィードではない ${n(count)} 件の項目を除外しました。`,
		later: '記事は次の取得の巡回で届きます。',
		export: 'OPML でエクスポート'
	},

	removeAccount: {
		title: 'アカウントを削除',
		intro: 'この rueder のあなたのアカウントを完全に削除します。元に戻すことはできません。',
		goes: '一緒に削除されるもの: すべてのデバイスのセッション、購読とフォルダ、既読の状態、スター、ラベル、フィルタルール、設定、エラーの報告、招待、シングルサインオンとの連携。',
		stays:
			'フィードは、それを読むほかの人のために残ります。先にアカウントを別の場所に移すには、「引っ越し」を使ってください。',
		password: 'パスワード',
		name: (name) => `確認のため、名前（${name}）を入力してください`,
		code: 'アプリのコード、または復旧コード',
		submit: 'アカウントを完全に削除',
		wrong: '正しくありません。何も削除されていません。',
		codeNeeded:
			'あなたのアカウントには 2 段階認証が設定されています。アプリのコードか復旧コードも入力するか、パスキーで確認してください。',
		busy: (count) => `間違いが多すぎます。${minutes(count)}後にもう一度お試しください。`,
		unreachable: 'サーバが応答しません。何も削除されていません。'
	},

	move: {
		intro: 'アカウントを 1 つのファイルで、別の rueder に移したり、ここに持ち込んだりできます。',
		howItWorks:
			'インポートはここにあるものに追加するだけで、何も削除しません。設定はファイルのものになります。',
		notInIt: 'ファイルに含まれないもの: パスワード、セッション、記事の本文。',
		export: 'アカウントをエクスポート',
		import: 'アカウントのファイルをインポート',
		importing: 'インポート中…',
		feeds: (feeds, known) => `新しいフィード ${n(feeds)} 件、購読済み ${n(known)} 件。`,
		folders: (count) => `新しいフォルダ ${n(count)} 件。`,
		articles: (found) => `${articles(found)}がここで見つかりました。`,
		missing: (count) =>
			`${articles(count)}がここで見つかりませんでした。それらの既読、スター、ラベルは失われます。`,
		marks: (labels, rules) => `新しいラベル ${n(labels)} 件と新しいフィルタルール ${n(rules)} 件。`,
		skipped: (count) => `ファイルの ${n(count)} 件の項目は取り込めませんでした。`,
		settings: '設定はファイルのものになりました。',
		later: '新しいフィードの記事は次の取得の巡回で届きます。既読にしたものは既読のままです。',
		busy: (seconds) =>
			seconds
				? `インポートが多すぎます。${minutes(Math.ceil(seconds / 60))}後にもう一度お試しください。`
				: 'インポートが多すぎます。しばらくしてからもう一度お試しください。'
	},

	status: {
		title: 'サーバの状態',
		open: 'サーバの状態',
		loading: 'サーバに問い合わせています…',
		failed: 'サーバが応答しませんでした。少し待ってからもう一度お試しください。',
		refresh: '更新',
		asOf: (time) => `${time} 時点。数えているのは直近 1 時間です。`,
		demo: 'ここにはサーバがないため、作り話です。',
		empty: '–',
		on: 'オン',
		off: 'オフ',
		took: (median, p95) => `中央値 ${median}、95% が ${p95} 以内`,
		summary: {
			label: '概要',
			hints: '注意点',
			failing: '失敗しているフィード',
			errors: '記録されたエラー',
			reported: '報告されたフィード'
		},
		hintsTitle: '確認したほうがよいこと',
		allWell: '目立った問題はありません。',
		hints: {
			slowRoute: (route, took) =>
				`${route} が遅くなっています。20 件に 1 件のリクエストが ${took} より長くかかります。`,
			serverErrors: (route, count) => `${route} がサーバのエラーで ${n(count)} 回応答しました。`,
			poolBusy: (size) =>
				`データベースへの ${n(size)} 本の接続がすべて使用中のため、リクエストが接続を待っています。`,
			fetchBacklog: (count, late) =>
				`${n(count)} 件のフィードの取得が遅れています。最も遅れているものは ${late} 遅れです。`,
			feedsFailing: (failing, subscribed) =>
				`${n(subscribed)} 件中 ${n(failing)} 件のフィードが失敗しています。`,
			notFetching: 'サーバはフィードを取得していません。FETCH が off になっていませんか？',
			fetchPaceFull: (most) =>
				`各巡回はできるだけ多くのフィードを取得していますが、まだ待っているフィードがあります。今のペースでは 1 時間に最大 ${n(most)} 回の取得です。FETCH_ROUND、FETCH_AT_ONCE、FETCH_EVERY で設定します。`,
			cacheMisses: (share) =>
				`データベースが読むもののうち、メモリ上にあるのは ${share} だけです。shared_buffers を増やすと改善します。`,
			databaseLarge: (size, warn) =>
				`データベースが ${size} になりました。DATABASE_WARN_GB は ${warn} から警告します。「rueder retention set」で古い記事やその本文を削除できます。`,
			longQuery: (took) => `${took} 実行中のクエリがあります。`,
			imageCacheFull: (share) => `画像のキャッシュが ${share} 埋まっています。`,
			errors: (count) => `直近 1 時間に、サーバは ${n(count)} 件のエラーを記録しました。`,
			limited: (count) => `${n(count)} 件のリクエストを、多すぎるとして断りました。`,
			browserErrors: (count) =>
				`直近 1 日に、ブラウザから ${n(count)} 種類のエラーが報告されました。`,
			reportedFeeds: (count) =>
				`読者から報告された、まだブロックされていないフィードが ${n(count)} 件あります。`
		},
		build: {
			title: 'ビルド',
			commit: 'コミット',
			dirty: (commit) => `${commit}（独自の変更あり）`,
			branch: 'ブランチ',
			tag: 'タグ',
			committed: 'コミット日時',
			where: '実行環境',
			container: 'コンテナ内',
			host: 'マシン上で直接',
			target: 'ターゲット',
			profile: 'コンパイル',
			profileValue: (profile, level) => `${profile}、最適化 ${level}`,
			ci: 'ビルド元',
			ciValue: 'CI の実行',
			byHand: '手作業'
		},
		server: {
			title: 'サーバ',
			version: 'バージョン',
			running: '稼働開始',
			memory: 'メモリ',
			accounts: 'アカウント',
			feeds: 'フィード',
			feedsValue: (fetched, all) => `取得済み ${n(fetched)} 件、合計 ${n(all)} 件`,
			articles: '記事',
			articlesValue: (count) => `約 ${n(count)} 件`,
			live: '開いているタブ'
		},
		database: {
			title: 'データベース',
			pool: 'rueder の接続',
			poolValue: (size, idle, max) =>
				`${n(size)} 本が開いていて、${n(idle)} 本が待機中、最大 ${n(max)} 本`,
			connections: 'すべての接続',
			size: 'サイズ',
			sizeWarn: (size, warn) => `${size}（${warn} から警告）`,
			hitRatio: 'メモリから読んだ割合',
			deadlocks: 'デッドロック',
			longest: '最も長く実行中のクエリ',
			tables: '大きいテーブル',
			table: 'テーブル',
			bytes: 'サイズ',
			rows: '行数（概算）'
		},
		fetcher: {
			title: 'フィードの取得',
			state: '取得',
			lastRound: '前回の巡回',
			lastRoundValue: (ago, feeds, took) => `${ago}: フィード ${n(feeds)} 件を ${took} で`,
			fetches: '取得',
			fetchesValue: (fetched, unchanged, failed) =>
				`フィードを取得 ${n(fetched)} 回、変更なし ${n(unchanged)} 回、失敗 ${n(failed)} 回`,
			broken: 'データベースで失敗',
			newArticles: '新しい記事',
			took: '取得 1 回あたりの時間',
			overdue: '遅れて取得',
			overdueValue: (count, late) => `${n(count)} 件、最も遅れているものは ${late}`,
			waiting: 'まだ取得されていない',
			pace: 'ペース',
			paceValue: (round, every, atOnce, most) =>
				`${every}ごとに ${n(round)} 件、同時に ${n(atOnce)} 件: 1 時間に最大 ${n(most)} 回`,
			perHour: '直近 1 時間の取得',
			behind: '遅れ',
			behindValue: (late) => `最も長く待っているフィードは予定より ${late} 遅れています`,
			notBehind: '待っているものはありません',
			failing: '失敗しているフィード（原因別）',
			noneFailing: '失敗しているフィードはありません。',
			feeds: (count) => `${n(count)} 件のフィード`,
			failures: {
				status: (code) =>
					code ? `サイトが ${code} を返しています` : 'サイトがエラーを返しています',
				timeout: 'サイトの応答に時間がかかりすぎます',
				blocked: 'rueder が取得しないネットワークのアドレスです',
				dns: 'その名前にアドレスがありません',
				network: '接続できないか、途中で切れました',
				too_large: '大きすぎます',
				redirects: 'リダイレクトが多すぎるか、壊れています',
				unreadable: 'rueder が読めるフィードではありません',
				other: 'その他'
			}
		},
		finder: {
			title: 'フィードの検索',
			state: 'アドレスの調査',
			lookups: '検索',
			lookupsValue: (found, none, failed) =>
				`フィードが見つかった ${n(found)} 回、見つからなかった ${n(none)} 回、失敗 ${n(failed)} 回`,
			limited: 'アカウントの上限を超えたもの',
			took: '検索 1 回あたりの時間'
		},
		rssifier: {
			title: 'フィードのないウェブサイト',
			state: 'RSSifier',
			pages: '読んでいるウェブサイト',
			pagesValue: (pages, failing) => `${n(pages)} 件、うち ${n(failing)} 件が失敗`,
			used: 'トークンの使用量',
			usedValue: (requests, fetches, underWay) =>
				`直近 1 分のリクエスト ${n(requests)} 件、直近 1 時間に取得したページ ${n(fetches)} 件、処理中 ${n(underWay)} 件`,
			unanswered: (why) => `RSSifier が応答しません: ${why}`,
			limits: 'トークンの上限',
			limitsValue: (perMinute, perHour, atOnce) =>
				`毎分 ${perMinute}、毎時 ${perHour} ページ、同時に ${atOnce}`,
			noLimit: '上限なし'
		},
		images: {
			title: '画像プロキシ',
			state: 'プロキシ',
			cache: 'キャッシュ',
			cacheValue: (used, max) => `${max} 中 ${used}`,
			served: '画像',
			servedValue: (cached, fetched) => `キャッシュから ${n(cached)} 件、取得 ${n(fetched)} 件`,
			refused: '拒否',
			refusedValue: (source, notImage, tooLarge, notFound) =>
				`取得元の失敗 ${n(source)} 件、画像でない ${n(notImage)} 件、大きすぎる ${n(tooLarge)} 件、アドレスの誤り ${n(notFound)} 件`,
			took: '取得した画像 1 件あたりの時間',
			failingSources: 'しばらく取得を控えている取得元'
		},
		requests: {
			title: 'リクエスト',
			all: 'すべてのリクエスト',
			allValue: (count, took) => `${n(count)} 件、${took}`,
			answers: '応答',
			answersValue: (fine, refused, errors) =>
				`正常 ${n(fine)} 件、拒否 ${n(refused)} 件、サーバのエラー ${n(errors)} 件`,
			route: 'ルート',
			count: '件数',
			median: '中央値',
			p95: '95%',
			max: '最長',
			errors: 'エラー',
			none: '直近 1 時間にリクエストはありません。'
		},
		problems: {
			title: '警告とエラー',
			counts: (errors, warnings) => `直近 1 時間のエラー ${n(errors)} 件、警告 ${n(warnings)} 件。`,
			none: '起動してから何も記録されていません。',
			error: 'エラー',
			warning: '警告',
			times: (count, last) => (count > 1 ? `${n(count)} 回、最後は ${last}` : last)
		},
		browsers: {
			title: 'ブラウザのエラー',
			off: 'このサーバはエラーの報告を受け付けません（CLIENT_REPORTS=off）。',
			consenting: (count) =>
				`${n(count)} 件のアカウントが報告を送っています。エラーは最後に起きてから 30 日間残ります。`,
			none: 'エラーを報告したブラウザはありません。',
			kinds: {
				error: 'エラー',
				rejection: '失敗した Promise',
				component: 'ページの一部',
				request: 'リクエスト'
			},
			hits: (count, accounts, last) =>
				`${n(accounts)} 件のアカウントで ${n(count)} 回、最後は ${last}`,
			where: (versions, browsers) => `バージョン ${versions} · ${browsers}`
		},
		reported: {
			title: '報告されたフィード',
			text: '誰が報告したかはここには表示されません。「rueder feed block <アドレス>」でフィードを全員に対してブロックし、「rueder feed unblock <アドレス>」で解除します。',
			none: 'フィードを報告した人はいません。',
			reports: (count, last) => `報告 ${n(count)} 件、最後は ${last}`,
			blocked: 'ブロック済み'
		}
	},

	update: {
		ready: 'rueder の新しいバージョンの準備ができました。',
		reload: '再読み込み',
		later: 'あとで'
	},

	crash: {
		title: 'このページで問題が起きました。',
		text: '続けるにはページを再読み込みしてください。読んだものや変更したものはサーバに保存されています。',
		reload: '再読み込み'
	},

	demo: {
		badge: '下書き',
		note: '架空のフィードを使った下書きです。サーバには何も保存されません。',
		arrive: '記事を 1 件届ける',
		revise: '開いている記事を変更',
		revised: '更新: 版を試すために、この段落を追加しました。',
		reviseHint: '先に記事を開いてください。'
	}
};

export default ja;
