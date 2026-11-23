// The Chinese texts in traditional characters, with the words used in Taiwan, in
// the shape of the English catalog (en.ts), addressing the reader with “你”.
// Browsers in zh-TW, zh-HK, zh-MO and zh-Hant get them. As in the style of Apple
// and Google for Taiwan, quotes are 「」, the ellipsis is ⋯, and a half-width space
// separates numbers and Latin words from the Chinese around them.

import type { Messages } from './en.ts';
import { formatNumber as n } from './intl.ts';

const articles = (count: number) => `${n(count)} 篇文章`;

const minutes = (count: number) => `${n(count)} 分鐘`;

const zhHant: Messages = {
	languageName: '繁體中文',

	articles,

	common: {
		close: '關閉',
		cancel: '取消',
		save: '儲存',
		delete: '刪除',
		edit: '編輯'
	},

	nav: {
		subscribe: '訂閱',
		home: '首頁',
		all: '所有文章',
		unreadItems: '未讀文章',
		starred: '已加星號的文章',
		hidden: '被篩選器隱藏',
		subscriptions: '訂閱',
		newFolder: '新增資料夾',
		groupedFolder: '新資料夾',
		folderName: '資料夾名稱',
		renameFolder: '重新命名',
		removeFolder: '刪除資料夾，保留其中的內容',
		folderActions: (title) => `資料夾「${title}」的動作`,
		feedName: '訂閱源名稱',
		feedNameHint: '留空即使用訂閱源本身的名稱',
		renameFeed: '重新命名',
		unsubscribe: '取消訂閱',
		reportFeed: '檢舉訂閱源⋯',
		feedInfo: '訂閱源資訊⋯',
		feedActions: (title) => `訂閱源「${title}」的動作`,
		unsubscribeTitle: '要取消訂閱嗎？',
		unsubscribeText: (title) => `要取消訂閱「${title}」嗎？你在其中的已讀紀錄和星號都會遺失。`,
		openFolder: '展開資料夾',
		closeFolder: '收合資料夾',
		openSidebar: '顯示側邊欄',
		closeSidebar: '隱藏側邊欄',
		search: '搜尋',
		searchPlaceholder: '在這些文章中搜尋',
		clearSearch: '清除搜尋',
		settings: '設定',
		accountSettings: '帳號設定',
		shortcuts: '鍵盤快速鍵',
		live: '即時',
		liveOn: '新文章會自動送達。按一下即可暫停。',
		liveOff: '已暫停。按一下即可讓新文章再次送達。',
		paused: '已暫停',
		account: (name) => `已登入為 ${name}`,
		logout: '登出',
		appSettings: '伺服器與 App 更新',
		install: '安裝為 App',
		installSafari:
			'要將 rueder 安裝為 App：在 Mac 上選擇「檔案 › 加入 Dock」，在 iPhone 或 iPad 上選擇「分享 › 加入主畫面」。',
		unread: (count) => `${n(count)} 篇未讀`,
		hiddenCount: (count) => `${n(count)} 篇被篩選器隱藏`,
		fetchFailed: (reason) => `上次擷取失敗：${reason}`,
		waiting: '尚未擷取',
		blocked: '已被這個 rueder 的營運者封鎖',
		moveHint: '拖曳即可排序，或按住 Alt 鍵使用方向鍵',
		resizeSidebar: '側邊欄寬度：拖曳或使用方向鍵',
		tooDeep: (levels) => `資料夾最多可以有 ${n(levels)} 層。`
	},

	toolbar: {
		refresh: '重新整理',
		showAll: '所有文章',
		showUnread: '僅未讀',
		show: '顯示',
		newestFirst: '最新的在前',
		oldestFirst: '最舊的在前',
		markAllRead: '全部標示為已讀',
		markOptions: '其他標示為已讀的方式',
		phoneMenu: (shown) => `檢視方式，目前顯示：${shown}`,
		markMenu: '標示為已讀⋯',
		markEverything: '所有文章',
		markOlderDay: '超過一天的文章',
		markOlderWeek: '超過一週的文章',
		listView: '列表檢視',
		expandedView: '展開檢視',
		splitView: '三欄：目錄、列表和文章',
		magazineView: '雜誌',
		previous: '上一篇文章',
		next: '下一篇文章',
		newArticles: (count) => `${n(count)} 篇新文章`
	},

	list: {
		empty: '這裡沒有文章。',
		emptyUnread: '這裡沒有未讀的內容。',
		emptySearch: '沒有符合搜尋的文章。',
		found: (count, more, words) => `含有「${words}」的文章 ${n(count)}${more ? '+' : ''} 篇`,
		searchAll: '在所有訂閱源中搜尋',
		feedFailing:
			'rueder 稍後會再試一次，每次失敗後或依網站的要求等待更久。如果一直失敗，訂閱源可能已經搬家。',
		hiddenNote: '你的篩選規則隱藏了這些文章。打開一篇就能看到是哪條規則。',
		toRules: '篩選規則',
		emptyFeed: '這個訂閱源還沒有文章。',
		waitingFeed: '這個訂閱源尚未擷取。它的文章會在下一輪擷取時送達，幾分鐘之內。',
		showAll: '顯示所有文章',
		more: '顯示更多',
		star: '加上星號',
		unstar: '移除星號',
		keepUnread: '保持未讀',
		openOriginal: '開啟原文',
		showFeed: '在 rueder 中顯示這個訂閱源的所有文章',
		from: '來自',
		by: (author) => `作者：${author}`,
		updated: '已更新',
		updatedHeadline: '新標題',
		updatedTitle: (count) => `首次擷取後有變更；先前的版本：${n(count)} 個`,
		updatedHeadlineTitle: (count) => `標題經過改寫，內文未變；先前的版本：${n(count)} 個`,
		compare: '比較版本',
		hiddenBy: (rule) => `被這條規則隱藏：${rule}`,
		textDropped: '這篇文章較舊，這台伺服器已不再保存它的內文。',
		textDroppedOriginal: '到原網站閱讀',
		loading: '正在載入內文⋯',
		loadingList: '正在載入⋯',
		toTop: '回到頂端',
		pick: '選擇一篇文章，就能在這裡閱讀。',
		resizeList: '列表寬度：拖曳或使用方向鍵',
		closeArticle: '關閉文章',
		blockedFeed: '這個 rueder 的營運者封鎖了這個訂閱源。它不會被擷取，文章也不會顯示。',
		articleActions: '文章的其他動作',
		articleInfo: '文章資訊⋯',
		articleSource: '原始碼⋯',
		reportArticle: '檢舉文章⋯'
	},

	report: {
		feedTitle: '檢舉訂閱源',
		articleTitle: '檢舉文章',
		text: (title) =>
			`這個 rueder 的營運者會看到「${title}」被檢舉、檢舉原因和你寫的內容，但不會看到是誰檢舉的。是否為所有人封鎖，由營運者決定。`,
		reason: '原因',
		reasons: {
			illegal: '違法',
			copyright: '他人的作品',
			malicious: '惡意軟體、網路釣魚或詐騙',
			spam: '垃圾內容',
			other: '其他'
		},
		note: '想補充的內容（選填）',
		left: (count) => `還可以輸入 ${n(count)} 個字元`,
		send: '送出檢舉',
		sending: '正在送出⋯',
		sent: '謝謝。檢舉已送達這個 rueder 的營運者。',
		tooMany: '你最近送出了很多檢舉。請稍後再試。',
		unreachable: '檢舉沒有送達伺服器。請稍後再試。'
	},

	feedInfo: {
		title: '訂閱源資訊',
		text: (title) =>
			`這個 rueder 對「${title}」所知的資訊，以及擷取的方式。如果訂閱源無法使用，請把這些複製到你給營運者的訊息裡。`,
		number: '編號',
		name: '名稱',
		address: '訂閱源網址',
		site: '網站',
		noSite: '無',
		agent: '擷取時的身分',
		agents: { rueder: 'rueder', browser: '瀏覽器' },
		every: '擷取間隔',
		fetched: '上次擷取',
		succeeded: '上次成功擷取',
		failed: '最近的錯誤',
		next: '下次擷取',
		blocked: '無，營運者已封鎖這個訂閱源',
		articles: '至今的文章',
		never: '從未',
		fetchAs: '擷取時的身分',
		fetchAsHint:
			'對所有閱讀這個訂閱源的人生效。網站拒絕 rueder 時，請選「瀏覽器」。會立即重新擷取。',
		notSwitched: '沒有成功。請稍後再試。',
		copy: '複製',
		copied: '已複製',
		loading: '正在載入⋯',
		unreachable: '伺服器沒有回應。請稍後再試。'
	},
	articleInfo: {
		title: '文章資訊',
		text: (title) =>
			`這個 rueder 對「${title}」及其版本所知的資訊。如果有不對的地方，請把這些複製到你給營運者的訊息裡。`,
		feed: '訂閱源',
		feedNumber: '訂閱源編號',
		number: '在訂閱源中的編號',
		name: '標題',
		id: '在訂閱源中的識別碼',
		link: '連結',
		noLink: '無',
		published: '發布',
		fetched: '首次擷取',
		changed: '上次變更',
		never: '從未',
		changedAt: (when, what) => `${when}（${what}）`,
		what: { title: '標題', text: '內文', both: '標題和內文' },
		version: '版本',
		kept: '保留的版本',
		textLabel: '內文',
		textDropped: '因為時間較久，已不再保留',
		copy: '複製',
		copied: '已複製',
		loading: '正在載入⋯',
		unreachable: '伺服器沒有回應。請稍後再試。'
	},
	articleSource: {
		title: '文章的原始碼',
		text: '文章的 HTML：可以看這個 rueder 清理後保存的樣子，也可以看訂閱源送來的原樣。連結和圖片在這裡保留各自的原始位址，所以看得出是否還剩下追蹤參數。',
		show: '顯示',
		kinds: { cleaned: '清理後', original: '訂閱源送來的原樣' },
		noText: '這個伺服器已不再保存這篇文章的內文。',
		noOriginal: '這個伺服器已不再保存訂閱源送來的內容。',
		copy: '複製',
		copied: '已複製',
		loading: '正在載入⋯',
		unreachable: '伺服器沒有回應。請稍後再試。'
	},

	media: {
		title: '附件',
		play: '播放',
		image: '圖片',
		zoomIn: '放大',
		zoomOut: '縮小'
	},

	connection: {
		reconnecting: '正在重新連線⋯',
		unreachable: (since) => `伺服器自 ${since} 起沒有回應。`,
		offline: (since) => `自 ${since} 起沒有網路。`,
		retry: '立即重試',
		unsent: (count) => `有 ${n(count)} 項變更正在等候。`
	},

	notice: {
		unreachable: '伺服器沒有回應。你看到的內容可能不是最新的。',
		notSaved: '伺服器沒有接受你上一次的變更。這裡已經復原。',
		tooManyUnsent: '等候伺服器的變更太多，這一項已經撤回。'
	},

	login: {
		title: '登入 rueder',
		about: '一個訂閱源閱讀器：它會擷取你追蹤的網站發布的內容，並顯示你還沒讀過的。',
		name: '使用者名稱',
		password: '密碼',
		submit: '登入',
		checking: '請稍候⋯',
		wrong: '使用者名稱或密碼錯誤。',
		codeNeeded:
			'這個帳號已開啟兩步驟驗證。請輸入 App 顯示的驗證碼或一組復原碼，或使用密碼金鑰登入。',
		wrongCode: '驗證碼不正確。驗證碼每 30 秒更換一次。',
		code: '驗證碼',
		codeNote: '驗證器 App 顯示的六位數字，或你的一組復原碼。',
		tooMany: (count) => `錯誤嘗試次數過多。請在 ${minutes(count)}後再試。`,
		unreachable: '伺服器沒有回應。請稍後再試。',
		retry: '重試',
		or: '或',
		oidc: '使用單一登入',
		passkey: '使用密碼金鑰登入',
		passkeyCancelled: '沒有使用密碼金鑰。請再試一次，或使用使用者名稱和密碼登入。',
		passkeyFailed: '這個瀏覽器或裝置無法在這裡使用密碼金鑰。',
		passkeyWrong: '這把密碼金鑰不屬於這裡的任何帳號。',
		oidcFailed: '單一登入沒有成功。請再試一次。',
		oidcDenied: '單一登入已取消。',
		oidcUnknown: '這裡沒有對應這次登入的帳號。請聯絡這個 rueder 的營運者進行連結。',
		disabled: '這個帳號已停用。原因請詢問這個 rueder 的營運者。',
		unconfirmed: '這個帳號正在等候確認電子郵件地址。請開啟郵件中的連結，或重新傳送。',
		resend: '重新傳送連結',
		resent: '連結已重新寄出。請查看收件匣，也看看垃圾郵件資料夾。',
		resendWrong: '要重新傳送連結，請輸入帳號的使用者名稱和密碼。',
		forgot: '忘記密碼？',
		removed: '你的帳號和屬於你的一切都已刪除。',
		note: '這裡不開放註冊。帳號由這個 rueder 的營運者建立。'
	},

	legal: {
		imprint: '法律聲明',
		privacy: '隱私權政策',
		terms: '使用條款',
		agree: { before: '我已閱讀並同意', link: '使用條款', after: '。' },
		changed: '使用條款已變更',
		changedIntro: '請閱讀新的使用條款。要繼續使用 rueder，請同意這些條款。',
		read: '閱讀使用條款',
		accept: '我同意',
		decline: '登出',
		notSaved: '沒有成功。請稍後再試。'
	},

	register: {
		title: '建立帳號',
		invited: '你受邀加入這個 rueder。請選擇使用者名稱和密碼。',
		again: '再輸入一次密碼',
		passwordRule: (characters) => `至少 ${n(characters)} 個字元。`,
		mismatch: '兩次輸入的密碼不同。',
		submit: '建立帳號',
		solving: '正在檢查這個瀏覽器⋯',
		toRegister: '建立帳號',
		toLogin: '登入現有帳號',
		closed: '沒有邀請，任何人都無法在這裡註冊。',
		inviteGone: '這個邀請已經無法使用。請索取新的邀請。',
		full: '這裡已經沒有空間容納更多帳號。',
		today: '今天在這裡建立的帳號已經夠多了。請明天再試。',
		captcha: '這個瀏覽器的檢查沒有通過。請再試一次。',
		taken: '已經有帳號使用這個使用者名稱。',
		terms: '要建立帳號，請同意使用條款。',
		invalid: (reason) => `無法建立這個帳號。伺服器表示：${reason}`,
		email: '電子郵件地址',
		emailOptional: '電子郵件地址（選填）',
		emailNote: '你會收到一個用於確認的連結。',
		emailRequiredNote: '你會收到一個用於確認的連結；開啟後即可用這個帳號登入。',
		emailInvalid: '請輸入可以收信的電子郵件地址。',
		unsent: '無法寄出連結，因此沒有建立帳號。請稍後再試。',
		tooMany: (count) => `嘗試次數過多。請在 ${minutes(count)}後再試。`,
		awaitingTitle: '請查看郵件',
		awaiting: (address, hours) =>
			`帳號已建立。連結已寄到 ${address}：請在 ${n(hours)} 小時內開啟，然後登入。`
	},

	forgot: {
		title: '忘記密碼？',
		intro: '請輸入使用者名稱或電子郵件地址。設定新密碼的連結會寄到已確認的地址。',
		field: '使用者名稱或電子郵件地址',
		submit: '寄出連結',
		asked: (count) =>
			`如果有這樣一個已確認地址的帳號，連結已寄到該地址，${minutes(count)}內有效。也請查看垃圾郵件資料夾。`,
		tooMany: (count) => `要求過於頻繁。請在 ${minutes(count)}後再試。`,
		back: '返回登入'
	},

	reset: {
		title: '設定新密碼',
		intro: '帳號在所有瀏覽器和裝置上都會登出。',
		fresh: '新密碼',
		again: '再輸入一次新密碼',
		submit: '設定密碼',
		mismatch: '兩次輸入不一致。',
		gone: '這個連結已經失效。請索取新的連結。',
		invalid: (reason) => `伺服器沒有接受這個密碼。它表示：${reason}`,
		done: '密碼已設定。請用新密碼登入。',
		cancel: '取消'
	},

	email: {
		title: '電子郵件地址',
		intro: 'rueder 只會為了你的帳號寄信給你。',
		current: '你的地址',
		none: '這個帳號沒有地址。',
		pending: (address, hours) =>
			`連結已寄到 ${address}。在 ${n(hours)} 小時內開啟後，新地址才會生效；在那之前保留原來的地址。`,
		address: '新地址',
		password: '你的密碼',
		passwordNote: '以免能使用這個瀏覽器的其他人把你的郵件轉到別處。',
		code: '兩步驟驗證的驗證碼',
		submit: '寄出連結',
		sent: '連結已寄出。請查看收件匣，也看看垃圾郵件資料夾。',
		remove: '刪除地址',
		removeIntro: '沒有地址就不會收到任何郵件。除此之外，帳號維持不變。',
		removed: '地址已刪除。',
		wrong: '密碼或驗證碼不正確。',
		codeNeeded: '這個帳號已開啟兩步驟驗證。請一併輸入驗證碼。',
		loginAgain: '你的帳號還沒有密碼，也沒有開啟兩步驟驗證。請重新登入，然後在十分鐘內填寫地址。',
		invalid: '請輸入可以收信的電子郵件地址。',
		same: '這就是這個帳號現有的地址。',
		required: '這個 rueder 的每個帳號都需要地址；可以變更，但不能刪除。',
		busy: (count) => `嘗試或郵件過多。請在 ${minutes(count)}後再試。`,
		unsent: '郵件無法寄出。請稍後再試。',
		unreachable: '伺服器沒有回應。沒有做任何變更。',
		askTitle: '新增電子郵件地址',
		askIntro:
			'這個 rueder 的每個帳號現在都需要電子郵件地址。請填寫你的地址，並開啟寄到該地址的連結。',
		askWaiting: '開啟連結後，這則提示會自動消失。沒有收到？請重新傳送，或寄到其他地址。',
		logOut: '登出',
		confirmed: (address) => `地址 ${address} 已確認。`,
		confirmedLogIn: '帳號已就緒：請登入。',
		gone: '這個連結已經無法使用：它已被使用過、已寄出了更新的連結，或已超過一天。',
		taken: '在這段期間，另一個帳號已經使用了這個地址。',
		confirmUnreachable: '伺服器沒有回應。請稍後再開啟連結。',
		close: '關閉'
	},

	invites: {
		intro: '透過連結，別人可以在這個 rueder 上建立帳號。連結只會在建立時於這裡顯示一次。',
		left: (count) => (count === 0 ? '你沒有可以寄出的邀請。' : `你還可以邀請 ${n(count)} 個人。`),
		admin: '身為管理員，你隨時都可以邀請。',
		uses: '可用於幾個帳號',
		make: '建立連結',
		noneLeft: '你剩下的邀請沒有這麼多。',
		failed: '沒有成功。請稍後再試。',
		link: '現在可以轉交的連結',
		copy: '複製',
		copied: '已複製',
		goodFor: (days) => `連結的有效期限為 ${n(days)} 天。`,
		yours: '你的連結',
		none: '你還沒有建立連結。',
		used: (uses, of) => `已使用 ${n(uses)}/${n(of)}`,
		until: (when) => `有效期限至 ${when}`,
		ran: (when) => `已於 ${when} 過期`,
		usedUp: '已用完',
		revoked: '已撤回',
		revoke: '撤回',
		revokeLink: (made) => `撤回 ${made} 建立的連結`
	},

	home: {
		newIn: '訂閱中的新內容',
		allRead: '你已經全部讀完了。新文章送達後會顯示在這裡。',
		recentlyStarred: '最近加上星號',
		noStars: '還沒有加上星號的內容。在文章上按 s 即可。',
		problems: '有問題的訂閱源',
		tips: '提示',
		tipKeys: '按 ? 查看所有鍵盤快速鍵；j 和 k 可在文章之間切換。',
		tipDrag: '在側邊欄中拖曳訂閱源和資料夾即可排序。',
		tipFilters: '設定中的篩選規則可以依關鍵字或作者隱藏文章。',
		showFeed: (title) => `顯示「${title}」`
	},

	versions: {
		title: '這篇文章的版本',
		current: '目前版本',
		version: (rev) => `第 ${n(rev)} 版`,
		fetched: (when) => `擷取於 ${when}`,
		compareWith: '將目前版本與這個版本比較',
		legendAdded: '新增',
		legendRemoved: '刪除',
		titleWas: '原標題：',
		same: '這個版本的內文與目前版本相同。',
		sameWords: '文字相同；圖片或連結有變動。',
		markup: '文字、圖片和連結都相同；只有內文的結構變了。',
		pictures: '有變動的圖片',
		noPicture: '無',
		address: '圖片位址',
		links: '有變動的連結'
	},

	account: {
		title: '帳號設定',
		tabs: {
			password: '密碼',
			email: '郵件',
			security: '安全性',
			apps: 'App',
			invites: '邀請',
			move: '搬家',
			privacy: '隱私權',
			remove: '刪除帳號'
		},
		password: {
			title: '變更密碼',
			titleFirst: '設定密碼',
			intro: '設定新密碼後，其他所有瀏覽器和裝置都會登出，所有 App 專用密碼也會被撤銷。',
			introFirst: '有了密碼，這個帳號除了單一登入之外，也能用使用者名稱登入。',
			current: '目前的密碼',
			fresh: '新密碼',
			again: '再輸入一次新密碼',
			submit: '變更密碼',
			submitFirst: '設定密碼',
			done: '密碼已變更。其他所有瀏覽器和裝置都已登出，App 專用密碼已被撤銷。',
			doneFirst: '密碼已設定。',
			mismatch: '兩次輸入不一致。',
			code: '兩步驟驗證的驗證碼',
			wrong: '目前的密碼或驗證碼不正確。',
			codeNeeded:
				'這個帳號已開啟兩步驟驗證。請一併輸入驗證器 App 的驗證碼或一組復原碼，或使用密碼金鑰確認。',
			loginAgain: '你的帳號還沒有密碼，也沒有開啟兩步驟驗證。請重新登入，然後在十分鐘內設定密碼。',
			invalid: (characters) => `伺服器沒有接受這個密碼。至少需要 ${n(characters)} 個字元。`,
			busy: (count) => `錯誤嘗試次數過多。請在 ${minutes(count)}後再試。`,
			unreachable: '伺服器沒有回應。密碼沒有變更。'
		},
		settingsNote: '外觀、閱讀、篩選規則和鍵盤位於「設定」中。'
	},

	settings: {
		title: '設定',
		tabs: {
			look: '外觀',
			reading: '閱讀',
			rules: '篩選規則',
			keys: '鍵盤'
		},
		accountNote: '密碼、兩步驟驗證、App 和邀請位於帳號選單的「帳號設定」中。',
		theme: '主題',
		themes: { system: '依照系統', light: '淺色', dark: '深色' },
		accent: '強調色',
		accents: {
			red: '紅色',
			blue: '藍色',
			green: '綠色',
			orange: '橙色',
			purple: '紫色',
			graphite: '石墨色'
		},
		density: '列表密度',
		densities: { compact: '緊密', comfortable: '寬鬆' },
		sidebarSide: '側邊欄',
		sidebarSides: { start: '左側', end: '右側' },
		textSize: '文章文字大小（這部裝置）',
		textSizes: { small: '小', medium: '中', large: '大' },
		readingFont: '文章字體',
		readingFonts: { sans: '無襯線', serif: '襯線' },
		counters: '未讀數',
		counterStyles: { parens: '括號內', badge: '標記' },
		language: '語言',
		languageAuto: '依照瀏覽器',
		view: '檢視方式',
		views: { list: '列表', expanded: '展開', split: '三欄', magazine: '雜誌' },
		markOnScroll: '在展開檢視中捲動經過的文章標示為已讀',
		teasers: '在列表中標題後顯示內文開頭',
		wakeLock: '開啟 rueder 時讓螢幕保持開啟',
		wakeLockMissing: '這個瀏覽器無法讓螢幕保持開啟。',
		note: '變更會立即生效，並和你的帳號一起儲存，所以在每部裝置上都一樣。',
		noteDemo: '變更會立即生效，並儲存在這個瀏覽器中。',
		apps: {
			intro:
				'其他訂閱源閱讀器 App 可以透過 Google Reader API 使用你的帳號。每個 App 都有自己的 App 專用密碼，你隨時可以撤銷。',
			none: '還沒有設定任何 App。',
			made: (when) => `設定於 ${when}`,
			used: (when) => `上次使用 ${when}`,
			unused: '尚未使用',
			revoke: '撤銷',
			revokeName: (name) => `撤銷 App 專用密碼「${name}」`,
			add: '設定 App',
			full: (max) => `每個帳號最多可以有 ${n(max)} 組 App 專用密碼。要設定其他 App，請先撤銷一組。`,
			choose: '你想使用哪個 App？',
			website: (app) => `${app} 的網站`,
			other: '其他 App',
			otherHint: '任何支援 Google Reader API 或 FreshRSS 的 App。',
			platforms: {
				apple: 'Mac、iPhone 和 iPad',
				android: 'Android'
			},
			name: 'App 專用密碼的名稱',
			password: '你的密碼',
			code: '驗證器 App 的驗證碼，或一組復原碼',
			create: '建立 App 專用密碼',
			creating: '請稍候⋯',
			back: '返回',
			how: {
				netnewswire:
					'在 NetNewsWire 中開啟「設定」，再開啟「帳號」，新增帳號（在 Mac 上按 +）並選擇 FreshRSS。將下方的使用者名稱填入 Login（在 iPhone 和 iPad 上為 Username or Email），App 專用密碼填入 Password，伺服器網址填入 API URL。',
				readyou:
					'在 Read You 中開啟「設定」、「帳號」、「新增帳號」，然後選擇 Google Reader。將下方的伺服器網址填入伺服器 URL，使用者名稱填入使用者名稱，App 專用密碼填入密碼。',
				other:
					'在你的 App 中新增「Google Reader API」類型的帳號，如果沒有，就新增「FreshRSS」。填入下方的伺服器網址、使用者名稱和 App 專用密碼。'
			},
			ownIcons: (app) =>
				`隱私權：${app} 會自行從各訂閱源的網站載入訂閱源圖示，這些網站因此會看到你的 IP 位址。文章和其中的圖片只透過 rueder 取得。`,
			shotsTitle: '圖文步驟',
			shots: {
				readyou: [
					'點一下左上角的齒輪。',
					'開啟「帳號」。',
					'點一下「新增帳號」。',
					'選擇 Google Reader。',
					'將伺服器網址填入伺服器 URL，使用者名稱填入使用者名稱，App 專用密碼填入密碼，然後點一下「新增」。'
				],
				netnewswire: [
					'在 Mac 上：開啟「設定」（⌘,），再開啟「帳號」，然後按一下左下角的 +。',
					'選擇 FreshRSS，然後按一下「繼續」。',
					'將使用者名稱填入 Login，App 專用密碼填入 Password，伺服器網址填入 API URL，然後按一下「建立」。'
				]
			},
			shotAlt: (app, step) => `${app}，第 ${n(step)} 步`,
			server: '伺服器網址',
			user: '使用者名稱',
			secret: 'App 專用密碼',
			copy: (what) => `複製${what}`,
			copied: '已複製',
			once: 'App 專用密碼只會在這時顯示。如果遺失，請撤銷並建立一組新的。',
			waiting: '正在等候 App 登入⋯',
			connected: '已連線：App 已登入。',
			done: '完成',
			refusals: {
				wrongPassword: '密碼不正確。',
				codeNeeded: '請一併輸入驗證器 App 的驗證碼或一組復原碼，或使用密碼金鑰確認。',
				wrongCode: '驗證碼不正確。驗證碼每 30 秒更換一次。',
				loginAgain:
					'你的帳號還沒有密碼，也沒有開啟兩步驟驗證。請重新登入，然後在十分鐘內建立 App 專用密碼。',
				tooManyApps: '每個帳號最多可以有 20 組 App 專用密碼。',
				tooMany: '錯誤嘗試次數過多。請過幾分鐘再試。',
				unreachable: '伺服器沒有回應。請稍後再試。'
			}
		},
		secondFactor: {
			title: '兩步驟驗證',
			intro: '開啟兩步驟驗證後，登入時還需要驗證器 App 的驗證碼，或一把密碼金鑰。',
			app: '驗證器 App',
			on: '已開啟。',
			left: (count) => `還剩 ${n(count)} 組復原碼。`,
			off: '已關閉。',
			byPasskeys: '未設定。你的密碼金鑰讓兩步驟驗證保持開啟。',
			adminNeeds:
				'這台伺服器只把開啟了兩步驟驗證的管理員視為管理員。要查看伺服器狀態，請設定兩步驟驗證，或用它的驗證碼重新登入。',
			password: '你的密碼',
			start: '設定',
			scan: '用驗證器 App 掃描這個 QR 碼，或輸入下方的金鑰。',
			qr: '驗證器 App 的 QR 碼',
			larger: '放大 QR 碼',
			smaller: '縮小 QR 碼',
			key: '金鑰',
			copyKey: '複製金鑰',
			copied: '已複製',
			code: 'App 目前顯示的驗證碼',
			codeOrRecovery: 'App 的驗證碼，或一組復原碼',
			confirm: '確認',
			cancel: '取消',
			codesTitle: '你的復原碼',
			codesIntro: '每組復原碼可以在沒有手機時登入一次。請妥善保管：它們只會在這時顯示。',
			copyCodes: '複製復原碼',
			codesDone: '我已經保存',
			switchOff: '關閉',
			switchOffNote: '關閉後，登入又只需要密碼。',
			switchOffNoteKeys: '你的密碼金鑰讓兩步驟驗證保持開啟。',
			startWithPasskey: '用密碼金鑰設定',
			switchOffWithPasskey: '用密碼金鑰關閉',
			refusals: {
				wrongPassword: '密碼不正確。',
				codeNeeded: '你的密碼金鑰讓兩步驟驗證保持開啟。請一併輸入一組復原碼。',
				wrongCode: '驗證碼不正確。驗證碼每 30 秒更換一次。',
				loginAgain:
					'你的帳號還沒有密碼，也沒有開啟兩步驟驗證。請重新登入，然後在十分鐘內設定 App。',
				tooMany: '錯誤嘗試次數過多。請過幾分鐘再試。',
				unreachable: '伺服器沒有回應。請稍後再試。'
			}
		},
		passkeys: {
			title: '密碼金鑰',
			intro:
				'使用密碼金鑰，不需要使用者名稱和密碼，只要裝置要求的 PIN 碼、指紋或臉孔就能登入。它也算是兩步驟驗證。',
			once: '有了密碼金鑰，單憑密碼就無法再開啟你的帳號。',
			none: '還沒有密碼金鑰。',
			made: (when) => `新增於 ${when}`,
			used: (when) => `上次使用 ${when}`,
			unused: '尚未使用',
			add: '新增密碼金鑰',
			name: '名稱，例如裝置名稱',
			defaultName: '密碼金鑰',
			password: '你的密碼',
			code: '驗證器 App 的驗證碼，或一組復原碼',
			create: '建立密碼金鑰',
			working: '請依照瀏覽器的指示操作⋯',
			cancel: '取消',
			remove: '刪除',
			removeName: (name) => `刪除密碼金鑰「${name}」`,
			confirmRemove: '刪除密碼金鑰',
			full: (max) => `每個帳號最多可以有 ${n(max)} 把密碼金鑰。`,
			or: '或',
			confirmWith: '用密碼金鑰確認',
			codesTitle: '你的復原碼',
			codesIntro:
				'你的第一把密碼金鑰開啟了兩步驟驗證。如果遺失裝置，每組復原碼可以登入一次。請妥善保管：它們只會在這時顯示。',
			copyCodes: '複製復原碼',
			copied: '已複製',
			codesDone: '我已經保存',
			refusals: {
				cancelled: '在使用密碼金鑰之前，瀏覽器視窗已經關閉。',
				known: '這部裝置上已經有你帳號的密碼金鑰。',
				failed: '這個瀏覽器或裝置無法建立或使用密碼金鑰。',
				wrong: '密碼金鑰未被接受。請再試一次。',
				wrongPassword: '密碼不正確。',
				codeNeeded:
					'你的帳號已開啟兩步驟驗證。請一併輸入 App 的驗證碼或一組復原碼，或使用密碼金鑰確認。',
				wrongCode: '驗證碼不正確。驗證碼每 30 秒更換一次。',
				tooManyPasskeys: '每個帳號最多可以有 10 把密碼金鑰。',
				loginAgain:
					'你的帳號還沒有密碼，也沒有開啟兩步驟驗證。請重新登入，然後在十分鐘內新增你的第一把密碼金鑰。',
				tooMany: '錯誤嘗試次數過多。請過幾分鐘再試。',
				unreachable: '伺服器沒有回應。請稍後再試。'
			}
		},
		reports: {
			send: '把這個瀏覽器中的錯誤報告傳送到這台伺服器',
			intro: '頁面出問題時，這台伺服器的管理員會知道。',
			what: '一份報告包含：',
			holds: [
				'錯誤訊息，不含網址，也不含指向某樣東西的數字',
				'錯誤發生在 rueder 本身檔案中的位置',
				'rueder 的版本，以及你的瀏覽器和它的主要版本號'
			],
			never: '絕不包含你讀的內容、你的訂閱源、標籤或規則。報告會在 30 天後刪除。',
			off: '關閉這個選項會刪除你已傳送的報告。'
		}
	},

	rules: {
		intro:
			'規則會在文章送達時將其隱藏或標示為已讀。被隱藏的文章不計入未讀，並保留在「被篩選器隱藏」中。',
		none: '還沒有規則。',
		add: '新增規則',
		new: '新規則',
		on: '開啟',
		scope: '範圍',
		scopeAll: '所有訂閱源',
		scopeFolder: (title) => `資料夾：${title}`,
		scopeFeed: (title) => `訂閱源：${title}`,
		field: '檢查',
		fields: { any: '全部', title: '標題', content: '內文', author: '作者', tag: '標籤' },
		match: '比較',
		matches: {
			contains: '包含',
			word: '包含單字',
			starts: '開頭是',
			ends: '結尾是',
			regex: '符合正規表示式',
			all: '每篇文章，不論內容'
		},
		pattern: '要尋找的文字',
		action: '然後',
		actions: { hide: '隱藏文章', read: '標示為已讀' },
		preview: (count, total) => `目前適用於這裡 ${n(total)} 篇文章中的 ${n(count)} 篇。`,
		regexTip: '正規表示式依 Rust 的方式解讀，不區分大小寫。',
		regexBuilder: '在 regex101 上撰寫並測試',
		invalid: '這不是有效的正規表示式。',
		empty: '請輸入要尋找的文字。',
		checking: '正在查看它會符合哪些文章⋯',
		hits: (count) => `${n(count)} 個符合`,
		summary: (field, match, pattern, action) => `${field}${match}「${pattern}」：${action}`,
		summaryAll: (action) => `每篇文章：${action}`
	},

	keys: {
		title: '鍵盤快速鍵',
		note: '你的按鍵會和設定一起儲存。Enter、Esc 和 g 維持不變。',
		change: (what) => `變更這個動作的按鍵：${what}`,
		changeShort: '變更',
		press: '請按一個鍵⋯',
		taken: (key, other) => `${key} 已經是這個動作的按鍵：${other}`,
		fixed: (key) => `${key} 不能指定為快速鍵。`,
		reset: '還原 rueder 的預設按鍵',
		groups: { articles: '文章', lists: '列表', go: '前往', app: '任何地方' },
		next: '開啟下一篇文章',
		previous: '開啟上一篇文章',
		nextCursor: '移到下一篇文章，不開啟',
		previousCursor: '移到上一篇文章，不開啟',
		toggle: '開啟或關閉文章',
		star: '加上或移除星號',
		read: '標示為已讀或未讀',
		original: '在新分頁中開啟原文',
		markAll: '全部標示為已讀',
		refresh: '重新整理列表',
		unreadOnly: '僅未讀，或所有文章',
		listView: '列表檢視',
		expandedView: '展開檢視',
		splitView: '三欄',
		magazineView: '雜誌',
		goHome: '首頁',
		goAll: '所有文章',
		goUnread: '未讀文章',
		goStarred: '已加星號的文章',
		search: '搜尋',
		sidebar: '顯示或隱藏側邊欄',
		help: '這份總覽',
		then: '然後'
	},

	labels: {
		title: '標籤',
		add: '新增標籤',
		name: '標籤名稱',
		none: '還沒有標籤。',
		actions: (title) => `標籤「${title}」的動作`,
		rename: '重新命名',
		color: '顏色',
		colors: ['紅色', '橙色', '黃色', '綠色', '青色', '藍色', '紫色', '灰色'],
		remove: '刪除標籤',
		show: '顯示標籤',
		hide: '隱藏標籤',
		forArticle: '標籤',
		set: (title) => `標籤：${title}`
	},

	subscribe: {
		title: '訂閱一個訂閱源',
		address: '訂閱源或網站的網址',
		find: '尋找訂閱源',
		finding: '正在尋找⋯',
		note: '草稿無法擷取任何內容，所以訂閱源會保持空白。',
		found: (count) => `找到 ${n(count)} 個訂閱源：`,
		none: '這個網址沒有訂閱源。',
		movedTo: (url) => `網站把要求轉到了 ${url}。`,
		loginWall: '有些網站會把沒有登入的人導向登入頁面。這時就無法讀取它們的訂閱源。',
		subscribed: '已訂閱',
		show: '顯示',
		untitled: '沒有名稱',
		page: '一個沒有訂閱源的網站：rueder 會為它產生一個，只有標題和摘要。',
		folder: '放入資料夾',
		topLevel: '不放入資料夾',
		add: '訂閱',
		adding: '正在訂閱⋯',
		refused: '沒有成功。',
		serverSays: (reason) => `伺服器表示：${reason}`,
		busy: (seconds) =>
			seconds ? `同時尋找的太多。請在 ${n(seconds)} 秒後再試。` : '同時尋找的太多。請稍後再試。',
		noFetching: '這台伺服器不擷取訂閱源。',
		tooLarge: (megabytes) => `檔案超過 ${n(megabytes)} MB。`,
		unreachable: '伺服器沒有回應。請稍後再試。'
	},

	opml: {
		title: '匯入和匯出',
		intro: '用 OPML 檔案從其他訂閱源閱讀器帶來你的訂閱，或把它們帶走。',
		import: '匯入 OPML 檔案',
		importing: '正在匯入⋯',
		imported: (feeds, folders) => `新訂閱源 ${n(feeds)} 個，新資料夾 ${n(folders)} 個。`,
		known: (count) => `有 ${n(count)} 個訂閱源已經訂閱過。`,
		skipped: (count) => `略過了 ${n(count)} 個無法讀取的項目。`,
		later: '它們的文章會在下一輪擷取時送達。',
		export: '匯出為 OPML'
	},

	removeAccount: {
		title: '刪除帳號',
		intro: '這會永久刪除你在這個 rueder 上的帳號。無法復原。',
		goes: '一併刪除的有：你在所有裝置上的工作階段、訂閱和資料夾、已讀紀錄、星號、標籤、篩選規則、設定、錯誤報告、邀請，以及與單一登入的連結。',
		stays: '訂閱源會為其他閱讀它們的人保留。要先把帳號搬到別處，請使用「搬家」。',
		password: '你的密碼',
		name: (name) => `請輸入你的使用者名稱「${name}」以確認`,
		code: 'App 的驗證碼，或一組復原碼',
		submit: '永久刪除我的帳號',
		wrong: '不正確。沒有刪除任何內容。',
		codeNeeded:
			'你的帳號已開啟兩步驟驗證。請一併輸入 App 的驗證碼或一組復原碼，或使用密碼金鑰確認。',
		busy: (count) => `錯誤嘗試次數過多。請在 ${minutes(count)}後再試。`,
		unreachable: '伺服器沒有回應。沒有刪除任何內容。'
	},

	move: {
		intro: '用一個檔案把你的帳號搬到另一個 rueder，或從那裡搬來這裡。',
		howItWorks: '匯入只會在這裡現有的內容上增加，不會刪除任何東西。你的設定會變成檔案中的設定。',
		notInIt: '檔案中不包含：你的密碼、工作階段和文章內文。',
		export: '匯出帳號',
		import: '匯入帳號檔案',
		importing: '正在匯入⋯',
		feeds: (feeds, known) => `新訂閱源 ${n(feeds)} 個，已訂閱 ${n(known)} 個。`,
		folders: (count) => `新資料夾 ${n(count)} 個。`,
		articles: (found) => `在這裡找到 ${articles(found)}。`,
		missing: (count) => `有 ${articles(count)}在這裡找不到；它們的已讀、星號和標籤紀錄已經遺失。`,
		marks: (labels, rules) => `新標籤 ${n(labels)} 個，新篩選規則 ${n(rules)} 條。`,
		skipped: (count) => `檔案中有 ${n(count)} 個項目無法匯入。`,
		settings: '你的設定現在是檔案中的設定。',
		later: '新訂閱源的文章會在下一輪擷取時送達。已讀的維持已讀。',
		busy: (seconds) =>
			seconds
				? `匯入次數過多。請在 ${minutes(Math.ceil(seconds / 60))}後再試。`
				: '匯入次數過多。請稍後再試。'
	},

	status: {
		title: '伺服器狀態',
		open: '伺服器狀態',
		loading: '正在詢問伺服器⋯',
		failed: '伺服器沒有回應。請稍後再試。',
		refresh: '重新整理',
		asOf: (time) => `截至 ${time}。統計的是最近一小時。`,
		demo: '這裡沒有伺服器，所以這些是虛構的。',
		empty: '–',
		on: '開啟',
		off: '關閉',
		took: (median, p95) => `中位數 ${median}，95% 在 ${p95} 以內`,
		summary: {
			label: '總覽',
			hints: '提示',
			failing: '失敗的訂閱源',
			errors: '記錄的錯誤',
			reported: '被檢舉的訂閱源'
		},
		hintsTitle: '值得一看',
		allWell: '沒有異常。',
		hints: {
			slowRoute: (route, took) => `${route} 較慢：每 20 個要求中就有 1 個超過 ${took}。`,
			serverErrors: (route, count) => `${route} 以伺服器錯誤回應了 ${n(count)} 次。`,
			poolBusy: (size) => `通往資料庫的 ${n(size)} 條連線全都在使用中，要求只能等候。`,
			fetchBacklog: (count, late) => `有 ${n(count)} 個訂閱源擷取延遲，最晚的延遲了 ${late}。`,
			feedsFailing: (failing, subscribed) => `${n(subscribed)} 個訂閱源中有 ${n(failing)} 個失敗。`,
			notFetching: '伺服器沒有擷取訂閱源。FETCH 是否設為 off？',
			fetchPaceFull: (most) =>
				`每一輪都盡可能多擷取，但仍有訂閱源在等候：目前的節奏每小時最多擷取 ${n(most)} 次。由 FETCH_ROUND、FETCH_AT_ONCE 和 FETCH_EVERY 設定。`,
			cacheMisses: (share) =>
				`資料庫讀取的內容中只有 ${share} 在記憶體中；加大 shared_buffers 會有幫助。`,
			databaseLarge: (size, warn) =>
				`資料庫已成長到 ${size}；DATABASE_WARN_GB 從 ${warn} 起發出警告。「rueder retention set」可刪除舊文章或其內文。`,
			longQuery: (took) => `有一個查詢已經執行了 ${took}。`,
			imageCacheFull: (share) => `圖片快取已使用 ${share}。`,
			errors: (count) => `伺服器在最近一小時記錄了 ${n(count)} 個錯誤。`,
			limited: (count) => `有 ${n(count)} 個要求因為過多而被拒絕。`,
			browserErrors: (count) => `瀏覽器在最近一天回報了 ${n(count)} 種錯誤。`,
			reportedFeeds: (count) => `讀者檢舉了 ${n(count)} 個尚未封鎖的訂閱源。`
		},
		build: {
			title: '建置',
			commit: '提交',
			dirty: (commit) => `${commit}，含有本機變更`,
			branch: '分支',
			tag: '標記',
			committed: '提交時間',
			where: '執行於',
			container: '容器中',
			host: '主機上',
			target: '目標',
			profile: '編譯',
			profileValue: (profile, level) => `${profile}，最佳化等級 ${level}`,
			ci: '建置者',
			ciValue: '一次 CI 執行',
			byHand: '手動'
		},
		server: {
			title: '伺服器',
			version: '版本',
			running: '開始執行於',
			memory: '記憶體',
			accounts: '帳號',
			feeds: '訂閱源',
			feedsValue: (fetched, all) => `擷取中 ${n(fetched)} 個，共 ${n(all)} 個`,
			articles: '文章',
			articlesValue: (count) => `約 ${n(count)} 篇`,
			live: '開啟的分頁'
		},
		database: {
			title: '資料庫',
			pool: 'rueder 的連線',
			poolValue: (size, idle, max) => `開啟 ${n(size)} 條，閒置 ${n(idle)} 條，最多 ${n(max)} 條`,
			connections: '所有連線',
			size: '大小',
			sizeWarn: (size, warn) => `${size}，從 ${warn} 起警告`,
			hitRatio: '從記憶體讀取',
			deadlocks: '死結',
			longest: '執行最久的查詢',
			tables: '最大的資料表',
			table: '資料表',
			bytes: '大小',
			rows: '列數（約）'
		},
		fetcher: {
			title: '擷取訂閱源',
			state: '擷取',
			lastRound: '上一輪',
			lastRoundValue: (ago, feeds, took) => `${ago}：${n(feeds)} 個訂閱源，耗時 ${took}`,
			fetches: '擷取次數',
			fetchesValue: (fetched, unchanged, failed) =>
				`帶來內容 ${n(fetched)} 次，沒有變化 ${n(unchanged)} 次，失敗 ${n(failed)} 次`,
			broken: '在資料庫端失敗',
			newArticles: '新文章',
			took: '每次擷取的時間',
			overdue: '延遲擷取',
			overdueValue: (count, late) => `${n(count)} 個，最晚的延遲了 ${late}`,
			waiting: '尚未擷取',
			pace: '節奏',
			paceValue: (round, every, atOnce, most) =>
				`每 ${every} ${n(round)} 個訂閱源，同時 ${n(atOnce)} 個：每小時最多 ${n(most)} 次`,
			perHour: '最近一小時的擷取次數',
			behind: '延遲',
			behindValue: (late) => `等候最久的訂閱源已逾時 ${late}`,
			notBehind: '沒有在等候的',
			failing: '失敗的訂閱源（依原因）',
			noneFailing: '沒有失敗的訂閱源。',
			feeds: (count) => `${n(count)} 個訂閱源`,
			failures: {
				status: (code) => (code ? `網站回應 ${code}` : '網站回應錯誤'),
				timeout: '網站回應太慢',
				blocked: '位址在 rueder 不從中擷取內容的網路裡',
				dns: '這個名稱沒有對應的位址',
				network: '無法連線，或連線中斷',
				too_large: '太大',
				redirects: '重新導向太多次，或重新導向有誤',
				unreadable: '不是 rueder 能讀取的訂閱源',
				other: '其他'
			}
		},
		finder: {
			title: '尋找訂閱源',
			state: '檢查網址',
			lookups: '尋找次數',
			lookupsValue: (found, none, failed) =>
				`找到訂閱源 ${n(found)} 次，沒有找到 ${n(none)} 次，失敗 ${n(failed)} 次`,
			limited: '超過帳號限額',
			took: '每次尋找的時間'
		},
		rssifier: {
			title: '沒有訂閱源的網站',
			state: 'RSSifier',
			pages: '閱讀中的網站',
			pagesValue: (pages, failing) => `${n(pages)} 個，其中 ${n(failing)} 個失敗`,
			used: '權杖用量',
			usedValue: (requests, fetches, underWay) =>
				`最近一分鐘 ${n(requests)} 次請求，最近一小時抓取 ${n(fetches)} 個頁面，進行中 ${n(underWay)} 個`,
			unanswered: (why) => `RSSifier 沒有回應：${why}`,
			limits: '權杖限額',
			limitsValue: (perMinute, perHour, atOnce) =>
				`每分鐘 ${perMinute}，每小時 ${perHour} 個頁面，同時 ${atOnce} 個`,
			noLimit: '無限制'
		},
		images: {
			title: '圖片代理伺服器',
			state: '代理伺服器',
			cache: '快取',
			cacheValue: (used, max) => `${used} / ${max}`,
			served: '圖片',
			servedValue: (cached, fetched) => `來自快取 ${n(cached)} 張，擷取 ${n(fetched)} 張`,
			refused: '已拒絕',
			refusedValue: (source, notImage, tooLarge, notFound) =>
				`來源失敗 ${n(source)} 張，不是圖片 ${n(notImage)} 張，太大 ${n(tooLarge)} 張，網址錯誤 ${n(notFound)} 張`,
			took: '每張擷取圖片的時間',
			failingSources: '暫時不再要求的來源'
		},
		requests: {
			title: '要求',
			all: '所有要求',
			allValue: (count, took) => `${n(count)} 個，${took}`,
			answers: '回應',
			answersValue: (fine, refused, errors) =>
				`正常 ${n(fine)} 個，拒絕 ${n(refused)} 個，伺服器錯誤 ${n(errors)} 個`,
			route: '路由',
			count: '次數',
			median: '中位數',
			p95: '95%',
			max: '最長',
			errors: '錯誤',
			none: '最近一小時沒有要求。'
		},
		problems: {
			title: '警告和錯誤',
			counts: (errors, warnings) => `最近一小時有 ${n(errors)} 個錯誤和 ${n(warnings)} 個警告。`,
			none: '自啟動以來沒有記錄任何內容。',
			error: '錯誤',
			warning: '警告',
			times: (count, last) => (count > 1 ? `${n(count)} 次，最近一次 ${last}` : last)
		},
		browsers: {
			title: '瀏覽器中的錯誤',
			off: '這台伺服器不接收錯誤報告（CLIENT_REPORTS=off）。',
			consenting: (count) => `有 ${n(count)} 個帳號傳送報告。錯誤在最後一次出現後保留 30 天。`,
			none: '沒有瀏覽器回報錯誤。',
			kinds: {
				error: '錯誤',
				rejection: '失敗的 Promise',
				component: '頁面的一部分',
				request: '要求'
			},
			hits: (count, accounts, last) =>
				`${n(accounts)} 個帳號中出現 ${n(count)} 次，最近一次 ${last}`,
			where: (versions, browsers) => `版本 ${versions} · ${browsers}`
		},
		reported: {
			title: '被檢舉的訂閱源',
			text: '這裡不顯示檢舉人。「rueder feed block <網址>」為所有人封鎖一個訂閱源，「rueder feed unblock <網址>」解除封鎖。',
			none: '沒有人檢舉訂閱源。',
			reports: (count, last) => `${n(count)} 次檢舉，最近一次 ${last}`,
			blocked: '已封鎖'
		}
	},

	update: {
		ready: 'rueder 的新版本已經就緒。',
		reload: '重新載入',
		later: '稍後'
	},

	crash: {
		title: '這個頁面出了問題。',
		text: '請重新載入頁面以繼續。你讀過和變更的內容都已儲存在伺服器上。',
		reload: '重新載入'
	},

	demo: {
		badge: '草稿',
		note: '使用虛構訂閱源的草稿。不會在伺服器上儲存任何內容。',
		arrive: '讓一篇文章送達',
		revise: '變更開啟的文章',
		revised: '更新：為了試用版本功能，加上了這一段。',
		reviseHint: '請先開啟一篇文章。'
	}
};

export default zhHant;
