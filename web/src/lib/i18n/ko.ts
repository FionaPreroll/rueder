// The Korean texts, in the shape of the English catalog (en.ts), in the polite
// forms (합니다, 하세요). Korean has no plural, so counts take a counter. No
// particle follows a value directly, as its form depends on the value's last
// sound; values stand in quotes or before a noun instead. An item is a “글”.

import type { Messages } from './en.ts';
import { formatNumber as n } from './intl.ts';

const articles = (count: number) => `글 ${n(count)}개`;

const minutes = (count: number) => `${n(count)}분`;

const ko: Messages = {
	languageName: '한국어',

	articles,

	common: {
		close: '닫기',
		cancel: '취소',
		save: '저장',
		delete: '삭제',
		edit: '편집'
	},

	nav: {
		subscribe: '구독',
		home: '홈',
		all: '모든 글',
		unreadItems: '읽지 않은 글',
		starred: '별표한 글',
		hidden: '필터로 숨긴 글',
		subscriptions: '구독',
		newFolder: '새 폴더',
		groupedFolder: '새 폴더',
		folderName: '폴더 이름',
		renameFolder: '이름 바꾸기',
		removeFolder: '폴더를 지우고 안의 항목은 남기기',
		folderActions: (title) => `‘${title}’ 폴더 작업`,
		feedName: '피드 이름',
		feedNameHint: '비워 두면 피드 자체의 이름을 씁니다',
		renameFeed: '이름 바꾸기',
		unsubscribe: '구독 취소',
		reportFeed: '피드 신고 …',
		feedInfo: '피드 정보 …',
		feedActions: (title) => `‘${title}’ 피드 작업`,
		unsubscribeTitle: '구독을 취소할까요?',
		unsubscribeText: (title) =>
			`‘${title}’ 구독을 취소할까요? 이 피드에서 읽은 기록과 별표가 사라집니다.`,
		openFolder: '폴더 열기',
		closeFolder: '폴더 닫기',
		openSidebar: '사이드바 보이기',
		closeSidebar: '사이드바 숨기기',
		search: '검색',
		searchPlaceholder: '이 글들에서 검색',
		clearSearch: '검색어 지우기',
		settings: '설정',
		accountSettings: '계정 설정',
		shortcuts: '키보드 단축키',
		live: '실시간',
		liveOn: '새 글이 저절로 들어옵니다. 클릭하면 일시 정지합니다.',
		liveOff: '일시 정지했습니다. 클릭하면 새 글이 다시 들어옵니다.',
		paused: '일시 정지',
		account: (name) => `${name} 계정으로 로그인함`,
		logout: '로그아웃',
		appSettings: '서버와 앱 업데이트',
		install: '앱으로 설치',
		installSafari:
			'rueder를 앱으로 설치하려면 Mac에서는 파일 › Dock에 추가, iPhone이나 iPad에서는 공유 › 홈 화면에 추가를 선택하세요.',
		unread: (count) => `읽지 않음 ${n(count)}개`,
		hiddenCount: (count) => `필터로 숨김 ${n(count)}개`,
		fetchFailed: (reason) => `마지막 가져오기에 실패했습니다: ${reason}`,
		waiting: '아직 가져오지 않음',
		blocked: '이 rueder의 운영자가 차단함',
		moveHint: '끌어서 정렬하거나 Alt와 화살표 키를 누르세요',
		resizeSidebar: '사이드바 너비: 끌거나 화살표 키를 쓰세요',
		tooDeep: (levels) => `폴더는 최대 ${n(levels)}단계까지 넣을 수 있습니다.`
	},

	toolbar: {
		refresh: '새로 고침',
		showAll: '모든 글',
		showUnread: '읽지 않은 글만',
		show: '보기',
		newestFirst: '최신순',
		oldestFirst: '오래된순',
		markAllRead: '모두 읽음으로 표시',
		markOptions: '읽음으로 표시하는 다른 방법',
		phoneMenu: (shown) => `보기: ${shown}`,
		markMenu: '읽음으로 표시 …',
		markEverything: '모든 글',
		markOlderDay: '하루보다 오래된 글',
		markOlderWeek: '일주일보다 오래된 글',
		listView: '목록 보기',
		expandedView: '펼쳐 보기',
		splitView: '3단 보기: 트리, 목록, 글',
		magazineView: '매거진',
		previous: '이전 글',
		next: '다음 글',
		newArticles: (count) => `새 글 ${n(count)}개`
	},

	list: {
		empty: '여기에는 글이 없습니다.',
		emptyUnread: '여기에는 읽지 않은 글이 없습니다.',
		emptySearch: '검색과 일치하는 글이 없습니다.',
		found: (count, more, words) => `“${words}” 포함 글 ${n(count)}${more ? '+' : ''}개`,
		searchAll: '모든 피드에서 검색',
		feedFailing:
			'rueder는 나중에 다시 시도하며, 실패할 때마다 또는 사이트가 요청하는 만큼 더 오래 기다립니다. 계속 실패한다면 피드 주소가 바뀌었을 수 있습니다.',
		hiddenNote: '필터 규칙이 이 글들을 숨겼습니다. 글을 열면 어떤 규칙인지 볼 수 있습니다.',
		toRules: '필터 규칙',
		emptyFeed: '이 피드에는 아직 글이 없습니다.',
		waitingFeed: '이 피드는 아직 가져오지 않았습니다. 글은 다음 가져오기 때 몇 분 안에 들어옵니다.',
		showAll: '모든 글 보기',
		more: '더 보기',
		star: '별표하기',
		unstar: '별표 해제',
		keepUnread: '읽지 않음으로 두기',
		openOriginal: '원문 열기',
		showFeed: '이 피드의 모든 글을 rueder에서 보기',
		from: '출처',
		by: (author) => `글쓴이: ${author}`,
		updated: '업데이트됨',
		updatedHeadline: '제목 바뀜',
		updatedTitle: (count) => `처음 가져온 뒤 바뀌었습니다. 이전 버전 ${n(count)}개`,
		updatedHeadlineTitle: (count) =>
			`제목이 다시 쓰였고 본문은 그대로입니다. 이전 버전 ${n(count)}개`,
		compare: '버전 비교',
		hiddenBy: (rule) => `다음 규칙으로 숨김: ${rule}`,
		textDropped: '이 글은 오래되어 이 서버에 본문이 더 이상 남아 있지 않습니다.',
		textDroppedOriginal: '원래 사이트에서 읽기',
		loading: '본문을 불러오는 중 …',
		loadingList: '불러오는 중 …',
		toTop: '맨 위로',
		pick: '글을 고르면 여기에서 읽을 수 있습니다.',
		resizeList: '목록 너비: 끌거나 화살표 키를 쓰세요',
		closeArticle: '글 닫기',
		blockedFeed:
			'이 rueder의 운영자가 이 피드를 차단했습니다. 가져오지 않으며 글도 보여 주지 않습니다.',
		articleActions: '글에 대한 다른 작업',
		articleInfo: '글 정보 …',
		articleSource: '소스 …',
		reportArticle: '글 신고 …'
	},

	report: {
		feedTitle: '피드 신고',
		articleTitle: '글 신고',
		text: (title) =>
			`이 rueder의 운영자는 ‘${title}’ 신고가 있었다는 것과 그 이유, 입력한 내용을 볼 수 있지만 누가 신고했는지는 볼 수 없습니다. 모두에게 차단할지는 운영자가 정합니다.`,
		reason: '이유',
		reasons: {
			illegal: '법에 어긋남',
			copyright: '다른 사람의 저작물',
			malicious: '악성 코드, 피싱 또는 사기',
			spam: '스팸',
			other: '기타'
		},
		note: '덧붙일 내용 (선택)',
		left: (count) => `${n(count)}자 남음`,
		send: '신고 보내기',
		sending: '보내는 중 …',
		sent: '감사합니다. 신고가 이 rueder의 운영자에게 전달되었습니다.',
		tooMany: '최근에 신고를 많이 보냈습니다. 나중에 다시 시도하세요.',
		unreachable: '신고가 서버에 닿지 않았습니다. 나중에 다시 시도하세요.'
	},

	feedInfo: {
		title: '피드 정보',
		text: (title) =>
			`이 rueder가 ‘${title}’ 피드에 대해 아는 내용과 가져오는 방식입니다. 피드가 동작하지 않으면 운영자에게 보낼 메시지에 이 내용을 복사하세요.`,
		number: '번호',
		name: '이름',
		address: '피드 주소',
		site: '웹사이트',
		noSite: '없음',
		agent: '가져올 때 이름',
		agents: { rueder: 'rueder', browser: '브라우저' },
		every: '가져오는 간격',
		fetched: '마지막 가져오기',
		succeeded: '마지막으로 성공한 가져오기',
		failed: '마지막 오류',
		next: '다음 가져오기',
		blocked: '없음, 운영자가 피드를 차단함',
		articles: '지금까지의 글',
		never: '없음',
		fetchAs: '가져올 때 이름',
		fetchAsHint:
			'이 피드를 읽는 모두에게 적용됩니다. rueder를 거부하는 사이트에는 브라우저를 고르세요. 바로 다시 가져옵니다.',
		notSwitched: '실패했습니다. 나중에 다시 시도하세요.',
		copy: '복사',
		copied: '복사했습니다',
		loading: '불러오는 중 …',
		unreachable: '서버가 응답하지 않았습니다. 나중에 다시 시도하세요.'
	},
	articleInfo: {
		title: '글 정보',
		text: (title) =>
			`이 rueder가 ‘${title}’ 글과 그 버전에 대해 아는 내용입니다. 잘못된 점이 보이면 운영자에게 보낼 메시지에 이 내용을 복사하세요.`,
		feed: '피드',
		feedNumber: '피드 번호',
		number: '피드 안의 번호',
		name: '제목',
		id: '피드 안의 ID',
		link: '링크',
		noLink: '없음',
		published: '게시',
		fetched: '처음 가져온 때',
		changed: '마지막 변경',
		never: '없음',
		changedAt: (when, what) => `${when} (${what})`,
		what: { title: '제목', text: '본문', both: '제목과 본문' },
		version: '버전',
		kept: '보관 중인 버전',
		textLabel: '본문',
		textDropped: '오래되어 더 이상 보관하지 않습니다',
		copy: '복사',
		copied: '복사했습니다',
		loading: '불러오는 중 …',
		unreachable: '서버가 응답하지 않았습니다. 나중에 다시 시도하세요.'
	},
	articleSource: {
		title: '글의 소스',
		text: '이 글의 HTML입니다. 이 rueder가 정리한 뒤 보관하는 형태나 피드가 보낸 그대로를 보여 줍니다. 여기서는 링크와 이미지가 원래 주소를 그대로 가지므로 남아 있는 추적 매개변수를 볼 수 있습니다.',
		show: '보기',
		kinds: { cleaned: '정리된 형태', original: '피드가 보낸 그대로' },
		noText: '이 서버는 이 글의 본문을 더 이상 보관하지 않습니다.',
		noOriginal: '이 서버는 피드가 보낸 내용을 더 이상 보관하지 않습니다.',
		copy: '복사',
		copied: '복사했습니다',
		loading: '불러오는 중 …',
		unreachable: '서버가 응답하지 않았습니다. 나중에 다시 시도하세요.'
	},

	media: {
		title: '첨부 파일',
		play: '재생',
		image: '이미지',
		zoomIn: '확대',
		zoomOut: '축소'
	},

	connection: {
		reconnecting: '다시 연결하는 중 …',
		unreachable: (since) => `${since}부터 서버가 응답하지 않습니다.`,
		offline: (since) => `${since}부터 네트워크에 연결되어 있지 않습니다.`,
		retry: '지금 다시 시도',
		unsent: (count) => `변경 사항 ${n(count)}개가 기다리고 있습니다.`
	},

	notice: {
		unreachable: '서버가 응답하지 않습니다. 보이는 내용이 최신이 아닐 수 있습니다.',
		notSaved: '서버가 마지막 변경을 받지 않았습니다. 여기에서는 되돌렸습니다.',
		tooManyUnsent: '서버를 기다리는 변경이 너무 많아 이 변경은 취소되었습니다.'
	},

	login: {
		title: 'rueder 로그인',
		about: '피드 리더입니다. 팔로우하는 사이트가 올린 글을 가져와 아직 읽지 않은 글을 보여 줍니다.',
		name: '이름',
		password: '비밀번호',
		submit: '로그인',
		checking: '잠시만 기다리세요 …',
		wrong: '이름이나 비밀번호가 틀렸습니다.',
		codeNeeded:
			'이 계정에는 2단계 인증이 있습니다. 앱에 표시된 코드나 복구 코드를 입력하거나 패스키로 로그인하세요.',
		wrongCode: '코드가 맞지 않습니다. 코드는 30초마다 바뀝니다.',
		code: '코드',
		codeNote: '인증 앱에 표시된 6자리 숫자 또는 복구 코드 중 하나입니다.',
		tooMany: (count) => `잘못된 시도가 너무 많습니다. ${minutes(count)} 뒤에 다시 시도하세요.`,
		unreachable: '서버가 응답하지 않습니다. 잠시 뒤에 다시 시도하세요.',
		retry: '다시 시도',
		or: '또는',
		oidc: '통합 로그인으로 로그인',
		passkey: '패스키로 로그인',
		passkeyCancelled: '패스키를 쓰지 않았습니다. 다시 시도하거나 이름과 비밀번호로 로그인하세요.',
		passkeyFailed: '이 브라우저나 기기에서는 여기서 패스키를 쓸 수 없습니다.',
		passkeyWrong: '이 패스키는 여기의 어떤 계정에도 속하지 않습니다.',
		oidcFailed: '통합 로그인에 실패했습니다. 다시 시도하세요.',
		oidcDenied: '통합 로그인을 취소했습니다.',
		oidcUnknown:
			'이 로그인과 연결된 계정이 여기에 없습니다. 이 rueder의 운영자에게 연결을 요청하세요.',
		disabled: '이 계정은 사용 중지되었습니다. 이유는 이 rueder의 운영자에게 문의하세요.',
		unconfirmed:
			'이 계정은 이메일 주소 확인을 기다리고 있습니다. 메일의 링크를 열거나 다시 보내도록 하세요.',
		resend: '링크 다시 보내기',
		resent: '링크를 다시 보냈습니다. 받은편지함과 스팸함을 확인하세요.',
		resendWrong: '링크를 다시 받으려면 계정의 이름과 비밀번호를 입력하세요.',
		forgot: '비밀번호를 잊으셨나요?',
		removed: '계정과 계정에 속한 모든 것을 삭제했습니다.',
		note: '직접 가입할 수는 없습니다. 계정은 이 rueder의 운영자가 만듭니다.'
	},

	legal: {
		imprint: '법적 고지',
		privacy: '개인정보처리방침',
		terms: '이용약관',
		agree: { before: '내용을 확인했으며 ', link: '이용약관', after: '에 동의합니다.' },
		changed: '이용약관이 바뀌었습니다',
		changedIntro: '새 이용약관을 읽어 보세요. rueder를 계속 쓰려면 동의해야 합니다.',
		read: '이용약관 읽기',
		accept: '동의합니다',
		decline: '로그아웃',
		notSaved: '실패했습니다. 잠시 뒤에 다시 시도하세요.'
	},

	register: {
		title: '계정 만들기',
		invited: '이 rueder에 초대받았습니다. 이름과 비밀번호를 정하세요.',
		again: '비밀번호 확인',
		passwordRule: (characters) => `${n(characters)}자 이상.`,
		mismatch: '두 비밀번호가 다릅니다.',
		submit: '계정 만들기',
		solving: '이 브라우저를 확인하는 중 …',
		toRegister: '계정 만들기',
		toLogin: '기존 계정으로 로그인',
		closed: '여기서는 초대 없이 가입할 수 없습니다.',
		inviteGone: '이 초대는 더 이상 쓸 수 없습니다. 새 초대를 요청하세요.',
		full: '여기에는 계정을 더 만들 자리가 없습니다.',
		today: '오늘 만들 수 있는 계정 수를 다 채웠습니다. 내일 다시 시도하세요.',
		captcha: '이 브라우저 확인을 마치지 못했습니다. 다시 시도하세요.',
		taken: '이 이름의 계정이 이미 있습니다.',
		terms: '계정을 만들려면 이용약관에 동의하세요.',
		invalid: (reason) => `계정으로 쓸 수 없습니다. 서버의 응답: ${reason}`,
		email: '이메일 주소',
		emailOptional: '이메일 주소 (선택)',
		emailNote: '확인 링크를 받게 됩니다.',
		emailRequiredNote: '확인 링크를 받게 됩니다. 링크를 열면 계정으로 로그인할 수 있습니다.',
		emailInvalid: '메일을 받을 수 있는 이메일 주소를 입력하세요.',
		unsent: '링크를 메일로 보내지 못해 계정을 만들지 않았습니다. 나중에 다시 시도하세요.',
		tooMany: (count) => `시도가 너무 많습니다. ${minutes(count)} 뒤에 다시 시도하세요.`,
		awaitingTitle: '메일을 확인하세요',
		awaiting: (address, hours) =>
			`계정을 만들었습니다. ${address} 주소로 링크를 보냈습니다. ${n(hours)}시간 안에 연 다음 로그인하세요.`
	},

	forgot: {
		title: '비밀번호를 잊으셨나요?',
		intro: '이름이나 이메일 주소를 입력하세요. 새 비밀번호를 정하는 링크가 확인된 주소로 갑니다.',
		field: '이름 또는 이메일 주소',
		submit: '링크 보내기',
		asked: (count) =>
			`확인된 주소가 있는 계정이라면 그 주소로 링크를 보냈습니다. 링크는 ${minutes(count)} 동안 유효합니다. 스팸함도 확인하세요.`,
		tooMany: (count) => `요청이 너무 많습니다. ${minutes(count)} 뒤에 다시 시도하세요.`,
		back: '로그인으로 돌아가기'
	},

	reset: {
		title: '새 비밀번호 정하기',
		intro: '계정의 모든 브라우저와 기기에서 로그아웃됩니다.',
		fresh: '새 비밀번호',
		again: '새 비밀번호 확인',
		submit: '비밀번호 정하기',
		mismatch: '두 비밀번호가 다릅니다.',
		gone: '이 링크는 더 이상 동작하지 않습니다. 새 링크를 요청하세요.',
		invalid: (reason) => `서버가 그 비밀번호를 받지 않았습니다. 서버의 응답: ${reason}`,
		done: '비밀번호를 정했습니다. 새 비밀번호로 로그인하세요.',
		cancel: '취소'
	},

	email: {
		title: '이메일 주소',
		intro: 'rueder는 계정에 관한 메일만 보냅니다.',
		current: '내 주소',
		none: '이 계정에는 주소가 없습니다.',
		pending: (address, hours) =>
			`${address} 주소로 링크를 보냈습니다. ${n(hours)}시간 안에 링크를 열면 새 주소가 적용되며, 그때까지는 이전 주소가 유지됩니다.`,
		address: '새 주소',
		password: '비밀번호',
		passwordNote: '이 브라우저를 쓰는 다른 사람이 메일 주소를 바꾸지 못하게 하기 위해서입니다.',
		code: '2단계 인증 코드',
		submit: '링크 보내기',
		sent: '링크를 보냈습니다. 받은편지함과 스팸함을 확인하세요.',
		remove: '주소 삭제',
		removeIntro: '주소가 없으면 메일을 받지 않습니다. 그 밖에는 계정이 그대로입니다.',
		removed: '주소를 삭제했습니다.',
		wrong: '비밀번호나 코드가 맞지 않습니다.',
		codeNeeded: '이 계정에는 2단계 인증이 있습니다. 그 코드도 입력하세요.',
		loginAgain:
			'계정에 아직 비밀번호도 2단계 인증도 없습니다. 다시 로그인한 뒤 10분 안에 주소를 입력하세요.',
		invalid: '메일을 받을 수 있는 이메일 주소를 입력하세요.',
		same: '이 계정의 현재 주소입니다.',
		required: '이 rueder의 모든 계정에는 주소가 필요합니다. 바꿀 수는 있지만 지울 수는 없습니다.',
		busy: (count) => `시도나 메일이 너무 많습니다. ${minutes(count)} 뒤에 다시 시도하세요.`,
		unsent: '메일을 보내지 못했습니다. 나중에 다시 시도하세요.',
		unreachable: '서버가 응답하지 않습니다. 아무것도 바뀌지 않았습니다.',
		askTitle: '이메일 주소 추가',
		askIntro:
			'이제 이 rueder의 모든 계정에는 이메일 주소가 필요합니다. 주소를 입력하고 그리로 온 링크를 여세요.',
		askWaiting:
			'링크를 열면 이 안내는 저절로 사라집니다. 아무것도 오지 않았나요? 다시 보내거나 다른 주소로 보내세요.',
		logOut: '로그아웃',
		confirmed: (address) => `${address} 주소를 확인했습니다.`,
		confirmedLogIn: '계정이 준비되었습니다. 로그인하세요.',
		gone: '이 링크는 더 이상 쓸 수 없습니다. 이미 썼거나, 새 링크를 보냈거나, 하루가 지났습니다.',
		taken: '그사이 다른 계정이 이 주소를 쓰게 되었습니다.',
		confirmUnreachable: '서버가 응답하지 않습니다. 잠시 뒤에 링크를 다시 여세요.',
		close: '닫기'
	},

	invites: {
		intro:
			'링크로 누군가 이 rueder에 계정을 만들 수 있습니다. 링크는 만들 때 여기에서 한 번만 보여 줍니다.',
		left: (count) =>
			count === 0 ? '줄 수 있는 초대가 없습니다.' : `${n(count)}명을 더 초대할 수 있습니다.`,
		admin: '관리자는 언제든 초대할 수 있습니다.',
		uses: '몇 개의 계정용',
		make: '링크 만들기',
		noneLeft: '그만큼 줄 초대가 남아 있지 않습니다.',
		failed: '실패했습니다. 잠시 뒤에 다시 시도하세요.',
		link: '지금 전달할 링크',
		copy: '복사',
		copied: '복사했습니다',
		goodFor: (days) => `링크는 ${n(days)}일 동안 유효합니다.`,
		yours: '내 링크',
		none: '아직 만든 링크가 없습니다.',
		used: (uses, of) => `${n(of)}개 중 ${n(uses)}개 사용`,
		until: (when) => `${when}까지 유효`,
		ran: (when) => `${when} 만료`,
		usedUp: '모두 사용함',
		revoked: '취소함',
		revoke: '취소',
		revokeLink: (made) => `${made}에 만든 링크 취소`
	},

	home: {
		newIn: '구독 중인 피드의 새 글',
		allRead: '모두 읽었습니다. 새 글은 들어오는 대로 여기에 나타납니다.',
		recentlyStarred: '최근에 별표한 글',
		noStars: '아직 별표한 글이 없습니다. 글에서 s를 누르세요.',
		problems: '문제가 있는 피드',
		tips: '팁',
		tipKeys: '?를 누르면 모든 키보드 단축키가 보입니다. j와 k로 글을 넘깁니다.',
		tipDrag: '사이드바의 피드와 폴더는 끌어서 정렬할 수 있습니다.',
		tipFilters: '설정의 필터 규칙으로 키워드나 글쓴이에 따라 글을 숨길 수 있습니다.',
		showFeed: (title) => `‘${title}’ 보기`
	},

	versions: {
		title: '이 글의 버전',
		current: '현재 버전',
		version: (rev) => `버전 ${n(rev)}`,
		fetched: (when) => `${when}에 가져옴`,
		compareWith: '현재 버전과 비교할 버전',
		legendAdded: '추가됨',
		legendRemoved: '삭제됨',
		titleWas: '이전 제목:',
		same: '이 버전의 본문은 현재 버전과 같습니다.',
		sameWords: '단어는 같습니다. 이미지나 링크가 바뀌었습니다.',
		markup: '단어, 이미지, 링크는 같습니다. 본문의 구성만 바뀌었습니다.',
		pictures: '바뀐 이미지',
		noPicture: '없음',
		address: '이미지 주소',
		links: '바뀐 링크'
	},

	account: {
		title: '계정 설정',
		tabs: {
			password: '비밀번호',
			email: '이메일',
			security: '보안',
			apps: '앱',
			invites: '초대',
			move: '이사',
			privacy: '개인정보',
			remove: '계정 삭제'
		},
		password: {
			title: '비밀번호 바꾸기',
			titleFirst: '비밀번호 정하기',
			intro:
				'새 비밀번호를 정하면 다른 모든 브라우저와 기기에서 로그아웃되고 모든 앱 비밀번호가 취소됩니다.',
			introFirst:
				'비밀번호가 있으면 이 계정은 통합 로그인뿐 아니라 이름으로도 로그인할 수 있습니다.',
			current: '현재 비밀번호',
			fresh: '새 비밀번호',
			again: '새 비밀번호 확인',
			submit: '비밀번호 바꾸기',
			submitFirst: '비밀번호 정하기',
			done: '비밀번호를 바꿨습니다. 다른 모든 브라우저와 기기에서 로그아웃되었고 앱 비밀번호는 취소되었습니다.',
			doneFirst: '비밀번호를 정했습니다.',
			mismatch: '두 비밀번호가 다릅니다.',
			code: '2단계 인증 코드',
			wrong: '현재 비밀번호나 코드가 맞지 않습니다.',
			codeNeeded:
				'이 계정에는 2단계 인증이 있습니다. 인증 앱의 코드나 복구 코드도 입력하거나 패스키로 확인하세요.',
			loginAgain:
				'계정에 아직 비밀번호도 2단계 인증도 없습니다. 다시 로그인한 뒤 10분 안에 비밀번호를 정하세요.',
			invalid: (characters) =>
				`서버가 그 비밀번호를 받지 않았습니다. ${n(characters)}자 이상이어야 합니다.`,
			busy: (count) => `잘못된 시도가 너무 많습니다. ${minutes(count)} 뒤에 다시 시도하세요.`,
			unreachable: '서버가 응답하지 않습니다. 비밀번호는 바뀌지 않았습니다.'
		},
		settingsNote: '모양, 읽기, 필터 규칙, 키보드는 설정에 있습니다.'
	},

	settings: {
		title: '설정',
		tabs: {
			look: '모양',
			reading: '읽기',
			rules: '필터 규칙',
			keys: '키보드'
		},
		accountNote: '비밀번호, 2단계 인증, 앱, 초대는 계정 메뉴의 계정 설정에 있습니다.',
		theme: '테마',
		themes: { system: '시스템 설정 따르기', light: '밝게', dark: '어둡게' },
		accent: '강조 색',
		accents: {
			red: '빨강',
			blue: '파랑',
			green: '초록',
			orange: '주황',
			purple: '보라',
			graphite: '흑연'
		},
		density: '목록 간격',
		densities: { compact: '촘촘하게', comfortable: '넉넉하게' },
		sidebarSide: '사이드바',
		sidebarSides: { start: '왼쪽', end: '오른쪽' },
		textSize: '글자 크기 (이 기기)',
		textSizes: { small: '작게', medium: '보통', large: '크게' },
		readingFont: '글 서체',
		readingFonts: { sans: '고딕', serif: '명조' },
		counters: '읽지 않은 글 수',
		counterStyles: { parens: '괄호 안에', badge: '배지로' },
		language: '언어',
		languageAuto: '브라우저 설정 따르기',
		view: '보기',
		views: { list: '목록', expanded: '펼쳐 보기', split: '3단', magazine: '매거진' },
		markOnScroll: '펼쳐 보기에서 스크롤해 지나간 글을 읽음으로 표시',
		teasers: '목록에서 제목 뒤에 본문 첫 부분 보이기',
		wakeLock: 'rueder가 열려 있는 동안 화면 켜 두기',
		wakeLockMissing: '이 브라우저는 화면을 켜 둘 수 없습니다.',
		note: '변경 사항은 바로 적용되고 계정에 저장되므로 모든 기기에서 같습니다.',
		noteDemo: '변경 사항은 바로 적용되고 이 브라우저에 남습니다.',
		apps: {
			intro:
				'다른 피드 리더 앱도 Google Reader API로 이 계정을 쓸 수 있습니다. 앱마다 전용 앱 비밀번호를 받으며, 언제든 취소할 수 있습니다.',
			none: '아직 설정한 앱이 없습니다.',
			made: (when) => `${when}에 설정`,
			used: (when) => `마지막 사용 ${when}`,
			unused: '아직 사용하지 않음',
			revoke: '취소',
			revokeName: (name) => `‘${name}’ 앱 비밀번호 취소`,
			add: '앱 설정',
			full: (max) =>
				`계정마다 앱 비밀번호는 최대 ${n(max)}개입니다. 다른 앱을 설정하려면 하나를 취소하세요.`,
			choose: '어떤 앱을 쓰시겠어요?',
			website: (app) => `${app} 웹사이트`,
			other: '다른 앱',
			otherHint: 'Google Reader API나 FreshRSS를 지원하는 앱이면 됩니다.',
			platforms: {
				apple: 'Mac, iPhone, iPad',
				android: 'Android'
			},
			name: '앱 비밀번호 이름',
			password: '비밀번호',
			code: '인증 앱의 코드 또는 복구 코드',
			create: '앱 비밀번호 만들기',
			creating: '잠시만 기다리세요 …',
			back: '뒤로',
			how: {
				netnewswire:
					'NetNewsWire에서 설정을 열고 계정에서 계정을 추가한 뒤(Mac에서는 +) FreshRSS를 고르세요. 아래 이름을 Login(iPhone과 iPad에서는 Username or Email)에, 앱 비밀번호를 Password에, 서버 주소를 API URL에 입력하세요.',
				readyou:
					'Read You에서 설정, 계정, 계정 추가를 열고 Google Reader를 고르세요. 아래 서버 주소를 서버 URL에, 이름을 사용자 이름에, 앱 비밀번호를 비밀번호에 입력하세요.',
				other:
					'앱에서 “Google Reader API” 종류의 계정을, 없다면 “FreshRSS” 계정을 추가하세요. 아래 서버 주소, 이름, 앱 비밀번호를 입력하세요.'
			},
			ownIcons: (app) =>
				`개인정보: ${app} 앱은 피드 아이콘을 각 피드의 웹사이트에서 직접 불러오므로, 그 사이트에 IP 주소가 전해집니다. 글과 이미지는 rueder를 통해서만 들어옵니다.`,
			shotsTitle: '그림으로 보는 단계',
			shots: {
				readyou: [
					'왼쪽 위의 톱니바퀴를 누르세요.',
					'계정을 여세요.',
					'계정 추가를 누르세요.',
					'Google Reader를 고르세요.',
					'서버 주소를 서버 URL에, 이름을 사용자 이름에, 앱 비밀번호를 비밀번호에 입력하고 추가를 누르세요.'
				],
				netnewswire: [
					'Mac에서 설정(⌘,)을 열고 계정에서 왼쪽 아래의 +를 클릭하세요.',
					'FreshRSS를 고르고 계속을 클릭하세요.',
					'이름을 Login에, 앱 비밀번호를 Password에, 서버 주소를 API URL에 입력하고 생성을 클릭하세요.'
				]
			},
			shotAlt: (app, step) => `${app}, ${n(step)}단계`,
			server: '서버 주소',
			user: '이름',
			secret: '앱 비밀번호',
			copy: (what) => `복사: ${what}`,
			copied: '복사했습니다',
			once: '앱 비밀번호는 지금만 보입니다. 잃어버리면 취소하고 새로 만드세요.',
			waiting: '앱이 로그인하기를 기다리는 중 …',
			connected: '연결되었습니다. 앱이 로그인했습니다.',
			done: '완료',
			refusals: {
				wrongPassword: '비밀번호가 맞지 않습니다.',
				codeNeeded: '인증 앱의 코드나 복구 코드도 입력하거나 패스키로 확인하세요.',
				wrongCode: '코드가 맞지 않습니다. 코드는 30초마다 바뀝니다.',
				loginAgain:
					'계정에 아직 비밀번호도 2단계 인증도 없습니다. 다시 로그인한 뒤 10분 안에 앱 비밀번호를 만드세요.',
				tooManyApps: '계정마다 앱 비밀번호는 최대 20개입니다.',
				tooMany: '잘못된 시도가 너무 많습니다. 몇 분 뒤에 다시 시도하세요.',
				unreachable: '서버가 응답하지 않습니다. 잠시 뒤에 다시 시도하세요.'
			}
		},
		secondFactor: {
			title: '2단계 인증',
			intro: '2단계 인증을 쓰면 로그인할 때 인증 앱의 코드나 패스키도 필요합니다.',
			app: '인증 앱',
			on: '켜짐.',
			left: (count) => `복구 코드 ${n(count)}개 남음.`,
			off: '꺼짐.',
			byPasskeys: '설정하지 않았습니다. 패스키가 있어 2단계 인증은 켜져 있습니다.',
			adminNeeds:
				'이 서버는 2단계 인증이 있는 관리자만 관리자로 대합니다. 서버 상태를 보려면 설정하거나 그 코드로 다시 로그인하세요.',
			password: '비밀번호',
			start: '설정',
			scan: '인증 앱으로 이 코드를 스캔하거나 아래 키를 입력하세요.',
			qr: '인증 앱용 QR 코드',
			larger: 'QR 코드 크게 보기',
			smaller: 'QR 코드 작게 보기',
			key: '키',
			copyKey: '키 복사',
			copied: '복사했습니다',
			code: '앱에 지금 표시된 코드',
			codeOrRecovery: '앱의 코드 또는 복구 코드',
			confirm: '확인',
			cancel: '취소',
			codesTitle: '복구 코드',
			codesIntro:
				'코드마다 휴대폰 없이 한 번 로그인할 수 있습니다. 안전하게 보관하세요. 지금만 보입니다.',
			copyCodes: '코드 복사',
			codesDone: '보관했습니다',
			switchOff: '끄기',
			switchOffNote: '끄면 다시 비밀번호만으로 로그인합니다.',
			switchOffNoteKeys: '패스키가 있어 2단계 인증은 켜진 채로 남습니다.',
			startWithPasskey: '패스키로 설정',
			switchOffWithPasskey: '패스키로 끄기',
			refusals: {
				wrongPassword: '비밀번호가 맞지 않습니다.',
				codeNeeded: '패스키로 2단계 인증이 켜져 있습니다. 복구 코드도 입력하세요.',
				wrongCode: '코드가 맞지 않습니다. 코드는 30초마다 바뀝니다.',
				loginAgain:
					'계정에 아직 비밀번호도 2단계 인증도 없습니다. 다시 로그인한 뒤 10분 안에 앱을 설정하세요.',
				tooMany: '잘못된 시도가 너무 많습니다. 몇 분 뒤에 다시 시도하세요.',
				unreachable: '서버가 응답하지 않습니다. 잠시 뒤에 다시 시도하세요.'
			}
		},
		passkeys: {
			title: '패스키',
			intro:
				'패스키를 쓰면 이름과 비밀번호 없이, 기기가 묻는 PIN이나 지문, 얼굴로 로그인합니다. 2단계 인증으로도 인정됩니다.',
			once: '패스키가 있으면 비밀번호만으로는 계정을 열 수 없습니다.',
			none: '아직 패스키가 없습니다.',
			made: (when) => `${when}에 추가`,
			used: (when) => `마지막 사용 ${when}`,
			unused: '아직 사용하지 않음',
			add: '패스키 추가',
			name: '이름 (예: 기기 이름)',
			defaultName: '패스키',
			password: '비밀번호',
			code: '인증 앱의 코드 또는 복구 코드',
			create: '패스키 만들기',
			working: '브라우저 안내를 따르세요 …',
			cancel: '취소',
			remove: '삭제',
			removeName: (name) => `‘${name}’ 패스키 삭제`,
			confirmRemove: '패스키 삭제',
			full: (max) => `계정마다 패스키는 최대 ${n(max)}개입니다.`,
			or: '또는',
			confirmWith: '패스키로 확인',
			codesTitle: '복구 코드',
			codesIntro:
				'첫 패스키로 2단계 인증이 켜졌습니다. 기기를 잃어버리면 코드마다 한 번 로그인할 수 있습니다. 안전하게 보관하세요. 지금만 보입니다.',
			copyCodes: '코드 복사',
			copied: '복사했습니다',
			codesDone: '보관했습니다',
			refusals: {
				cancelled: '패스키를 쓰기 전에 브라우저 창이 닫혔습니다.',
				known: '이 기기에는 이미 이 계정의 패스키가 있습니다.',
				failed: '이 브라우저나 기기에서 패스키를 만들거나 쓰지 못했습니다.',
				wrong: '패스키를 받지 않았습니다. 다시 시도하세요.',
				wrongPassword: '비밀번호가 맞지 않습니다.',
				codeNeeded:
					'이 계정에는 2단계 인증이 있습니다. 앱의 코드나 복구 코드도 입력하거나 패스키로 확인하세요.',
				wrongCode: '코드가 맞지 않습니다. 코드는 30초마다 바뀝니다.',
				tooManyPasskeys: '계정마다 패스키는 최대 10개입니다.',
				loginAgain:
					'계정에 아직 비밀번호도 2단계 인증도 없습니다. 다시 로그인한 뒤 10분 안에 첫 패스키를 추가하세요.',
				tooMany: '잘못된 시도가 너무 많습니다. 몇 분 뒤에 다시 시도하세요.',
				unreachable: '서버가 응답하지 않습니다. 잠시 뒤에 다시 시도하세요.'
			}
		},
		reports: {
			send: '이 브라우저의 오류 보고를 이 서버로 보내기',
			intro: '페이지에서 문제가 생기면 이 서버의 관리자가 알게 됩니다.',
			what: '보고에 담기는 것:',
			holds: [
				'오류 메시지 (주소나 무언가를 가리키는 숫자는 빼고)',
				'rueder 자체 파일의 어디에서 일어났는지',
				'rueder의 버전과 브라우저 및 그 주 버전'
			],
			never: '읽는 글, 피드, 라벨, 규칙은 담기지 않습니다. 보고는 30일 뒤에 사라집니다.',
			off: '끄면 보낸 보고가 삭제됩니다.'
		}
	},

	rules: {
		intro:
			'규칙은 들어오는 글을 숨기거나 읽음으로 표시합니다. 숨긴 글은 읽지 않은 글로 세지 않으며 ‘필터로 숨긴 글’에 남습니다.',
		none: '아직 규칙이 없습니다.',
		add: '규칙 추가',
		new: '새 규칙',
		change: '규칙 변경',
		on: '켜짐',
		scope: '대상',
		scopeAll: '모든 피드',
		scopeFolder: (title) => `폴더: ${title}`,
		scopeFeed: (title) => `피드: ${title}`,
		field: '살펴볼 곳',
		fields: { any: '전체', title: '제목', content: '본문', author: '글쓴이', tag: '태그' },
		match: '비교',
		matches: {
			contains: '포함',
			word: '단어로 포함',
			starts: '앞부분 일치',
			ends: '뒷부분 일치',
			regex: '정규 표현식과 일치',
			all: '모든 글 (내용과 상관없이)'
		},
		pattern: '찾을 텍스트',
		action: '동작',
		actions: { hide: '글 숨기기', read: '읽음으로 표시' },
		preview: (count, total) => `지금 여기의 글 ${n(total)}개 중 ${n(count)}개에 해당합니다.`,
		regexTip: '정규 표현식은 Rust와 같은 방식으로 읽으며, 대소문자를 구분하지 않습니다.',
		regexBuilder: 'regex101에서 만들어 보고 시험하기',
		invalid: '올바른 정규 표현식이 아닙니다.',
		empty: '찾을 텍스트를 입력하세요.',
		checking: '해당하는 글을 찾는 중 …',
		hits: (count) => `${n(count)}개 일치`,
		summary: (field, match, pattern, action) => `${field}: “${pattern}” ${match} → ${action}`,
		summaryAll: (action) => `모든 글 → ${action}`
	},

	keys: {
		title: '키보드 단축키',
		note: '키 설정은 설정과 함께 저장됩니다. Enter, Esc, g는 바뀌지 않습니다.',
		change: (what) => `다음 키 바꾸기: ${what}`,
		changeShort: '바꾸기',
		press: '키를 누르세요 …',
		taken: (key, other) => `${key} 키는 이미 다음에 쓰입니다: ${other}`,
		fixed: (key) => `${key} 키는 단축키로 지정할 수 없습니다.`,
		reset: 'rueder 기본 키로 되돌리기',
		groups: { articles: '글', lists: '목록', go: '이동', app: '어디서나' },
		next: '다음 글 열기',
		previous: '이전 글 열기',
		nextCursor: '다음 글로 (열지 않고)',
		previousCursor: '이전 글로 (열지 않고)',
		toggle: '글 열기/닫기',
		star: '별표하기/해제',
		read: '읽음/읽지 않음으로 표시',
		original: '원문을 새 탭에서 열기',
		markAll: '모두 읽음으로 표시',
		refresh: '목록 새로 고침',
		unreadOnly: '읽지 않은 글만 또는 모든 글',
		listView: '목록 보기',
		expandedView: '펼쳐 보기',
		splitView: '3단 보기',
		magazineView: '매거진',
		goHome: '홈',
		goAll: '모든 글',
		goUnread: '읽지 않은 글',
		goStarred: '별표한 글',
		search: '검색',
		sidebar: '사이드바 보이기/숨기기',
		help: '이 목록',
		then: '다음에'
	},

	labels: {
		title: '라벨',
		add: '새 라벨',
		name: '라벨 이름',
		none: '아직 라벨이 없습니다.',
		actions: (title) => `‘${title}’ 라벨 작업`,
		rename: '이름 바꾸기',
		color: '색',
		colors: ['빨강', '주황', '노랑', '초록', '청록', '파랑', '보라', '회색'],
		remove: '라벨 삭제',
		show: '라벨 보이기',
		hide: '라벨 숨기기',
		forArticle: '라벨',
		set: (title) => `라벨: ${title}`
	},

	subscribe: {
		title: '피드 구독',
		address: '피드 또는 웹사이트 주소',
		find: '피드 찾기',
		finding: '찾는 중 …',
		note: '초안은 아무것도 가져올 수 없어 피드가 비어 있습니다.',
		found: (count) => `피드 ${n(count)}개가 있습니다:`,
		none: '이 주소에는 피드가 없습니다.',
		movedTo: (url) => `사이트가 요청을 다음 주소로 넘겼습니다: ${url}`,
		loginWall:
			'로그인하지 않은 사람을 로그인 페이지로 보내는 사이트가 있습니다. 그런 사이트의 피드는 읽을 수 없습니다.',
		subscribed: '구독함',
		show: '보기',
		untitled: '이름 없음',
		page: '피드가 없는 웹사이트입니다. rueder가 제목과 요약만 담은 피드를 만듭니다.',
		folder: '넣을 폴더',
		topLevel: '폴더 없음',
		add: '구독',
		adding: '구독하는 중 …',
		refused: '실패했습니다.',
		serverSays: (reason) => `서버의 응답: ${reason}`,
		busy: (seconds) =>
			seconds
				? `동시에 찾는 요청이 너무 많습니다. ${n(seconds)}초 뒤에 다시 시도하세요.`
				: '동시에 찾는 요청이 너무 많습니다. 잠시 뒤에 다시 시도하세요.',
		noFetching: '이 서버는 피드를 가져오지 않습니다.',
		tooLarge: (megabytes) => `파일이 ${n(megabytes)}MB보다 큽니다.`,
		unreachable: '서버가 응답하지 않습니다. 잠시 뒤에 다시 시도하세요.'
	},

	opml: {
		title: '가져오기와 내보내기',
		intro: '다른 피드 리더의 구독을 OPML 파일로 가져오거나 내보낼 수 있습니다.',
		import: 'OPML 파일 가져오기',
		importing: '가져오는 중 …',
		imported: (feeds, folders) => `새 피드 ${n(feeds)}개, 새 폴더 ${n(folders)}개.`,
		known: (count) => `피드 ${n(count)}개는 이미 구독 중이었습니다.`,
		skipped: (count) => `읽을 수 있는 피드가 아닌 항목 ${n(count)}개는 건너뛰었습니다.`,
		later: '글은 다음 가져오기 때 들어옵니다.',
		export: 'OPML로 내보내기'
	},

	removeAccount: {
		title: '계정 삭제',
		intro: '이 rueder의 계정을 영구히 삭제합니다. 되돌릴 수 없습니다.',
		goes: '함께 삭제되는 것: 모든 기기의 세션, 구독과 폴더, 읽은 기록, 별표, 라벨, 필터 규칙, 설정, 오류 보고, 초대, 통합 로그인 연결.',
		stays:
			'피드는 그것을 읽는 다른 사람들을 위해 남습니다. 먼저 계정을 다른 곳으로 옮기려면 ‘이사’를 쓰세요.',
		password: '비밀번호',
		name: (name) => `확인을 위해 이름(${name})을 입력하세요`,
		code: '앱의 코드 또는 복구 코드',
		submit: '내 계정 영구 삭제',
		wrong: '맞지 않습니다. 아무것도 삭제하지 않았습니다.',
		codeNeeded:
			'이 계정에는 2단계 인증이 있습니다. 앱의 코드나 복구 코드도 입력하거나 패스키로 확인하세요.',
		busy: (count) => `잘못된 시도가 너무 많습니다. ${minutes(count)} 뒤에 다시 시도하세요.`,
		unreachable: '서버가 응답하지 않습니다. 아무것도 삭제하지 않았습니다.'
	},

	move: {
		intro: '파일 하나로 계정을 다른 rueder로 옮기거나 여기로 가져올 수 있습니다.',
		howItWorks:
			'가져오기는 여기 있는 것에 더할 뿐 아무것도 지우지 않습니다. 설정은 파일의 설정이 됩니다.',
		notInIt: '파일에 들어 있지 않은 것: 비밀번호, 세션, 글의 본문.',
		export: '계정 내보내기',
		import: '계정 파일 가져오기',
		importing: '가져오는 중 …',
		feeds: (feeds, known) => `새 피드 ${n(feeds)}개, 이미 구독 중 ${n(known)}개.`,
		folders: (count) => `새 폴더 ${n(count)}개.`,
		articles: (found) => `여기에서 찾은 ${articles(found)}.`,
		missing: (count) =>
			`여기에서 찾지 못한 ${articles(count)}. 그 글들의 읽음, 별표, 라벨 기록은 사라집니다.`,
		marks: (labels, rules) => `새 라벨 ${n(labels)}개, 새 필터 규칙 ${n(rules)}개.`,
		skipped: (count) => `파일의 항목 ${n(count)}개는 가져올 수 없었습니다.`,
		settings: '설정이 파일의 설정으로 바뀌었습니다.',
		later: '새 피드의 글은 다음 가져오기 때 들어옵니다. 읽은 글은 읽은 채로 남습니다.',
		busy: (seconds) =>
			seconds
				? `가져오기가 너무 많습니다. ${minutes(Math.ceil(seconds / 60))} 뒤에 다시 시도하세요.`
				: '가져오기가 너무 많습니다. 나중에 다시 시도하세요.'
	},

	status: {
		title: '서버 상태',
		open: '서버 상태',
		loading: '서버에 묻는 중 …',
		failed: '서버가 응답하지 않았습니다. 잠시 뒤에 다시 시도하세요.',
		refresh: '새로 고침',
		asOf: (time) => `${time} 기준. 지난 1시간을 셉니다.`,
		demo: '여기에는 서버가 없어 지어낸 값입니다.',
		empty: '–',
		on: '켜짐',
		off: '꺼짐',
		took: (median, p95) => `중앙값 ${median}, 95%가 ${p95} 이내`,
		summary: {
			label: '한눈에 보기',
			hints: '주의할 점',
			failing: '실패하는 피드',
			errors: '기록된 오류',
			reported: '신고된 피드'
		},
		hintsTitle: '살펴볼 만한 것',
		allWell: '눈에 띄는 문제가 없습니다.',
		hints: {
			slowRoute: (route, took) =>
				`${route} 경로가 느립니다. 요청 스무 건 중 한 건이 ${took}보다 오래 걸립니다.`,
			serverErrors: (route, count) => `${route} 경로가 서버 오류로 ${n(count)}번 응답했습니다.`,
			poolBusy: (size) =>
				`데이터베이스 연결 ${n(size)}개가 모두 사용 중이라 요청이 연결을 기다립니다.`,
			fetchBacklog: (count, late) =>
				`피드 ${n(count)}개를 늦게 가져오고 있으며, 가장 늦은 것은 ${late} 늦었습니다.`,
			feedsFailing: (failing, subscribed) =>
				`피드 ${n(subscribed)}개 중 ${n(failing)}개가 실패합니다.`,
			notFetching: '서버가 피드를 가져오지 않습니다. FETCH가 off로 되어 있나요?',
			fetchPaceFull: (most) =>
				`매번 가져올 수 있는 만큼 가져오는데도 기다리는 피드가 있습니다. 지금 속도로는 한 시간에 최대 ${n(most)}번 가져옵니다. FETCH_ROUND, FETCH_AT_ONCE, FETCH_EVERY로 정합니다.`,
			cacheMisses: (share) =>
				`데이터베이스가 읽는 것 중 ${share}만 메모리에 있습니다. shared_buffers를 늘리면 도움이 됩니다.`,
			databaseLarge: (size, warn) =>
				`데이터베이스가 ${size}까지 커졌습니다. DATABASE_WARN_GB는 ${warn}부터 경고합니다. “rueder retention set”으로 오래된 글이나 그 본문을 지울 수 있습니다.`,
			longQuery: (took) => `쿼리 하나가 ${took} 동안 실행 중입니다.`,
			imageCacheFull: (share) => `이미지 캐시가 ${share} 찼습니다.`,
			errors: (count) => `서버가 지난 1시간 동안 오류 ${n(count)}개를 기록했습니다.`,
			limited: (count) => `요청 ${n(count)}건을 너무 많아 거절했습니다.`,
			browserErrors: (count) =>
				`지난 하루 동안 브라우저에서 서로 다른 오류 ${n(count)}개를 보고했습니다.`,
			reportedFeeds: (count) => `독자들이 신고했지만 차단되지 않은 피드가 ${n(count)}개 있습니다.`
		},
		build: {
			title: '빌드',
			commit: '커밋',
			dirty: (commit) => `${commit} (자체 변경 포함)`,
			branch: '브랜치',
			tag: '태그',
			committed: '커밋 시각',
			where: '실행 환경',
			container: '컨테이너 안',
			host: '머신에서 직접',
			target: '대상',
			profile: '컴파일',
			profileValue: (profile, level) => `${profile}, 최적화 ${level}`,
			ci: '빌드한 곳',
			ciValue: 'CI 실행',
			byHand: '수동'
		},
		server: {
			title: '서버',
			version: '버전',
			running: '실행 시작',
			memory: '메모리',
			accounts: '계정',
			feeds: '피드',
			feedsValue: (fetched, all) => `가져오는 피드 ${n(fetched)}개, 전체 ${n(all)}개`,
			articles: '글',
			articlesValue: (count) => `약 ${n(count)}개`,
			live: '열린 탭'
		},
		database: {
			title: '데이터베이스',
			pool: 'rueder의 연결',
			poolValue: (size, idle, max) => `열림 ${n(size)}개, 대기 ${n(idle)}개, 최대 ${n(max)}개`,
			connections: '모든 연결',
			size: '크기',
			sizeWarn: (size, warn) => `${size}, ${warn}부터 경고`,
			hitRatio: '메모리에서 읽은 비율',
			deadlocks: '교착 상태',
			longest: '가장 오래 실행 중인 쿼리',
			tables: '가장 큰 테이블',
			table: '테이블',
			bytes: '크기',
			rows: '행 수 (대략)'
		},
		fetcher: {
			title: '피드 가져오기',
			state: '가져오기',
			lastRound: '마지막 회차',
			lastRoundValue: (ago, feeds, took) => `${ago}: 피드 ${n(feeds)}개, ${took} 걸림`,
			fetches: '가져오기 횟수',
			fetchesValue: (fetched, unchanged, failed) =>
				`피드를 가져옴 ${n(fetched)}번, 변화 없음 ${n(unchanged)}번, 실패 ${n(failed)}번`,
			broken: '데이터베이스에서 실패',
			newArticles: '새 글',
			took: '가져오기 1회당 시간',
			overdue: '늦게 가져옴',
			overdueValue: (count, late) => `${n(count)}개, 가장 늦은 것은 ${late}`,
			waiting: '아직 가져오지 않음',
			pace: '속도',
			paceValue: (round, every, atOnce, most) =>
				`${every}마다 ${n(round)}개, 동시에 ${n(atOnce)}개: 한 시간에 최대 ${n(most)}번`,
			perHour: '지난 1시간의 가져오기',
			behind: '지연',
			behindValue: (late) => `가장 오래 기다린 피드는 예정보다 ${late} 늦었습니다`,
			notBehind: '기다리는 것이 없습니다',
			failing: '실패하는 피드 (원인별)',
			noneFailing: '실패하는 피드가 없습니다.',
			feeds: (count) => `피드 ${n(count)}개`,
			failures: {
				status: (code) => (code ? `사이트가 ${code}로 응답함` : '사이트가 오류로 응답함'),
				timeout: '사이트가 너무 오래 걸림',
				blocked: 'rueder가 가져오지 않는 네트워크의 주소임',
				dns: '이름에 해당하는 주소가 없음',
				network: '연결이 없거나 끊김',
				too_large: '너무 큼',
				redirects: '리디렉션이 너무 많거나 잘못됨',
				unreadable: 'rueder가 읽을 수 있는 피드가 아님',
				other: '기타'
			}
		},
		finder: {
			title: '피드 찾기',
			state: '주소 살펴보기',
			lookups: '찾기',
			lookupsValue: (found, none, failed) =>
				`피드를 찾음 ${n(found)}번, 찾지 못함 ${n(none)}번, 실패 ${n(failed)}번`,
			limited: '계정 한도를 넘음',
			took: '찾기 1회당 시간'
		},
		rssifier: {
			title: '피드가 없는 웹사이트',
			state: 'RSSifier',
			pages: '읽는 웹사이트',
			pagesValue: (pages, failing) => `${n(pages)}개, 그중 ${n(failing)}개 실패`,
			used: '토큰 사용량',
			usedValue: (requests, fetches, underWay) =>
				`최근 1분 요청 ${n(requests)}건, 최근 1시간 가져온 페이지 ${n(fetches)}건, 진행 중 ${n(underWay)}건`,
			unanswered: (why) => `RSSifier가 응답하지 않습니다: ${why}`,
			limits: '토큰 한도',
			limitsValue: (perMinute, perHour, atOnce) =>
				`분당 ${perMinute}, 시간당 페이지 ${perHour}, 동시에 ${atOnce}`,
			noLimit: '제한 없음'
		},
		images: {
			title: '이미지 프록시',
			state: '프록시',
			cache: '캐시',
			cacheValue: (used, max) => `${max} 중 ${used}`,
			served: '이미지',
			servedValue: (cached, fetched) => `캐시에서 ${n(cached)}개, 가져옴 ${n(fetched)}개`,
			refused: '거절',
			refusedValue: (source, notImage, tooLarge, notFound) =>
				`원본 실패 ${n(source)}개, 이미지 아님 ${n(notImage)}개, 너무 큼 ${n(tooLarge)}개, 잘못된 주소 ${n(notFound)}개`,
			took: '가져온 이미지 1개당 시간',
			failingSources: '당분간 건드리지 않는 원본'
		},
		requests: {
			title: '요청',
			all: '모든 요청',
			allValue: (count, took) => `${n(count)}건, ${took}`,
			answers: '응답',
			answersValue: (fine, refused, errors) =>
				`정상 ${n(fine)}건, 거절 ${n(refused)}건, 서버 오류 ${n(errors)}건`,
			route: '경로',
			count: '건수',
			median: '중앙값',
			p95: '95%',
			max: '최장',
			errors: '오류',
			none: '지난 1시간 동안 요청이 없습니다.'
		},
		problems: {
			title: '경고와 오류',
			counts: (errors, warnings) => `지난 1시간 동안 오류 ${n(errors)}개, 경고 ${n(warnings)}개.`,
			none: '시작한 뒤로 기록된 것이 없습니다.',
			error: '오류',
			warning: '경고',
			times: (count, last) => (count > 1 ? `${n(count)}번, 마지막 ${last}` : last)
		},
		browsers: {
			title: '브라우저의 오류',
			off: '이 서버는 오류 보고를 받지 않습니다 (CLIENT_REPORTS=off).',
			consenting: (count) =>
				`계정 ${n(count)}개가 보고를 보냅니다. 오류는 마지막으로 일어난 뒤 30일 동안 남습니다.`,
			none: '오류를 보고한 브라우저가 없습니다.',
			kinds: {
				error: '오류',
				rejection: '실패한 Promise',
				component: '페이지의 일부',
				request: '요청'
			},
			hits: (count, accounts, last) => `계정 ${n(accounts)}개에서 ${n(count)}번, 마지막 ${last}`,
			where: (versions, browsers) => `버전 ${versions} · ${browsers}`
		},
		reported: {
			title: '신고된 피드',
			text: '누가 신고했는지는 여기 나오지 않습니다. “rueder feed block <주소>”로 모두에게 피드를 차단하고, “rueder feed unblock <주소>”로 풉니다.',
			none: '신고된 피드가 없습니다.',
			reports: (count, last) => `신고 ${n(count)}건, 마지막 ${last}`,
			blocked: '차단됨'
		}
	},

	update: {
		ready: 'rueder의 새 버전이 준비되었습니다.',
		reload: '새로 고침',
		later: '나중에'
	},

	crash: {
		title: '이 페이지에서 문제가 생겼습니다.',
		text: '계속하려면 페이지를 새로 고치세요. 읽은 기록과 바꾼 내용은 서버에 저장되어 있습니다.',
		reload: '새로 고침'
	},

	demo: {
		badge: '초안',
		note: '지어낸 피드로 만든 초안입니다. 서버에는 아무것도 저장되지 않습니다.',
		arrive: '글 하나 들어오게 하기',
		revise: '열린 글 바꾸기',
		revised: '업데이트: 버전을 시험해 보려고 이 문단을 추가했습니다.',
		reviseHint: '먼저 글을 여세요.'
	}
};

export default ko;
