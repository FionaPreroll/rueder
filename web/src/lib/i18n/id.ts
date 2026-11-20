// The Indonesian texts, in the shape of the English catalog (en.ts). They address
// the reader as “Anda”. Indonesian has no plural, so a count stands before the
// word as it is.

import type { Messages } from './en.ts';
import { formatNumber as n } from './intl.ts';

const articles = (count: number) => `${n(count)} artikel`;

const minutes = (count: number) => `${n(count)} menit`;

const id: Messages = {
	languageName: 'Bahasa Indonesia',

	articles,

	common: {
		close: 'Tutup',
		cancel: 'Batal',
		save: 'Simpan',
		delete: 'Hapus',
		edit: 'Edit'
	},

	nav: {
		subscribe: 'Berlangganan',
		home: 'Beranda',
		all: 'Semua artikel',
		unreadItems: 'Belum dibaca',
		starred: 'Berbintang',
		hidden: 'Disembunyikan filter',
		subscriptions: 'Langganan',
		newFolder: 'Folder baru',
		groupedFolder: 'Folder baru',
		folderName: 'Nama folder',
		renameFolder: 'Ganti nama',
		removeFolder: 'Hapus folder, pertahankan isinya',
		folderActions: (title) => `Tindakan untuk folder ${title}`,
		feedName: 'Nama feed',
		feedNameHint: 'Kosongkan untuk memakai nama feed itu sendiri',
		renameFeed: 'Ganti nama',
		unsubscribe: 'Berhenti berlangganan',
		reportFeed: 'Laporkan feed …',
		feedInfo: 'Info feed …',
		feedActions: (title) => `Tindakan untuk feed ${title}`,
		unsubscribeTitle: 'Berhenti berlangganan?',
		unsubscribeText: (title) =>
			`Berhenti berlangganan “${title}”? Catatan baca dan bintang Anda di sana akan hilang.`,
		openFolder: 'Buka folder',
		closeFolder: 'Tutup folder',
		openSidebar: 'Tampilkan bilah samping',
		closeSidebar: 'Sembunyikan bilah samping',
		search: 'Cari',
		searchPlaceholder: 'Cari di artikel ini',
		clearSearch: 'Hapus pencarian',
		settings: 'Pengaturan',
		accountSettings: 'Pengaturan akun',
		shortcuts: 'Pintasan keyboard',
		live: 'Langsung',
		liveOn: 'Artikel baru datang dengan sendirinya. Klik untuk menjeda.',
		liveOff: 'Dijeda. Klik agar artikel baru datang lagi.',
		paused: 'Dijeda',
		account: (name) => `Masuk sebagai ${name}`,
		logout: 'Keluar',
		appSettings: 'Server dan pembaruan aplikasi',
		install: 'Pasang sebagai aplikasi',
		installSafari:
			'Untuk memasang rueder sebagai aplikasi: File › Tambahkan ke Dock di Mac, Bagikan › Tambahkan ke Layar Utama di iPhone atau iPad.',
		unread: (count) => `${n(count)} belum dibaca`,
		hiddenCount: (count) => `${n(count)} disembunyikan filter`,
		fetchFailed: (reason) => `Pengambilan terakhir gagal: ${reason}`,
		waiting: 'Belum diambil',
		blocked: 'Diblokir oleh pengelola rueder ini',
		moveHint: 'Seret untuk mengurutkan, atau tekan Alt dengan tombol panah',
		resizeSidebar: 'Lebar bilah samping: seret, atau gunakan tombol panah',
		tooDeep: (levels) => `Folder bisa bertingkat paling banyak ${n(levels)} lapis.`
	},

	toolbar: {
		refresh: 'Segarkan',
		showAll: 'Semua artikel',
		showUnread: 'Hanya yang belum dibaca',
		show: 'Tampilkan',
		newestFirst: 'Terbaru dahulu',
		oldestFirst: 'Terlama dahulu',
		markAllRead: 'Tandai semua sudah dibaca',
		markOptions: 'Cara lain untuk menandai sudah dibaca',
		phoneMenu: (shown) => `Tampilan, kini: ${shown}`,
		markMenu: 'Tandai sudah dibaca …',
		markEverything: 'Semua artikel',
		markOlderDay: 'Artikel yang lebih dari sehari',
		markOlderWeek: 'Artikel yang lebih dari seminggu',
		listView: 'Tampilan daftar',
		expandedView: 'Tampilan diperluas',
		splitView: 'Tiga panel: pohon, daftar, dan artikel',
		magazineView: 'Majalah',
		previous: 'Artikel sebelumnya',
		next: 'Artikel berikutnya',
		newArticles: (count) => `${n(count)} artikel baru`
	},

	list: {
		empty: 'Tidak ada artikel di sini.',
		emptyUnread: 'Tidak ada yang belum dibaca di sini.',
		emptySearch: 'Tidak ada artikel yang cocok dengan pencarian.',
		found: (count, more, words) => `${n(count)}${more ? '+' : ''} artikel dengan “${words}”`,
		searchAll: 'Cari di semua feed',
		feedFailing:
			'rueder akan mencoba lagi nanti, dan menunggu lebih lama setelah setiap kegagalan atau selama yang diminta situs. Jika terus gagal, feed mungkin sudah pindah.',
		hiddenNote:
			'Aturan filter Anda menyembunyikan artikel ini. Buka salah satunya untuk melihat aturan mana.',
		toRules: 'Aturan filter',
		emptyFeed: 'Feed ini belum punya artikel.',
		waitingFeed:
			'Feed ini belum diambil. Artikelnya datang pada putaran pengambilan berikutnya, dalam beberapa menit.',
		showAll: 'Tampilkan semua artikel',
		more: 'Tampilkan lebih banyak',
		star: 'Beri bintang',
		unstar: 'Hapus bintang',
		keepUnread: 'Biarkan belum dibaca',
		openOriginal: 'Buka aslinya',
		showFeed: 'Tampilkan semua artikel feed ini di rueder',
		from: 'dari',
		by: (author) => `oleh ${author}`,
		updated: 'diperbarui',
		updatedHeadline: 'judul baru',
		updatedTitle: (count) => `Berubah sejak pertama kali diambil; ${n(count)} versi sebelumnya`,
		updatedHeadlineTitle: (count) =>
			`Judulnya ditulis ulang, teksnya tetap; ${n(count)} versi sebelumnya`,
		compare: 'Bandingkan versi',
		hiddenBy: (rule) => `Disembunyikan oleh aturan: ${rule}`,
		textDropped: 'Server ini tidak lagi menyimpan teks artikel ini karena sudah lama.',
		textDroppedOriginal: 'Baca di sumbernya',
		loading: 'Memuat teks …',
		loadingList: 'Memuat …',
		toTop: 'Kembali ke atas',
		pick: 'Pilih artikel untuk membacanya di sini.',
		resizeList: 'Lebar daftar: seret, atau gunakan tombol panah',
		closeArticle: 'Tutup artikel',
		blockedFeed:
			'Pengelola rueder ini memblokir feed ini. Feed ini tidak diambil, dan artikelnya tidak ditampilkan.',
		articleActions: 'Tindakan lain untuk artikel',
		articleInfo: 'Info artikel …',
		articleSource: 'Kode sumber …',
		reportArticle: 'Laporkan artikel …'
	},

	report: {
		feedTitle: 'Laporkan feed',
		articleTitle: 'Laporkan artikel',
		text: (title) =>
			`Pengelola rueder ini melihat bahwa “${title}” dilaporkan, alasannya, dan apa yang Anda tulis, tetapi tidak melihat siapa yang melaporkannya. Pengelola memutuskan apakah akan memblokirnya untuk semua orang.`,
		reason: 'Alasan',
		reasons: {
			illegal: 'Melanggar hukum',
			copyright: 'Karya orang lain',
			malicious: 'Malware, phishing, atau penipuan',
			spam: 'Spam',
			other: 'Lainnya'
		},
		note: 'Yang ingin Anda tambahkan (opsional)',
		left: (count) => `Tersisa ${n(count)} karakter`,
		send: 'Kirim laporan',
		sending: 'Mengirim …',
		sent: 'Terima kasih. Laporan telah sampai ke pengelola rueder ini.',
		tooMany: 'Anda baru saja mengirim banyak laporan. Coba lagi nanti.',
		unreachable: 'Laporan tidak sampai ke server. Coba lagi nanti.'
	},

	feedInfo: {
		title: 'Info feed',
		text: (title) =>
			`Yang diketahui rueder ini tentang “${title}” dan cara mengambilnya. Jika feed tidak berfungsi, salin ini ke pesan Anda untuk pengelola.`,
		number: 'Nomor',
		name: 'Nama',
		address: 'Alamat feed',
		site: 'Situs web',
		noSite: 'Tidak ada',
		agent: 'Diambil sebagai',
		agents: { rueder: 'rueder', browser: 'Browser' },
		every: 'Diambil setiap',
		fetched: 'Pengambilan terakhir',
		succeeded: 'Pengambilan terakhir yang berhasil',
		failed: 'Kesalahan terakhir',
		next: 'Pengambilan berikutnya',
		blocked: 'Tidak ada, pengelola memblokir feed ini',
		articles: 'Artikel sejauh ini',
		never: 'Belum pernah',
		fetchAs: 'Ambil sebagai',
		fetchAsHint:
			'Berlaku untuk semua yang membaca feed ini. Pilih browser untuk situs yang menolak rueder. Feed langsung diambil lagi.',
		notSwitched: 'Tidak berhasil. Coba lagi nanti.',
		copy: 'Salin',
		copied: 'Disalin',
		loading: 'Memuat …',
		unreachable: 'Server tidak menjawab. Coba lagi nanti.'
	},
	articleInfo: {
		title: 'Info artikel',
		text: (title) =>
			`Yang diketahui rueder ini tentang “${title}” dan versinya. Jika ada yang tampak salah, salin ini ke pesan Anda untuk pengelola.`,
		feed: 'Feed',
		feedNumber: 'Nomor feed',
		number: 'Nomor di feed',
		name: 'Judul',
		id: 'ID di feed',
		link: 'Tautan',
		noLink: 'Tidak ada',
		published: 'Diterbitkan',
		fetched: 'Pertama diambil',
		changed: 'Terakhir diubah',
		never: 'Belum pernah',
		changedAt: (when, what) => `${when} (${what})`,
		what: { title: 'judul', text: 'teks', both: 'judul dan teks' },
		version: 'Versi',
		kept: 'Versi yang disimpan',
		textLabel: 'Teks',
		textDropped: 'Tidak disimpan lagi karena sudah lama',
		copy: 'Salin',
		copied: 'Disalin',
		loading: 'Memuat …',
		unreachable: 'Server tidak menjawab. Coba lagi nanti.'
	},
	articleSource: {
		title: 'Kode sumber artikel',
		text: 'Artikel sebagai HTML, seperti yang disimpan rueder ini setelah dibersihkan, atau seperti yang dikirim feed. Tautan dan gambar tetap memakai alamatnya sendiri di sini, jadi parameter pelacakan yang tersisa akan terlihat.',
		show: 'Tampilkan',
		kinds: { cleaned: 'Sudah dibersihkan', original: 'Seperti yang dikirim feed' },
		noText: 'Server ini tidak lagi menyimpan teks artikel ini.',
		noOriginal: 'Server ini tidak lagi menyimpan yang dikirim feed.',
		copy: 'Salin',
		copied: 'Disalin',
		loading: 'Memuat …',
		unreachable: 'Server tidak menjawab. Coba lagi nanti.'
	},

	media: {
		title: 'Lampiran',
		play: 'Putar'
	},

	connection: {
		reconnecting: 'Menyambung ulang …',
		unreachable: (since) => `Server tidak menjawab sejak ${since}.`,
		offline: (since) => `Tidak ada jaringan sejak ${since}.`,
		retry: 'Coba lagi sekarang',
		unsent: (count) => `${n(count)} perubahan sedang menunggu.`
	},

	notice: {
		unreachable: 'Server tidak menjawab. Yang Anda lihat mungkin tidak terbaru.',
		notSaved: 'Server tidak menerima perubahan terakhir Anda. Perubahan itu dibatalkan di sini.',
		tooManyUnsent: 'Terlalu banyak perubahan menunggu server, jadi yang ini ditarik kembali.'
	},

	login: {
		title: 'Masuk ke rueder',
		about:
			'Pembaca feed: rueder mengambil yang diterbitkan situs yang Anda ikuti dan menampilkan yang belum Anda baca.',
		name: 'Nama',
		password: 'Kata sandi',
		submit: 'Masuk',
		checking: 'Sebentar …',
		wrong: 'Nama atau kata sandi salah.',
		codeNeeded:
			'Akun ini memakai verifikasi dua langkah. Masukkan kode yang ditampilkan aplikasi Anda atau kode pemulihan, atau masuk dengan kunci sandi.',
		wrongCode: 'Kode itu tidak benar. Kode berganti setiap 30 detik.',
		code: 'Kode',
		codeNote: 'Enam angka dari aplikasi autentikator Anda, atau salah satu kode pemulihan Anda.',
		tooMany: (count) => `Terlalu banyak percobaan yang salah. Coba lagi dalam ${minutes(count)}.`,
		unreachable: 'Server tidak menjawab. Coba lagi sebentar lagi.',
		retry: 'Coba lagi',
		or: 'atau',
		oidc: 'Masuk dengan sistem masuk tunggal',
		passkey: 'Masuk dengan kunci sandi',
		passkeyCancelled:
			'Tidak ada kunci sandi yang dipakai. Coba lagi, atau masuk dengan nama dan kata sandi.',
		passkeyFailed: 'Browser atau perangkat ini tidak bisa memakai kunci sandi di sini.',
		passkeyWrong: 'Kunci sandi ini bukan milik akun mana pun di sini.',
		oidcFailed: 'Masuk melalui sistem masuk tunggal tidak berhasil. Coba lagi.',
		oidcDenied: 'Masuk melalui sistem masuk tunggal dibatalkan.',
		oidcUnknown:
			'Tidak ada akun di sini untuk login itu. Minta pengelola rueder ini untuk menautkannya.',
		disabled: 'Akun ini dinonaktifkan. Tanyakan alasannya kepada pengelola rueder ini.',
		unconfirmed:
			'Akun ini menunggu alamat emailnya dikonfirmasi. Buka tautan di email, atau minta dikirim lagi.',
		resend: 'Kirim tautan lagi',
		resent: 'Tautan sedang dikirim lagi. Periksa email Anda, juga folder spam.',
		resendWrong: 'Masukkan nama dan kata sandi akun agar tautan dikirim lagi.',
		forgot: 'Lupa kata sandi?',
		removed: 'Akun Anda telah dihapus, bersama semua milik Anda.',
		note: 'Tidak ada pendaftaran di sini. Akun dibuat oleh pengelola rueder ini.'
	},

	legal: {
		imprint: 'Informasi hukum',
		privacy: 'Kebijakan privasi',
		terms: 'Ketentuan penggunaan',
		agree: { before: 'Saya menyetujui ', link: 'ketentuan penggunaan', after: '.' },
		changed: 'Ketentuan penggunaan telah berubah',
		changedIntro:
			'Silakan baca ketentuan penggunaan yang baru. Untuk terus memakai rueder, setujui ketentuan itu.',
		read: 'Baca ketentuan penggunaan',
		accept: 'Saya setuju',
		decline: 'Keluar',
		notSaved: 'Tidak berhasil. Coba lagi sebentar lagi.'
	},

	register: {
		title: 'Buat akun',
		invited: 'Anda diundang ke rueder ini. Pilih nama dan kata sandi.',
		again: 'Kata sandi sekali lagi',
		passwordRule: (characters) => `Minimal ${n(characters)} karakter.`,
		mismatch: 'Kedua kata sandi berbeda.',
		submit: 'Buat akun',
		solving: 'Memeriksa browser ini …',
		toRegister: 'Buat akun',
		toLogin: 'Masuk ke akun yang sudah ada',
		closed: 'Tidak ada yang bisa mendaftar di sini tanpa undangan.',
		inviteGone: 'Undangan ini tidak bisa dipakai lagi. Minta yang baru.',
		full: 'Tidak ada tempat untuk akun lain di sini.',
		today: 'Sudah cukup banyak akun dibuat di sini hari ini. Coba lagi besok.',
		captcha: 'Pemeriksaan browser ini tidak lolos. Coba lagi.',
		taken: 'Sudah ada akun dengan nama ini.',
		terms: 'Setujui ketentuan penggunaan untuk membuat akun.',
		invalid: (reason) => `Akun itu tidak bisa dibuat. Kata server: ${reason}`,
		email: 'Alamat email',
		emailOptional: 'Alamat email (opsional)',
		emailNote: 'Anda akan menerima tautan untuk mengonfirmasinya.',
		emailRequiredNote:
			'Anda akan menerima tautan untuk mengonfirmasinya; akun bisa masuk setelah Anda membukanya.',
		emailInvalid: 'Masukkan alamat email yang bisa menerima email.',
		unsent: 'Tautan tidak bisa dikirim, jadi akun tidak dibuat. Coba lagi nanti.',
		tooMany: (count) => `Terlalu banyak percobaan. Coba lagi dalam ${minutes(count)}.`,
		awaitingTitle: 'Periksa email Anda',
		awaiting: (address, hours) =>
			`Akun sudah dibuat. Tautan telah dikirim ke ${address}: buka dalam ${n(hours)} jam, lalu masuk.`
	},

	forgot: {
		title: 'Lupa kata sandi?',
		intro:
			'Masukkan nama atau alamat email Anda. Tautan untuk mengatur kata sandi baru dikirim ke alamat yang sudah dikonfirmasi.',
		field: 'Nama atau alamat email',
		submit: 'Kirim tautan',
		asked: (count) =>
			`Jika ada akun seperti itu dengan alamat yang sudah dikonfirmasi, tautan sedang dikirim ke sana, berlaku ${minutes(count)}. Periksa juga folder spam.`,
		tooMany: (count) => `Terlalu sering diminta. Coba lagi dalam ${minutes(count)}.`,
		back: 'Kembali ke halaman masuk'
	},

	reset: {
		title: 'Atur kata sandi baru',
		intro: 'Akun akan dikeluarkan dari semua browser dan perangkat.',
		fresh: 'Kata sandi baru',
		again: 'Kata sandi baru sekali lagi',
		submit: 'Atur kata sandi',
		mismatch: 'Keduanya tidak sama.',
		gone: 'Tautan ini tidak berlaku lagi. Minta yang baru.',
		invalid: (reason) => `Server tidak menerima kata sandi itu. Katanya: ${reason}`,
		done: 'Kata sandi sudah diatur. Masuklah dengan kata sandi itu.',
		cancel: 'Batal'
	},

	email: {
		title: 'Alamat email',
		intro: 'rueder hanya mengirim email kepada Anda tentang akun Anda.',
		current: 'Alamat Anda',
		none: 'Akun ini tidak punya alamat.',
		pending: (address, hours) =>
			`Tautan telah dikirim ke ${address}. Alamat baru berlaku setelah tautan dibuka, dalam ${n(hours)} jam; sampai saat itu alamat lama tetap dipakai.`,
		address: 'Alamat baru',
		password: 'Kata sandi Anda',
		passwordNote: 'Agar orang lain yang memakai browser ini tidak mengalihkan email Anda.',
		code: 'Kode verifikasi dua langkah',
		submit: 'Kirim tautan',
		sent: 'Tautan sedang dikirim. Periksa email Anda, juga folder spam.',
		remove: 'Hapus alamat',
		removeIntro:
			'Tanpa alamat, tidak ada email yang dikirim kepada Anda. Selain itu, akun tetap seperti semula.',
		removed: 'Alamat telah dihapus.',
		wrong: 'Kata sandi atau kodenya tidak benar.',
		codeNeeded: 'Akun ini memakai verifikasi dua langkah. Masukkan juga kodenya.',
		loginAgain:
			'Akun Anda belum punya kata sandi maupun verifikasi dua langkah. Masuk lagi, lalu isi alamat dalam sepuluh menit.',
		invalid: 'Masukkan alamat email yang bisa menerima email.',
		same: 'Itu memang alamat akun ini.',
		required:
			'Setiap akun di rueder ini memerlukan alamat; alamat bisa diubah, tetapi tidak dihapus.',
		busy: (count) => `Terlalu banyak percobaan atau email. Coba lagi dalam ${minutes(count)}.`,
		unsent: 'Email tidak bisa dikirim. Coba lagi nanti.',
		unreachable: 'Server tidak menjawab. Tidak ada yang diubah.',
		askTitle: 'Tambahkan alamat email',
		askIntro:
			'Setiap akun di rueder ini kini memerlukan alamat email. Masukkan alamat Anda, lalu buka tautan yang dikirim ke sana.',
		askWaiting:
			'Setelah Anda membuka tautan, pesan ini hilang dengan sendirinya. Tidak ada yang datang? Kirim lagi, atau ke alamat lain.',
		logOut: 'Keluar',
		confirmed: (address) => `Alamat ${address} telah dikonfirmasi.`,
		confirmedLogIn: 'Akun sudah siap: silakan masuk.',
		gone: 'Tautan ini tidak bisa dipakai lagi: sudah dipakai, tautan yang lebih baru telah dikirim, atau umurnya sudah lebih dari sehari.',
		taken: 'Sementara itu, akun lain sudah memakai alamat ini.',
		confirmUnreachable: 'Server tidak menjawab. Buka lagi tautannya sebentar lagi.',
		close: 'Tutup'
	},

	invites: {
		intro:
			'Dengan tautan, orang lain bisa membuat akun di rueder ini. Tautan hanya ditampilkan sekali, di sini, saat Anda membuatnya.',
		left: (count) =>
			count === 0
				? 'Anda tidak punya undangan untuk diberikan.'
				: `Anda masih bisa mengundang ${n(count)} orang.`,
		admin: 'Sebagai admin, Anda selalu bisa mengundang.',
		uses: 'Untuk berapa akun',
		make: 'Buat tautan',
		noneLeft: 'Undangan Anda tidak cukup untuk sebanyak itu.',
		failed: 'Tidak berhasil. Coba lagi sebentar lagi.',
		link: 'Tautan, untuk diteruskan sekarang',
		copy: 'Salin',
		copied: 'Disalin',
		goodFor: (days) => `Tautan berlaku ${n(days)} hari.`,
		yours: 'Tautan Anda',
		none: 'Anda belum membuat tautan.',
		used: (uses, of) => `${n(uses)} dari ${n(of)} dipakai`,
		until: (when) => `berlaku sampai ${when}`,
		ran: (when) => `kedaluwarsa ${when}`,
		usedUp: 'habis dipakai',
		revoked: 'ditarik kembali',
		revoke: 'Tarik kembali',
		revokeLink: (made) => `Tarik kembali tautan yang dibuat ${made}`
	},

	home: {
		newIn: 'Baru di langganan Anda',
		allRead: 'Anda sudah membaca semuanya. Artikel baru muncul di sini saat datang.',
		recentlyStarred: 'Baru diberi bintang',
		noStars: 'Belum ada yang diberi bintang. Tekan s pada artikel.',
		problems: 'Feed yang bermasalah',
		tips: 'Tips',
		tipKeys: 'Tekan ? untuk semua pintasan keyboard; j dan k menelusuri artikel.',
		tipDrag: 'Seret feed dan folder di bilah samping untuk mengurutkannya.',
		tipFilters:
			'Aturan filter di pengaturan menyembunyikan artikel berdasarkan kata kunci atau penulis.',
		showFeed: (title) => `Tampilkan ${title}`
	},

	versions: {
		title: 'Versi artikel ini',
		current: 'Versi saat ini',
		version: (rev) => `Versi ${n(rev)}`,
		fetched: (when) => `diambil ${when}`,
		compareWith: 'Bandingkan versi saat ini dengan',
		legendAdded: 'ditambahkan',
		legendRemoved: 'dihapus',
		titleWas: 'Judul sebelumnya:',
		same: 'Teks versi ini sama dengan versi saat ini.',
		sameWords: 'Kata-katanya sama; gambar atau tautan berubah.',
		markup: 'Kata, gambar, dan tautan sama; hanya susunan teks yang berubah.',
		pictures: 'Gambar yang berubah',
		noPicture: 'Tidak ada',
		links: 'Tautan yang berubah'
	},

	account: {
		title: 'Pengaturan akun',
		tabs: {
			password: 'Kata sandi',
			email: 'Email',
			security: 'Keamanan',
			apps: 'Aplikasi',
			invites: 'Undangan',
			move: 'Pindah',
			privacy: 'Privasi',
			remove: 'Hapus akun'
		},
		password: {
			title: 'Ubah kata sandi',
			titleFirst: 'Atur kata sandi',
			intro:
				'Kata sandi baru mengeluarkan akun dari semua browser dan perangkat lain dan mencabut semua sandi aplikasi.',
			introFirst:
				'Dengan kata sandi, akun ini juga bisa masuk dengan namanya, tidak hanya melalui sistem masuk tunggal.',
			current: 'Kata sandi saat ini',
			fresh: 'Kata sandi baru',
			again: 'Kata sandi baru sekali lagi',
			submit: 'Ubah kata sandi',
			submitFirst: 'Atur kata sandi',
			done: 'Kata sandi telah diubah. Akun dikeluarkan dari semua browser dan perangkat lain, dan sandi aplikasi dicabut.',
			doneFirst: 'Kata sandi sudah diatur.',
			mismatch: 'Keduanya tidak sama.',
			code: 'Kode verifikasi dua langkah',
			wrong: 'Kata sandi saat ini atau kodenya tidak benar.',
			codeNeeded:
				'Akun ini memakai verifikasi dua langkah. Masukkan juga kode dari aplikasi autentikator Anda atau kode pemulihan, atau konfirmasi dengan kunci sandi.',
			loginAgain:
				'Akun Anda belum punya kata sandi maupun verifikasi dua langkah. Masuk lagi, lalu atur kata sandi dalam sepuluh menit.',
			invalid: (characters) =>
				`Server tidak menerima kata sandi itu. Minimal ${n(characters)} karakter.`,
			busy: (count) => `Terlalu banyak percobaan yang salah. Coba lagi dalam ${minutes(count)}.`,
			unreachable: 'Server tidak menjawab. Kata sandi tidak berubah.'
		},
		settingsNote: 'Tampilan, membaca, aturan filter, dan keyboard ada di Pengaturan.'
	},

	settings: {
		title: 'Pengaturan',
		tabs: {
			look: 'Tampilan',
			reading: 'Membaca',
			rules: 'Aturan filter',
			keys: 'Keyboard'
		},
		accountNote:
			'Kata sandi, verifikasi dua langkah, aplikasi, dan undangan ada di Pengaturan akun, di menu akun Anda.',
		theme: 'Tema',
		themes: { system: 'Ikuti sistem', light: 'Terang', dark: 'Gelap' },
		accent: 'Warna aksen',
		accents: {
			red: 'Merah',
			blue: 'Biru',
			green: 'Hijau',
			orange: 'Oranye',
			purple: 'Ungu',
			graphite: 'Grafit'
		},
		density: 'Kerapatan daftar',
		densities: { compact: 'Rapat', comfortable: 'Lega' },
		sidebarSide: 'Bilah samping',
		sidebarSides: { start: 'Kiri', end: 'Kanan' },
		textSize: 'Ukuran teks artikel, di perangkat ini',
		textSizes: { small: 'Kecil', medium: 'Sedang', large: 'Besar' },
		readingFont: 'Huruf artikel',
		readingFonts: { sans: 'Sans-serif', serif: 'Serif' },
		counters: 'Jumlah belum dibaca',
		counterStyles: { parens: 'Dalam kurung', badge: 'Sebagai lencana' },
		language: 'Bahasa',
		languageAuto: 'Ikuti browser',
		view: 'Tampilan',
		views: { list: 'Daftar', expanded: 'Diperluas', split: 'Tiga panel', magazine: 'Majalah' },
		markOnScroll: 'Tandai artikel sudah dibaca saat digulir lewat di tampilan diperluas',
		teasers: 'Tampilkan kata-kata pertama teks setelah judul di daftar',
		wakeLock: 'Biarkan layar menyala selama rueder terbuka',
		wakeLockMissing: 'Browser ini tidak bisa membiarkan layar menyala.',
		note: 'Perubahan langsung berlaku dan disimpan bersama akun Anda, jadi ikut ke setiap perangkat.',
		noteDemo: 'Perubahan langsung berlaku dan tetap di browser ini.',
		apps: {
			intro:
				'Aplikasi pembaca feed lain bisa memakai akun Anda melalui Google Reader API. Masing-masing mendapat sandi aplikasinya sendiri, yang bisa Anda cabut kapan saja.',
			none: 'Belum ada aplikasi yang disiapkan.',
			made: (when) => `disiapkan ${when}`,
			used: (when) => `terakhir dipakai ${when}`,
			unused: 'belum dipakai',
			revoke: 'Cabut',
			revokeName: (name) => `Cabut sandi aplikasi ${name}`,
			add: 'Siapkan aplikasi',
			full: (max) =>
				`Satu akun paling banyak punya ${n(max)} sandi aplikasi. Cabut satu untuk menyiapkan aplikasi lain.`,
			choose: 'Aplikasi mana yang ingin Anda pakai?',
			website: (app) => `Situs web ${app}`,
			other: 'Aplikasi lain',
			otherHint: 'Aplikasi apa pun yang mendukung Google Reader API atau FreshRSS.',
			platforms: {
				apple: 'Mac, iPhone, dan iPad',
				android: 'Android'
			},
			name: 'Nama sandi aplikasi',
			password: 'Kata sandi Anda',
			code: 'Kode dari aplikasi autentikator Anda, atau kode pemulihan',
			create: 'Buat sandi aplikasi',
			creating: 'Sebentar …',
			back: 'Kembali',
			how: {
				netnewswire:
					'Di NetNewsWire, buka Settings, lalu Accounts, tambahkan akun (di Mac dengan +) dan pilih FreshRSS. Masukkan nama di bawah sebagai Login (di iPhone dan iPad: Username or Email), sandi aplikasi sebagai Password, dan alamat server sebagai API URL.',
				readyou:
					'Di Read You, buka Pengaturan, Akun, Tambahkan akun, lalu pilih Google Reader. Masukkan alamat server di bawah sebagai URL Server, nama sebagai Nama Pengguna, dan sandi aplikasi sebagai Kata Sandi.',
				other:
					'Di aplikasi Anda, tambahkan akun jenis “Google Reader API” atau, jika tidak ada, “FreshRSS”. Masukkan alamat server, nama, dan sandi aplikasi di bawah.'
			},
			ownIcons: (app) =>
				`Privasi: ${app} memuat sendiri ikon feed Anda dari situs web feed tersebut, yang melihat alamat IP Anda saat itu. Artikel dan gambarnya hanya datang melalui rueder.`,
			shotsTitle: 'Langkah demi langkah, dengan gambar',
			shots: {
				readyou: [
					'Ketuk roda gigi di kiri atas.',
					'Buka Akun.',
					'Ketuk Tambahkan akun.',
					'Pilih Google Reader.',
					'Masukkan alamat server sebagai URL Server, nama sebagai Nama Pengguna, dan sandi aplikasi sebagai Kata Sandi, lalu ketuk Tambahkan.'
				],
				netnewswire: [
					'Di Mac: buka Settings (⌘,), lalu Accounts, dan klik + di kiri bawah.',
					'Pilih FreshRSS dan klik Continue.',
					'Masukkan nama sebagai Login, sandi aplikasi sebagai Password, dan alamat server sebagai API URL, lalu klik Create.'
				]
			},
			shotAlt: (app, step) => `${app}, langkah ${n(step)}`,
			server: 'Alamat server',
			user: 'Nama',
			secret: 'Sandi aplikasi',
			copy: (what) => `Salin ${what}`,
			copied: 'Disalin',
			once: 'Sandi aplikasi hanya ditampilkan sekarang. Jika hilang, cabut lalu buat yang baru.',
			waiting: 'Menunggu aplikasi masuk …',
			connected: 'Terhubung: aplikasi sudah masuk.',
			done: 'Selesai',
			refusals: {
				wrongPassword: 'Kata sandinya tidak benar.',
				codeNeeded:
					'Masukkan juga kode dari aplikasi autentikator Anda atau kode pemulihan, atau konfirmasi dengan kunci sandi.',
				wrongCode: 'Kode itu tidak benar. Kode berganti setiap 30 detik.',
				loginAgain:
					'Akun Anda belum punya kata sandi maupun verifikasi dua langkah. Masuk lagi, lalu buat sandi aplikasi dalam sepuluh menit.',
				tooManyApps: 'Satu akun paling banyak punya 20 sandi aplikasi.',
				tooMany: 'Terlalu banyak percobaan yang salah. Coba lagi dalam beberapa menit.',
				unreachable: 'Server tidak menjawab. Coba lagi sebentar lagi.'
			}
		},
		secondFactor: {
			title: 'Verifikasi dua langkah',
			intro:
				'Dengan verifikasi dua langkah, masuk juga memerlukan kode dari aplikasi autentikator, atau kunci sandi.',
			app: 'Aplikasi autentikator',
			on: 'Aktif.',
			left: (count) => `Tersisa ${n(count)} kode pemulihan.`,
			off: 'Nonaktif.',
			byPasskeys: 'Belum disiapkan. Kunci sandi Anda membuat verifikasi dua langkah tetap aktif.',
			adminNeeds:
				'Server ini hanya menganggap admin sebagai admin jika memakai verifikasi dua langkah. Siapkan, atau masuk lagi dengan kodenya, untuk melihat status server.',
			password: 'Kata sandi Anda',
			start: 'Siapkan',
			scan: 'Pindai kode ini dengan aplikasi autentikator Anda, atau ketik kunci di bawah.',
			qr: 'Kode QR untuk aplikasi autentikator',
			larger: 'Perbesar kode QR',
			smaller: 'Perkecil kode QR',
			key: 'Kunci',
			copyKey: 'Salin kunci',
			copied: 'Disalin',
			code: 'Kode yang ditampilkan aplikasi sekarang',
			codeOrRecovery: 'Kode dari aplikasi, atau kode pemulihan',
			confirm: 'Konfirmasi',
			cancel: 'Batal',
			codesTitle: 'Kode pemulihan Anda',
			codesIntro:
				'Setiap kode bisa dipakai sekali untuk masuk tanpa ponsel. Simpan baik-baik: kode hanya ditampilkan sekarang.',
			copyCodes: 'Salin kode',
			codesDone: 'Sudah saya simpan',
			switchOff: 'Nonaktifkan',
			switchOffNote: 'Setelah itu, masuk hanya memerlukan kata sandi lagi.',
			switchOffNoteKeys: 'Kunci sandi Anda membuat verifikasi dua langkah tetap aktif.',
			startWithPasskey: 'Siapkan dengan kunci sandi',
			switchOffWithPasskey: 'Nonaktifkan dengan kunci sandi',
			refusals: {
				wrongPassword: 'Kata sandinya tidak benar.',
				codeNeeded:
					'Kunci sandi Anda membuat verifikasi dua langkah tetap aktif. Masukkan juga kode pemulihan.',
				wrongCode: 'Kode itu tidak benar. Kode berganti setiap 30 detik.',
				loginAgain:
					'Akun Anda belum punya kata sandi maupun verifikasi dua langkah. Masuk lagi, lalu siapkan aplikasi dalam sepuluh menit.',
				tooMany: 'Terlalu banyak percobaan yang salah. Coba lagi dalam beberapa menit.',
				unreachable: 'Server tidak menjawab. Coba lagi sebentar lagi.'
			}
		},
		passkeys: {
			title: 'Kunci sandi',
			intro:
				'Kunci sandi memungkinkan Anda masuk tanpa nama dan kata sandi, dengan PIN, sidik jari, atau wajah yang diminta perangkat Anda. Kunci sandi juga dihitung sebagai verifikasi dua langkah.',
			once: 'Dengan kunci sandi, kata sandi saja tidak lagi bisa membuka akun Anda.',
			none: 'Belum ada kunci sandi.',
			made: (when) => `ditambahkan ${when}`,
			used: (when) => `terakhir dipakai ${when}`,
			unused: 'belum dipakai',
			add: 'Tambahkan kunci sandi',
			name: 'Nama, misalnya perangkatnya',
			defaultName: 'Kunci sandi',
			password: 'Kata sandi Anda',
			code: 'Kode dari aplikasi autentikator Anda, atau kode pemulihan',
			create: 'Buat kunci sandi',
			working: 'Ikuti petunjuk browser Anda …',
			cancel: 'Batal',
			remove: 'Hapus',
			removeName: (name) => `Hapus kunci sandi ${name}`,
			confirmRemove: 'Hapus kunci sandi',
			full: (max) => `Satu akun paling banyak punya ${n(max)} kunci sandi.`,
			or: 'atau',
			confirmWith: 'Konfirmasi dengan kunci sandi',
			codesTitle: 'Kode pemulihan Anda',
			codesIntro:
				'Kunci sandi pertama Anda mengaktifkan verifikasi dua langkah. Setiap kode bisa dipakai sekali untuk masuk jika perangkat Anda hilang. Simpan baik-baik: kode hanya ditampilkan sekarang.',
			copyCodes: 'Salin kode',
			copied: 'Disalin',
			codesDone: 'Sudah saya simpan',
			refusals: {
				cancelled: 'Jendela browser ditutup sebelum kunci sandi dipakai.',
				known: 'Perangkat ini sudah punya kunci sandi untuk akun Anda.',
				failed: 'Browser atau perangkat ini tidak bisa membuat atau memakai kunci sandi.',
				wrong: 'Kunci sandi tidak diterima. Coba lagi.',
				wrongPassword: 'Kata sandinya tidak benar.',
				codeNeeded:
					'Akun Anda memakai verifikasi dua langkah. Masukkan juga kode dari aplikasi Anda atau kode pemulihan, atau konfirmasi dengan kunci sandi.',
				wrongCode: 'Kode itu tidak benar. Kode berganti setiap 30 detik.',
				tooManyPasskeys: 'Satu akun paling banyak punya 10 kunci sandi.',
				loginAgain:
					'Akun Anda belum punya kata sandi maupun verifikasi dua langkah. Masuk lagi, lalu tambahkan kunci sandi pertama Anda dalam sepuluh menit.',
				tooMany: 'Terlalu banyak percobaan yang salah. Coba lagi dalam beberapa menit.',
				unreachable: 'Server tidak menjawab. Coba lagi sebentar lagi.'
			}
		},
		reports: {
			send: 'Kirim laporan kesalahan di browser ini ke server ini',
			intro: 'Saat ada yang salah di halaman, admin server ini mengetahuinya.',
			what: 'Satu laporan berisi:',
			holds: [
				'pesan kesalahan, tanpa alamat dan tanpa angka yang menunjuk sesuatu',
				'di mana dalam berkas rueder sendiri kesalahan itu terjadi',
				'versi rueder dan browser Anda beserta versi utamanya'
			],
			never:
				'Tidak pernah yang Anda baca, feed, label, atau aturan Anda. Laporan dihapus setelah 30 hari.',
			off: 'Menonaktifkan ini menghapus laporan yang sudah Anda kirim.'
		}
	},

	rules: {
		intro:
			'Aturan menyembunyikan artikel atau menandainya sudah dibaca saat artikel datang. Artikel yang disembunyikan tidak dihitung belum dibaca dan tetap ada di “Disembunyikan filter”.',
		none: 'Belum ada aturan.',
		add: 'Tambahkan aturan',
		new: 'Aturan baru',
		on: 'Aktif',
		scope: 'Di',
		scopeAll: 'Semua feed',
		scopeFolder: (title) => `Folder: ${title}`,
		scopeFeed: (title) => `Feed: ${title}`,
		field: 'Periksa',
		fields: { any: 'Semuanya', title: 'Judul', content: 'Teks', author: 'Penulis', tag: 'Tag' },
		match: 'Cocokkan',
		matches: {
			contains: 'berisi',
			word: 'memuat kata',
			starts: 'diawali dengan',
			ends: 'diakhiri dengan',
			regex: 'cocok dengan ekspresi reguler',
			all: 'setiap artikel, apa pun isinya'
		},
		pattern: 'Teks yang dicari',
		action: 'Lalu',
		actions: { hide: 'sembunyikan artikel', read: 'tandai sudah dibaca' },
		preview: (count, total) =>
			`Saat ini berlaku untuk ${n(count)} dari ${n(total)} artikel di sini.`,
		regexTip:
			'Ekspresi reguler dibaca seperti Rust membacanya, tanpa membedakan huruf besar-kecil.',
		regexBuilder: 'Susun dan coba di regex101',
		invalid: 'Ini bukan ekspresi reguler yang valid.',
		empty: 'Masukkan teks yang dicari.',
		checking: 'Melihat artikel mana yang akan terkena …',
		hits: (count) => `${n(count)} cocok`,
		summary: (field, match, pattern, action) => `${field} ${match} “${pattern}”: ${action}`,
		summaryAll: (action) => `Setiap artikel: ${action}`
	},

	keys: {
		title: 'Pintasan keyboard',
		note: 'Tombol Anda disimpan bersama pengaturan. Enter, Escape, dan g tetap seperti semula.',
		change: (what) => `Ubah tombol untuk: ${what}`,
		changeShort: 'Ubah',
		press: 'Tekan sebuah tombol …',
		taken: (key, other) => `${key} sudah menjadi tombol untuk: ${other}`,
		fixed: (key) => `${key} tidak bisa diberikan ke pintasan.`,
		reset: 'Kembali ke tombol bawaan rueder',
		groups: { articles: 'Artikel', lists: 'Daftar', go: 'Buka', app: 'Di mana saja' },
		next: 'Artikel berikutnya, dibuka',
		previous: 'Artikel sebelumnya, dibuka',
		nextCursor: 'Artikel berikutnya, tanpa dibuka',
		previousCursor: 'Artikel sebelumnya, tanpa dibuka',
		toggle: 'Buka atau tutup artikel',
		star: 'Beri atau hapus bintang',
		read: 'Tandai sudah atau belum dibaca',
		original: 'Buka aslinya di tab baru',
		markAll: 'Tandai semua sudah dibaca',
		refresh: 'Segarkan daftar',
		unreadOnly: 'Hanya yang belum dibaca, atau semua artikel',
		listView: 'Tampilan daftar',
		expandedView: 'Tampilan diperluas',
		splitView: 'Tiga panel',
		magazineView: 'Majalah',
		goHome: 'Beranda',
		goAll: 'Semua artikel',
		goUnread: 'Belum dibaca',
		goStarred: 'Berbintang',
		search: 'Cari',
		sidebar: 'Tampilkan atau sembunyikan bilah samping',
		help: 'Ringkasan ini',
		then: 'lalu'
	},

	labels: {
		title: 'Label',
		add: 'Label baru',
		name: 'Nama label',
		none: 'Belum ada label.',
		actions: (title) => `Tindakan untuk label ${title}`,
		rename: 'Ganti nama',
		color: 'Warna',
		colors: ['Merah', 'Oranye', 'Kuning', 'Hijau', 'Hijau toska', 'Biru', 'Ungu', 'Abu-abu'],
		remove: 'Hapus label',
		show: 'Tampilkan label',
		hide: 'Sembunyikan label',
		forArticle: 'Label',
		set: (title) => `Label: ${title}`
	},

	subscribe: {
		title: 'Berlangganan feed',
		address: 'Alamat feed atau situs web',
		find: 'Cari feed',
		finding: 'Mencari …',
		note: 'Draf tidak bisa mengambil apa pun, jadi feed tetap kosong.',
		found: (count) => (count === 1 ? 'Ada satu feed:' : `Ada ${n(count)} feed:`),
		none: 'Tidak ada feed di alamat ini.',
		movedTo: (url) => `Situs meneruskan permintaan ke ${url}.`,
		loginWall:
			'Beberapa situs mengarahkan siapa pun yang belum masuk ke halaman masuknya. Feed situs itu tidak bisa dibaca.',
		subscribed: 'Sudah berlangganan',
		show: 'Tampilkan',
		untitled: 'Tanpa nama',
		folder: 'Ke folder',
		topLevel: 'Tanpa folder',
		add: 'Berlangganan',
		adding: 'Berlangganan …',
		refused: 'Tidak berhasil.',
		serverSays: (reason) => `Kata server: ${reason}`,
		busy: (seconds) =>
			seconds
				? `Terlalu banyak pencarian sekaligus. Coba lagi dalam ${n(seconds)} detik.`
				: 'Terlalu banyak pencarian sekaligus. Coba lagi sebentar lagi.',
		noFetching: 'Server ini tidak mengambil feed.',
		tooLarge: (megabytes) => `Berkas lebih besar dari ${n(megabytes)} MB.`,
		unreachable: 'Server tidak menjawab. Coba lagi sebentar lagi.'
	},

	opml: {
		title: 'Impor dan ekspor',
		intro: 'Bawa langganan Anda dari pembaca feed lain sebagai berkas OPML, atau bawa pergi.',
		import: 'Impor berkas OPML',
		importing: 'Mengimpor …',
		imported: (feeds, folders) => `${n(feeds)} feed baru dan ${n(folders)} folder baru.`,
		known: (count) => `${n(count)} feed sudah dilanggan sebelumnya.`,
		skipped: (count) => `${n(count)} entri dilewati karena bukan feed yang bisa dibaca.`,
		later: 'Artikelnya datang pada putaran pengambilan berikutnya.',
		export: 'Ekspor sebagai OPML'
	},

	removeAccount: {
		title: 'Hapus akun',
		intro: 'Ini menghapus akun Anda di rueder ini untuk selamanya. Tidak bisa dibatalkan.',
		goes: 'Yang ikut terhapus: sesi Anda di semua perangkat, langganan dan folder, catatan baca, bintang, label, aturan filter, pengaturan, laporan kesalahan, undangan, dan tautan ke sistem masuk tunggal.',
		stays:
			'Feed tetap ada untuk orang lain yang membacanya. Untuk memindahkan akun Anda ke tempat lain lebih dulu, gunakan “Pindah”.',
		password: 'Kata sandi Anda',
		name: (name) => `Ketik nama Anda, ${name}, untuk mengonfirmasi`,
		code: 'Kode dari aplikasi Anda, atau kode pemulihan',
		submit: 'Hapus akun saya untuk selamanya',
		wrong: 'Itu tidak benar. Tidak ada yang dihapus.',
		codeNeeded:
			'Akun Anda memakai verifikasi dua langkah. Ketik juga kode dari aplikasi Anda atau kode pemulihan, atau konfirmasi dengan kunci sandi.',
		busy: (count) => `Terlalu banyak percobaan yang salah. Coba lagi dalam ${minutes(count)}.`,
		unreachable: 'Server tidak menjawab. Tidak ada yang dihapus.'
	},

	move: {
		intro: 'Bawa akun Anda ke rueder lain, atau bawa ke sini dari rueder lain, dalam satu berkas.',
		howItWorks:
			'Impor menambah yang sudah ada di sini dan tidak menghapus apa pun. Pengaturan Anda menjadi pengaturan dari berkas.',
		notInIt: 'Tidak ada di dalam berkas: kata sandi, sesi, dan teks artikel Anda.',
		export: 'Ekspor akun',
		import: 'Impor berkas akun',
		importing: 'Mengimpor …',
		feeds: (feeds, known) => `${n(feeds)} feed baru, ${n(known)} sudah dilanggan.`,
		folders: (count) => `${n(count)} folder baru.`,
		articles: (found) => `${articles(found)} ditemukan di sini.`,
		missing: (count) =>
			`${articles(count)} tidak ditemukan di sini; catatan baca, bintang, dan label Anda untuk artikel itu hilang.`,
		marks: (labels, rules) => `${n(labels)} label baru dan ${n(rules)} aturan filter baru.`,
		skipped: (count) => `${n(count)} entri dari berkas tidak bisa diambil.`,
		settings: 'Pengaturan Anda kini adalah pengaturan dari berkas.',
		later:
			'Feed baru membawa artikelnya pada putaran pengambilan berikutnya. Yang sudah Anda baca tetap sudah dibaca.',
		busy: (seconds) =>
			seconds
				? `Terlalu banyak impor. Coba lagi dalam ${minutes(Math.ceil(seconds / 60))}.`
				: 'Terlalu banyak impor. Coba lagi nanti.'
	},

	status: {
		title: 'Status server',
		open: 'Status server',
		loading: 'Bertanya ke server …',
		failed: 'Server tidak menjawab. Coba lagi sebentar lagi.',
		refresh: 'Segarkan',
		asOf: (time) => `Per ${time}. Yang dihitung adalah satu jam terakhir.`,
		demo: 'Rekaan, karena tidak ada server di sini.',
		empty: '–',
		on: 'Aktif',
		off: 'Nonaktif',
		took: (median, p95) => `median ${median}, 95% dalam ${p95}`,
		summary: {
			label: 'Sekilas',
			hints: 'Petunjuk',
			failing: 'Feed gagal',
			errors: 'Kesalahan tercatat',
			reported: 'Feed dilaporkan'
		},
		hintsTitle: 'Patut dilihat',
		allWell: 'Tidak ada yang menonjol.',
		hints: {
			slowRoute: (route, took) =>
				`${route} lambat: satu dari dua puluh permintaan memakan waktu lebih dari ${took}.`,
			serverErrors: (route, count) =>
				`${route} menjawab dengan kesalahan server sebanyak ${n(count)} kali.`,
			poolBusy: (size) =>
				`Semua ${n(size)} koneksi ke basis data sedang dipakai, jadi permintaan menunggu.`,
			fetchBacklog: (count, late) =>
				`${n(count)} feed terlambat diambil, yang paling lambat ${late}.`,
			feedsFailing: (failing, subscribed) => `${n(failing)} dari ${n(subscribed)} feed gagal.`,
			notFetching: 'Server tidak mengambil feed. Apakah FETCH diatur ke off?',
			fetchPaceFull: (most) =>
				`Setiap putaran mengambil feed sebanyak yang diizinkan, dan masih ada feed yang menunggu: laju ini mengizinkan ${n(most)} pengambilan per jam. FETCH_ROUND, FETCH_AT_ONCE, dan FETCH_EVERY mengaturnya.`,
			cacheMisses: (share) =>
				`Basis data hanya menemukan ${share} dari yang dibacanya di memori; shared_buffers yang lebih besar akan membantu.`,
			databaseLarge: (size, warn) =>
				`Basis data sudah mencapai ${size}; DATABASE_WARN_GB memperingatkan mulai ${warn}. “rueder retention set” menghapus artikel lama atau teksnya.`,
			longQuery: (took) => `Sebuah kueri sudah berjalan selama ${took}.`,
			imageCacheFull: (share) => `Cache gambar sudah terisi ${share}.`,
			errors: (count) => `Server mencatat ${n(count)} kesalahan dalam satu jam terakhir.`,
			limited: (count) => `${n(count)} permintaan ditolak karena terlalu banyak.`,
			browserErrors: (count) =>
				`Browser melaporkan ${n(count)} jenis kesalahan dalam sehari terakhir.`,
			reportedFeeds: (count) => `Pembaca melaporkan ${n(count)} feed yang belum diblokir.`
		},
		build: {
			title: 'Build',
			commit: 'Commit',
			dirty: (commit) => `${commit}, dengan perubahan sendiri`,
			branch: 'Branch',
			tag: 'Tag',
			committed: 'Di-commit',
			where: 'Berjalan',
			container: 'di dalam kontainer',
			host: 'langsung di mesin',
			target: 'Untuk',
			profile: 'Dikompilasi',
			profileValue: (profile, level) => `${profile}, optimasi ${level}`,
			ci: 'Dibuat oleh',
			ciValue: 'satu proses CI',
			byHand: 'tangan'
		},
		server: {
			title: 'Server',
			version: 'Versi',
			running: 'Berjalan sejak',
			memory: 'Memori',
			accounts: 'Akun',
			feeds: 'Feed',
			feedsValue: (fetched, all) => `${n(fetched)} diambil, ${n(all)} seluruhnya`,
			articles: 'Artikel',
			articlesValue: (count) => `sekitar ${n(count)}`,
			live: 'Tab terbuka'
		},
		database: {
			title: 'Basis data',
			pool: 'Koneksi rueder',
			poolValue: (size, idle, max) =>
				`${n(size)} terbuka, ${n(idle)} menganggur, paling banyak ${n(max)}`,
			connections: 'Semua koneksi',
			size: 'Ukuran',
			sizeWarn: (size, warn) => `${size}, peringatan mulai ${warn}`,
			hitRatio: 'Dibaca dari memori',
			deadlocks: 'Deadlock',
			longest: 'Kueri yang paling lama berjalan',
			tables: 'Tabel terbesar',
			table: 'Tabel',
			bytes: 'Ukuran',
			rows: 'Baris, sekitar'
		},
		fetcher: {
			title: 'Mengambil feed',
			state: 'Pengambilan',
			lastRound: 'Putaran terakhir',
			lastRoundValue: (ago, feeds, took) => `${ago}: ${n(feeds)} feed dalam ${took}`,
			fetches: 'Pengambilan',
			fetchesValue: (fetched, unchanged, failed) =>
				`${n(fetched)} membawa feed, ${n(unchanged)} tidak berubah, ${n(failed)} gagal`,
			broken: 'Gagal di basis data',
			newArticles: 'Artikel baru',
			took: 'Waktu per pengambilan',
			overdue: 'Terlambat diambil',
			overdueValue: (count, late) => `${n(count)}, yang paling lambat ${late}`,
			waiting: 'Belum diambil',
			pace: 'Laju',
			paceValue: (round, every, atOnce, most) =>
				`${n(round)} feed setiap ${every}, ${n(atOnce)} sekaligus: paling banyak ${n(most)} per jam`,
			perHour: 'Pengambilan dalam satu jam terakhir',
			behind: 'Tertinggal',
			behindValue: (late) => `feed yang menunggu paling lama sudah lewat ${late} dari waktunya`,
			notBehind: 'Tidak ada yang menunggu',
			failing: 'Feed yang gagal, menurut alasannya',
			noneFailing: 'Tidak ada feed yang gagal.',
			feeds: (count) => `${n(count)} feed`,
			failures: {
				status: (code) => (code ? `Situs menjawab ${code}` : 'Situs menjawab dengan kesalahan'),
				timeout: 'Situs terlalu lama',
				blocked: 'Alamatnya ada di jaringan yang tidak diambil rueder',
				dns: 'Nama itu tidak punya alamat',
				network: 'Tidak ada koneksi, atau koneksi terputus',
				too_large: 'Terlalu besar',
				redirects: 'Terlalu banyak pengalihan, atau pengalihan yang rusak',
				unreadable: 'Bukan feed yang bisa dibaca rueder',
				other: 'Lainnya'
			}
		},
		finder: {
			title: 'Pencarian feed',
			state: 'Memeriksa alamat',
			lookups: 'Pencarian',
			lookupsValue: (found, none, failed) =>
				`${n(found)} menemukan feed, ${n(none)} tidak menemukan, ${n(failed)} gagal`,
			limited: 'Melewati batas akun',
			took: 'Waktu per pencarian'
		},
		images: {
			title: 'Proksi gambar',
			state: 'Proksi',
			cache: 'Cache',
			cacheValue: (used, max) => `${used} dari ${max}`,
			served: 'Gambar',
			servedValue: (cached, fetched) => `${n(cached)} dari cache, ${n(fetched)} diambil`,
			refused: 'Ditolak',
			refusedValue: (source, notImage, tooLarge, notFound) =>
				`${n(source)} sumber gagal, ${n(notImage)} bukan gambar, ${n(tooLarge)} terlalu besar, ${n(notFound)} alamat salah`,
			took: 'Waktu per gambar yang diambil',
			failingSources: 'Sumber yang dibiarkan untuk sementara'
		},
		requests: {
			title: 'Permintaan',
			all: 'Semua permintaan',
			allValue: (count, took) => `${n(count)}, ${took}`,
			answers: 'Jawaban',
			answersValue: (fine, refused, errors) =>
				`${n(fine)} baik, ${n(refused)} ditolak, ${n(errors)} kesalahan server`,
			route: 'Rute',
			count: 'Jumlah',
			median: 'Median',
			p95: '95%',
			max: 'Terlama',
			errors: 'Kesalahan',
			none: 'Tidak ada permintaan dalam satu jam terakhir.'
		},
		problems: {
			title: 'Peringatan dan kesalahan',
			counts: (errors, warnings) =>
				`Dalam satu jam terakhir ${n(errors)} kesalahan dan ${n(warnings)} peringatan.`,
			none: 'Tidak ada yang tercatat sejak awal.',
			error: 'Kesalahan',
			warning: 'Peringatan',
			times: (count, last) => (count > 1 ? `${n(count)} kali, terakhir ${last}` : last)
		},
		browsers: {
			title: 'Kesalahan di browser',
			off: 'Server ini tidak menerima laporan kesalahan (CLIENT_REPORTS=off).',
			consenting: (count) =>
				`${n(count)} akun mengirim laporan. Kesalahan disimpan 30 hari setelah terakhir terjadi.`,
			none: 'Tidak ada browser yang melaporkan kesalahan.',
			kinds: {
				error: 'Kesalahan',
				rejection: 'Promise yang gagal',
				component: 'Bagian halaman',
				request: 'Permintaan'
			},
			hits: (count, accounts, last) => `${n(count)} kali di ${n(accounts)} akun, terakhir ${last}`,
			where: (versions, browsers) => `Versi ${versions} · ${browsers}`
		},
		reported: {
			title: 'Feed yang dilaporkan',
			text: 'Siapa yang melapor tidak ditampilkan di sini. “rueder feed block <alamat>” memblokir feed untuk semua orang, “rueder feed unblock <alamat>” membukanya lagi.',
			none: 'Belum ada yang melaporkan feed.',
			reports: (count, last) => `${n(count)} laporan, terakhir ${last}`,
			blocked: 'Diblokir'
		}
	},

	update: {
		ready: 'Versi baru rueder sudah siap.',
		reload: 'Muat ulang',
		later: 'Nanti'
	},

	crash: {
		title: 'Ada yang salah di halaman ini.',
		text: 'Muat ulang halaman untuk melanjutkan. Yang sudah Anda baca dan ubah tersimpan di server.',
		reload: 'Muat ulang'
	},

	demo: {
		badge: 'Draf',
		note: 'Draf dengan feed rekaan. Tidak ada yang disimpan di server.',
		arrive: 'Datangkan sebuah artikel',
		revise: 'Ubah artikel yang terbuka',
		revised: 'Pembaruan: paragraf ini ditambahkan untuk mencoba fitur versi.',
		reviseHint: 'Buka sebuah artikel dulu.'
	}
};

export default id;
