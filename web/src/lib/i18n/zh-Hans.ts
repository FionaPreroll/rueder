// The Chinese texts in simplified characters, in the shape of the English catalog
// (en.ts), addressing the reader with “你”. Browsers in zh, zh-CN and zh-SG get
// them. Chinese has no plural, so counts take a measure word; as in the style of
// Apple and Google, a half-width space separates numbers and Latin words from the
// Chinese around them.

import type { Messages } from './en.ts';
import { formatNumber as n } from './intl.ts';

const articles = (count: number) => `${n(count)} 篇文章`;

const minutes = (count: number) => `${n(count)} 分钟`;

const zhHans: Messages = {
	languageName: '简体中文',

	articles,

	common: {
		close: '关闭',
		cancel: '取消',
		save: '保存',
		delete: '删除',
		edit: '编辑'
	},

	nav: {
		subscribe: '订阅',
		home: '首页',
		all: '全部文章',
		unreadItems: '未读文章',
		starred: '加星标的文章',
		hidden: '被过滤器隐藏',
		subscriptions: '订阅',
		newFolder: '新建文件夹',
		groupedFolder: '新建文件夹',
		folderName: '文件夹名称',
		renameFolder: '重命名',
		removeFolder: '删除文件夹，保留其中内容',
		folderActions: (title) => `文件夹“${title}”的操作`,
		feedName: '订阅源名称',
		feedNameHint: '留空则使用订阅源自己的名称',
		renameFeed: '重命名',
		unsubscribe: '取消订阅',
		reportFeed: '举报订阅源…',
		feedInfo: '订阅源信息…',
		feedActions: (title) => `订阅源“${title}”的操作`,
		unsubscribeTitle: '取消订阅？',
		unsubscribeText: (title) => `要取消订阅“${title}”吗？你在其中的已读记录和星标都会丢失。`,
		openFolder: '展开文件夹',
		closeFolder: '收起文件夹',
		openSidebar: '显示侧边栏',
		closeSidebar: '隐藏侧边栏',
		search: '搜索',
		searchPlaceholder: '在这些文章中搜索',
		clearSearch: '清除搜索',
		settings: '设置',
		accountSettings: '账户设置',
		shortcuts: '键盘快捷键',
		live: '实时',
		liveOn: '新文章会自动到达。点击可暂停。',
		liveOff: '已暂停。点击可让新文章重新到达。',
		paused: '已暂停',
		account: (name) => `已登录为 ${name}`,
		logout: '退出登录',
		appSettings: '服务器与应用更新',
		install: '安装为应用',
		installSafari:
			'要将 rueder 安装为应用：在 Mac 上选择“文件 › 添加到程序坞”，在 iPhone 或 iPad 上选择“共享 › 添加到主屏幕”。',
		unread: (count) => `${n(count)} 篇未读`,
		hiddenCount: (count) => `${n(count)} 篇被过滤器隐藏`,
		fetchFailed: (reason) => `上次获取失败：${reason}`,
		waiting: '尚未获取',
		blocked: '已被此 rueder 的运营者屏蔽',
		moveHint: '拖动以排序，或按住 Alt 键使用方向键',
		resizeSidebar: '侧边栏宽度：拖动或使用方向键',
		tooDeep: (levels) => `文件夹最多可嵌套 ${n(levels)} 层。`
	},

	toolbar: {
		refresh: '刷新',
		showAll: '全部文章',
		showUnread: '仅未读',
		show: '显示',
		newestFirst: '最新的在前',
		oldestFirst: '最早的在前',
		markAllRead: '全部标记为已读',
		markOptions: '更多标记为已读的方式',
		phoneMenu: (shown) => `视图，当前显示：${shown}`,
		markMenu: '标记为已读…',
		markEverything: '全部文章',
		markOlderDay: '一天前的文章',
		markOlderWeek: '一周前的文章',
		listView: '列表视图',
		expandedView: '展开视图',
		splitView: '三栏：目录、列表和文章',
		magazineView: '杂志',
		previous: '上一篇文章',
		next: '下一篇文章',
		newArticles: (count) => `${n(count)} 篇新文章`
	},

	list: {
		empty: '这里没有文章。',
		emptyUnread: '这里没有未读内容。',
		emptySearch: '没有与搜索匹配的文章。',
		found: (count, more, words) => `包含“${words}”的文章 ${n(count)}${more ? '+' : ''} 篇`,
		searchAll: '在所有订阅源中搜索',
		feedFailing:
			'rueder 稍后会重试，每次失败后或按网站的要求等待更长时间。如果一直失败，订阅源可能已经迁移。',
		hiddenNote: '你的过滤规则隐藏了这些文章。打开一篇即可看到是哪条规则。',
		toRules: '过滤规则',
		emptyFeed: '此订阅源还没有文章。',
		waitingFeed: '此订阅源尚未获取。其文章会在下一轮获取中到达，几分钟之内。',
		showAll: '显示全部文章',
		more: '显示更多',
		star: '加星标',
		unstar: '取消星标',
		keepUnread: '保持未读',
		openOriginal: '打开原文',
		showFeed: '在 rueder 中显示此订阅源的所有文章',
		from: '来自',
		by: (author) => `作者：${author}`,
		updated: '已更新',
		updatedHeadline: '新标题',
		updatedTitle: (count) => `首次获取后有改动；之前的版本：${n(count)} 个`,
		updatedHeadlineTitle: (count) => `标题被改写，正文未变；之前的版本：${n(count)} 个`,
		compare: '比较版本',
		hiddenBy: (rule) => `被此规则隐藏：${rule}`,
		textDropped: '这篇文章较旧，此服务器已不再保存其正文。',
		textDroppedOriginal: '到原网站阅读',
		loading: '正在加载正文…',
		loadingList: '正在加载…',
		toTop: '回到顶部',
		pick: '选择一篇文章即可在此阅读。',
		resizeList: '列表宽度：拖动或使用方向键',
		closeArticle: '关闭文章',
		blockedFeed: '此 rueder 的运营者屏蔽了这个订阅源。它不会被获取，其文章也不会显示。',
		articleActions: '文章的更多操作',
		articleInfo: '文章信息…',
		articleSource: '源代码…',
		reportArticle: '举报文章…'
	},

	report: {
		feedTitle: '举报订阅源',
		articleTitle: '举报文章',
		text: (title) =>
			`此 rueder 的运营者会看到“${title}”被举报、举报原因以及你写的内容，但不会看到举报人是谁。是否为所有人屏蔽由运营者决定。`,
		reason: '原因',
		reasons: {
			illegal: '违法',
			copyright: '他人的作品',
			malicious: '恶意软件、网络钓鱼或诈骗',
			spam: '垃圾内容',
			other: '其他'
		},
		note: '想补充的内容（可选）',
		left: (count) => `还可输入 ${n(count)} 个字符`,
		send: '发送举报',
		sending: '正在发送…',
		sent: '谢谢。举报已送达此 rueder 的运营者。',
		tooMany: '你最近发送的举报很多。请稍后再试。',
		unreachable: '举报未能送达服务器。请稍后再试。'
	},

	feedInfo: {
		title: '订阅源信息',
		text: (title) =>
			`此 rueder 对“${title}”所知的信息以及获取方式。如果订阅源无法使用，请把这些复制到你发给运营者的消息里。`,
		number: '编号',
		name: '名称',
		address: '订阅源地址',
		site: '网站',
		noSite: '无',
		agent: '获取时的身份',
		agents: { rueder: 'rueder', browser: '浏览器' },
		every: '获取间隔',
		fetched: '上次获取',
		succeeded: '上次成功获取',
		failed: '最近的错误',
		next: '下次获取',
		blocked: '无，运营者已屏蔽此订阅源',
		articles: '至今的文章',
		never: '从未',
		fetchAs: '获取时的身份',
		fetchAsHint: '对所有阅读此订阅源的人生效。对拒绝 rueder 的网站，请选“浏览器”。会立即重新获取。',
		notSwitched: '没有成功。请稍后再试。',
		copy: '复制',
		copied: '已复制',
		loading: '正在加载…',
		unreachable: '服务器没有响应。请稍后再试。'
	},
	articleInfo: {
		title: '文章信息',
		text: (title) =>
			`此 rueder 对“${title}”及其版本所知的信息。如果有不对的地方，请把这些复制到你发给运营者的消息里。`,
		feed: '订阅源',
		feedNumber: '订阅源编号',
		number: '在订阅源中的编号',
		name: '标题',
		id: '在订阅源中的标识',
		link: '链接',
		noLink: '无',
		published: '发布',
		fetched: '首次获取',
		changed: '上次更改',
		never: '从未',
		changedAt: (when, what) => `${when}（${what}）`,
		what: { title: '标题', text: '正文', both: '标题和正文' },
		version: '版本',
		kept: '保留的版本',
		textLabel: '正文',
		textDropped: '因时间较久，已不再保留',
		copy: '复制',
		copied: '已复制',
		loading: '正在加载…',
		unreachable: '服务器没有响应。请稍后再试。'
	},
	articleSource: {
		title: '文章的源代码',
		text: '文章的 HTML：可以看此 rueder 清理后保存的样子，也可以看订阅源发来的原样。链接和图片在这里保留各自的原始地址，所以能看出是否还剩下跟踪参数。',
		show: '显示',
		kinds: { cleaned: '清理后', original: '订阅源发来的原样' },
		noText: '此服务器已不再保存这篇文章的正文。',
		noOriginal: '此服务器已不再保存订阅源发来的内容。',
		copy: '复制',
		copied: '已复制',
		loading: '正在加载…',
		unreachable: '服务器没有响应。请稍后再试。'
	},

	media: {
		title: '附件',
		play: '播放',
		image: '图片',
		zoomIn: '放大',
		zoomOut: '缩小'
	},

	connection: {
		reconnecting: '正在重新连接…',
		unreachable: (since) => `服务器自 ${since} 起没有响应。`,
		offline: (since) => `自 ${since} 起没有网络。`,
		retry: '立即重试',
		unsent: (count) => `有 ${n(count)} 项更改正在等待。`
	},

	notice: {
		unreachable: '服务器没有响应。你看到的内容可能不是最新的。',
		notSaved: '服务器没有接受你上一次的更改。这里已撤销。',
		tooManyUnsent: '等待服务器的更改太多，这一项已被撤回。'
	},

	login: {
		title: '登录 rueder',
		about: '一个订阅源阅读器：它获取你关注的网站发布的内容，并显示你还没读过的。',
		name: '用户名',
		password: '密码',
		submit: '登录',
		checking: '请稍候…',
		wrong: '用户名或密码错误。',
		codeNeeded: '此账户已开启两步验证。请输入应用显示的验证码或一个恢复码，或使用通行密钥登录。',
		wrongCode: '验证码不正确。验证码每 30 秒更换一次。',
		code: '验证码',
		codeNote: '身份验证器应用显示的六位数字，或你的一个恢复码。',
		tooMany: (count) => `错误尝试次数过多。请在 ${minutes(count)}后重试。`,
		unreachable: '服务器没有响应。请稍后再试。',
		retry: '重试',
		or: '或',
		oidc: '使用单点登录',
		passkey: '使用通行密钥登录',
		passkeyCancelled: '没有使用通行密钥。请重试，或使用用户名和密码登录。',
		passkeyFailed: '此浏览器或设备无法在这里使用通行密钥。',
		passkeyWrong: '这个通行密钥不属于这里的任何账户。',
		oidcFailed: '单点登录没有成功。请重试。',
		oidcDenied: '单点登录已取消。',
		oidcUnknown: '这里没有与该登录对应的账户。请联系此 rueder 的运营者进行关联。',
		disabled: '此账户已被停用。原因请询问此 rueder 的运营者。',
		unconfirmed: '此账户正在等待确认电子邮件地址。请打开邮件中的链接，或重新发送。',
		resend: '重新发送链接',
		resent: '链接已重新发出。请查看收件箱，也看看垃圾邮件文件夹。',
		resendWrong: '要重新发送链接，请输入账户的用户名和密码。',
		forgot: '忘记密码？',
		removed: '你的账户及属于你的一切都已删除。',
		note: '这里不开放注册。账户由此 rueder 的运营者创建。'
	},

	legal: {
		imprint: '法律声明',
		privacy: '隐私政策',
		terms: '使用条款',
		agree: { before: '我已阅读并同意', link: '使用条款', after: '。' },
		changed: '使用条款已更改',
		changedIntro: '请阅读新的使用条款。要继续使用 rueder，请同意这些条款。',
		read: '阅读使用条款',
		accept: '我同意',
		decline: '退出登录',
		notSaved: '没有成功。请稍后再试。'
	},

	register: {
		title: '创建账户',
		invited: '你受邀加入此 rueder。请选择用户名和密码。',
		again: '再次输入密码',
		passwordRule: (characters) => `至少 ${n(characters)} 个字符。`,
		mismatch: '两次输入的密码不同。',
		submit: '创建账户',
		solving: '正在检查此浏览器…',
		toRegister: '创建账户',
		toLogin: '登录已有账户',
		closed: '没有邀请，任何人都无法在这里注册。',
		inviteGone: '此邀请已无法使用。请索取新的邀请。',
		full: '这里已没有空间容纳更多账户。',
		today: '今天这里创建的账户已经足够。请明天再试。',
		captcha: '此浏览器的检查未通过。请重试。',
		taken: '已有使用此用户名的账户。',
		terms: '要创建账户，请同意使用条款。',
		invalid: (reason) => `无法创建这个账户。服务器提示：${reason}`,
		email: '电子邮件地址',
		emailOptional: '电子邮件地址（可选）',
		emailNote: '你会收到一个用于确认的链接。',
		emailRequiredNote: '你会收到一个用于确认的链接；打开后即可用此账户登录。',
		emailInvalid: '请输入一个能接收邮件的电子邮件地址。',
		unsent: '无法发送链接，因此没有创建账户。请稍后再试。',
		tooMany: (count) => `尝试次数过多。请在 ${minutes(count)}后重试。`,
		awaitingTitle: '请查看邮件',
		awaiting: (address, hours) =>
			`账户已创建。链接已发送到 ${address}：请在 ${n(hours)} 小时内打开，然后登录。`
	},

	forgot: {
		title: '忘记密码？',
		intro: '请输入用户名或电子邮件地址。设置新密码的链接会发送到已确认的地址。',
		field: '用户名或电子邮件地址',
		submit: '发送链接',
		asked: (count) =>
			`如果存在这样一个已确认地址的账户，链接已发往该地址，${minutes(count)}内有效。也请查看垃圾邮件文件夹。`,
		tooMany: (count) => `请求过于频繁。请在 ${minutes(count)}后重试。`,
		back: '返回登录'
	},

	reset: {
		title: '设置新密码',
		intro: '账户的所有浏览器和设备都会退出登录。',
		fresh: '新密码',
		again: '再次输入新密码',
		submit: '设置密码',
		mismatch: '两次输入不一致。',
		gone: '此链接已失效。请索取新的链接。',
		invalid: (reason) => `服务器没有接受这个密码。它提示：${reason}`,
		done: '密码已设置。请用新密码登录。',
		cancel: '取消'
	},

	email: {
		title: '电子邮件地址',
		intro: 'rueder 只会就你的账户给你发邮件。',
		current: '你的地址',
		none: '此账户没有地址。',
		pending: (address, hours) =>
			`链接已发送到 ${address}。在 ${n(hours)} 小时内打开后，新地址才生效；在此之前保留原来的地址。`,
		address: '新地址',
		password: '你的密码',
		passwordNote: '以免能使用此浏览器的其他人把你的邮件转到别处。',
		code: '两步验证的验证码',
		submit: '发送链接',
		sent: '链接已发出。请查看收件箱，也看看垃圾邮件文件夹。',
		remove: '删除地址',
		removeIntro: '没有地址就不会收到任何邮件。除此之外，账户保持不变。',
		removed: '地址已删除。',
		wrong: '密码或验证码不正确。',
		codeNeeded: '此账户已开启两步验证。请同时输入其验证码。',
		loginAgain: '你的账户还没有密码，也没有开启两步验证。请重新登录，然后在十分钟内填写地址。',
		invalid: '请输入一个能接收邮件的电子邮件地址。',
		same: '这就是此账户现有的地址。',
		required: '此 rueder 的每个账户都需要一个地址；可以更改，但不能删除。',
		busy: (count) => `尝试或邮件过多。请在 ${minutes(count)}后重试。`,
		unsent: '邮件无法发送。请稍后再试。',
		unreachable: '服务器没有响应。没有做任何更改。',
		askTitle: '添加电子邮件地址',
		askIntro:
			'此 rueder 的每个账户现在都需要电子邮件地址。请填写你的地址，并打开发送到该地址的链接。',
		askWaiting: '打开链接后，此提示会自动消失。没有收到？请重新发送，或发送到其他地址。',
		logOut: '退出登录',
		confirmed: (address) => `地址 ${address} 已确认。`,
		confirmedLogIn: '账户已就绪：请登录。',
		gone: '此链接已无法使用：它已被使用、已发送了更新的链接，或已超过一天。',
		taken: '在此期间，另一个账户已使用了这个地址。',
		confirmUnreachable: '服务器没有响应。请稍后再打开链接。',
		close: '关闭'
	},

	invites: {
		intro: '通过链接，别人可以在此 rueder 上创建账户。链接只在创建时于此处显示一次。',
		left: (count) => (count === 0 ? '你没有可发出的邀请。' : `你还可以邀请 ${n(count)} 个人。`),
		admin: '作为管理员，你随时都可以邀请。',
		uses: '可用于多少个账户',
		make: '创建链接',
		noneLeft: '你剩下的邀请不够这么多。',
		failed: '没有成功。请稍后再试。',
		link: '现在可转交的链接',
		copy: '复制',
		copied: '已复制',
		goodFor: (days) => `链接的有效期为 ${n(days)} 天。`,
		yours: '你的链接',
		none: '你还没有创建链接。',
		used: (uses, of) => `已使用 ${n(uses)}/${n(of)}`,
		until: (when) => `有效期至 ${when}`,
		ran: (when) => `已于 ${when} 过期`,
		usedUp: '已用完',
		revoked: '已撤回',
		revoke: '撤回',
		revokeLink: (made) => `撤回 ${made} 创建的链接`
	},

	home: {
		newIn: '订阅中的新内容',
		allRead: '你已全部读完。新文章到达后会显示在这里。',
		recentlyStarred: '最近加星标',
		noStars: '还没有加星标的内容。在文章上按 s 即可。',
		problems: '有问题的订阅源',
		tips: '提示',
		tipKeys: '按 ? 查看所有键盘快捷键；j 和 k 可在文章间切换。',
		tipDrag: '在侧边栏中拖动订阅源和文件夹即可排序。',
		tipFilters: '设置中的过滤规则可按关键词或作者隐藏文章。',
		showFeed: (title) => `显示“${title}”`
	},

	versions: {
		title: '这篇文章的版本',
		current: '当前版本',
		version: (rev) => `第 ${n(rev)} 版`,
		fetched: (when) => `获取于 ${when}`,
		compareWith: '将当前版本与此版本比较',
		legendAdded: '新增',
		legendRemoved: '删除',
		titleWas: '原标题：',
		same: '此版本的正文与当前版本相同。',
		sameWords: '文字相同；图片或链接有变化。',
		markup: '文字、图片和链接都相同；只有正文的结构变了。',
		pictures: '有变化的图片',
		noPicture: '无',
		address: '图片地址',
		links: '有变化的链接'
	},

	account: {
		title: '账户设置',
		tabs: {
			password: '密码',
			email: '邮件',
			security: '安全',
			apps: '应用',
			invites: '邀请',
			move: '迁移',
			privacy: '隐私',
			remove: '删除账户'
		},
		password: {
			title: '更改密码',
			titleFirst: '设置密码',
			intro: '设置新密码后，所有其他浏览器和设备都会退出登录，所有应用专用密码都会被撤销。',
			introFirst: '有了密码，此账户除了单点登录外，也能用用户名登录。',
			current: '当前密码',
			fresh: '新密码',
			again: '再次输入新密码',
			submit: '更改密码',
			submitFirst: '设置密码',
			done: '密码已更改。所有其他浏览器和设备都已退出登录，应用专用密码已被撤销。',
			doneFirst: '密码已设置。',
			mismatch: '两次输入不一致。',
			code: '两步验证的验证码',
			wrong: '当前密码或验证码不正确。',
			codeNeeded:
				'此账户已开启两步验证。请同时输入身份验证器应用的验证码或一个恢复码，或使用通行密钥确认。',
			loginAgain: '你的账户还没有密码，也没有开启两步验证。请重新登录，然后在十分钟内设置密码。',
			invalid: (characters) => `服务器没有接受这个密码。至少需要 ${n(characters)} 个字符。`,
			busy: (count) => `错误尝试次数过多。请在 ${minutes(count)}后重试。`,
			unreachable: '服务器没有响应。密码未更改。'
		},
		settingsNote: '外观、阅读、过滤规则和键盘位于“设置”中。'
	},

	settings: {
		title: '设置',
		tabs: {
			look: '外观',
			reading: '阅读',
			rules: '过滤规则',
			keys: '键盘'
		},
		accountNote: '密码、两步验证、应用和邀请位于账户菜单中的“账户设置”里。',
		theme: '主题',
		themes: { system: '跟随系统', light: '浅色', dark: '深色' },
		accent: '强调色',
		accents: {
			red: '红色',
			blue: '蓝色',
			green: '绿色',
			orange: '橙色',
			purple: '紫色',
			graphite: '石墨色'
		},
		density: '列表密度',
		densities: { compact: '紧凑', comfortable: '宽松' },
		sidebarSide: '侧边栏',
		sidebarSides: { start: '左侧', end: '右侧' },
		textSize: '文章文字大小（此设备）',
		textSizes: { small: '小', medium: '中', large: '大' },
		readingFont: '文章字体',
		readingFonts: { sans: '无衬线', serif: '衬线' },
		counters: '未读数',
		counterStyles: { parens: '括号内', badge: '徽标' },
		language: '语言',
		languageAuto: '跟随浏览器',
		view: '视图',
		views: { list: '列表', expanded: '展开', split: '三栏', magazine: '杂志' },
		markOnScroll: '在展开视图中滚动经过的文章标记为已读',
		teasers: '在列表中标题后显示正文开头',
		wakeLock: '打开 rueder 时保持屏幕常亮',
		wakeLockMissing: '此浏览器无法保持屏幕常亮。',
		note: '更改立即生效，并与你的账户一起保存，所以在每台设备上都一样。',
		noteDemo: '更改立即生效，并保存在此浏览器中。',
		apps: {
			intro:
				'其他订阅源阅读器应用可通过 Google Reader API 使用你的账户。每个应用都有自己的应用专用密码，你可随时撤销。',
			none: '还没有设置任何应用。',
			made: (when) => `设置于 ${when}`,
			used: (when) => `上次使用 ${when}`,
			unused: '尚未使用',
			revoke: '撤销',
			revokeName: (name) => `撤销应用专用密码“${name}”`,
			add: '设置应用',
			full: (max) => `每个账户最多有 ${n(max)} 个应用专用密码。要设置其他应用，请先撤销一个。`,
			choose: '你想使用哪个应用？',
			website: (app) => `${app} 的网站`,
			other: '其他应用',
			otherHint: '任何支持 Google Reader API 或 FreshRSS 的应用。',
			platforms: {
				apple: 'Mac、iPhone 和 iPad',
				android: 'Android'
			},
			name: '应用专用密码的名称',
			password: '你的密码',
			code: '身份验证器应用的验证码，或一个恢复码',
			create: '创建应用专用密码',
			creating: '请稍候…',
			back: '返回',
			how: {
				netnewswire:
					'在 NetNewsWire 中打开“设置”，再打开“账户”，添加账户（在 Mac 上点 +）并选择 FreshRSS。将下面的用户名填入 Login（在 iPhone 和 iPad 上为 Username or Email），应用专用密码填入 Password，服务器地址填入 API URL。',
				readyou:
					'在 Read You 中打开“设置”、“账户”、“添加账户”，然后选择 Google Reader。将下面的服务器地址填入服务器 URL，用户名填入用户名，应用专用密码填入密码。',
				other:
					'在你的应用中添加“Google Reader API”类型的账户，如果没有，则添加“FreshRSS”。填入下面的服务器地址、用户名和应用专用密码。'
			},
			ownIcons: (app) =>
				`隐私：${app} 会自行从各订阅源的网站加载订阅源图标，这些网站因此会看到你的 IP 地址。文章及其图片只通过 rueder 获取。`,
			shotsTitle: '图文步骤',
			shots: {
				readyou: [
					'点按左上角的齿轮。',
					'打开“账户”。',
					'点按“添加账户”。',
					'选择 Google Reader。',
					'将服务器地址填入服务器 URL，用户名填入用户名，应用专用密码填入密码，然后点按“添加”。'
				],
				netnewswire: [
					'在 Mac 上：打开“设置”（⌘,），再打开“账户”，然后点击左下角的 +。',
					'选择 FreshRSS，然后点击“继续”。',
					'将用户名填入 Login，应用专用密码填入 Password，服务器地址填入 API URL，然后点击“创建”。'
				]
			},
			shotAlt: (app, step) => `${app}，第 ${n(step)} 步`,
			server: '服务器地址',
			user: '用户名',
			secret: '应用专用密码',
			copy: (what) => `复制${what}`,
			copied: '已复制',
			once: '应用专用密码只在此时显示。如果丢失，请撤销并创建一个新的。',
			waiting: '正在等待应用登录…',
			connected: '已连接：应用已登录。',
			done: '完成',
			refusals: {
				wrongPassword: '密码不正确。',
				codeNeeded: '请同时输入身份验证器应用的验证码或一个恢复码，或使用通行密钥确认。',
				wrongCode: '验证码不正确。验证码每 30 秒更换一次。',
				loginAgain:
					'你的账户还没有密码，也没有开启两步验证。请重新登录，然后在十分钟内创建应用专用密码。',
				tooManyApps: '每个账户最多有 20 个应用专用密码。',
				tooMany: '错误尝试次数过多。请过几分钟再试。',
				unreachable: '服务器没有响应。请稍后再试。'
			}
		},
		secondFactor: {
			title: '两步验证',
			intro: '开启两步验证后，登录时还需要身份验证器应用的验证码，或一个通行密钥。',
			app: '身份验证器应用',
			on: '已开启。',
			left: (count) => `还剩 ${n(count)} 个恢复码。`,
			off: '已关闭。',
			byPasskeys: '未设置。你的通行密钥让两步验证保持开启。',
			adminNeeds:
				'此服务器只把开启了两步验证的管理员当作管理员。要查看服务器状态，请设置两步验证，或用其验证码重新登录。',
			password: '你的密码',
			start: '设置',
			scan: '用身份验证器应用扫描此二维码，或输入下面的密钥。',
			qr: '身份验证器应用的二维码',
			larger: '放大二维码',
			smaller: '缩小二维码',
			key: '密钥',
			copyKey: '复制密钥',
			copied: '已复制',
			code: '应用当前显示的验证码',
			codeOrRecovery: '应用的验证码，或一个恢复码',
			confirm: '确认',
			cancel: '取消',
			codesTitle: '你的恢复码',
			codesIntro: '每个恢复码可在没有手机时登录一次。请妥善保管：它们只在此时显示。',
			copyCodes: '复制恢复码',
			codesDone: '我已保存',
			switchOff: '关闭',
			switchOffNote: '关闭后，登录又只需要密码。',
			switchOffNoteKeys: '你的通行密钥让两步验证保持开启。',
			startWithPasskey: '用通行密钥设置',
			switchOffWithPasskey: '用通行密钥关闭',
			refusals: {
				wrongPassword: '密码不正确。',
				codeNeeded: '你的通行密钥让两步验证保持开启。请同时输入一个恢复码。',
				wrongCode: '验证码不正确。验证码每 30 秒更换一次。',
				loginAgain: '你的账户还没有密码，也没有开启两步验证。请重新登录，然后在十分钟内设置应用。',
				tooMany: '错误尝试次数过多。请过几分钟再试。',
				unreachable: '服务器没有响应。请稍后再试。'
			}
		},
		passkeys: {
			title: '通行密钥',
			intro:
				'使用通行密钥，无需用户名和密码，只要设备要求的 PIN 码、指纹或面容即可登录。它也算作两步验证。',
			once: '有了通行密钥，单凭密码将无法再打开你的账户。',
			none: '还没有通行密钥。',
			made: (when) => `添加于 ${when}`,
			used: (when) => `上次使用 ${when}`,
			unused: '尚未使用',
			add: '添加通行密钥',
			name: '名称，例如设备名',
			defaultName: '通行密钥',
			password: '你的密码',
			code: '身份验证器应用的验证码，或一个恢复码',
			create: '创建通行密钥',
			working: '请按浏览器的提示操作…',
			cancel: '取消',
			remove: '删除',
			removeName: (name) => `删除通行密钥“${name}”`,
			confirmRemove: '删除通行密钥',
			full: (max) => `每个账户最多有 ${n(max)} 个通行密钥。`,
			or: '或',
			confirmWith: '用通行密钥确认',
			codesTitle: '你的恢复码',
			codesIntro:
				'你的第一个通行密钥开启了两步验证。如果丢失设备，每个恢复码可登录一次。请妥善保管：它们只在此时显示。',
			copyCodes: '复制恢复码',
			copied: '已复制',
			codesDone: '我已保存',
			refusals: {
				cancelled: '在使用通行密钥之前，浏览器窗口已被关闭。',
				known: '此设备上已有你账户的通行密钥。',
				failed: '此浏览器或设备无法创建或使用通行密钥。',
				wrong: '通行密钥未被接受。请重试。',
				wrongPassword: '密码不正确。',
				codeNeeded:
					'你的账户已开启两步验证。请同时输入应用的验证码或一个恢复码，或使用通行密钥确认。',
				wrongCode: '验证码不正确。验证码每 30 秒更换一次。',
				tooManyPasskeys: '每个账户最多有 10 个通行密钥。',
				loginAgain:
					'你的账户还没有密码，也没有开启两步验证。请重新登录，然后在十分钟内添加你的第一个通行密钥。',
				tooMany: '错误尝试次数过多。请过几分钟再试。',
				unreachable: '服务器没有响应。请稍后再试。'
			}
		},
		reports: {
			send: '把此浏览器中的错误报告发送到此服务器',
			intro: '页面出现问题时，此服务器的管理员会知道。',
			what: '一份报告包含：',
			holds: [
				'错误消息，不含地址，也不含指代某物的数字',
				'错误发生在 rueder 自身文件中的位置',
				'rueder 的版本，以及你的浏览器和它的主版本号'
			],
			never: '绝不包含你读的内容、你的订阅源、标签或规则。报告 30 天后删除。',
			off: '关闭此项会删除你已发送的报告。'
		}
	},

	rules: {
		intro:
			'规则会在文章到达时将其隐藏或标记为已读。被隐藏的文章不计入未读，并保留在“被过滤器隐藏”中。',
		none: '还没有规则。',
		add: '添加规则',
		new: '新规则',
		on: '开启',
		scope: '范围',
		scopeAll: '所有订阅源',
		scopeFolder: (title) => `文件夹：${title}`,
		scopeFeed: (title) => `订阅源：${title}`,
		field: '检查',
		fields: { any: '全部', title: '标题', content: '正文', author: '作者', tag: '标签' },
		match: '比较',
		matches: {
			contains: '包含',
			word: '包含单词',
			starts: '开头是',
			ends: '结尾是',
			regex: '匹配正则表达式',
			all: '每篇文章，无论内容'
		},
		pattern: '要查找的文字',
		action: '然后',
		actions: { hide: '隐藏文章', read: '标记为已读' },
		preview: (count, total) => `目前适用于此处 ${n(total)} 篇文章中的 ${n(count)} 篇。`,
		regexTip: '正则表达式按 Rust 的方式解读，不区分大小写。',
		regexBuilder: '在 regex101 上编写并测试',
		invalid: '这不是有效的正则表达式。',
		empty: '请输入要查找的文字。',
		checking: '正在查看它会匹配哪些文章…',
		hits: (count) => `${n(count)} 个匹配`,
		summary: (field, match, pattern, action) => `${field}${match}“${pattern}”：${action}`,
		summaryAll: (action) => `每篇文章：${action}`
	},

	keys: {
		title: '键盘快捷键',
		note: '你的按键与设置一起保存。Enter、Esc 和 g 保持不变。',
		change: (what) => `更改此操作的按键：${what}`,
		changeShort: '更改',
		press: '请按一个键…',
		taken: (key, other) => `${key} 已是此操作的按键：${other}`,
		fixed: (key) => `${key} 不能分配给快捷键。`,
		reset: '恢复 rueder 的默认按键',
		groups: { articles: '文章', lists: '列表', go: '前往', app: '任何地方' },
		next: '打开下一篇文章',
		previous: '打开上一篇文章',
		nextCursor: '移到下一篇文章，不打开',
		previousCursor: '移到上一篇文章，不打开',
		toggle: '打开或关闭文章',
		star: '加星标或取消星标',
		read: '标记为已读或未读',
		original: '在新标签页中打开原文',
		markAll: '全部标记为已读',
		refresh: '刷新列表',
		unreadOnly: '仅未读，或全部文章',
		listView: '列表视图',
		expandedView: '展开视图',
		splitView: '三栏',
		magazineView: '杂志',
		goHome: '首页',
		goAll: '全部文章',
		goUnread: '未读文章',
		goStarred: '加星标的文章',
		search: '搜索',
		sidebar: '显示或隐藏侧边栏',
		help: '本概览',
		then: '然后'
	},

	labels: {
		title: '标签',
		add: '新建标签',
		name: '标签名称',
		none: '还没有标签。',
		actions: (title) => `标签“${title}”的操作`,
		rename: '重命名',
		color: '颜色',
		colors: ['红色', '橙色', '黄色', '绿色', '青色', '蓝色', '紫色', '灰色'],
		remove: '删除标签',
		show: '显示标签',
		hide: '隐藏标签',
		forArticle: '标签',
		set: (title) => `标签：${title}`
	},

	subscribe: {
		title: '订阅一个订阅源',
		address: '订阅源或网站的地址',
		find: '查找订阅源',
		finding: '正在查找…',
		note: '草稿无法获取任何内容，所以订阅源保持为空。',
		found: (count) => `找到 ${n(count)} 个订阅源：`,
		none: '此地址没有订阅源。',
		movedTo: (url) => `网站把请求转到了 ${url}。`,
		loginWall: '有些网站会把未登录的人引到登录页面。这时它们的订阅源无法读取。',
		subscribed: '已订阅',
		show: '显示',
		untitled: '无名称',
		page: '一个没有订阅源的网站：rueder 会为它生成一个，只有标题和摘要。',
		folder: '放入文件夹',
		topLevel: '不放入文件夹',
		add: '订阅',
		adding: '正在订阅…',
		refused: '没有成功。',
		serverSays: (reason) => `服务器提示：${reason}`,
		busy: (seconds) =>
			seconds ? `同时查找的太多。请在 ${n(seconds)} 秒后重试。` : '同时查找的太多。请稍后再试。',
		noFetching: '此服务器不获取订阅源。',
		tooLarge: (megabytes) => `文件超过 ${n(megabytes)} MB。`,
		unreachable: '服务器没有响应。请稍后再试。'
	},

	opml: {
		title: '导入和导出',
		intro: '以 OPML 文件从其他订阅源阅读器带来你的订阅，或把它们带走。',
		import: '导入 OPML 文件',
		importing: '正在导入…',
		imported: (feeds, folders) => `新订阅源 ${n(feeds)} 个，新文件夹 ${n(folders)} 个。`,
		known: (count) => `有 ${n(count)} 个订阅源已经订阅过。`,
		skipped: (count) => `略过了 ${n(count)} 个无法读取的条目。`,
		later: '它们的文章会在下一轮获取中到达。',
		export: '导出为 OPML'
	},

	removeAccount: {
		title: '删除账户',
		intro: '这会永久删除你在此 rueder 上的账户。无法撤销。',
		goes: '一同删除的有：你在所有设备上的会话、订阅和文件夹、已读记录、星标、标签、过滤规则、设置、错误报告、邀请以及与单点登录的关联。',
		stays: '订阅源会为其他阅读它们的人保留。要先把账户迁到别处，请使用“迁移”。',
		password: '你的密码',
		name: (name) => `请输入你的用户名“${name}”以确认`,
		code: '应用的验证码，或一个恢复码',
		submit: '永久删除我的账户',
		wrong: '不正确。没有删除任何内容。',
		codeNeeded: '你的账户已开启两步验证。请同时输入应用的验证码或一个恢复码，或使用通行密钥确认。',
		busy: (count) => `错误尝试次数过多。请在 ${minutes(count)}后重试。`,
		unreachable: '服务器没有响应。没有删除任何内容。'
	},

	move: {
		intro: '用一个文件把你的账户迁到另一个 rueder，或从那里迁来这里。',
		howItWorks: '导入只会在这里现有内容上增加，不会删除任何东西。你的设置会变成文件中的设置。',
		notInIt: '文件中不包含：你的密码、会话和文章正文。',
		export: '导出账户',
		import: '导入账户文件',
		importing: '正在导入…',
		feeds: (feeds, known) => `新订阅源 ${n(feeds)} 个，已订阅 ${n(known)} 个。`,
		folders: (count) => `新文件夹 ${n(count)} 个。`,
		articles: (found) => `在这里找到 ${articles(found)}。`,
		missing: (count) => `有 ${articles(count)}在这里找不到；它们的已读、星标和标签记录已丢失。`,
		marks: (labels, rules) => `新标签 ${n(labels)} 个，新过滤规则 ${n(rules)} 条。`,
		skipped: (count) => `文件中有 ${n(count)} 个条目无法导入。`,
		settings: '你的设置现在是文件中的设置。',
		later: '新订阅源的文章会在下一轮获取中到达。已读的保持已读。',
		busy: (seconds) =>
			seconds
				? `导入次数过多。请在 ${minutes(Math.ceil(seconds / 60))}后重试。`
				: '导入次数过多。请稍后再试。'
	},

	status: {
		title: '服务器状态',
		open: '服务器状态',
		loading: '正在询问服务器…',
		failed: '服务器没有响应。请稍后再试。',
		refresh: '刷新',
		asOf: (time) => `截至 ${time}。统计的是最近一小时。`,
		demo: '这里没有服务器，所以这些是虚构的。',
		empty: '–',
		on: '开启',
		off: '关闭',
		took: (median, p95) => `中位数 ${median}，95% 在 ${p95} 以内`,
		summary: {
			label: '概览',
			hints: '提示',
			failing: '失败的订阅源',
			errors: '记录的错误',
			reported: '被举报的订阅源'
		},
		hintsTitle: '值得一看',
		allWell: '没有异常。',
		hints: {
			slowRoute: (route, took) => `${route} 较慢：每 20 个请求中就有 1 个耗时超过 ${took}。`,
			serverErrors: (route, count) => `${route} 以服务器错误响应了 ${n(count)} 次。`,
			poolBusy: (size) => `到数据库的 ${n(size)} 个连接全部在用，请求只能等待。`,
			fetchBacklog: (count, late) => `有 ${n(count)} 个订阅源获取延迟，最晚的延迟了 ${late}。`,
			feedsFailing: (failing, subscribed) => `${n(subscribed)} 个订阅源中有 ${n(failing)} 个失败。`,
			notFetching: '服务器没有获取订阅源。FETCH 是否设为 off？',
			fetchPaceFull: (most) =>
				`每轮都尽可能多地获取，但仍有订阅源在等待：当前节奏每小时最多获取 ${n(most)} 次。由 FETCH_ROUND、FETCH_AT_ONCE 和 FETCH_EVERY 设置。`,
			cacheMisses: (share) =>
				`数据库读取的内容中只有 ${share} 在内存中；增大 shared_buffers 会有帮助。`,
			databaseLarge: (size, warn) =>
				`数据库已增长到 ${size}；DATABASE_WARN_GB 从 ${warn} 起发出警告。“rueder retention set”可删除旧文章或其正文。`,
			longQuery: (took) => `有一个查询已运行 ${took}。`,
			imageCacheFull: (share) => `图片缓存已用 ${share}。`,
			errors: (count) => `服务器在最近一小时记录了 ${n(count)} 个错误。`,
			limited: (count) => `有 ${n(count)} 个请求因过多而被拒绝。`,
			browserErrors: (count) => `浏览器在最近一天报告了 ${n(count)} 种错误。`,
			reportedFeeds: (count) => `读者举报了 ${n(count)} 个尚未屏蔽的订阅源。`
		},
		build: {
			title: '构建',
			commit: '提交',
			dirty: (commit) => `${commit}，带有本地更改`,
			branch: '分支',
			tag: '标签',
			committed: '提交时间',
			where: '运行于',
			container: '容器中',
			host: '主机上',
			target: '目标',
			profile: '编译',
			profileValue: (profile, level) => `${profile}，优化级别 ${level}`,
			ci: '构建者',
			ciValue: '一次 CI 运行',
			byHand: '手动'
		},
		server: {
			title: '服务器',
			version: '版本',
			running: '运行开始于',
			memory: '内存',
			accounts: '账户',
			feeds: '订阅源',
			feedsValue: (fetched, all) => `获取中 ${n(fetched)} 个，共 ${n(all)} 个`,
			articles: '文章',
			articlesValue: (count) => `约 ${n(count)} 篇`,
			live: '打开的标签页'
		},
		database: {
			title: '数据库',
			pool: 'rueder 的连接',
			poolValue: (size, idle, max) => `打开 ${n(size)} 个，空闲 ${n(idle)} 个，最多 ${n(max)} 个`,
			connections: '全部连接',
			size: '大小',
			sizeWarn: (size, warn) => `${size}，从 ${warn} 起警告`,
			hitRatio: '从内存读取',
			deadlocks: '死锁',
			longest: '运行最久的查询',
			tables: '最大的表',
			table: '表',
			bytes: '大小',
			rows: '行数（约）'
		},
		fetcher: {
			title: '获取订阅源',
			state: '获取',
			lastRound: '上一轮',
			lastRoundValue: (ago, feeds, took) => `${ago}：${n(feeds)} 个订阅源，用时 ${took}`,
			fetches: '获取次数',
			fetchesValue: (fetched, unchanged, failed) =>
				`带来内容 ${n(fetched)} 次，无变化 ${n(unchanged)} 次，失败 ${n(failed)} 次`,
			broken: '在数据库处失败',
			newArticles: '新文章',
			took: '每次获取的时间',
			overdue: '延迟获取',
			overdueValue: (count, late) => `${n(count)} 个，最晚的延迟了 ${late}`,
			waiting: '尚未获取',
			pace: '节奏',
			paceValue: (round, every, atOnce, most) =>
				`每 ${every} ${n(round)} 个订阅源，同时 ${n(atOnce)} 个：每小时最多 ${n(most)} 次`,
			perHour: '最近一小时的获取次数',
			behind: '延迟',
			behindValue: (late) => `等待最久的订阅源已超时 ${late}`,
			notBehind: '没有在等待的',
			failing: '失败的订阅源（按原因）',
			noneFailing: '没有失败的订阅源。',
			feeds: (count) => `${n(count)} 个订阅源`,
			failures: {
				status: (code) => (code ? `网站返回 ${code}` : '网站返回错误'),
				timeout: '网站响应太慢',
				blocked: '地址位于 rueder 不从中获取内容的网络',
				dns: '该名称没有对应的地址',
				network: '无法连接，或连接中断',
				too_large: '太大',
				redirects: '重定向过多，或重定向有误',
				unreadable: '不是 rueder 能读取的订阅源',
				other: '其他'
			}
		},
		finder: {
			title: '查找订阅源',
			state: '检查地址',
			lookups: '查找次数',
			lookupsValue: (found, none, failed) =>
				`找到订阅源 ${n(found)} 次，未找到 ${n(none)} 次，失败 ${n(failed)} 次`,
			limited: '超过账户限额',
			took: '每次查找的时间'
		},
		rssifier: {
			title: '没有订阅源的网站',
			state: 'RSSifier',
			pages: '阅读中的网站',
			pagesValue: (pages, failing) => `${n(pages)} 个，其中 ${n(failing)} 个失败`,
			used: '令牌用量',
			usedValue: (requests, fetches, underWay) =>
				`最近一分钟 ${n(requests)} 次请求，最近一小时抓取 ${n(fetches)} 个页面，进行中 ${n(underWay)} 个`,
			unanswered: (why) => `RSSifier 没有回应：${why}`,
			limits: '令牌限额',
			limitsValue: (perMinute, perHour, atOnce) =>
				`每分钟 ${perMinute}，每小时 ${perHour} 个页面，同时 ${atOnce} 个`,
			noLimit: '无限制'
		},
		images: {
			title: '图片代理',
			state: '代理',
			cache: '缓存',
			cacheValue: (used, max) => `${used} / ${max}`,
			served: '图片',
			servedValue: (cached, fetched) => `来自缓存 ${n(cached)} 张，获取 ${n(fetched)} 张`,
			refused: '已拒绝',
			refusedValue: (source, notImage, tooLarge, notFound) =>
				`来源失败 ${n(source)} 张，不是图片 ${n(notImage)} 张，太大 ${n(tooLarge)} 张，地址错误 ${n(notFound)} 张`,
			took: '每张获取图片的时间',
			failingSources: '暂时不再请求的来源'
		},
		requests: {
			title: '请求',
			all: '全部请求',
			allValue: (count, took) => `${n(count)} 个，${took}`,
			answers: '响应',
			answersValue: (fine, refused, errors) =>
				`正常 ${n(fine)} 个，拒绝 ${n(refused)} 个，服务器错误 ${n(errors)} 个`,
			route: '路由',
			count: '次数',
			median: '中位数',
			p95: '95%',
			max: '最长',
			errors: '错误',
			none: '最近一小时没有请求。'
		},
		problems: {
			title: '警告和错误',
			counts: (errors, warnings) => `最近一小时有 ${n(errors)} 个错误和 ${n(warnings)} 个警告。`,
			none: '自启动以来没有记录任何内容。',
			error: '错误',
			warning: '警告',
			times: (count, last) => (count > 1 ? `${n(count)} 次，最近一次 ${last}` : last)
		},
		browsers: {
			title: '浏览器中的错误',
			off: '此服务器不接收错误报告（CLIENT_REPORTS=off）。',
			consenting: (count) => `有 ${n(count)} 个账户发送报告。错误在最后一次出现后保留 30 天。`,
			none: '没有浏览器报告错误。',
			kinds: {
				error: '错误',
				rejection: '失败的 Promise',
				component: '页面的一部分',
				request: '请求'
			},
			hits: (count, accounts, last) =>
				`${n(accounts)} 个账户中出现 ${n(count)} 次，最近一次 ${last}`,
			where: (versions, browsers) => `版本 ${versions} · ${browsers}`
		},
		reported: {
			title: '被举报的订阅源',
			text: '这里不显示举报人。“rueder feed block <地址>”为所有人屏蔽一个订阅源，“rueder feed unblock <地址>”解除屏蔽。',
			none: '没有人举报订阅源。',
			reports: (count, last) => `${n(count)} 次举报，最近一次 ${last}`,
			blocked: '已屏蔽'
		}
	},

	update: {
		ready: 'rueder 的新版本已就绪。',
		reload: '重新加载',
		later: '稍后'
	},

	crash: {
		title: '此页面出了问题。',
		text: '请重新加载页面以继续。你读过和更改的内容都已保存在服务器上。',
		reload: '重新加载'
	},

	demo: {
		badge: '草稿',
		note: '使用虚构订阅源的草稿。不会在服务器上保存任何内容。',
		arrive: '让一篇文章到达',
		revise: '更改打开的文章',
		revised: '更新：为了试用版本功能，添加了这一段。',
		reviseHint: '请先打开一篇文章。'
	}
};

export default zhHans;
