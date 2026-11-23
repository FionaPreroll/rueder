// The Polish texts, in the shape of the English catalog (en.ts), addressing the
// reader with “ty”. Past tenses that would say whether the reader is a man or a
// woman are left out for impersonal forms (“przeczytano”). A feed is a “kanał”,
// an item an “artykuł”. Polish counts in three forms: 1, 2–4, 5 and more.

import type { Messages } from './en.ts';
import { formatNumber as n, plural } from './intl.ts';

/** The form for `count`: one (1), few (2–4, 22–24 …), many (5–21, 25 …). */
const forms = (count: number, one: string, few: string, many: string) =>
	plural(count, { one, few, many, other: few });

const counted = (count: number, one: string, few: string, many: string) =>
	`${n(count)} ${forms(count, one, few, many)}`;

const articles = (count: number) => counted(count, 'artykuł', 'artykuły', 'artykułów');

const minutes = (count: number) => counted(count, 'minutę', 'minuty', 'minut');

const pl: Messages = {
	languageName: 'Polski',

	articles,

	common: {
		close: 'Zamknij',
		cancel: 'Anuluj',
		save: 'Zapisz',
		delete: 'Usuń',
		edit: 'Edytuj'
	},

	nav: {
		subscribe: 'Subskrybuj',
		home: 'Start',
		all: 'Wszystkie artykuły',
		unreadItems: 'Nieprzeczytane',
		starred: 'Z gwiazdką',
		hidden: 'Ukryte przez filtry',
		subscriptions: 'Subskrypcje',
		newFolder: 'Nowy folder',
		groupedFolder: 'Nowy folder',
		folderName: 'Nazwa folderu',
		renameFolder: 'Zmień nazwę',
		removeFolder: 'Usuń folder, zachowaj jego zawartość',
		folderActions: (title) => `Działania dla folderu ${title}`,
		feedName: 'Nazwa kanału',
		feedNameHint: 'Puste, aby użyć nazwy kanału',
		renameFeed: 'Zmień nazwę',
		unsubscribe: 'Anuluj subskrypcję',
		reportFeed: 'Zgłoś kanał …',
		feedInfo: 'Informacje o kanale …',
		feedActions: (title) => `Działania dla kanału ${title}`,
		unsubscribeTitle: 'Anulować subskrypcję?',
		unsubscribeText: (title) =>
			`Anulować subskrypcję „${title}”? To, co tam przeczytano i oznaczono gwiazdką, przepadnie.`,
		openFolder: 'Otwórz folder',
		closeFolder: 'Zamknij folder',
		openSidebar: 'Pokaż pasek boczny',
		closeSidebar: 'Ukryj pasek boczny',
		search: 'Szukaj',
		searchPlaceholder: 'Szukaj w tych artykułach',
		clearSearch: 'Wyczyść wyszukiwanie',
		settings: 'Ustawienia',
		accountSettings: 'Ustawienia konta',
		shortcuts: 'Skróty klawiszowe',
		live: 'Na żywo',
		liveOn: 'Nowe artykuły przychodzą same. Kliknij, aby wstrzymać.',
		liveOff: 'Wstrzymano. Kliknij, aby nowe artykuły znów przychodziły.',
		paused: 'Wstrzymano',
		account: (name) => `Zalogowano jako ${name}`,
		logout: 'Wyloguj się',
		appSettings: 'Serwer i aktualizacje aplikacji',
		install: 'Zainstaluj jako aplikację',
		installSafari:
			'Aby zainstalować rueder jako aplikację: Plik › Dodaj do Docka na Macu, Udostępnij › Do ekranu początkowego na iPhonie lub iPadzie.',
		unread: (count) => `${n(count)} nieprzeczytanych`,
		hiddenCount: (count) => `${n(count)} ukrytych przez filtry`,
		fetchFailed: (reason) => `Ostatnie pobieranie się nie udało: ${reason}`,
		waiting: 'Jeszcze nie pobrano',
		blocked: 'Zablokowany przez operatora tego rueder',
		moveHint: 'Przeciągnij, aby posortować, lub naciśnij Alt ze strzałkami',
		resizeSidebar: 'Szerokość paska bocznego: przeciągnij lub użyj strzałek',
		tooDeep: (levels) =>
			`Foldery można zagnieżdżać najwyżej na ${counted(levels, 'poziom', 'poziomy', 'poziomów')}.`
	},

	toolbar: {
		refresh: 'Odśwież',
		showAll: 'Wszystkie artykuły',
		showUnread: 'Tylko nieprzeczytane',
		show: 'Pokaż',
		newestFirst: 'Najnowsze najpierw',
		oldestFirst: 'Najstarsze najpierw',
		markAllRead: 'Oznacz wszystko jako przeczytane',
		markOptions: 'Więcej sposobów oznaczania jako przeczytane',
		phoneMenu: (shown) => `Widok, pokazuje: ${shown}`,
		markMenu: 'Oznacz jako przeczytane …',
		markEverything: 'Wszystkie artykuły',
		markOlderDay: 'Artykuły starsze niż dzień',
		markOlderWeek: 'Artykuły starsze niż tydzień',
		listView: 'Widok listy',
		expandedView: 'Widok rozwinięty',
		splitView: 'Trzy panele: drzewo, lista i artykuł',
		magazineView: 'Magazyn',
		previous: 'Poprzedni artykuł',
		next: 'Następny artykuł',
		newArticles: (count) => counted(count, 'nowy artykuł', 'nowe artykuły', 'nowych artykułów')
	},

	list: {
		empty: 'Brak artykułów.',
		emptyUnread: 'Nie ma tu nic nieprzeczytanego.',
		emptySearch: 'Żaden artykuł nie pasuje do wyszukiwania.',
		found: (count, more, words) =>
			`${n(count)}${more ? '+' : ''} ${forms(count, 'artykuł', 'artykuły', 'artykułów')} z „${words}”`,
		searchAll: 'Szukaj we wszystkich kanałach',
		feedFailing:
			'rueder spróbuje ponownie później i po każdym niepowodzeniu czeka dłużej albo tyle, ile prosi strona. Jeśli nadal się nie udaje, kanał mógł się przenieść.',
		hiddenNote:
			'Twoje reguły filtrów ukryły te artykuły. Otwórz któryś, aby zobaczyć, która to reguła.',
		toRules: 'Reguły filtrów',
		emptyFeed: 'Ten kanał nie ma jeszcze artykułów.',
		waitingFeed:
			'Tego kanału jeszcze nie pobrano. Jego artykuły przyjdą w następnej rundzie pobierania, w ciągu kilku minut.',
		showAll: 'Pokaż wszystkie artykuły',
		more: 'Pokaż więcej',
		star: 'Dodaj gwiazdkę',
		unstar: 'Usuń gwiazdkę',
		keepUnread: 'Zostaw jako nieprzeczytany',
		openOriginal: 'Otwórz oryginał',
		showFeed: 'Pokaż wszystkie artykuły tego kanału w rueder',
		from: 'z',
		by: (author) => `autor: ${author}`,
		updated: 'zaktualizowany',
		updatedHeadline: 'nowy nagłówek',
		updatedTitle: (count) => `Zmieniony od pierwszego pobrania; wcześniejsze wersje: ${n(count)}`,
		updatedHeadlineTitle: (count) =>
			`Nagłówek przepisano, tekst bez zmian; wcześniejsze wersje: ${n(count)}`,
		compare: 'Porównaj wersje',
		hiddenBy: (rule) => `Ukryty przez regułę: ${rule}`,
		textDropped: 'Ten serwer nie przechowuje już tekstu tego artykułu, bo jest stary.',
		textDroppedOriginal: 'Przeczytaj w źródle',
		loading: 'Wczytywanie tekstu …',
		loadingList: 'Wczytywanie …',
		toTop: 'Wróć na górę',
		pick: 'Wybierz artykuł, aby go tu przeczytać.',
		resizeList: 'Szerokość listy: przeciągnij lub użyj strzałek',
		closeArticle: 'Zamknij artykuł',
		blockedFeed:
			'Operator tego rueder zablokował ten kanał. Nie jest pobierany, a jego artykuły nie są pokazywane.',
		articleActions: 'Więcej działań dla artykułu',
		articleInfo: 'Informacje o artykule …',
		articleSource: 'Źródło …',
		reportArticle: 'Zgłoś artykuł …'
	},

	report: {
		feedTitle: 'Zgłoś kanał',
		articleTitle: 'Zgłoś artykuł',
		text: (title) =>
			`Operator tego rueder zobaczy, że zgłoszono „${title}”, z jakiego powodu i co napiszesz, ale nie zobaczy, kto zgłosił. To on decyduje, czy zablokować to dla wszystkich.`,
		reason: 'Powód',
		reasons: {
			illegal: 'Niezgodne z prawem',
			copyright: 'Cudza twórczość',
			malicious: 'Złośliwe oprogramowanie, phishing lub oszustwo',
			spam: 'Spam',
			other: 'Coś innego'
		},
		note: 'Co chcesz dodać (opcjonalnie)',
		left: (count) => `Pozostało znaków: ${n(count)}`,
		send: 'Wyślij zgłoszenie',
		sending: 'Wysyłanie …',
		sent: 'Dziękujemy. Zgłoszenie dotarło do operatora tego rueder.',
		tooMany: 'Ostatnio wysłano od ciebie wiele zgłoszeń. Spróbuj ponownie później.',
		unreachable: 'Zgłoszenie nie dotarło do serwera. Spróbuj ponownie później.'
	},

	feedInfo: {
		title: 'Informacje o kanale',
		text: (title) =>
			`Co ten rueder wie o „${title}” i jak go pobiera. Jeśli kanał nie działa, skopiuj to do wiadomości do operatora.`,
		number: 'Numer',
		name: 'Nazwa',
		address: 'Adres kanału',
		site: 'Strona internetowa',
		noSite: 'Brak',
		agent: 'Pobierany jako',
		agents: { rueder: 'rueder', browser: 'Przeglądarka' },
		every: 'Pobierany co',
		fetched: 'Ostatnie pobieranie',
		succeeded: 'Ostatnie udane pobieranie',
		failed: 'Ostatni błąd',
		next: 'Następne pobieranie',
		blocked: 'Brak, operator zablokował kanał',
		articles: 'Artykuły do tej pory',
		never: 'Nigdy',
		fetchAs: 'Pobieraj jako',
		fetchAsHint:
			'Dla wszystkich, którzy czytają ten kanał. Jako przeglądarka dla stron, które odrzucają rueder. Kanał zostanie od razu pobrany ponownie.',
		notSwitched: 'Nie udało się. Spróbuj ponownie później.',
		copy: 'Kopiuj',
		copied: 'Skopiowano',
		loading: 'Wczytywanie …',
		unreachable: 'Serwer nie odpowiedział. Spróbuj ponownie później.'
	},
	articleInfo: {
		title: 'Informacje o artykule',
		text: (title) =>
			`Co ten rueder wie o „${title}” i jego wersjach. Jeśli coś wygląda źle, skopiuj to do wiadomości do operatora.`,
		feed: 'Kanał',
		feedNumber: 'Numer kanału',
		number: 'Numer w kanale',
		name: 'Tytuł',
		id: 'Identyfikator w kanale',
		link: 'Link',
		noLink: 'Brak',
		published: 'Opublikowano',
		fetched: 'Pierwsze pobranie',
		changed: 'Ostatnia zmiana',
		never: 'Nigdy',
		changedAt: (when, what) => `${when} (${what})`,
		what: { title: 'tytuł', text: 'tekst', both: 'tytuł i tekst' },
		version: 'Wersja',
		kept: 'Przechowywane wersje',
		textLabel: 'Tekst',
		textDropped: 'Już nieprzechowywany, bo jest stary',
		copy: 'Kopiuj',
		copied: 'Skopiowano',
		loading: 'Wczytywanie …',
		unreachable: 'Serwer nie odpowiedział. Spróbuj ponownie później.'
	},
	articleSource: {
		title: 'Źródło artykułu',
		text: 'Artykuł jako HTML: tak, jak ten rueder go przechowuje po oczyszczeniu, albo tak, jak przysłał go kanał. Linki i obrazy mają tu swoje własne adresy, więc widać, czy został jakiś parametr śledzący.',
		show: 'Pokaż',
		kinds: { cleaned: 'Oczyszczony', original: 'Tak, jak przysłał kanał' },
		noText: 'Ten serwer nie przechowuje już tekstu tego artykułu.',
		noOriginal: 'Ten serwer nie przechowuje już tego, co przysłał kanał.',
		copy: 'Kopiuj',
		copied: 'Skopiowano',
		loading: 'Wczytywanie …',
		unreachable: 'Serwer nie odpowiedział. Spróbuj ponownie później.'
	},

	media: {
		title: 'Załączniki',
		play: 'Odtwórz',
		image: 'Obraz',
		zoomIn: 'Powiększ',
		zoomOut: 'Pomniejsz'
	},

	connection: {
		reconnecting: 'Ponowne łączenie …',
		unreachable: (since) => `Serwer nie odpowiada od ${since}.`,
		offline: (since) => `Brak sieci od ${since}.`,
		retry: 'Spróbuj teraz',
		unsent: (count) =>
			`${counted(count, 'zmiana czeka', 'zmiany czekają', 'zmian czeka')} na wysłanie.`
	},

	notice: {
		unreachable: 'Serwer nie odpowiada. To, co widzisz, może być nieaktualne.',
		notSaved: 'Serwer nie przyjął twojej ostatniej zmiany. Została tu cofnięta.',
		tooManyUnsent: 'Na serwer czeka tak wiele zmian, że tę cofnięto.'
	},

	login: {
		title: 'Zaloguj się do rueder',
		about:
			'Czytnik kanałów: pobiera to, co publikują obserwowane strony, i pokazuje, czego jeszcze nie przeczytano.',
		name: 'Nazwa',
		password: 'Hasło',
		submit: 'Zaloguj się',
		checking: 'Chwileczkę …',
		wrong: 'Nieprawidłowa nazwa lub hasło.',
		codeNeeded:
			'To konto ma drugi składnik. Wpisz kod z aplikacji albo kod odzyskiwania lub zaloguj się kluczem dostępu.',
		wrongCode: 'Ten kod jest nieprawidłowy. Kody zmieniają się co 30 sekund.',
		code: 'Kod',
		codeNote: 'Sześć cyfr z aplikacji uwierzytelniającej albo jeden z kodów odzyskiwania.',
		tooMany: (count) => `Za dużo błędnych prób. Spróbuj ponownie za ${minutes(count)}.`,
		unreachable: 'Serwer nie odpowiada. Spróbuj ponownie za chwilę.',
		retry: 'Spróbuj ponownie',
		or: 'lub',
		oidc: 'Zaloguj się przez logowanie jednokrotne',
		passkey: 'Zaloguj się kluczem dostępu',
		passkeyCancelled: 'Nie użyto klucza dostępu. Spróbuj ponownie lub zaloguj się nazwą i hasłem.',
		passkeyFailed: 'Ta przeglądarka lub to urządzenie nie może tu użyć klucza dostępu.',
		passkeyWrong: 'Ten klucz dostępu nie należy do żadnego konta tutaj.',
		oidcFailed: 'Logowanie jednokrotne się nie udało. Spróbuj ponownie.',
		oidcDenied: 'Logowanie jednokrotne zostało anulowane.',
		oidcUnknown:
			'Żadne konto tutaj nie należy do tego logowania. Poproś operatora tego rueder o połączenie.',
		disabled: 'To konto jest wyłączone. Zapytaj operatora tego rueder o powód.',
		unconfirmed:
			'To konto czeka na potwierdzenie adresu e-mail. Otwórz link z wiadomości albo wyślij go ponownie.',
		resend: 'Wyślij link ponownie',
		resent: 'Link jest znowu w drodze. Sprawdź pocztę, także folder spamu.',
		resendWrong: 'Wpisz nazwę i hasło konta, aby wysłać link ponownie.',
		forgot: 'Nie pamiętasz hasła?',
		removed: 'Twoje konto zostało usunięte razem ze wszystkim, co do ciebie należało.',
		note: 'Nie ma tu rejestracji. Konta zakłada operator tego rueder.'
	},

	legal: {
		imprint: 'Informacje prawne',
		privacy: 'Polityka prywatności',
		terms: 'Regulamin',
		agree: { before: 'Akceptuję ', link: 'regulamin', after: '.' },
		changed: 'Regulamin się zmienił',
		changedIntro: 'Przeczytaj nowy regulamin. Aby dalej korzystać z rueder, zaakceptuj go.',
		read: 'Przeczytaj regulamin',
		accept: 'Akceptuję',
		decline: 'Wyloguj się',
		notSaved: 'Nie udało się. Spróbuj ponownie za chwilę.'
	},

	register: {
		title: 'Załóż konto',
		invited: 'Masz zaproszenie do tego rueder. Wybierz nazwę i hasło.',
		again: 'Hasło jeszcze raz',
		passwordRule: (characters) => `Co najmniej ${n(characters)} znaków.`,
		mismatch: 'Hasła się różnią.',
		submit: 'Załóż konto',
		solving: 'Sprawdzanie tej przeglądarki …',
		toRegister: 'Załóż konto',
		toLogin: 'Zaloguj się na istniejące konto',
		closed: 'Tutaj nikt nie może się zarejestrować bez zaproszenia.',
		inviteGone: 'Tego zaproszenia nie można już użyć. Poproś o nowe.',
		full: 'Nie ma tu miejsca na kolejne konto.',
		today: 'Na dziś założono tu już wystarczająco wiele kont. Spróbuj jutro.',
		captcha: 'Sprawdzanie tej przeglądarki się nie powiodło. Spróbuj ponownie.',
		taken: 'Konto o tej nazwie już istnieje.',
		terms: 'Zaakceptuj regulamin, aby założyć konto.',
		invalid: (reason) => `To nie może być konto. Serwer mówi: ${reason}`,
		email: 'Adres e-mail',
		emailOptional: 'Adres e-mail (opcjonalnie)',
		emailNote: 'Dostaniesz link, aby go potwierdzić.',
		emailRequiredNote:
			'Dostaniesz link, aby go potwierdzić; konto będzie mogło się zalogować po jego otwarciu.',
		emailInvalid: 'Wpisz adres e-mail, na który może przyjść poczta.',
		unsent: 'Nie udało się wysłać linku, więc konta nie założono. Spróbuj ponownie później.',
		tooMany: (count) => `Za dużo prób. Spróbuj ponownie za ${minutes(count)}.`,
		awaitingTitle: 'Sprawdź pocztę',
		awaiting: (address, hours) =>
			`Konto jest założone. Link poszedł na ${address}: otwórz go w ciągu ${counted(hours, 'godziny', 'godzin', 'godzin')}, a potem się zaloguj.`
	},

	forgot: {
		title: 'Nie pamiętasz hasła?',
		intro:
			'Podaj nazwę lub adres e-mail. Link do ustawienia nowego hasła pójdzie na potwierdzony adres.',
		field: 'Nazwa lub adres e-mail',
		submit: 'Wyślij link',
		asked: (count) =>
			`Jeśli takie konto ma potwierdzony adres, link jest już w drodze i jest ważny przez ${minutes(count)}. Sprawdź też folder spamu.`,
		tooMany: (count) => `Za dużo próśb. Spróbuj ponownie za ${minutes(count)}.`,
		back: 'Wróć do logowania'
	},

	reset: {
		title: 'Ustaw nowe hasło',
		intro: 'Wszystkie przeglądarki i urządzenia konta zostaną wylogowane.',
		fresh: 'Nowe hasło',
		again: 'Nowe hasło jeszcze raz',
		submit: 'Ustaw hasło',
		mismatch: 'Hasła się różnią.',
		gone: 'Ten link już nie działa. Poproś o nowy.',
		invalid: (reason) => `Serwer nie przyjął tego hasła. Mówi: ${reason}`,
		done: 'Hasło jest ustawione. Zaloguj się nim.',
		cancel: 'Anuluj'
	},

	email: {
		title: 'Adres e-mail',
		intro: 'rueder pisze do ciebie tylko w sprawie twojego konta.',
		current: 'Twój adres',
		none: 'Konto nie ma adresu.',
		pending: (address, hours) =>
			`Link poszedł na ${address}. Adres zacznie obowiązywać po jego otwarciu, w ciągu ${counted(hours, 'godziny', 'godzin', 'godzin')}; do tego czasu zostaje poprzedni.`,
		address: 'Nowy adres',
		password: 'Twoje hasło',
		passwordNote:
			'Aby nikt inny, kto ma dostęp do tej przeglądarki, nie przekierował twojej poczty.',
		code: 'Kod drugiego składnika',
		submit: 'Wyślij link',
		sent: 'Link jest w drodze. Sprawdź pocztę, także folder spamu.',
		remove: 'Usuń adres',
		removeIntro: 'Bez adresu nic do ciebie nie przyjdzie. Poza tym konto pozostaje bez zmian.',
		removed: 'Adres został usunięty.',
		wrong: 'Hasło lub kod jest nieprawidłowy.',
		codeNeeded: 'To konto ma drugi składnik. Wpisz też jego kod.',
		loginAgain:
			'Twoje konto nie ma jeszcze ani hasła, ani drugiego składnika. Zaloguj się ponownie, a potem podaj adres w ciągu dziesięciu minut.',
		invalid: 'Wpisz adres e-mail, na który może przyjść poczta.',
		same: 'To jest adres, który konto już ma.',
		required: 'Każde konto w tym rueder potrzebuje adresu; można go zmienić, ale nie usunąć.',
		busy: (count) => `Za dużo prób lub wiadomości. Spróbuj ponownie za ${minutes(count)}.`,
		unsent: 'Nie udało się wysłać wiadomości. Spróbuj ponownie później.',
		unreachable: 'Serwer nie odpowiada. Nic nie zostało zmienione.',
		askTitle: 'Dodaj adres e-mail',
		askIntro:
			'Każde konto w tym rueder potrzebuje teraz adresu e-mail. Podaj swój i otwórz link, który na niego przyjdzie.',
		askWaiting:
			'Po otwarciu linku to zniknie samo. Nic nie przyszło? Wyślij ponownie albo na inny adres.',
		logOut: 'Wyloguj się',
		confirmed: (address) => `Adres ${address} jest potwierdzony.`,
		confirmedLogIn: 'Konto jest gotowe: zaloguj się.',
		gone: 'Tego linku nie można już użyć: został użyty, wysłano nowszy albo ma więcej niż dzień.',
		taken: 'W międzyczasie inne konto przejęło ten adres.',
		confirmUnreachable: 'Serwer nie odpowiada. Otwórz link ponownie za chwilę.',
		close: 'Zamknij'
	},

	invites: {
		intro:
			'Link pozwala komuś założyć konto w tym rueder. Jest pokazywany tylko raz, tutaj, gdy go tworzysz.',
		left: (count) =>
			count === 0
				? 'Nie masz zaproszeń do rozdania.'
				: `Możesz zaprosić jeszcze ${counted(count, 'osobę', 'osoby', 'osób')}.`,
		admin: 'Jako admin zawsze możesz zapraszać.',
		uses: 'Na ile kont',
		make: 'Utwórz link',
		noneLeft: 'Nie zostało ci tyle zaproszeń.',
		failed: 'Nie udało się. Spróbuj ponownie za chwilę.',
		link: 'Link do przekazania teraz',
		copy: 'Kopiuj',
		copied: 'Skopiowano',
		goodFor: (days) => `Link jest ważny przez ${counted(days, 'dzień', 'dni', 'dni')}.`,
		yours: 'Twoje linki',
		none: 'Nie masz jeszcze żadnych linków.',
		used: (uses, of) => `użyto ${n(uses)} z ${n(of)}`,
		until: (when) => `ważny do ${when}`,
		ran: (when) => `wygasł ${when}`,
		usedUp: 'wykorzystany',
		revoked: 'wycofany',
		revoke: 'Wycofaj',
		revokeLink: (made) => `Wycofaj link utworzony ${made}`
	},

	home: {
		newIn: 'Nowości w subskrypcjach',
		allRead: 'Wszystko przeczytane. Nowe artykuły pojawią się tu, gdy przyjdą.',
		recentlyStarred: 'Ostatnio z gwiazdką',
		noStars: 'Nic jeszcze nie ma gwiazdki. Naciśnij s na artykule.',
		problems: 'Kanały z problemami',
		tips: 'Wskazówki',
		tipKeys:
			'Naciśnij ?, aby zobaczyć wszystkie skróty klawiszowe; j i k przechodzą między artykułami.',
		tipDrag: 'Przeciągaj kanały i foldery na pasku bocznym, aby je posortować.',
		tipFilters:
			'Reguły filtrów w ustawieniach ukrywają artykuły według słowa kluczowego lub autora.',
		showFeed: (title) => `Pokaż ${title}`
	},

	versions: {
		title: 'Wersje tego artykułu',
		current: 'Obecna wersja',
		version: (rev) => `Wersja ${n(rev)}`,
		fetched: (when) => `pobrana ${when}`,
		compareWith: 'Porównaj obecną wersję z',
		legendAdded: 'dodano',
		legendRemoved: 'usunięto',
		titleWas: 'Tytuł brzmiał:',
		same: 'Tekst tej wersji jest taki sam jak obecny.',
		sameWords: 'Słowa są te same; zmieniły się obrazy lub linki.',
		markup: 'Słowa, obrazy i linki są te same; zmienił się tylko układ tekstu.',
		pictures: 'Zmienione obrazy',
		noPicture: 'Brak',
		address: 'Adres obrazu',
		links: 'Zmienione linki'
	},

	account: {
		title: 'Ustawienia konta',
		tabs: {
			password: 'Hasło',
			email: 'E-mail',
			security: 'Bezpieczeństwo',
			apps: 'Aplikacje',
			invites: 'Zaproszenia',
			move: 'Przeprowadzka',
			privacy: 'Prywatność',
			remove: 'Usuń konto'
		},
		password: {
			title: 'Zmień hasło',
			titleFirst: 'Ustaw hasło',
			intro:
				'Nowe hasło wylogowuje wszystkie inne przeglądarki i urządzenia oraz unieważnia wszystkie hasła aplikacji.',
			introFirst:
				'Z hasłem to konto może logować się także nazwą, nie tylko przez logowanie jednokrotne.',
			current: 'Obecne hasło',
			fresh: 'Nowe hasło',
			again: 'Nowe hasło jeszcze raz',
			submit: 'Zmień hasło',
			submitFirst: 'Ustaw hasło',
			done: 'Hasło zostało zmienione. Wszystkie inne przeglądarki i urządzenia są wylogowane, a hasła aplikacji unieważnione.',
			doneFirst: 'Hasło jest ustawione.',
			mismatch: 'Hasła się różnią.',
			code: 'Kod drugiego składnika',
			wrong: 'Obecne hasło lub kod jest nieprawidłowy.',
			codeNeeded:
				'To konto ma drugi składnik. Wpisz też kod z aplikacji uwierzytelniającej albo kod odzyskiwania lub potwierdź kluczem dostępu.',
			loginAgain:
				'Twoje konto nie ma jeszcze ani hasła, ani drugiego składnika. Zaloguj się ponownie, a potem ustaw hasło w ciągu dziesięciu minut.',
			invalid: (characters) =>
				`Serwer nie przyjął tego hasła. Co najmniej ${n(characters)} znaków.`,
			busy: (count) => `Za dużo błędnych prób. Spróbuj ponownie za ${minutes(count)}.`,
			unreachable: 'Serwer nie odpowiada. Hasło się nie zmieniło.'
		},
		settingsNote: 'Wygląd, czytanie, reguły filtrów i klawiatura są w Ustawieniach.'
	},

	settings: {
		title: 'Ustawienia',
		tabs: {
			look: 'Wygląd',
			reading: 'Czytanie',
			rules: 'Reguły filtrów',
			keys: 'Klawiatura'
		},
		accountNote:
			'Hasło, drugi składnik, aplikacje i zaproszenia są w Ustawieniach konta, w menu twojego konta.',
		theme: 'Motyw',
		themes: { system: 'Jak system', light: 'Jasny', dark: 'Ciemny' },
		accent: 'Kolor akcentu',
		accents: {
			red: 'Czerwony',
			blue: 'Niebieski',
			green: 'Zielony',
			orange: 'Pomarańczowy',
			purple: 'Fioletowy',
			graphite: 'Grafitowy'
		},
		density: 'Gęstość listy',
		densities: { compact: 'Zwarta', comfortable: 'Wygodna' },
		sidebarSide: 'Pasek boczny',
		sidebarSides: { start: 'Po lewej', end: 'Po prawej' },
		textSize: 'Rozmiar tekstu artykułów, na tym urządzeniu',
		textSizes: { small: 'Mały', medium: 'Średni', large: 'Duży' },
		readingFont: 'Czcionka artykułów',
		readingFonts: { sans: 'Bezszeryfowa', serif: 'Szeryfowa' },
		counters: 'Liczniki nieprzeczytanych',
		counterStyles: { parens: 'W nawiasach', badge: 'Jako plakietki' },
		language: 'Język',
		languageAuto: 'Jak przeglądarka',
		view: 'Widok',
		views: { list: 'Lista', expanded: 'Rozwinięty', split: 'Trzy panele', magazine: 'Magazyn' },
		markOnScroll: 'Oznaczaj artykuły jako przeczytane przy przewijaniu w widoku rozwiniętym',
		teasers: 'Pokazuj na liście pierwsze słowa tekstu po tytule',
		wakeLock: 'Nie wygaszaj ekranu, gdy rueder jest otwarty',
		wakeLockMissing: 'Ta przeglądarka nie potrafi utrzymać włączonego ekranu.',
		note: 'Zmiany działają od razu i zapisują się na koncie, więc są na każdym urządzeniu.',
		noteDemo: 'Zmiany działają od razu i zostają w tej przeglądarce.',
		apps: {
			intro:
				'Inne aplikacje do czytania kanałów mogą korzystać z twojego konta przez API Google Reader. Każda dostaje własne hasło aplikacji, które możesz w każdej chwili unieważnić.',
			none: 'Nie skonfigurowano jeszcze żadnej aplikacji.',
			made: (when) => `skonfigurowana ${when}`,
			used: (when) => `ostatnio użyta ${when}`,
			unused: 'jeszcze nieużywana',
			revoke: 'Unieważnij',
			revokeName: (name) => `Unieważnij hasło aplikacji ${name}`,
			add: 'Skonfiguruj aplikację',
			full: (max) =>
				`Konto może mieć najwyżej ${n(max)} haseł aplikacji. Unieważnij jedno, aby skonfigurować kolejną aplikację.`,
			choose: 'Której aplikacji chcesz używać?',
			website: (app) => `Strona aplikacji ${app}`,
			other: 'Inna aplikacja',
			otherHint: 'Każda aplikacja, która obsługuje API Google Reader lub FreshRSS.',
			platforms: {
				apple: 'Mac, iPhone i iPad',
				android: 'Android'
			},
			name: 'Nazwa hasła aplikacji',
			password: 'Twoje hasło',
			code: 'Kod z aplikacji uwierzytelniającej albo kod odzyskiwania',
			create: 'Utwórz hasło aplikacji',
			creating: 'Chwileczkę …',
			back: 'Wstecz',
			how: {
				netnewswire:
					'W NetNewsWire otwórz Ustawienia, potem Konta, dodaj konto (na Macu przez +) i wybierz FreshRSS. Wpisz poniższą nazwę jako Login (na iPhonie i iPadzie: Username or Email), hasło aplikacji jako Password, a adres serwera jako API URL.',
				readyou:
					'W Read You otwórz Ustawienia, Konta, Dodaj konta i wybierz Google Reader. Wpisz poniższy adres serwera jako adres URL serwera, nazwę jako nazwę użytkownika, a hasło aplikacji jako hasło.',
				other:
					'W swojej aplikacji dodaj konto typu „Google Reader API” albo, jeśli go nie ma, „FreshRSS”. Wpisz poniższy adres serwera, nazwę i hasło aplikacji.'
			},
			ownIcons: (app) =>
				`Prywatność: ${app} sama wczytuje ikony twoich kanałów ze stron tych kanałów, które widzą wtedy twój adres IP. Artykuły i ich obrazy przychodzą tylko przez rueder.`,
			shotsTitle: 'Krok po kroku, z obrazkami',
			shots: {
				readyou: [
					'Stuknij koło zębate w lewym górnym rogu.',
					'Otwórz Konta.',
					'Stuknij Dodaj konta.',
					'Wybierz Google Reader.',
					'Wpisz adres serwera jako adres URL serwera, nazwę jako nazwę użytkownika i hasło aplikacji jako hasło, a potem stuknij Dodaj.'
				],
				netnewswire: [
					'Na Macu: otwórz Ustawienia (⌘,), potem Konta i kliknij + w lewym dolnym rogu.',
					'Wybierz FreshRSS i kliknij Kontynuuj.',
					'Wpisz nazwę jako Login, hasło aplikacji jako Password i adres serwera jako API URL, a potem kliknij Utwórz.'
				]
			},
			shotAlt: (app, step) => `${app}, krok ${n(step)}`,
			server: 'Adres serwera',
			user: 'Nazwa',
			secret: 'Hasło aplikacji',
			copy: (what) => `Kopiuj: ${what}`,
			copied: 'Skopiowano',
			once: 'Hasło aplikacji jest pokazywane tylko teraz. Jeśli je zgubisz, unieważnij je i utwórz nowe.',
			waiting: 'Czekanie, aż aplikacja się zaloguje …',
			connected: 'Połączono: aplikacja się zalogowała.',
			done: 'Gotowe',
			refusals: {
				wrongPassword: 'Hasło jest nieprawidłowe.',
				codeNeeded:
					'Wpisz też kod z aplikacji uwierzytelniającej albo kod odzyskiwania lub potwierdź kluczem dostępu.',
				wrongCode: 'Ten kod jest nieprawidłowy. Kody zmieniają się co 30 sekund.',
				loginAgain:
					'Twoje konto nie ma jeszcze ani hasła, ani drugiego składnika. Zaloguj się ponownie, a potem utwórz hasło aplikacji w ciągu dziesięciu minut.',
				tooManyApps: 'Konto może mieć najwyżej 20 haseł aplikacji.',
				tooMany: 'Za dużo błędnych prób. Spróbuj ponownie za kilka minut.',
				unreachable: 'Serwer nie odpowiada. Spróbuj ponownie za chwilę.'
			}
		},
		secondFactor: {
			title: 'Drugi składnik',
			intro:
				'Z drugim składnikiem logowanie wymaga też kodu z aplikacji uwierzytelniającej albo klucza dostępu.',
			app: 'Aplikacja uwierzytelniająca',
			on: 'Włączony.',
			left: (count) =>
				`Pozostało: ${counted(count, 'kod odzyskiwania', 'kody odzyskiwania', 'kodów odzyskiwania')}.`,
			off: 'Wyłączony.',
			byPasskeys: 'Nieskonfigurowana. Twoje klucze dostępu utrzymują drugi składnik.',
			adminNeeds:
				'Ten serwer uznaje adminów za adminów tylko z drugim składnikiem. Skonfiguruj go albo zaloguj się ponownie z jego kodem, aby zobaczyć stan serwera.',
			password: 'Twoje hasło',
			start: 'Skonfiguruj',
			scan: 'Zeskanuj ten kod aplikacją uwierzytelniającą albo wpisz poniższy klucz.',
			qr: 'Kod QR dla aplikacji uwierzytelniającej',
			larger: 'Pokaż większy kod QR',
			smaller: 'Pokaż mniejszy kod QR',
			key: 'Klucz',
			copyKey: 'Kopiuj klucz',
			copied: 'Skopiowano',
			code: 'Kod, który aplikacja pokazuje teraz',
			codeOrRecovery: 'Kod z aplikacji albo kod odzyskiwania',
			confirm: 'Potwierdź',
			cancel: 'Anuluj',
			codesTitle: 'Twoje kody odzyskiwania',
			codesIntro:
				'Każdy pozwala raz zalogować się bez telefonu. Przechowuj je bezpiecznie: są pokazywane tylko teraz.',
			copyCodes: 'Kopiuj kody',
			codesDone: 'Zachowane',
			switchOff: 'Wyłącz',
			switchOffNote: 'Do logowania znów wystarczy samo hasło.',
			switchOffNoteKeys: 'Twoje klucze dostępu utrzymują drugi składnik.',
			startWithPasskey: 'Skonfiguruj kluczem dostępu',
			switchOffWithPasskey: 'Wyłącz kluczem dostępu',
			refusals: {
				wrongPassword: 'Hasło jest nieprawidłowe.',
				codeNeeded: 'Twoje klucze dostępu utrzymują drugi składnik. Wpisz też kod odzyskiwania.',
				wrongCode: 'Ten kod jest nieprawidłowy. Kody zmieniają się co 30 sekund.',
				loginAgain:
					'Twoje konto nie ma jeszcze ani hasła, ani drugiego składnika. Zaloguj się ponownie, a potem skonfiguruj aplikację w ciągu dziesięciu minut.',
				tooMany: 'Za dużo błędnych prób. Spróbuj ponownie za kilka minut.',
				unreachable: 'Serwer nie odpowiada. Spróbuj ponownie za chwilę.'
			}
		},
		passkeys: {
			title: 'Klucze dostępu',
			intro:
				'Klucz dostępu loguje cię bez nazwy i hasła, za pomocą PIN-u, odcisku palca lub twarzy, o które prosi urządzenie. Liczy się jako drugi składnik.',
			once: 'Z kluczem dostępu samo hasło nie otwiera już twojego konta.',
			none: 'Nie ma jeszcze kluczy dostępu.',
			made: (when) => `dodany ${when}`,
			used: (when) => `ostatnio użyty ${when}`,
			unused: 'jeszcze nieużywany',
			add: 'Dodaj klucz dostępu',
			name: 'Nazwa, np. urządzenia',
			defaultName: 'Klucz dostępu',
			password: 'Twoje hasło',
			code: 'Kod z aplikacji uwierzytelniającej albo kod odzyskiwania',
			create: 'Utwórz klucz dostępu',
			working: 'Postępuj według przeglądarki …',
			cancel: 'Anuluj',
			remove: 'Usuń',
			removeName: (name) => `Usuń klucz dostępu ${name}`,
			confirmRemove: 'Usuń klucz dostępu',
			full: (max) => `Konto może mieć najwyżej ${n(max)} kluczy dostępu.`,
			or: 'lub',
			confirmWith: 'Potwierdź kluczem dostępu',
			codesTitle: 'Twoje kody odzyskiwania',
			codesIntro:
				'Twój pierwszy klucz dostępu włączył drugi składnik. Każdy kod pozwala raz się zalogować, jeśli zgubisz urządzenie. Przechowuj je bezpiecznie: są pokazywane tylko teraz.',
			copyCodes: 'Kopiuj kody',
			copied: 'Skopiowano',
			codesDone: 'Zachowane',
			refusals: {
				cancelled: 'Okno przeglądarki zostało zamknięte, zanim użyto klucza dostępu.',
				known: 'To urządzenie ma już klucz dostępu do twojego konta.',
				failed: 'Ta przeglądarka lub to urządzenie nie mogło utworzyć ani użyć klucza dostępu.',
				wrong: 'Klucz dostępu nie został przyjęty. Spróbuj ponownie.',
				wrongPassword: 'Hasło jest nieprawidłowe.',
				codeNeeded:
					'Twoje konto ma drugi składnik. Wpisz też kod z aplikacji albo kod odzyskiwania lub potwierdź kluczem dostępu.',
				wrongCode: 'Ten kod jest nieprawidłowy. Kody zmieniają się co 30 sekund.',
				tooManyPasskeys: 'Konto może mieć najwyżej 10 kluczy dostępu.',
				loginAgain:
					'Twoje konto nie ma jeszcze ani hasła, ani drugiego składnika. Zaloguj się ponownie, a potem dodaj pierwszy klucz dostępu w ciągu dziesięciu minut.',
				tooMany: 'Za dużo błędnych prób. Spróbuj ponownie za kilka minut.',
				unreachable: 'Serwer nie odpowiada. Spróbuj ponownie za chwilę.'
			}
		},
		reports: {
			send: 'Wysyłaj do tego serwera raporty o błędach z tej przeglądarki',
			intro: 'Gdy coś na stronie pójdzie nie tak, admini tego serwera się o tym dowiedzą.',
			what: 'Raport zawiera:',
			holds: [
				'komunikat błędu, bez adresów i bez liczb, które coś identyfikują',
				'miejsce w plikach rueder, w którym wystąpił',
				'wersję rueder oraz twoją przeglądarkę z jej wersją główną'
			],
			never:
				'Nigdy tego, co czytasz, twoich kanałów, etykiet ani reguł. Raporty znikają po 30 dniach.',
			off: 'Wyłączenie tego usuwa wysłane przez ciebie raporty.'
		}
	},

	rules: {
		intro:
			'Reguły ukrywają artykuły albo oznaczają je jako przeczytane, gdy przychodzą. Ukryte artykuły nie liczą się jako nieprzeczytane i zostają w „Ukryte przez filtry”.',
		none: 'Nie ma jeszcze reguł.',
		add: 'Dodaj regułę',
		new: 'Nowa reguła',
		on: 'Włączona',
		scope: 'W',
		scopeAll: 'Wszystkie kanały',
		scopeFolder: (title) => `Folder: ${title}`,
		scopeFeed: (title) => `Kanał: ${title}`,
		field: 'Sprawdzaj',
		fields: { any: 'Wszystko', title: 'Tytuł', content: 'Tekst', author: 'Autor', tag: 'Tag' },
		match: 'Porównanie',
		matches: {
			contains: 'zawiera',
			word: 'zawiera słowo',
			starts: 'zaczyna się od',
			ends: 'kończy się na',
			regex: 'pasuje do wyrażenia regularnego',
			all: 'każdy artykuł, bez względu na treść'
		},
		pattern: 'Szukany tekst',
		action: 'Wtedy',
		actions: { hide: 'ukryj artykuł', read: 'oznacz jako przeczytany' },
		preview: (count, total) => `Pasuje teraz do ${n(count)} z ${n(total)} artykułów tutaj.`,
		regexTip: 'Wyrażenia regularne są czytane jak w Ruście, bez rozróżniania wielkości liter.',
		regexBuilder: 'Zbuduj i wypróbuj je w regex101',
		invalid: 'To nie jest prawidłowe wyrażenie regularne.',
		empty: 'Wpisz szukany tekst.',
		checking: 'Sprawdzanie, co by złapała …',
		hits: (count) => counted(count, 'trafienie', 'trafienia', 'trafień'),
		summary: (field, match, pattern, action) => `${field} ${match} „${pattern}”: ${action}`,
		summaryAll: (action) => `Każdy artykuł: ${action}`
	},

	keys: {
		title: 'Skróty klawiszowe',
		note: 'Twoje klawisze są zapisywane z ustawieniami. Enter, Escape i g zostają bez zmian.',
		change: (what) => `Zmień klawisz dla: ${what}`,
		changeShort: 'Zmień',
		press: 'Naciśnij klawisz …',
		taken: (key, other) => `${key} jest już klawiszem dla: ${other}`,
		fixed: (key) => `${key} nie może być przypisany do skrótu.`,
		reset: 'Przywróć klawisze rueder',
		groups: { articles: 'Artykuły', lists: 'Listy', go: 'Przejdź do', app: 'Wszędzie' },
		next: 'Następny artykuł, otwarty',
		previous: 'Poprzedni artykuł, otwarty',
		nextCursor: 'Następny artykuł, bez otwierania',
		previousCursor: 'Poprzedni artykuł, bez otwierania',
		toggle: 'Otwórz lub zamknij artykuł',
		star: 'Dodaj lub usuń gwiazdkę',
		read: 'Oznacz jako przeczytany lub nieprzeczytany',
		original: 'Otwórz oryginał w nowej karcie',
		markAll: 'Oznacz wszystko jako przeczytane',
		refresh: 'Odśwież listę',
		unreadOnly: 'Tylko nieprzeczytane albo wszystkie artykuły',
		listView: 'Widok listy',
		expandedView: 'Widok rozwinięty',
		splitView: 'Trzy panele',
		magazineView: 'Magazyn',
		goHome: 'Start',
		goAll: 'Wszystkie artykuły',
		goUnread: 'Nieprzeczytane',
		goStarred: 'Z gwiazdką',
		search: 'Szukaj',
		sidebar: 'Pokaż lub ukryj pasek boczny',
		help: 'Ten przegląd',
		then: 'potem'
	},

	labels: {
		title: 'Etykiety',
		add: 'Nowa etykieta',
		name: 'Nazwa etykiety',
		none: 'Nie ma jeszcze etykiet.',
		actions: (title) => `Działania dla etykiety ${title}`,
		rename: 'Zmień nazwę',
		color: 'Kolor',
		colors: [
			'Czerwony',
			'Pomarańczowy',
			'Żółty',
			'Zielony',
			'Turkusowy',
			'Niebieski',
			'Fioletowy',
			'Szary'
		],
		remove: 'Usuń etykietę',
		show: 'Pokaż etykiety',
		hide: 'Ukryj etykiety',
		forArticle: 'Etykiety',
		set: (title) => `Etykieta: ${title}`
	},

	subscribe: {
		title: 'Subskrybuj kanał',
		address: 'Adres kanału lub strony',
		find: 'Znajdź kanały',
		finding: 'Szukanie …',
		note: 'Szkic nie może niczego pobrać, więc kanał zostaje pusty.',
		found: (count) =>
			count === 1
				? 'Jest jeden kanał:'
				: `${forms(count, 'Jest', 'Są', 'Jest')} ${counted(count, 'kanał', 'kanały', 'kanałów')}:`,
		none: 'Pod tym adresem nie ma kanału.',
		movedTo: (url) => `Strona przekazała żądanie dalej do ${url}.`,
		loginWall:
			'Niektóre strony kierują niezalogowanych na stronę logowania. Ich kanałów nie da się wtedy czytać.',
		subscribed: 'Subskrybowany',
		show: 'Pokaż',
		untitled: 'Bez nazwy',
		page: 'Strona bez kanału: rueder robi z niej kanał, tylko z tytułami i streszczeniami.',
		folder: 'Do folderu',
		topLevel: 'Bez folderu',
		add: 'Subskrybuj',
		adding: 'Subskrybowanie …',
		refused: 'Nie udało się.',
		serverSays: (reason) => `Serwer mówi: ${reason}`,
		busy: (seconds) =>
			seconds
				? `Za dużo wyszukiwań naraz. Spróbuj ponownie za ${counted(seconds, 'sekundę', 'sekundy', 'sekund')}.`
				: 'Za dużo wyszukiwań naraz. Spróbuj ponownie za chwilę.',
		noFetching: 'Ten serwer nie pobiera kanałów.',
		tooLarge: (megabytes) => `Plik jest większy niż ${n(megabytes)} MB.`,
		unreachable: 'Serwer nie odpowiada. Spróbuj ponownie za chwilę.'
	},

	opml: {
		title: 'Import i eksport',
		intro: 'Przenieś subskrypcje z innego czytnika jako plik OPML albo zabierz je stąd.',
		import: 'Importuj plik OPML',
		importing: 'Importowanie …',
		imported: (feeds, folders) => `Nowe kanały: ${n(feeds)}, nowe foldery: ${n(folders)}.`,
		known: (count) => `Już subskrybowane kanały: ${n(count)}.`,
		skipped: (count) => `Pominięte wpisy, które nie są czytelnymi kanałami: ${n(count)}.`,
		later: 'Ich artykuły przyjdą w następnej rundzie pobierania.',
		export: 'Eksportuj jako OPML'
	},

	removeAccount: {
		title: 'Usuń konto',
		intro: 'To usuwa twoje konto w tym rueder na zawsze. Nie da się tego cofnąć.',
		goes: 'Razem z nim znikają: sesje na wszystkich urządzeniach, subskrypcje i foldery, stan przeczytania, gwiazdki, etykiety, reguły filtrów, ustawienia, raporty o błędach, zaproszenia i połączenie z logowaniem jednokrotnym.',
		stays:
			'Kanały zostają dla innych, którzy je czytają. Aby najpierw przenieść konto gdzie indziej, użyj „Przeprowadzki”.',
		password: 'Twoje hasło',
		name: (name) => `Wpisz swoją nazwę, ${name}, aby potwierdzić`,
		code: 'Kod z aplikacji albo kod odzyskiwania',
		submit: 'Usuń moje konto na zawsze',
		wrong: 'To się nie zgadza. Nic nie zostało usunięte.',
		codeNeeded:
			'Twoje konto ma drugi składnik. Wpisz też kod z aplikacji albo kod odzyskiwania lub potwierdź kluczem dostępu.',
		busy: (count) => `Za dużo błędnych prób. Spróbuj ponownie za ${minutes(count)}.`,
		unreachable: 'Serwer nie odpowiada. Nic nie zostało usunięte.'
	},

	move: {
		intro: 'Przenieś konto do innego rueder albo sprowadź je tu z innego, w jednym pliku.',
		howItWorks:
			'Import dodaje do tego, co tu jest, i niczego nie zabiera. Twoje ustawienia stają się ustawieniami z pliku.',
		notInIt: 'Nie ma w pliku: twojego hasła, sesji ani tekstów artykułów.',
		export: 'Eksportuj konto',
		import: 'Importuj plik konta',
		importing: 'Importowanie …',
		feeds: (feeds, known) => `Nowe kanały: ${n(feeds)}, już subskrybowane: ${n(known)}.`,
		folders: (count) => `Nowe foldery: ${n(count)}.`,
		articles: (found) => `Znalezione tu artykuły: ${n(found)}.`,
		missing: (count) =>
			`Nie znaleziono tu artykułów: ${n(count)}; ich stan przeczytania, gwiazdki i etykiety przepadają.`,
		marks: (labels, rules) => `Nowe etykiety: ${n(labels)}, nowe reguły filtrów: ${n(rules)}.`,
		skipped: (count) => `Wpisy z pliku, których nie dało się przyjąć: ${n(count)}.`,
		settings: 'Twoje ustawienia są teraz ustawieniami z pliku.',
		later:
			'Nowe kanały przyniosą swoje artykuły w następnej rundzie pobierania. To, co przeczytano, zostaje przeczytane.',
		busy: (seconds) =>
			seconds
				? `Za dużo importów. Spróbuj ponownie za ${minutes(Math.ceil(seconds / 60))}.`
				: 'Za dużo importów. Spróbuj ponownie później.'
	},

	status: {
		title: 'Stan serwera',
		open: 'Stan serwera',
		loading: 'Pytanie serwera …',
		failed: 'Serwer nie odpowiedział. Spróbuj ponownie za chwilę.',
		refresh: 'Odśwież',
		asOf: (time) => `Stan na ${time}. Liczy się ostatnia godzina.`,
		demo: 'Zmyślone, bo nie ma tu serwera.',
		empty: '–',
		on: 'Włączone',
		off: 'Wyłączone',
		took: (median, p95) => `mediana ${median}, 95% w ciągu ${p95}`,
		summary: {
			label: 'W skrócie',
			hints: 'Uwagi',
			failing: 'Kanały z błędami',
			errors: 'Zapisane błędy',
			reported: 'Zgłoszone kanały'
		},
		hintsTitle: 'Warto sprawdzić',
		allWell: 'Nic się nie wyróżnia.',
		hints: {
			slowRoute: (route, took) =>
				`${route} działa wolno: jedno żądanie na dwadzieścia trwa dłużej niż ${took}.`,
			serverErrors: (route, count) =>
				`${route} odpowiedziała błędem serwera ${counted(count, 'raz', 'razy', 'razy')}.`,
			poolBusy: (size) =>
				`Wszystkie połączenia z bazą danych (${n(size)}) są zajęte, więc żądania na nie czekają.`,
			fetchBacklog: (count, late) =>
				`Kanały pobierane z opóźnieniem: ${n(count)}, najbardziej spóźniony o ${late}.`,
			feedsFailing: (failing, subscribed) => `Z ${n(subscribed)} kanałów nie działa ${n(failing)}.`,
			notFetching: 'Serwer nie pobiera kanałów. Czy FETCH jest ustawione na off?',
			fetchPaceFull: (most) =>
				`Każda runda bierze tyle kanałów, ile może, a kanały nadal czekają: tempo pozwala na ${n(most)} pobrań na godzinę. Ustawiają je FETCH_ROUND, FETCH_AT_ONCE i FETCH_EVERY.`,
			cacheMisses: (share) =>
				`Baza danych znajduje w pamięci tylko ${share} tego, co czyta; więcej shared_buffers by pomogło.`,
			databaseLarge: (size, warn) =>
				`Baza danych urosła do ${size}; DATABASE_WARN_GB ostrzega od ${warn}. „rueder retention set” usuwa stare artykuły albo ich teksty.`,
			longQuery: (took) => `Zapytanie trwa już ${took}.`,
			imageCacheFull: (share) => `Pamięć podręczna obrazów jest zapełniona w ${share}.`,
			errors: (count) => `Błędy zapisane przez serwer w ostatniej godzinie: ${n(count)}.`,
			limited: (count) => `Żądania odrzucone jako zbyt liczne: ${n(count)}.`,
			browserErrors: (count) =>
				`Różne błędy zgłoszone przez przeglądarki w ostatniej dobie: ${n(count)}.`,
			reportedFeeds: (count) =>
				`Zgłoszone przez czytelników kanały, które nie są zablokowane: ${n(count)}.`
		},
		build: {
			title: 'Kompilacja',
			commit: 'Commit',
			dirty: (commit) => `${commit}, z własnymi zmianami`,
			branch: 'Gałąź',
			tag: 'Tag',
			committed: 'Data commitu',
			where: 'Działa',
			container: 'w kontenerze',
			host: 'bezpośrednio na maszynie',
			target: 'Dla',
			profile: 'Skompilowano',
			profileValue: (profile, level) => `${profile}, optymalizacja ${level}`,
			ci: 'Zbudowane przez',
			ciValue: 'przebieg CI',
			byHand: 'ręcznie'
		},
		server: {
			title: 'Serwer',
			version: 'Wersja',
			running: 'Działa od',
			memory: 'Pamięć',
			accounts: 'Konta',
			feeds: 'Kanały',
			feedsValue: (fetched, all) => `pobierane: ${n(fetched)}, łącznie: ${n(all)}`,
			articles: 'Artykuły',
			articlesValue: (count) => `około ${n(count)}`,
			live: 'Otwarte karty'
		},
		database: {
			title: 'Baza danych',
			pool: 'Połączenia rueder',
			poolValue: (size, idle, max) =>
				`otwarte: ${n(size)}, bezczynne: ${n(idle)}, najwyżej ${n(max)}`,
			connections: 'Wszystkie połączenia',
			size: 'Rozmiar',
			sizeWarn: (size, warn) => `${size}, ostrzeżenie od ${warn}`,
			hitRatio: 'Odczytane z pamięci',
			deadlocks: 'Zakleszczenia',
			longest: 'Najdłużej trwające zapytanie',
			tables: 'Największe tabele',
			table: 'Tabela',
			bytes: 'Rozmiar',
			rows: 'Wiersze, około'
		},
		fetcher: {
			title: 'Pobieranie kanałów',
			state: 'Pobieranie',
			lastRound: 'Ostatnia runda',
			lastRoundValue: (ago, feeds, took) =>
				`${ago}: ${counted(feeds, 'kanał', 'kanały', 'kanałów')} w ${took}`,
			fetches: 'Pobrania',
			fetchesValue: (fetched, unchanged, failed) =>
				`z kanałem: ${n(fetched)}, bez zmian: ${n(unchanged)}, nieudane: ${n(failed)}`,
			broken: 'Nieudane w bazie danych',
			newArticles: 'Nowe artykuły',
			took: 'Czas na pobranie',
			overdue: 'Pobrane z opóźnieniem',
			overdueValue: (count, late) => `${n(count)}, najbardziej spóźniony o ${late}`,
			waiting: 'Jeszcze nie pobrane',
			pace: 'Tempo',
			paceValue: (round, every, atOnce, most) =>
				`${counted(round, 'kanał', 'kanały', 'kanałów')} co ${every}, ${n(atOnce)} naraz: najwyżej ${n(most)} na godzinę`,
			perHour: 'Pobrania w ostatniej godzinie',
			behind: 'Opóźnienie',
			behindValue: (late) => `kanał czekający najdłużej jest spóźniony o ${late}`,
			notBehind: 'Nic nie czeka',
			failing: 'Kanały z błędami, według przyczyny',
			noneFailing: 'Żaden kanał nie zawodzi.',
			feeds: (count) => counted(count, 'kanał', 'kanały', 'kanałów'),
			failures: {
				status: (code) => (code ? `Strona odpowiada ${code}` : 'Strona odpowiada błędem'),
				timeout: 'Strona odpowiada za długo',
				blocked: 'Adres jest w sieci, z której rueder nie pobiera',
				dns: 'Nazwa nie ma adresu',
				network: 'Brak połączenia albo zostało przerwane',
				too_large: 'Za duży',
				redirects: 'Za dużo przekierowań albo jedno zepsute',
				unreadable: 'To nie kanał, który rueder potrafi przeczytać',
				other: 'Coś innego'
			}
		},
		finder: {
			title: 'Wyszukiwanie kanałów',
			state: 'Sprawdzanie adresów',
			lookups: 'Wyszukiwania',
			lookupsValue: (found, none, failed) =>
				`znalazły kanały: ${n(found)}, nic nie znalazły: ${n(none)}, nieudane: ${n(failed)}`,
			limited: 'Ponad limit konta',
			took: 'Czas na wyszukiwanie'
		},
		rssifier: {
			title: 'Strony bez kanału',
			state: 'RSSifier',
			pages: 'Czytane strony',
			pagesValue: (pages, failing) => `${n(pages)}, z błędami: ${n(failing)}`,
			used: 'Zużycie tokenu',
			usedValue: (requests, fetches, underWay) =>
				`żądania w ostatniej minucie: ${n(requests)}, strony pobrane w ostatniej godzinie: ${n(fetches)}, w toku: ${n(underWay)}`,
			unanswered: (why) => `RSSifier nie odpowiada: ${why}`,
			limits: 'Limity tokenu',
			limitsValue: (perMinute, perHour, atOnce) =>
				`na minutę: ${perMinute}, stron na godzinę: ${perHour}, naraz: ${atOnce}`,
			noLimit: 'bez limitu'
		},
		images: {
			title: 'Proxy obrazów',
			state: 'Proxy',
			cache: 'Pamięć podręczna',
			cacheValue: (used, max) => `${used} z ${max}`,
			served: 'Obrazy',
			servedValue: (cached, fetched) =>
				`z pamięci podręcznej: ${n(cached)}, pobrane: ${n(fetched)}`,
			refused: 'Odrzucone',
			refusedValue: (source, notImage, tooLarge, notFound) =>
				`błąd źródła: ${n(source)}, nie obraz: ${n(notImage)}, za duże: ${n(tooLarge)}, zły adres: ${n(notFound)}`,
			took: 'Czas na pobrany obraz',
			failingSources: 'Źródła na razie pomijane'
		},
		requests: {
			title: 'Żądania',
			all: 'Wszystkie żądania',
			allValue: (count, took) => `${n(count)}, ${took}`,
			answers: 'Odpowiedzi',
			answersValue: (fine, refused, errors) =>
				`w porządku: ${n(fine)}, odrzucone: ${n(refused)}, błędy serwera: ${n(errors)}`,
			route: 'Trasa',
			count: 'Liczba',
			median: 'Mediana',
			p95: '95%',
			max: 'Najdłuższe',
			errors: 'Błędy',
			none: 'Brak żądań w ostatniej godzinie.'
		},
		problems: {
			title: 'Ostrzeżenia i błędy',
			counts: (errors, warnings) =>
				`W ostatniej godzinie błędy: ${n(errors)}, ostrzeżenia: ${n(warnings)}.`,
			none: 'Od startu nic nie zapisano.',
			error: 'Błąd',
			warning: 'Ostrzeżenie',
			times: (count, last) =>
				count > 1 ? `${counted(count, 'raz', 'razy', 'razy')}, ostatnio ${last}` : last
		},
		browsers: {
			title: 'Błędy w przeglądarkach',
			off: 'Ten serwer nie przyjmuje raportów o błędach (CLIENT_REPORTS=off).',
			consenting: (count) =>
				`Konta wysyłające raporty: ${n(count)}. Błędy zostają 30 dni od ostatniego wystąpienia.`,
			none: 'Żadna przeglądarka nie zgłosiła błędu.',
			kinds: {
				error: 'Błąd',
				rejection: 'Odrzucona obietnica',
				component: 'Część strony',
				request: 'Żądanie'
			},
			hits: (count, accounts, last) =>
				`${counted(count, 'raz', 'razy', 'razy')} na kontach: ${n(accounts)}, ostatnio ${last}`,
			where: (versions, browsers) => `Wersja ${versions} · ${browsers}`
		},
		reported: {
			title: 'Zgłoszone kanały',
			text: 'Nie widać tu, kto zgłosił. „rueder feed block <adres>” blokuje kanał dla wszystkich, „rueder feed unblock <adres>” zdejmuje blokadę.',
			none: 'Nikt nie zgłosił kanału.',
			reports: (count, last) =>
				`${counted(count, 'zgłoszenie', 'zgłoszenia', 'zgłoszeń')}, ostatnio ${last}`,
			blocked: 'Zablokowany'
		}
	},

	update: {
		ready: 'Nowa wersja rueder jest gotowa.',
		reload: 'Wczytaj ponownie',
		later: 'Później'
	},

	crash: {
		title: 'Na tej stronie coś poszło nie tak.',
		text: 'Wczytaj stronę ponownie, aby kontynuować. To, co przeczytano i zmieniono, jest zapisane na serwerze.',
		reload: 'Wczytaj ponownie'
	},

	demo: {
		badge: 'Szkic',
		note: 'Szkic ze zmyślonymi kanałami. Nic nie jest zapisywane na serwerze.',
		arrive: 'Niech przyjdzie artykuł',
		revise: 'Zmień otwarty artykuł',
		revised: 'Aktualizacja: ten akapit dodano, aby wypróbować wersje.',
		reviseHint: 'Najpierw otwórz artykuł.'
	}
};

export default pl;
