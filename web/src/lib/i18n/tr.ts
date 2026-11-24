// The Turkish texts, in the shape of the English catalog (en.ts), addressing the
// reader with “siz”. A noun after a number stays singular. No suffix follows a
// value, as its form depends on the value's vowels; values stand before a noun
// or after a colon instead; “rueder” takes the suffixes of a front vowel. A feed is an “akış”, an item a “makale”.

import type { Messages } from './en.ts';
import { formatNumber as n } from './intl.ts';

const articles = (count: number) => `${n(count)} makale`;

const minutes = (count: number) => `${n(count)} dakika`;

const tr: Messages = {
	languageName: 'Türkçe',

	articles,

	common: {
		close: 'Kapat',
		cancel: 'İptal',
		save: 'Kaydet',
		delete: 'Sil',
		edit: 'Düzenle'
	},

	nav: {
		subscribe: 'Abone ol',
		home: 'Ana sayfa',
		all: 'Tüm makaleler',
		unreadItems: 'Okunmamış makaleler',
		starred: 'Yıldızlı makaleler',
		hidden: 'Filtrelerin gizledikleri',
		subscriptions: 'Abonelikler',
		newFolder: 'Yeni klasör',
		groupedFolder: 'Yeni klasör',
		folderName: 'Klasörün adı',
		renameFolder: 'Yeniden adlandır',
		removeFolder: 'Klasörü kaldır, içindekileri koru',
		folderActions: (title) => `Klasör işlemleri: ${title}`,
		feedName: 'Akışın adı',
		feedNameHint: 'Akışın kendi adı için boş bırakın',
		renameFeed: 'Yeniden adlandır',
		unsubscribe: 'Abonelikten çık',
		reportFeed: 'Akışı bildir …',
		feedInfo: 'Akış bilgileri …',
		feedActions: (title) => `Akış işlemleri: ${title}`,
		unsubscribeTitle: 'Abonelikten çıkılsın mı?',
		unsubscribeText: (title) =>
			`“${title}” aboneliğinden çıkılsın mı? Orada okuduklarınız ve yıldızladıklarınız kaybolur.`,
		openFolder: 'Klasörü aç',
		closeFolder: 'Klasörü kapat',
		openSidebar: 'Kenar çubuğunu göster',
		closeSidebar: 'Kenar çubuğunu gizle',
		search: 'Ara',
		searchPlaceholder: 'Bu makalelerde ara',
		clearSearch: 'Aramayı temizle',
		settings: 'Ayarlar',
		accountSettings: 'Hesap ayarları',
		shortcuts: 'Klavye kısayolları',
		live: 'Canlı',
		liveOn: 'Yeni makaleler kendiliğinden gelir. Duraklatmak için tıklayın.',
		liveOff: 'Duraklatıldı. Yeni makalelerin yeniden gelmesi için tıklayın.',
		paused: 'Duraklatıldı',
		account: (name) => `Giriş yapan: ${name}`,
		logout: 'Çıkış yap',
		appSettings: 'Sunucu ve uygulama güncellemeleri',
		install: 'Uygulama olarak yükle',
		installSafari:
			'rueder’i uygulama olarak yüklemek için: Mac’te Dosya › Dock’a Ekle, iPhone veya iPad’de Paylaş › Ana Ekrana Ekle.',
		unread: (count) => `${n(count)} okunmamış`,
		hiddenCount: (count) => `filtrelerce gizlenen: ${n(count)}`,
		fetchFailed: (reason) => `Son getirme başarısız oldu: ${reason}`,
		waiting: 'Henüz getirilmedi',
		blocked: 'Bu rueder’in yöneticisi tarafından engellendi',
		moveHint: 'Sıralamak için sürükleyin ya da Alt ile ok tuşlarına basın',
		resizeSidebar: 'Kenar çubuğunun genişliği: sürükleyin ya da ok tuşlarını kullanın',
		tooDeep: (levels) => `Klasörler en fazla ${n(levels)} düzey iç içe olabilir.`
	},

	toolbar: {
		refresh: 'Yenile',
		showAll: 'Tüm makaleler',
		showUnread: 'Yalnızca okunmamışlar',
		show: 'Göster',
		newestFirst: 'Önce en yeniler',
		oldestFirst: 'Önce en eskiler',
		markAllRead: 'Tümünü okundu olarak işaretle',
		markOptions: 'Okundu olarak işaretlemenin diğer yolları',
		phoneMenu: (shown) => `Görünüm, gösterilen: ${shown}`,
		markMenu: 'Okundu olarak işaretle …',
		markEverything: 'Tüm makaleler',
		markOlderDay: 'Bir günden eski makaleler',
		markOlderWeek: 'Bir haftadan eski makaleler',
		listView: 'Liste görünümü',
		expandedView: 'Geniş görünüm',
		splitView: 'Üç bölme: ağaç, liste ve makale',
		magazineView: 'Dergi',
		previous: 'Önceki makale',
		next: 'Sonraki makale',
		newArticles: (count) => `${n(count)} yeni makale`
	},

	list: {
		empty: 'Burada makale yok.',
		emptyUnread: 'Burada okunmamış bir şey yok.',
		emptySearch: 'Aramaya uyan makale yok.',
		found: (count, more, words) => `“${words}” geçen ${n(count)}${more ? '+' : ''} makale`,
		searchAll: 'Tüm akışlarda ara',
		feedFailing:
			'rueder daha sonra yeniden dener ve her başarısızlıktan sonra ya da sitenin istediği kadar daha uzun bekler. Hata sürerse akış taşınmış olabilir.',
		hiddenNote:
			'Filtre kurallarınız bu makaleleri gizledi. Hangi kuralın gizlediğini görmek için birini açın.',
		toRules: 'Filtre kuralları',
		emptyFeed: 'Bu akışta henüz makale yok.',
		waitingFeed:
			'Bu akış henüz getirilmedi. Makaleleri birkaç dakika içinde, bir sonraki getirme turunda gelir.',
		showAll: 'Tüm makaleleri göster',
		more: 'Daha fazla göster',
		star: 'Yıldız ekle',
		unstar: 'Yıldızı kaldır',
		keepUnread: 'Okunmamış bırak',
		openOriginal: 'Aslını aç',
		showFeed: 'Bu akışın tüm makalelerini rueder’de göster',
		from: 'kaynak',
		by: (author) => `yazar: ${author}`,
		updated: 'güncellendi',
		updatedHeadline: 'yeni başlık',
		updatedTitle: (count) => `İlk getirildiğinden beri değişti; önceki sürüm sayısı: ${n(count)}`,
		updatedHeadlineTitle: (count) =>
			`Başlığı yeniden yazıldı, metni aynı kaldı; önceki sürüm sayısı: ${n(count)}`,
		compare: 'Sürümleri karşılaştır',
		hiddenBy: (rule) => `Gizleyen kural: ${rule}`,
		textDropped: 'Bu makale eski olduğundan sunucu metnini artık saklamıyor.',
		textDroppedOriginal: 'Kaynağında okuyun',
		loading: 'Metin yükleniyor …',
		loadingList: 'Yükleniyor …',
		toTop: 'Başa dön',
		pick: 'Burada okumak için bir makale seçin.',
		resizeList: 'Listenin genişliği: sürükleyin ya da ok tuşlarını kullanın',
		closeArticle: 'Makaleyi kapat',
		blockedFeed:
			'Bu rueder’in yöneticisi bu akışı engelledi. Getirilmiyor ve makaleleri gösterilmiyor.',
		articleActions: 'Makale için diğer işlemler',
		articleInfo: 'Makale bilgileri …',
		articleSource: 'Kaynak kodu …',
		reportArticle: 'Makaleyi bildir …'
	},

	report: {
		feedTitle: 'Akışı bildir',
		articleTitle: 'Makaleyi bildir',
		text: (title) =>
			`Bu rueder’in yöneticisi “${title}” için bildirim yapıldığını, nedenini ve yazdıklarınızı görür, ama kimin bildirdiğini görmez. Herkes için engellenip engellenmeyeceğine o karar verir.`,
		reason: 'Neden',
		reasons: {
			illegal: 'Yasaya aykırı',
			copyright: 'Başkasının eseri',
			malicious: 'Kötü amaçlı yazılım, kimlik avı ya da dolandırıcılık',
			spam: 'İstenmeyen içerik',
			other: 'Başka bir şey'
		},
		note: 'Eklemek istedikleriniz (isteğe bağlı)',
		left: (count) => `Kalan karakter: ${n(count)}`,
		send: 'Bildirimi gönder',
		sending: 'Gönderiliyor …',
		sent: 'Teşekkürler. Bildirim bu rueder’in yöneticisine ulaştı.',
		tooMany: 'Son zamanlarda çok sayıda bildirim gönderdiniz. Daha sonra yeniden deneyin.',
		unreachable: 'Bildirim sunucuya ulaşmadı. Daha sonra yeniden deneyin.'
	},

	feedInfo: {
		title: 'Akış bilgileri',
		text: (title) =>
			`Bu rueder’in “${title}” hakkında bildikleri ve onu nasıl getirdiği. Akış çalışmıyorsa bunu yöneticiye yazdığınız mesaja kopyalayın.`,
		number: 'Numara',
		name: 'Ad',
		address: 'Akış adresi',
		site: 'Web sitesi',
		noSite: 'Yok',
		agent: 'Getirilirken kullanılan kimlik',
		agents: { rueder: 'rueder', browser: 'Tarayıcı' },
		every: 'Getirme aralığı',
		fetched: 'Son getirme',
		succeeded: 'Son başarılı getirme',
		failed: 'Son hata',
		next: 'Sonraki getirme',
		blocked: 'Yok, yönetici akışı engelledi',
		articles: 'Şimdiye kadarki makaleler',
		never: 'Hiç',
		fetchAs: 'Getirilirken kullanılacak kimlik',
		fetchAsHint:
			'Akışı okuyan herkes için geçerlidir. rueder’i geri çeviren siteler için tarayıcıyı seçin. Akış hemen yeniden getirilir.',
		notSwitched: 'Olmadı. Daha sonra yeniden deneyin.',
		copy: 'Kopyala',
		copied: 'Kopyalandı',
		loading: 'Yükleniyor …',
		unreachable: 'Sunucu yanıt vermedi. Daha sonra yeniden deneyin.'
	},
	articleInfo: {
		title: 'Makale bilgileri',
		text: (title) =>
			`Bu rueder’in “${title}” ve sürümleri hakkında bildikleri. Bir şey yanlış görünüyorsa bunu yöneticiye yazdığınız mesaja kopyalayın.`,
		feed: 'Akış',
		feedNumber: 'Akış numarası',
		number: 'Akıştaki numara',
		name: 'Başlık',
		id: 'Akıştaki kimlik',
		link: 'Bağlantı',
		noLink: 'Yok',
		published: 'Yayımlanma',
		fetched: 'İlk getirme',
		changed: 'Son değişiklik',
		never: 'Hiç',
		changedAt: (when, what) => `${when} (${what})`,
		what: { title: 'başlık', text: 'metin', both: 'başlık ve metin' },
		version: 'Sürüm',
		kept: 'Saklanan sürümler',
		textLabel: 'Metin',
		textDropped: 'Eski olduğu için artık saklanmıyor',
		copy: 'Kopyala',
		copied: 'Kopyalandı',
		loading: 'Yükleniyor …',
		unreachable: 'Sunucu yanıt vermedi. Daha sonra yeniden deneyin.'
	},
	articleSource: {
		title: 'Makalenin kaynak kodu',
		text: 'Makalenin HTML hâli: bu rueder’in temizledikten sonra sakladığı biçimiyle ya da akışın gönderdiği gibi. Bağlantılar ve resimler burada kendi adreslerini korur; böylece kalan bir izleme parametresi görünür.',
		show: 'Göster',
		kinds: { cleaned: 'Temizlenmiş', original: 'Akışın gönderdiği gibi' },
		noText: 'Bu sunucu bu makalenin metnini artık saklamıyor.',
		noOriginal: 'Bu sunucu akışın gönderdiğini artık saklamıyor.',
		copy: 'Kopyala',
		copied: 'Kopyalandı',
		loading: 'Yükleniyor …',
		unreachable: 'Sunucu yanıt vermedi. Daha sonra yeniden deneyin.'
	},

	media: {
		title: 'Ekler',
		play: 'Oynat',
		image: 'Görsel',
		zoomIn: 'Yakınlaştır',
		zoomOut: 'Uzaklaştır'
	},

	connection: {
		reconnecting: 'Yeniden bağlanılıyor …',
		unreachable: (since) => `Sunucu şu zamandan beri yanıt vermiyor: ${since}.`,
		offline: (since) => `Şu zamandan beri ağ yok: ${since}.`,
		retry: 'Şimdi yeniden dene',
		unsent: (count) => `${n(count)} değişiklik bekliyor.`
	},

	notice: {
		unreachable: 'Sunucu yanıt vermiyor. Gördükleriniz güncel olmayabilir.',
		notSaved: 'Sunucu son değişikliğinizi kabul etmedi. Burada geri alındı.',
		tooManyUnsent: 'Sunucuyu bekleyen o kadar çok değişiklik var ki bu değişiklik geri alındı.'
	},

	login: {
		title: 'rueder’e giriş yapın',
		about:
			'Bir akış okuyucusu: takip ettiğiniz sitelerin yayımladıklarını getirir ve henüz okumadıklarınızı gösterir.',
		name: 'Ad',
		password: 'Parola',
		submit: 'Giriş yap',
		checking: 'Bir dakika …',
		wrong: 'Ad ya da parola yanlış.',
		codeNeeded:
			'Bu hesapta ikinci faktör var. Uygulamanızın gösterdiği kodu ya da bir kurtarma kodunu girin veya geçiş anahtarıyla giriş yapın.',
		wrongCode: 'Bu kod doğru değil. Kodlar her 30 saniyede değişir.',
		code: 'Kod',
		codeNote: 'Kimlik doğrulama uygulamanızdaki altı haneli kod ya da kurtarma kodlarınızdan biri.',
		tooMany: (count) => `Çok fazla yanlış deneme. ${minutes(count)} sonra yeniden deneyin.`,
		unreachable: 'Sunucu yanıt vermiyor. Biraz sonra yeniden deneyin.',
		retry: 'Yeniden dene',
		or: 'ya da',
		oidc: 'Tek oturum açma ile giriş yap',
		passkey: 'Geçiş anahtarıyla giriş yap',
		passkeyCancelled:
			'Geçiş anahtarı kullanılmadı. Yeniden deneyin ya da ad ve parolayla giriş yapın.',
		passkeyFailed: 'Bu tarayıcı ya da cihaz burada geçiş anahtarı kullanamıyor.',
		passkeyWrong: 'Bu geçiş anahtarı buradaki hiçbir hesaba ait değil.',
		oidcFailed: 'Tek oturum açma ile giriş olmadı. Yeniden deneyin.',
		oidcDenied: 'Tek oturum açma ile giriş iptal edildi.',
		oidcUnknown:
			'Bu girişe ait bir hesap burada yok. Bağlamasını bu rueder’in yöneticisinden isteyin.',
		disabled: 'Bu hesap devre dışı. Nedenini bu rueder’in yöneticisine sorun.',
		unconfirmed:
			'Bu hesap e-posta adresinin onaylanmasını bekliyor. E-postadaki bağlantıyı açın ya da yeniden gönderilmesini sağlayın.',
		resend: 'Bağlantıyı yeniden gönder',
		resent: 'Bağlantı yeniden yolda. Gelen kutunuza ve istenmeyen klasörüne bakın.',
		resendWrong: 'Bağlantının yeniden gönderilmesi için hesabın adını ve parolasını girin.',
		forgot: 'Parolanızı mı unuttunuz?',
		removed: 'Hesabınız, size ait her şeyle birlikte silindi.',
		note: 'Burada kendi kendine kaydolunamaz. Hesapları bu rueder’in yöneticisi açar.'
	},

	legal: {
		imprint: 'Künye',
		privacy: 'Gizlilik politikası',
		terms: 'Kullanım koşulları',
		agree: { before: 'Şunu kabul ediyorum: ', link: 'kullanım koşulları', after: '.' },
		changed: 'Kullanım koşulları değişti',
		changedIntro:
			'Lütfen yeni kullanım koşullarını okuyun. rueder’i kullanmaya devam etmek için kabul edin.',
		read: 'Kullanım koşullarını oku',
		accept: 'Kabul ediyorum',
		decline: 'Çıkış yap',
		notSaved: 'Olmadı. Biraz sonra yeniden deneyin.'
	},

	register: {
		title: 'Hesap aç',
		invited: 'Bu rueder’e davet edildiniz. Bir ad ve parola seçin.',
		again: 'Parola yeniden',
		passwordRule: (characters) => `En az ${n(characters)} karakter.`,
		mismatch: 'İki parola birbirinden farklı.',
		submit: 'Hesabı aç',
		solving: 'Bu tarayıcı denetleniyor …',
		toRegister: 'Hesap aç',
		toLogin: 'Var olan bir hesaba giriş yap',
		closed: 'Burada davetsiz kimse kaydolamaz.',
		inviteGone: 'Bu davet artık kullanılamaz. Yenisini isteyin.',
		full: 'Burada başka bir hesaba yer yok.',
		today: 'Bugün burada yeterince hesap açıldı. Yarın yeniden deneyin.',
		captcha: 'Bu tarayıcının denetimi tamamlanamadı. Yeniden deneyin.',
		taken: 'Bu adla bir hesap zaten var.',
		terms: 'Hesap açmak için kullanım koşullarını kabul edin.',
		invalid: (reason) => `Bu bir hesap olamaz. Sunucunun yanıtı: ${reason}`,
		email: 'E-posta adresi',
		emailOptional: 'E-posta adresi (isteğe bağlı)',
		emailNote: 'Onaylamanız için bir bağlantı alacaksınız.',
		emailRequiredNote:
			'Onaylamanız için bir bağlantı alacaksınız; bağlantıyı açtıktan sonra hesapla giriş yapılabilir.',
		emailInvalid: 'E-posta alabilen bir adres girin.',
		unsent: 'Bağlantı gönderilemediği için hesap açılmadı. Daha sonra yeniden deneyin.',
		tooMany: (count) => `Çok fazla deneme. ${minutes(count)} sonra yeniden deneyin.`,
		awaitingTitle: 'E-postanıza bakın',
		awaiting: (address, hours) =>
			`Hesap açıldı. Şu adrese bir bağlantı gitti: ${address}. Bağlantıyı ${n(hours)} saat içinde açın, sonra giriş yapın.`
	},

	forgot: {
		title: 'Parolanızı mı unuttunuz?',
		intro:
			'Adınızı ya da e-posta adresinizi girin. Yeni parola belirleme bağlantısı onaylı adrese gider.',
		field: 'Ad ya da e-posta adresi',
		submit: 'Bağlantıyı gönder',
		asked: (count) =>
			`Onaylı adresi olan böyle bir hesap varsa ona bir bağlantı yolda; bağlantı ${minutes(count)} geçerli. İstenmeyen klasörüne de bakın.`,
		tooMany: (count) => `Çok sık istendi. ${minutes(count)} sonra yeniden deneyin.`,
		back: 'Girişe dön'
	},

	reset: {
		title: 'Yeni parola belirleyin',
		intro: 'Hesabın tüm tarayıcı ve cihazlarında oturum kapatılır.',
		fresh: 'Yeni parola',
		again: 'Yeni parola yeniden',
		submit: 'Parolayı belirle',
		mismatch: 'İkisi birbirini tutmuyor.',
		gone: 'Bu bağlantı artık çalışmıyor. Yenisini isteyin.',
		invalid: (reason) => `Sunucu bu parolayı kabul etmedi. Yanıtı: ${reason}`,
		done: 'Parola belirlendi. Onunla giriş yapın.',
		cancel: 'İptal'
	},

	email: {
		title: 'E-posta adresi',
		intro: 'rueder size yalnızca hesabınızla ilgili yazar.',
		current: 'Adresiniz',
		none: 'Hesabın adresi yok.',
		pending: (address, hours) =>
			`Şu adrese bir bağlantı gitti: ${address}. Adres, bağlantı ${n(hours)} saat içinde açılınca geçerli olur; o zamana kadar öncekisi kalır.`,
		address: 'Yeni adres',
		password: 'Parolanız',
		passwordNote:
			'Bu tarayıcıyı kullanabilen başka biri e-postalarınızı başka yere yönlendiremesin diye.',
		code: 'İkinci faktörün kodu',
		submit: 'Bağlantıyı gönder',
		sent: 'Bağlantı yolda. Gelen kutunuza ve istenmeyen klasörüne bakın.',
		remove: 'Adresi kaldır',
		removeIntro:
			'Adres olmadan size hiçbir e-posta gönderilmez. Hesabın geri kalanı olduğu gibi kalır.',
		removed: 'Adres kaldırıldı.',
		wrong: 'Parola ya da kod doğru değil.',
		codeNeeded: 'Bu hesapta ikinci faktör var. Onun kodunu da girin.',
		loginAgain:
			'Hesabınızın henüz parolası ve ikinci faktörü yok. Yeniden giriş yapın, sonra adresi on dakika içinde girin.',
		invalid: 'E-posta alabilen bir adres girin.',
		same: 'Hesabın adresi zaten bu.',
		required:
			'Bu rueder’deki her hesabın bir adresi olmalı; adres değiştirilebilir ama kaldırılamaz.',
		busy: (count) => `Çok fazla deneme ya da e-posta. ${minutes(count)} sonra yeniden deneyin.`,
		unsent: 'E-posta gönderilemedi. Daha sonra yeniden deneyin.',
		unreachable: 'Sunucu yanıt vermiyor. Hiçbir şey değişmedi.',
		askTitle: 'Bir e-posta adresi ekleyin',
		askIntro:
			'Bu rueder’deki her hesabın artık bir e-posta adresi olmalı. Adresinizi girin ve ona gönderilen bağlantıyı açın.',
		askWaiting:
			'Bağlantıyı açtığınızda bu kendiliğinden kaybolur. Hiçbir şey gelmedi mi? Yeniden ya da başka bir adrese gönderin.',
		logOut: 'Çıkış yap',
		confirmed: (address) => `Bu adres onaylandı: ${address}.`,
		confirmedLogIn: 'Hesap hazır: giriş yapın.',
		gone: 'Bu bağlantı artık kullanılamaz: kullanıldı, daha yenisi gönderildi ya da bir günden eski.',
		taken: 'Bu arada başka bir hesap bu adresi aldı.',
		confirmUnreachable: 'Sunucu yanıt vermiyor. Biraz sonra bağlantıyı yeniden açın.',
		close: 'Kapat'
	},

	invites: {
		intro:
			'Bir bağlantı, birinin bu rueder’de hesap açmasını sağlar. Bağlantı yalnızca bir kez, burada, oluşturduğunuzda gösterilir.',
		left: (count) =>
			count === 0
				? 'Verebileceğiniz davetiniz yok.'
				: `${n(count)} kişiyi daha davet edebilirsiniz.`,
		admin: 'Yönetici olarak her zaman davet edebilirsiniz.',
		uses: 'Kaç hesap için',
		make: 'Bağlantı oluştur',
		noneLeft: 'Bu kadarı için davetiniz kalmadı.',
		failed: 'Olmadı. Biraz sonra yeniden deneyin.',
		link: 'Şimdi iletilecek bağlantı',
		copy: 'Kopyala',
		copied: 'Kopyalandı',
		goodFor: (days) => `Bir bağlantı ${n(days)} gün geçerlidir.`,
		yours: 'Bağlantılarınız',
		none: 'Henüz bağlantı oluşturmadınız.',
		used: (uses, of) => `${n(of)} kullanımdan ${n(uses)} kullanıldı`,
		until: (when) => `geçerlilik sonu: ${when}`,
		ran: (when) => `süresi doldu: ${when}`,
		usedUp: 'tükendi',
		revoked: 'geri alındı',
		revoke: 'Geri al',
		revokeLink: (made) => `Bağlantıyı geri al (oluşturulma: ${made})`
	},

	home: {
		newIn: 'Aboneliklerinizdeki yenilikler',
		allRead: 'Her şeyi okudunuz. Yeni makaleler geldikçe burada görünür.',
		recentlyStarred: 'Son yıldızlananlar',
		noStars: 'Henüz yıldızlı bir şey yok. Bir makalede s tuşuna basın.',
		problems: 'Sorunlu akışlar',
		tips: 'İpuçları',
		tipKeys: 'Tüm klavye kısayolları için ? tuşuna basın; j ve k makaleler arasında gezinir.',
		tipDrag: 'Sıralamak için kenar çubuğundaki akışları ve klasörleri sürükleyin.',
		tipFilters: 'Ayarlardaki filtre kuralları makaleleri anahtar sözcüğe ya da yazara göre gizler.',
		showFeed: (title) => `Göster: ${title}`
	},

	versions: {
		title: 'Bu makalenin sürümleri',
		current: 'Geçerli sürüm',
		version: (rev) => `Sürüm ${n(rev)}`,
		fetched: (when) => `getirilme: ${when}`,
		compareWith: 'Geçerli sürümü şununla karşılaştır',
		legendAdded: 'eklendi',
		legendRemoved: 'çıkarıldı',
		titleWas: 'Başlık şuydu:',
		same: 'Bu sürümün metni geçerli sürümle aynı.',
		sameWords: 'Sözcükler aynı; resimler ya da bağlantılar değişti.',
		markup: 'Sözcükler, resimler ve bağlantılar aynı; yalnızca metnin düzeni değişti.',
		pictures: 'Değişen resimler',
		noPicture: 'Yok',
		address: 'Resmin adresi',
		links: 'Değişen bağlantılar'
	},

	account: {
		title: 'Hesap ayarları',
		tabs: {
			password: 'Parola',
			email: 'E-posta',
			security: 'Güvenlik',
			apps: 'Uygulamalar',
			invites: 'Davetler',
			move: 'Taşıma',
			privacy: 'Gizlilik',
			remove: 'Hesabı sil'
		},
		password: {
			title: 'Parolayı değiştir',
			titleFirst: 'Parola belirle',
			intro:
				'Yeni bir parola diğer tüm tarayıcı ve cihazlarda oturumu kapatır ve tüm uygulama parolalarını iptal eder.',
			introFirst:
				'Bir parolayla bu hesap yalnızca tek oturum açma ile değil, adıyla da giriş yapabilir.',
			current: 'Geçerli parola',
			fresh: 'Yeni parola',
			again: 'Yeni parola yeniden',
			submit: 'Parolayı değiştir',
			submitFirst: 'Parolayı belirle',
			done: 'Parola değişti. Diğer tüm tarayıcı ve cihazlarda oturum kapatıldı, uygulama parolaları iptal edildi.',
			doneFirst: 'Parola belirlendi.',
			mismatch: 'İkisi birbirini tutmuyor.',
			code: 'İkinci faktörün kodu',
			wrong: 'Geçerli parola ya da kod doğru değil.',
			codeNeeded:
				'Bu hesapta ikinci faktör var. Kimlik doğrulama uygulamanızdan bir kodu ya da bir kurtarma kodunu da girin veya geçiş anahtarıyla onaylayın.',
			loginAgain:
				'Hesabınızın henüz parolası ve ikinci faktörü yok. Yeniden giriş yapın, sonra parolayı on dakika içinde belirleyin.',
			invalid: (characters) =>
				`Sunucu bu parolayı kabul etmedi. En az ${n(characters)} karakter olmalı.`,
			busy: (count) => `Çok fazla yanlış deneme. ${minutes(count)} sonra yeniden deneyin.`,
			unreachable: 'Sunucu yanıt vermiyor. Parola değişmedi.'
		},
		settingsNote: 'Görünüm, okuma, filtre kuralları ve klavye Ayarlar’dadır.'
	},

	settings: {
		title: 'Ayarlar',
		tabs: {
			look: 'Görünüm',
			reading: 'Okuma',
			rules: 'Filtre kuralları',
			keys: 'Klavye'
		},
		accountNote:
			'Parola, ikinci faktör, uygulamalar ve davetler, hesabınızın menüsündeki Hesap ayarları’ndadır.',
		theme: 'Tema',
		themes: { system: 'Sistem gibi', light: 'Açık', dark: 'Koyu' },
		accent: 'Vurgu rengi',
		accents: {
			red: 'Kırmızı',
			blue: 'Mavi',
			green: 'Yeşil',
			orange: 'Turuncu',
			purple: 'Mor',
			graphite: 'Grafit'
		},
		density: 'Listenin yoğunluğu',
		densities: { compact: 'Sıkı', comfortable: 'Rahat' },
		sidebarSide: 'Kenar çubuğu',
		sidebarSides: { start: 'Solda', end: 'Sağda' },
		textSize: 'Makale metninin boyutu, bu cihazda',
		textSizes: { small: 'Küçük', medium: 'Orta', large: 'Büyük' },
		readingFont: 'Makalelerin yazı tipi',
		readingFonts: { sans: 'Tırnaksız', serif: 'Tırnaklı' },
		counters: 'Okunmamış sayıları',
		counterStyles: { parens: 'Parantez içinde', badge: 'Rozet olarak' },
		language: 'Dil',
		languageAuto: 'Tarayıcı gibi',
		view: 'Görünüm',
		views: { list: 'Liste', expanded: 'Geniş', split: 'Üç bölme', magazine: 'Dergi' },
		markOnScroll: 'Geniş görünümde kaydırarak geçilen makaleleri okundu olarak işaretle',
		teasers: 'Listede başlıktan sonra metnin ilk sözcüklerini göster',
		wakeLock: 'rueder açıkken ekranı açık tut',
		wakeLockMissing: 'Bu tarayıcı ekranı açık tutamıyor.',
		note: 'Değişiklikler hemen uygulanır ve hesabınızla kaydedilir; böylece her cihazda sizinle olur.',
		noteDemo: 'Değişiklikler hemen uygulanır ve bu tarayıcıda kalır.',
		apps: {
			intro:
				'Diğer akış okuyucu uygulamaları Google Reader API ile hesabınızı kullanabilir. Her biri, istediğiniz zaman iptal edebileceğiniz kendi uygulama parolasını alır.',
			none: 'Henüz kurulu uygulama yok.',
			made: (when) => `kurulma: ${when}`,
			used: (when) => `son kullanım: ${when}`,
			unused: 'henüz kullanılmadı',
			revoke: 'İptal et',
			revokeName: (name) => `Uygulama parolasını iptal et: ${name}`,
			add: 'Uygulama kur',
			full: (max) =>
				`Bir hesabın en fazla ${n(max)} uygulama parolası olabilir. Başka bir uygulama kurmak için birini iptal edin.`,
			choose: 'Hangi uygulamayı kullanmak istiyorsunuz?',
			website: (app) => `Web sitesi: ${app}`,
			other: 'Başka bir uygulama',
			otherHint: 'Google Reader API’yi ya da FreshRSS’i destekleyen herhangi bir uygulama.',
			platforms: {
				apple: 'Mac, iPhone ve iPad',
				android: 'Android'
			},
			name: 'Uygulama parolasının adı',
			password: 'Parolanız',
			code: 'Kimlik doğrulama uygulamanızın kodu ya da bir kurtarma kodu',
			create: 'Uygulama parolası oluştur',
			creating: 'Bir dakika …',
			back: 'Geri',
			how: {
				netnewswire:
					'NetNewsWire’da Ayarlar’ı, ardından Hesaplar’ı açın, bir hesap ekleyin (Mac’te + ile) ve FreshRSS’i seçin. Aşağıdaki adı Login’e (iPhone ve iPad’de Username or Email), uygulama parolasını Password’e ve sunucu adresini API URL’ye girin.',
				readyou:
					'Read You’da Ayarlar, Hesaplar, Hesap ekle’yi açın ve Google Reader’ı seçin. Aşağıdaki sunucu adresini sunucu URL’si, adı kullanıcı adı ve uygulama parolasını parola olarak girin.',
				other:
					'Uygulamanızda “Google Reader API” türünde ya da yoksa “FreshRSS” türünde bir hesap ekleyin. Aşağıdaki sunucu adresini, adı ve uygulama parolasını girin.'
			},
			ownIcons: (app) =>
				`Gizlilik: ${app} akışlarınızın simgelerini akışların web sitelerinden kendisi yükler; bu siteler o sırada IP adresinizi görür. Makaleler ve resimleri yalnızca rueder üzerinden gelir.`,
			shotsTitle: 'Resimlerle adım adım',
			shots: {
				readyou: [
					'Sol üstteki dişliye dokunun.',
					'Hesaplar’ı açın.',
					'Hesap ekle’ye dokunun.',
					'Google Reader’ı seçin.',
					'Sunucu adresini sunucu URL’si, adı kullanıcı adı ve uygulama parolasını parola olarak girin, sonra Ekle’ye dokunun.'
				],
				netnewswire: [
					'Mac’te: Ayarlar’ı (⌘,), ardından Hesaplar’ı açın ve sol alttaki + düğmesine tıklayın.',
					'FreshRSS’i seçin ve Devam’a tıklayın.',
					'Adı Login’e, uygulama parolasını Password’e ve sunucu adresini API URL’ye girin, sonra Oluştur’a tıklayın.'
				]
			},
			shotAlt: (app, step) => `${app}, adım ${n(step)}`,
			server: 'Sunucu adresi',
			user: 'Ad',
			secret: 'Uygulama parolası',
			copy: (what) => `Kopyala: ${what}`,
			copied: 'Kopyalandı',
			once: 'Uygulama parolası yalnızca şimdi gösterilir. Kaybederseniz iptal edin ve yenisini oluşturun.',
			waiting: 'Uygulamanın giriş yapması bekleniyor …',
			connected: 'Bağlandı: uygulama giriş yaptı.',
			done: 'Tamam',
			refusals: {
				wrongPassword: 'Parola doğru değil.',
				codeNeeded:
					'Kimlik doğrulama uygulamanızdan bir kodu ya da bir kurtarma kodunu da girin veya geçiş anahtarıyla onaylayın.',
				wrongCode: 'Bu kod doğru değil. Kodlar her 30 saniyede değişir.',
				loginAgain:
					'Hesabınızın henüz parolası ve ikinci faktörü yok. Yeniden giriş yapın, sonra uygulama parolasını on dakika içinde oluşturun.',
				tooManyApps: 'Bir hesabın en fazla 20 uygulama parolası olabilir.',
				tooMany: 'Çok fazla yanlış deneme. Birkaç dakika sonra yeniden deneyin.',
				unreachable: 'Sunucu yanıt vermiyor. Biraz sonra yeniden deneyin.'
			}
		},
		secondFactor: {
			title: 'İkinci faktör',
			intro:
				'İkinci faktörle giriş yapmak için bir kimlik doğrulama uygulamasının kodu ya da bir geçiş anahtarı da gerekir.',
			app: 'Kimlik doğrulama uygulaması',
			on: 'Açık.',
			left: (count) => `Kalan kurtarma kodu: ${n(count)}.`,
			off: 'Kapalı.',
			byPasskeys: 'Kurulmadı. Geçiş anahtarlarınız ikinci faktörü açık tutuyor.',
			adminNeeds:
				'Bu sunucu yöneticileri yalnızca ikinci faktörle yönetici sayar. Sunucu durumunu görmek için kurun ya da koduyla yeniden giriş yapın.',
			password: 'Parolanız',
			start: 'Kur',
			scan: 'Bu kodu kimlik doğrulama uygulamanızla tarayın ya da aşağıdaki anahtarı yazın.',
			qr: 'Kimlik doğrulama uygulaması için QR kodu',
			larger: 'QR kodunu büyük göster',
			smaller: 'QR kodunu küçük göster',
			key: 'Anahtar',
			copyKey: 'Anahtarı kopyala',
			copied: 'Kopyalandı',
			code: 'Uygulamanın şu an gösterdiği kod',
			codeOrRecovery: 'Uygulamanın kodu ya da bir kurtarma kodu',
			confirm: 'Onayla',
			cancel: 'İptal',
			codesTitle: 'Kurtarma kodlarınız',
			codesIntro:
				'Her biri telefonunuz olmadan bir kez giriş yapmanızı sağlar. Güvenle saklayın: yalnızca şimdi gösterilir.',
			copyCodes: 'Kodları kopyala',
			codesDone: 'Sakladım',
			switchOff: 'Kapat',
			switchOffNote: 'Giriş için o zaman yine yalnızca parola yeter.',
			switchOffNoteKeys: 'Geçiş anahtarlarınız ikinci faktörü açık tutuyor.',
			startWithPasskey: 'Geçiş anahtarıyla kur',
			switchOffWithPasskey: 'Geçiş anahtarıyla kapat',
			refusals: {
				wrongPassword: 'Parola doğru değil.',
				codeNeeded: 'Geçiş anahtarlarınız ikinci faktörü açık tutuyor. Bir kurtarma kodu da girin.',
				wrongCode: 'Bu kod doğru değil. Kodlar her 30 saniyede değişir.',
				loginAgain:
					'Hesabınızın henüz parolası ve ikinci faktörü yok. Yeniden giriş yapın, sonra uygulamayı on dakika içinde kurun.',
				tooMany: 'Çok fazla yanlış deneme. Birkaç dakika sonra yeniden deneyin.',
				unreachable: 'Sunucu yanıt vermiyor. Biraz sonra yeniden deneyin.'
			}
		},
		passkeys: {
			title: 'Geçiş anahtarları',
			intro:
				'Bir geçiş anahtarı, cihazınızın istediği PIN, parmak izi ya da yüzle, ad ve parola olmadan giriş yapmanızı sağlar. İkinci faktör sayılır.',
			once: 'Bir geçiş anahtarıyla parolanız tek başına artık hesabınızı açmaz.',
			none: 'Henüz geçiş anahtarı yok.',
			made: (when) => `eklenme: ${when}`,
			used: (when) => `son kullanım: ${when}`,
			unused: 'henüz kullanılmadı',
			add: 'Geçiş anahtarı ekle',
			name: 'Ad, örneğin cihazın adı',
			defaultName: 'Geçiş anahtarı',
			password: 'Parolanız',
			code: 'Kimlik doğrulama uygulamanızın kodu ya da bir kurtarma kodu',
			create: 'Geçiş anahtarı oluştur',
			working: 'Tarayıcınızın yönergelerini izleyin …',
			cancel: 'İptal',
			remove: 'Kaldır',
			removeName: (name) => `Geçiş anahtarını kaldır: ${name}`,
			confirmRemove: 'Geçiş anahtarını kaldır',
			full: (max) => `Bir hesabın en fazla ${n(max)} geçiş anahtarı olabilir.`,
			or: 'ya da',
			confirmWith: 'Geçiş anahtarıyla onayla',
			codesTitle: 'Kurtarma kodlarınız',
			codesIntro:
				'İlk geçiş anahtarınız ikinci faktörü açtı. Cihazınızı kaybederseniz her kod bir kez giriş yapmanızı sağlar. Güvenle saklayın: yalnızca şimdi gösterilir.',
			copyCodes: 'Kodları kopyala',
			copied: 'Kopyalandı',
			codesDone: 'Sakladım',
			refusals: {
				cancelled: 'Geçiş anahtarı kullanılmadan tarayıcının penceresi kapatıldı.',
				known: 'Bu cihazda hesabınız için zaten bir geçiş anahtarı var.',
				failed: 'Bu tarayıcı ya da cihaz geçiş anahtarı oluşturamadı ya da kullanamadı.',
				wrong: 'Geçiş anahtarı kabul edilmedi. Yeniden deneyin.',
				wrongPassword: 'Parola doğru değil.',
				codeNeeded:
					'Hesabınızda ikinci faktör var. Uygulamanızdan bir kodu ya da bir kurtarma kodunu da girin veya geçiş anahtarıyla onaylayın.',
				wrongCode: 'Bu kod doğru değil. Kodlar her 30 saniyede değişir.',
				tooManyPasskeys: 'Bir hesabın en fazla 10 geçiş anahtarı olabilir.',
				loginAgain:
					'Hesabınızın henüz parolası ve ikinci faktörü yok. Yeniden giriş yapın, sonra ilk geçiş anahtarınızı on dakika içinde ekleyin.',
				tooMany: 'Çok fazla yanlış deneme. Birkaç dakika sonra yeniden deneyin.',
				unreachable: 'Sunucu yanıt vermiyor. Biraz sonra yeniden deneyin.'
			}
		},
		reports: {
			send: 'Bu tarayıcıdaki hata raporlarını bu sunucuya gönder',
			intro: 'Sayfada bir şey ters gittiğinde bu sunucunun yöneticileri bundan haberdar olur.',
			what: 'Bir rapor şunları içerir:',
			holds: [
				'hatanın iletisi, adresler ve bir şeyi belirten sayılar olmadan',
				'rueder’in kendi dosyalarında nerede olduğu',
				'rueder’in sürümü ve ana sürümüyle birlikte tarayıcınız'
			],
			never:
				'Okuduklarınız, akışlarınız, etiketleriniz ya da kurallarınız asla. Raporlar 30 gün sonra silinir.',
			off: 'Bunu kapatmak gönderdiğiniz raporları siler.'
		}
	},

	rules: {
		intro:
			'Kurallar makaleleri gelir gelmez gizler ya da okundu olarak işaretler. Gizlenen makaleler okunmamış sayılmaz ve “Filtrelerin gizledikleri” altında kalır.',
		none: 'Henüz kural yok.',
		add: 'Kural ekle',
		new: 'Yeni kural',
		change: 'Kuralı değiştir',
		on: 'Açık',
		scope: 'Kapsam',
		scopeAll: 'Tüm akışlar',
		scopeFolder: (title) => `Klasör: ${title}`,
		scopeFeed: (title) => `Akış: ${title}`,
		field: 'Bakılacak yer',
		fields: { any: 'Her yer', title: 'Başlık', content: 'Metin', author: 'Yazar', tag: 'Etiket' },
		match: 'Karşılaştırma',
		matches: {
			contains: 'içerir',
			word: 'sözcük olarak içerir',
			starts: 'ile başlar',
			ends: 'ile biter',
			regex: 'düzenli ifadeye uyar',
			all: 'her makale, ne yazarsa yazsın'
		},
		pattern: 'Aranacak metin',
		action: 'Sonra',
		actions: { hide: 'makaleyi gizle', read: 'okundu olarak işaretle' },
		preview: (count, total) => `Buradaki ${n(total)} makaleden ${n(count)} tanesine şu an uyuyor.`,
		regexTip: 'Düzenli ifadeler Rust’ın okuduğu gibi okunur, büyük ve küçük harf ayrımı yapılmaz.',
		regexBuilder: 'regex101’de oluşturup deneyin',
		invalid: 'Bu geçerli bir düzenli ifade değil.',
		empty: 'Aranacak metni girin.',
		checking: 'Neleri yakalayacağına bakılıyor …',
		hits: (count) => `${n(count)} eşleşme`,
		summary: (field, match, pattern, action) => `${field}: “${pattern}” ${match} → ${action}`,
		summaryAll: (action) => `Her makale → ${action}`
	},

	keys: {
		title: 'Klavye kısayolları',
		note: 'Tuşlarınız ayarlarla birlikte saklanır. Enter, Escape ve g olduğu gibi kalır.',
		change: (what) => `Şunun tuşunu değiştir: ${what}`,
		changeShort: 'Değiştir',
		press: 'Bir tuşa basın …',
		taken: (key, other) => `${key} zaten şunun tuşu: ${other}`,
		fixed: (key) => `${key} bir kısayola verilemez.`,
		reset: 'rueder’in tuşlarına dön',
		groups: { articles: 'Makaleler', lists: 'Listeler', go: 'Git', app: 'Her yerde' },
		next: 'Sonraki makale, açık',
		previous: 'Önceki makale, açık',
		nextCursor: 'Sonraki makale, açmadan',
		previousCursor: 'Önceki makale, açmadan',
		toggle: 'Makaleyi aç ya da kapat',
		star: 'Yıldız ekle ya da kaldır',
		read: 'Okundu ya da okunmadı olarak işaretle',
		original: 'Aslını yeni sekmede aç',
		markAll: 'Tümünü okundu olarak işaretle',
		refresh: 'Listeyi yenile',
		unreadOnly: 'Yalnızca okunmamışlar ya da tüm makaleler',
		listView: 'Liste görünümü',
		expandedView: 'Geniş görünüm',
		splitView: 'Üç bölme',
		magazineView: 'Dergi',
		goHome: 'Ana sayfa',
		goAll: 'Tüm makaleler',
		goUnread: 'Okunmamış makaleler',
		goStarred: 'Yıldızlı makaleler',
		search: 'Ara',
		sidebar: 'Kenar çubuğunu göster ya da gizle',
		help: 'Bu özet',
		then: 'sonra'
	},

	labels: {
		title: 'Etiketler',
		add: 'Yeni etiket',
		name: 'Etiketin adı',
		none: 'Henüz etiket yok.',
		actions: (title) => `Etiket işlemleri: ${title}`,
		rename: 'Yeniden adlandır',
		color: 'Renk',
		colors: ['Kırmızı', 'Turuncu', 'Sarı', 'Yeşil', 'Turkuaz', 'Mavi', 'Mor', 'Gri'],
		remove: 'Etiketi sil',
		show: 'Etiketleri göster',
		hide: 'Etiketleri gizle',
		forArticle: 'Etiketler',
		set: (title) => `Etiket: ${title}`
	},

	subscribe: {
		title: 'Bir akışa abone olun',
		address: 'Akışın ya da web sitesinin adresi',
		find: 'Akış bul',
		finding: 'Aranıyor …',
		note: 'Taslak hiçbir şey getiremez, bu yüzden akış boş kalır.',
		found: (count) => (count === 1 ? 'Bir akış var:' : `${n(count)} akış var:`),
		none: 'Bu adreste akış yok.',
		movedTo: (url) => `Site isteği şu adrese yönlendirdi: ${url}`,
		loginWall:
			'Bazı siteler giriş yapmamış olanları giriş sayfalarına gönderir. O zaman akışları okunamaz.',
		subscribed: 'Abone olundu',
		show: 'Göster',
		untitled: 'Adsız',
		page: 'Akışı olmayan bir web sitesi: rueder ondan yalnızca başlıklar ve özetlerle bir akış yapar.',
		folder: 'Klasöre',
		topLevel: 'Klasör yok',
		add: 'Abone ol',
		adding: 'Abone olunuyor …',
		refused: 'Olmadı.',
		serverSays: (reason) => `Sunucunun yanıtı: ${reason}`,
		busy: (seconds) =>
			seconds
				? `Aynı anda çok fazla arama var. ${n(seconds)} saniye sonra yeniden deneyin.`
				: 'Aynı anda çok fazla arama var. Biraz sonra yeniden deneyin.',
		noFetching: 'Bu sunucu akış getirmiyor.',
		tooLarge: (megabytes) => `Dosya ${n(megabytes)} MB’tan büyük.`,
		unreachable: 'Sunucu yanıt vermiyor. Biraz sonra yeniden deneyin.'
	},

	opml: {
		title: 'İçe ve dışa aktar',
		intro:
			'Aboneliklerinizi başka bir akış okuyucusundan OPML dosyası olarak getirin ya da yanınızda götürün.',
		import: 'OPML dosyası içe aktar',
		importing: 'İçe aktarılıyor …',
		imported: (feeds, folders) => `${n(feeds)} yeni akış ve ${n(folders)} yeni klasör.`,
		known: (count) => `${n(count)} akışa zaten aboneydiniz.`,
		skipped: (count) => `Okunabilir akış olmayan ${n(count)} kayıt atlandı.`,
		later: 'Makaleleri bir sonraki getirme turunda gelir.',
		export: 'OPML olarak dışa aktar'
	},

	removeAccount: {
		title: 'Hesabı sil',
		intro: 'Bu, bu rueder’deki hesabınızı kalıcı olarak siler. Geri alınamaz.',
		goes: 'Hesapla birlikte gidenler: tüm cihazlardaki oturumlarınız, abonelikleriniz ve klasörleriniz, okuduklarınız, yıldızlar, etiketler, filtre kuralları, ayarlar, hata raporları, davetler ve tek oturum açma bağlantısı.',
		stays:
			'Akışlar onları okuyan diğerleri için kalır. Hesabınızı önce başka bir yere götürmek için “Taşıma”yı kullanın.',
		password: 'Parolanız',
		name: (name) => `Onaylamak için adınızı yazın: ${name}`,
		code: 'Uygulamanızın kodu ya da bir kurtarma kodu',
		submit: 'Hesabımı kalıcı olarak sil',
		wrong: 'Bu doğru değil. Hiçbir şey silinmedi.',
		codeNeeded:
			'Hesabınızda ikinci faktör var. Uygulamanızdan bir kodu ya da bir kurtarma kodunu da yazın veya geçiş anahtarıyla onaylayın.',
		busy: (count) => `Çok fazla yanlış deneme. ${minutes(count)} sonra yeniden deneyin.`,
		unreachable: 'Sunucu yanıt vermiyor. Hiçbir şey silinmedi.'
	},

	move: {
		intro: 'Hesabınızı tek bir dosyayla başka bir rueder’e götürün ya da oradan buraya getirin.',
		howItWorks:
			'İçe aktarma buradakilere ekler, hiçbir şey çıkarmaz. Ayarlarınız dosyadaki ayarlar olur.',
		notInIt: 'Dosyada olmayanlar: parolanız, oturumlarınız ve makalelerin metinleri.',
		export: 'Hesabı dışa aktar',
		import: 'Hesap dosyası içe aktar',
		importing: 'İçe aktarılıyor …',
		feeds: (feeds, known) => `${n(feeds)} yeni akış, ${n(known)} akışa zaten abone.`,
		folders: (count) => `${n(count)} yeni klasör.`,
		articles: (found) => `Burada bulunan: ${articles(found)}.`,
		missing: (count) =>
			`Burada bulunamayan: ${articles(count)}; bunlardaki okuma durumu, yıldız ve etiketler kayboldu.`,
		marks: (labels, rules) => `${n(labels)} yeni etiket ve ${n(rules)} yeni filtre kuralı.`,
		skipped: (count) => `Dosyadaki ${n(count)} kayıt alınamadı.`,
		settings: 'Ayarlarınız artık dosyadaki ayarlar.',
		later:
			'Yeni akışlar makalelerini bir sonraki getirme turunda getirir. Okuduklarınız okunmuş kalır.',
		busy: (seconds) =>
			seconds
				? `Çok fazla içe aktarma. ${minutes(Math.ceil(seconds / 60))} sonra yeniden deneyin.`
				: 'Çok fazla içe aktarma. Daha sonra yeniden deneyin.'
	},

	status: {
		title: 'Sunucu durumu',
		open: 'Sunucu durumu',
		loading: 'Sunucuya soruluyor …',
		failed: 'Sunucu yanıt vermedi. Biraz sonra yeniden deneyin.',
		refresh: 'Yenile',
		asOf: (time) => `Saat: ${time}. Sayılan, son bir saattir.`,
		demo: 'Burada sunucu olmadığı için uydurma.',
		empty: '–',
		on: 'Açık',
		off: 'Kapalı',
		took: (median, p95) => `ortanca ${median}, %95’i ${p95} içinde`,
		summary: {
			label: 'Bir bakışta',
			hints: 'Dikkat',
			failing: 'Başarısız akışlar',
			errors: 'Kaydedilen hatalar',
			reported: 'Bildirilen akışlar'
		},
		hintsTitle: 'Bakmaya değer',
		allWell: 'Göze çarpan bir şey yok.',
		hints: {
			slowRoute: (route, took) =>
				`Yavaş rota: ${route}. Yirmi istekten biri ${took} süreden uzun sürüyor.`,
			serverErrors: (route, count) =>
				`Sunucu hatasıyla yanıt veren rota: ${route}, ${n(count)} kez.`,
			poolBusy: (size) =>
				`Veritabanına giden ${n(size)} bağlantının hepsi kullanımda, bu yüzden istekler bekliyor.`,
			fetchBacklog: (count, late) =>
				`${n(count)} akış geç getiriliyor; en geç kalanın gecikmesi: ${late}.`,
			feedsFailing: (failing, subscribed) =>
				`${n(subscribed)} akıştan ${n(failing)} tanesi başarısız.`,
			notFetching: 'Sunucu akış getirmiyor. FETCH off mu?',
			fetchPaceFull: (most) =>
				`Her tur alabildiği kadar akış alıyor, yine de bekleyen akışlar var: tempo saatte en fazla ${n(most)} getirmeye izin veriyor. Bunu FETCH_ROUND, FETCH_AT_ONCE ve FETCH_EVERY belirler.`,
			cacheMisses: (share) =>
				`Veritabanı okuduklarının yalnızca ${share} kadarını bellekte buluyor; daha fazla shared_buffers işe yarar.`,
			databaseLarge: (size, warn) =>
				`Veritabanı ${size} boyutuna ulaştı; DATABASE_WARN_GB ${warn} itibarıyla uyarır. “rueder retention set” eski makaleleri ya da metinlerini siler.`,
			longQuery: (took) => `Bir sorgu ${took} süredir çalışıyor.`,
			imageCacheFull: (share) => `Resim önbelleğinin doluluğu: ${share}.`,
			errors: (count) => `Sunucu son bir saatte ${n(count)} hata kaydetti.`,
			limited: (count) => `${n(count)} istek çok fazla olduğu için geri çevrildi.`,
			browserErrors: (count) => `Tarayıcılar son bir günde ${n(count)} farklı hata bildirdi.`,
			reportedFeeds: (count) => `Okurların bildirdiği ve engellenmemiş ${n(count)} akış var.`
		},
		build: {
			title: 'Derleme',
			commit: 'Commit',
			dirty: (commit) => `${commit}, kendi değişiklikleriyle`,
			branch: 'Dal',
			tag: 'Etiket',
			committed: 'Commit zamanı',
			where: 'Çalıştığı yer',
			container: 'bir kapsayıcıda',
			host: 'doğrudan makinede',
			target: 'Hedef',
			profile: 'Derleme profili',
			profileValue: (profile, level) => `${profile}, optimizasyon ${level}`,
			ci: 'Derleyen',
			ciValue: 'bir CI çalışması',
			byHand: 'elle'
		},
		server: {
			title: 'Sunucu',
			version: 'Sürüm',
			running: 'Çalışma başlangıcı',
			memory: 'Bellek',
			accounts: 'Hesaplar',
			feeds: 'Akışlar',
			feedsValue: (fetched, all) => `${n(fetched)} getiriliyor, toplam ${n(all)}`,
			articles: 'Makaleler',
			articlesValue: (count) => `yaklaşık ${n(count)}`,
			live: 'Açık sekmeler'
		},
		database: {
			title: 'Veritabanı',
			pool: 'rueder’in bağlantıları',
			poolValue: (size, idle, max) => `${n(size)} açık, ${n(idle)} boşta, en fazla ${n(max)}`,
			connections: 'Tüm bağlantılar',
			size: 'Boyut',
			sizeWarn: (size, warn) => `${size}, uyarı sınırı ${warn}`,
			hitRatio: 'Bellekten okunan',
			deadlocks: 'Kilitlenmeler',
			longest: 'En uzun süren sorgu',
			tables: 'En büyük tablolar',
			table: 'Tablo',
			bytes: 'Boyut',
			rows: 'Satır, yaklaşık'
		},
		fetcher: {
			title: 'Akışların getirilmesi',
			state: 'Getirme',
			lastRound: 'Son tur',
			lastRoundValue: (ago, feeds, took) => `${ago}: ${n(feeds)} akış, süre ${took}`,
			fetches: 'Getirmeler',
			fetchesValue: (fetched, unchanged, failed) =>
				`${n(fetched)} akışı getirdi, ${n(unchanged)} değişmedi, ${n(failed)} başarısız`,
			broken: 'Veritabanında başarısız',
			newArticles: 'Yeni makaleler',
			took: 'Getirme başına süre',
			overdue: 'Geç getirilen',
			overdueValue: (count, late) => `${n(count)}, en geç kalanın gecikmesi ${late}`,
			waiting: 'Henüz getirilmeyen',
			pace: 'Tempo',
			paceValue: (round, every, atOnce, most) =>
				`her ${every} için ${n(round)} akış, aynı anda ${n(atOnce)}: saatte en fazla ${n(most)}`,
			perHour: 'Son bir saatteki getirmeler',
			behind: 'Gecikme',
			behindValue: (late) => `en uzun bekleyen akışın gecikmesi: ${late}`,
			notBehind: 'Bekleyen bir şey yok',
			failing: 'Başarısız akışlar, nedene göre',
			noneFailing: 'Başarısız akış yok.',
			feeds: (count) => `${n(count)} akış`,
			failures: {
				status: (code) => (code ? `Sitenin yanıtı: ${code}` : 'Site hatayla yanıt veriyor'),
				timeout: 'Site çok uzun sürüyor',
				blocked: 'Adres, rueder’in getirmediği bir ağda',
				dns: 'Adın adresi yok',
				network: 'Bağlantı yok ya da koptu',
				too_large: 'Çok büyük',
				redirects: 'Çok fazla yönlendirme ya da bozuk bir yönlendirme',
				unreadable: 'rueder’in okuyabildiği bir akış değil',
				other: 'Başka bir şey'
			}
		},
		finder: {
			title: 'Akış arama',
			state: 'Adreslere bakma',
			lookups: 'Aramalar',
			lookupsValue: (found, none, failed) =>
				`${n(found)} akış buldu, ${n(none)} bulamadı, ${n(failed)} başarısız`,
			limited: 'Hesap sınırının üstünde',
			took: 'Arama başına süre'
		},
		rssifier: {
			title: 'Akışı olmayan web siteleri',
			state: 'RSSifier',
			pages: 'Okunan web siteleri',
			pagesValue: (pages, failing) => `${n(pages)}, bunlardan ${n(failing)} tanesi hatalı`,
			used: 'Belirtecin kullanımı',
			usedValue: (requests, fetches, underWay) =>
				`son dakikada ${n(requests)} istek, son saatte ${n(fetches)} sayfa alındı, ${n(underWay)} sürüyor`,
			unanswered: (why) => `RSSifier yanıt vermiyor: ${why}`,
			limits: 'Belirtecin sınırları',
			limitsValue: (perMinute, perHour, atOnce) =>
				`dakikada ${perMinute}, saatte ${perHour} sayfa, aynı anda ${atOnce}`,
			noLimit: 'sınırsız'
		},
		images: {
			title: 'Resim proxy’si',
			state: 'Proxy',
			cache: 'Önbellek',
			cacheValue: (used, max) => `${max} içinde ${used}`,
			served: 'Resimler',
			servedValue: (cached, fetched) => `${n(cached)} önbellekten, ${n(fetched)} getirildi`,
			refused: 'Reddedilen',
			refusedValue: (source, notImage, tooLarge, notFound) =>
				`${n(source)} kaynak hatası, ${n(notImage)} resim değil, ${n(tooLarge)} çok büyük, ${n(notFound)} yanlış adres`,
			took: 'Getirilen resim başına süre',
			failingSources: 'Şimdilik rahat bırakılan kaynaklar'
		},
		requests: {
			title: 'İstekler',
			all: 'Tüm istekler',
			allValue: (count, took) => `${n(count)}, ${took}`,
			answers: 'Yanıtlar',
			answersValue: (fine, refused, errors) =>
				`${n(fine)} sorunsuz, ${n(refused)} reddedildi, ${n(errors)} sunucu hatası`,
			route: 'Rota',
			count: 'Sayı',
			median: 'Ortanca',
			p95: '%95',
			max: 'En uzun',
			errors: 'Hatalar',
			none: 'Son bir saatte istek yok.'
		},
		problems: {
			title: 'Uyarılar ve hatalar',
			counts: (errors, warnings) => `Son bir saatte ${n(errors)} hata ve ${n(warnings)} uyarı.`,
			none: 'Başlangıçtan beri hiçbir şey kaydedilmedi.',
			error: 'Hata',
			warning: 'Uyarı',
			times: (count, last) => (count > 1 ? `${n(count)} kez, son: ${last}` : last)
		},
		browsers: {
			title: 'Tarayıcılardaki hatalar',
			off: 'Bu sunucu hata raporu almıyor (CLIENT_REPORTS=off).',
			consenting: (count) =>
				`${n(count)} hesap rapor gönderiyor. Hatalar son görüldükleri andan itibaren 30 gün kalır.`,
			none: 'Hiçbir tarayıcı hata bildirmedi.',
			kinds: {
				error: 'Hata',
				rejection: 'Başarısız promise',
				component: 'Sayfanın bir parçası',
				request: 'İstek'
			},
			hits: (count, accounts, last) => `${n(accounts)} hesapta ${n(count)} kez, son: ${last}`,
			where: (versions, browsers) => `Sürüm ${versions} · ${browsers}`
		},
		reported: {
			title: 'Bildirilen akışlar',
			text: 'Kimin bildirdiği burada gösterilmez. “rueder feed block <adres>” bir akışı herkes için engeller, “rueder feed unblock <adres>” engeli kaldırır.',
			none: 'Kimse akış bildirmedi.',
			reports: (count, last) => `${n(count)} bildirim, son: ${last}`,
			blocked: 'Engellendi'
		}
	},

	update: {
		ready: 'rueder’in yeni bir sürümü hazır.',
		reload: 'Yeniden yükle',
		later: 'Sonra'
	},

	crash: {
		title: 'Bu sayfada bir şeyler ters gitti.',
		text: 'Devam etmek için sayfayı yeniden yükleyin. Okuduklarınız ve değiştirdikleriniz sunucuda kayıtlı.',
		reload: 'Yeniden yükle'
	},

	demo: {
		badge: 'Taslak',
		note: 'Uydurma akışlarla bir taslak. Sunucuya hiçbir şey kaydedilmez.',
		arrive: 'Bir makale gelsin',
		revise: 'Açık makaleyi değiştir',
		revised: 'Güncelleme: bu paragraf sürümleri denemek için eklendi.',
		reviseHint: 'Önce bir makale açın.'
	}
};

export default tr;
