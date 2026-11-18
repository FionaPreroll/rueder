// The texts of the start page, in every language of rueder's page
// (web/src/lib/i18n/locale.svelte.ts), with the same form of address.

const en = {
	intro: 'Your feed reader, on your own server.',
	address: 'Address of your rueder',
	open: 'Open',
	opening: 'Opening …',
	updates: 'Look for a new version of this app at every start',
	updatesHint:
		'The app then asks GitHub once at every start whether there is a newer build, and offers to install it. Nothing else is sent.',
	lookNow: 'Look now',
	looking: 'Looking …',
	current: 'This is the newest version.',
	lookFailed: 'GitHub did not answer. Try again later.',
	unreachable: (address) =>
		`${address} does not answer. Check the address, or try again once you are online.`,
	empty: 'Enter the address of your rueder.',
	invalid: 'This is not the address of a web page.',
	insecure: 'The address must start with https://: rueder logs in only over a secure connection.',
	version: (name, build) => `App version ${name} (build ${build})`,
	imprint: 'Imprint',
	privacy: 'Privacy policy',
	terms: 'Terms of use'
};

const de = {
	intro: 'Dein Feedreader, auf deinem eigenen Server.',
	address: 'Adresse deines rueder',
	open: 'Öffnen',
	opening: 'Öffne …',
	updates: 'Bei jedem Start nach einer neuen Version dieser App suchen',
	updatesHint:
		'Die App fragt dann bei jedem Start einmal bei GitHub, ob es einen neueren Build gibt, und bietet an, ihn zu installieren. Sonst wird nichts gesendet.',
	lookNow: 'Jetzt suchen',
	looking: 'Suche …',
	current: 'Das ist die neueste Version.',
	lookFailed: 'GitHub hat nicht geantwortet. Versuch es später noch einmal.',
	unreachable: (address) =>
		`${address} antwortet nicht. Prüf die Adresse, oder versuch es noch einmal, wenn du online bist.`,
	empty: 'Gib die Adresse deines rueder ein.',
	invalid: 'Das ist keine Adresse einer Webseite.',
	insecure:
		'Die Adresse muss mit https:// beginnen: rueder meldet nur über eine sichere Verbindung an.',
	version: (name, build) => `App-Version ${name} (Build ${build})`,
	imprint: 'Impressum',
	privacy: 'Datenschutzerklärung',
	terms: 'Nutzungsbedingungen'
};

const es = {
	intro: 'Tu lector de feeds, en tu propio servidor.',
	address: 'Dirección de tu rueder',
	open: 'Abrir',
	opening: 'Abriendo …',
	updates: 'Buscar una versión nueva de esta app en cada inicio',
	updatesHint:
		'La app pregunta entonces a GitHub una vez en cada inicio si hay un build más nuevo, y ofrece instalarlo. No se envía nada más.',
	lookNow: 'Buscar ahora',
	looking: 'Buscando …',
	current: 'Esta es la versión más nueva.',
	lookFailed: 'GitHub no respondió. Inténtalo de nuevo más tarde.',
	unreachable: (address) =>
		`${address} no responde. Revisa la dirección, o inténtalo de nuevo cuando tengas conexión.`,
	empty: 'Escribe la dirección de tu rueder.',
	invalid: 'Esta no es la dirección de una página web.',
	insecure:
		'La dirección debe empezar con https://: rueder solo inicia sesión mediante una conexión segura.',
	version: (name, build) => `Versión de la app ${name} (build ${build})`,
	imprint: 'Aviso legal',
	privacy: 'Política de privacidad',
	terms: 'Condiciones de uso'
};

const fr = {
	intro: 'Votre lecteur de flux, sur votre propre serveur.',
	address: 'Adresse de votre rueder',
	open: 'Ouvrir',
	opening: 'Ouverture …',
	updates: 'Chercher une nouvelle version de cette app à chaque démarrage',
	updatesHint:
		'L’app demande alors à GitHub, une fois à chaque démarrage, s’il existe un build plus récent, et propose de l’installer. Rien d’autre n’est envoyé.',
	lookNow: 'Chercher maintenant',
	looking: 'Recherche …',
	current: 'C’est la version la plus récente.',
	lookFailed: 'GitHub n’a pas répondu. Réessayez plus tard.',
	unreachable: (address) =>
		`${address} ne répond pas. Vérifiez l’adresse, ou réessayez une fois en ligne.`,
	empty: 'Saisissez l’adresse de votre rueder.',
	invalid: 'Ce n’est pas l’adresse d’une page web.',
	insecure:
		'L’adresse doit commencer par https:// : rueder ne se connecte que par une connexion sécurisée.',
	version: (name, build) => `Version de l’app ${name} (build ${build})`,
	imprint: 'Mentions légales',
	privacy: 'Politique de confidentialité',
	terms: 'Conditions d’utilisation'
};

const ptBR = {
	intro: 'Seu leitor de feeds, no seu próprio servidor.',
	address: 'Endereço do seu rueder',
	open: 'Abrir',
	opening: 'Abrindo …',
	updates: 'Procurar uma nova versão deste app a cada início',
	updatesHint:
		'O app então pergunta ao GitHub uma vez a cada início se há um build mais novo, e oferece instalá-lo. Nada mais é enviado.',
	lookNow: 'Procurar agora',
	looking: 'Procurando …',
	current: 'Esta é a versão mais nova.',
	lookFailed: 'O GitHub não respondeu. Tente de novo mais tarde.',
	unreachable: (address) =>
		`${address} não responde. Confira o endereço, ou tente de novo quando estiver on-line.`,
	empty: 'Digite o endereço do seu rueder.',
	invalid: 'Este não é o endereço de uma página da web.',
	insecure: 'O endereço precisa começar com https://: o rueder só entra por uma conexão segura.',
	version: (name, build) => `Versão do app ${name} (build ${build})`,
	imprint: 'Informações legais',
	privacy: 'Política de privacidade',
	terms: 'Termos de uso'
};

const it = {
	intro: 'Il tuo lettore di feed, sul tuo server.',
	address: 'Indirizzo del tuo rueder',
	open: 'Apri',
	opening: 'Apertura …',
	updates: 'Cerca una nuova versione di questa app a ogni avvio',
	updatesHint:
		'L’app chiede allora a GitHub, una volta a ogni avvio, se c’è una build più recente, e propone di installarla. Non viene inviato nient’altro.',
	lookNow: 'Cerca ora',
	looking: 'Ricerca …',
	current: 'Questa è la versione più recente.',
	lookFailed: 'GitHub non ha risposto. Riprova più tardi.',
	unreachable: (address) =>
		`${address} non risponde. Controlla l’indirizzo, oppure riprova quando sei online.`,
	empty: 'Inserisci l’indirizzo del tuo rueder.',
	invalid: 'Questo non è l’indirizzo di una pagina web.',
	insecure:
		'L’indirizzo deve iniziare con https://: rueder accede solo tramite una connessione sicura.',
	version: (name, build) => `Versione dell’app ${name} (build ${build})`,
	imprint: 'Note legali',
	privacy: 'Informativa sulla privacy',
	terms: 'Condizioni d’uso'
};

const ja = {
	intro: 'あなた自身のサーバで動くフィードリーダーです。',
	address: 'あなたの rueder のアドレス',
	open: '開く',
	opening: '開いています…',
	updates: '起動するたびにこのアプリの新しいバージョンを探す',
	updatesHint:
		'起動するたびに一度 GitHub に新しいビルドがあるか問い合わせ、あればインストールを提案します。それ以外は何も送信しません。',
	lookNow: '今すぐ探す',
	looking: '探しています…',
	current: 'これが最新のバージョンです。',
	lookFailed: 'GitHub が応答しませんでした。しばらくしてからもう一度お試しください。',
	unreachable: (address) =>
		`${address} が応答しません。アドレスを確認するか、オンラインになってからもう一度お試しください。`,
	empty: 'あなたの rueder のアドレスを入力してください。',
	invalid: 'ウェブページのアドレスではありません。',
	insecure: 'アドレスは https:// で始める必要があります。rueder は安全な接続でのみログインします。',
	version: (name, build) => `アプリのバージョン ${name}（ビルド ${build}）`,
	imprint: '運営者情報',
	privacy: 'プライバシーポリシー',
	terms: '利用規約'
};

const pl = {
	intro: 'Twój czytnik kanałów, na twoim własnym serwerze.',
	address: 'Adres twojego rueder',
	open: 'Otwórz',
	opening: 'Otwieranie …',
	updates: 'Szukaj nowej wersji tej aplikacji przy każdym uruchomieniu',
	updatesHint:
		'Aplikacja pyta wtedy GitHub raz przy każdym uruchomieniu, czy jest nowsza kompilacja, i proponuje ją zainstalować. Nic innego nie jest wysyłane.',
	lookNow: 'Szukaj teraz',
	looking: 'Szukanie …',
	current: 'To najnowsza wersja.',
	lookFailed: 'GitHub nie odpowiedział. Spróbuj ponownie później.',
	unreachable: (address) =>
		`${address} nie odpowiada. Sprawdź adres albo spróbuj ponownie, gdy będziesz online.`,
	empty: 'Wpisz adres swojego rueder.',
	invalid: 'To nie jest adres strony internetowej.',
	insecure: 'Adres musi zaczynać się od https://: rueder loguje tylko przez bezpieczne połączenie.',
	version: (name, build) => `Wersja aplikacji ${name} (kompilacja ${build})`,
	imprint: 'Informacje prawne',
	privacy: 'Polityka prywatności',
	terms: 'Regulamin'
};

const nl = {
	intro: 'Je feedlezer, op je eigen server.',
	address: 'Adres van je rueder',
	open: 'Openen',
	opening: 'Openen …',
	updates: 'Bij elke start zoeken naar een nieuwe versie van deze app',
	updatesHint:
		'De app vraagt GitHub dan bij elke start één keer of er een nieuwere build is, en biedt aan die te installeren. Er wordt verder niets verstuurd.',
	lookNow: 'Nu zoeken',
	looking: 'Zoeken …',
	current: 'Dit is de nieuwste versie.',
	lookFailed: 'GitHub antwoordde niet. Probeer het later opnieuw.',
	unreachable: (address) =>
		`${address} antwoordt niet. Controleer het adres, of probeer het opnieuw zodra je online bent.`,
	empty: 'Voer het adres van je rueder in.',
	invalid: 'Dit is geen adres van een webpagina.',
	insecure:
		'Het adres moet met https:// beginnen: rueder logt alleen in via een beveiligde verbinding.',
	version: (name, build) => `App-versie ${name} (build ${build})`,
	imprint: 'Colofon',
	privacy: 'Privacybeleid',
	terms: 'Gebruiksvoorwaarden'
};

const ko = {
	intro: '내 서버에서 돌아가는 나만의 피드 리더입니다.',
	address: '내 rueder 주소',
	open: '열기',
	opening: '여는 중 …',
	updates: '시작할 때마다 이 앱의 새 버전 찾기',
	updatesHint:
		'앱이 시작할 때마다 한 번 GitHub에 새 빌드가 있는지 묻고, 있으면 설치를 제안합니다. 그 밖에는 아무것도 보내지 않습니다.',
	lookNow: '지금 찾기',
	looking: '찾는 중 …',
	current: '최신 버전입니다.',
	lookFailed: 'GitHub가 응답하지 않았습니다. 나중에 다시 시도하세요.',
	unreachable: (address) =>
		`${address} 주소가 응답하지 않습니다. 주소를 확인하거나 온라인 상태에서 다시 시도하세요.`,
	empty: '내 rueder 주소를 입력하세요.',
	invalid: '웹 페이지 주소가 아닙니다.',
	insecure: '주소는 https://로 시작해야 합니다. rueder는 안전한 연결로만 로그인합니다.',
	version: (name, build) => `앱 버전 ${name} (빌드 ${build})`,
	imprint: '법적 고지',
	privacy: '개인정보처리방침',
	terms: '이용약관'
};

const tr = {
	intro: 'Kendi sunucunuzdaki akış okuyucunuz.',
	address: 'rueder’inizin adresi',
	open: 'Aç',
	opening: 'Açılıyor …',
	updates: 'Her açılışta bu uygulamanın yeni sürümünü ara',
	updatesHint:
		'Uygulama her açılışta GitHub’a bir kez daha yeni bir derleme olup olmadığını sorar ve yüklemeyi önerir. Başka hiçbir şey gönderilmez.',
	lookNow: 'Şimdi ara',
	looking: 'Aranıyor …',
	current: 'Bu en yeni sürüm.',
	lookFailed: 'GitHub yanıt vermedi. Daha sonra yeniden deneyin.',
	unreachable: (address) =>
		`${address} yanıt vermiyor. Adresi denetleyin ya da çevrimiçi olduğunuzda yeniden deneyin.`,
	empty: 'rueder’inizin adresini girin.',
	invalid: 'Bu bir web sayfasının adresi değil.',
	insecure:
		'Adres https:// ile başlamalı: rueder yalnızca güvenli bir bağlantı üzerinden giriş yapar.',
	version: (name, build) => `Uygulama sürümü ${name} (derleme ${build})`,
	imprint: 'Künye',
	privacy: 'Gizlilik politikası',
	terms: 'Kullanım koşulları'
};

const zhHans = {
	intro: '你的订阅源阅读器，在你自己的服务器上。',
	address: '你的 rueder 的地址',
	open: '打开',
	opening: '正在打开…',
	updates: '每次启动时检查此应用的新版本',
	updatesHint:
		'应用会在每次启动时询问 GitHub 一次是否有更新的构建，并提供安装。不会发送任何其他内容。',
	lookNow: '立即检查',
	looking: '正在检查…',
	current: '这是最新版本。',
	lookFailed: 'GitHub 没有响应。请稍后再试。',
	unreachable: (address) => `${address} 没有响应。请检查地址，或在联网后重试。`,
	empty: '请输入你的 rueder 的地址。',
	invalid: '这不是网页的地址。',
	insecure: '地址必须以 https:// 开头：rueder 只通过安全连接登录。',
	version: (name, build) => `应用版本 ${name}（构建 ${build}）`,
	imprint: '法律声明',
	privacy: '隐私政策',
	terms: '使用条款'
};

export const catalogs = {
	en,
	de,
	es,
	fr,
	'pt-BR': ptBR,
	it,
	ja,
	pl,
	nl,
	ko,
	tr,
	'zh-Hans': zhHans
};

/** The language of a tag such as "de-AT" without its region: "de". */
const primary = (tag) =>
	String(tag ?? '')
		.split(/[-_]/)[0]
		.toLowerCase();

/** Which way a language is written, by its primary subtag, as on the page. */
export const directionOf = (language) =>
	['ar', 'fa', 'he', 'ur'].includes(primary(language)) ? 'rtl' : 'ltr';

/** The code of the language for a tag such as "de-AT" or "pt-PT", whatever its region; "en" for any other. */
export const codeFor = (language) =>
	Object.keys(catalogs).find((code) => primary(code) === primary(language)) ?? 'en';

/** The texts for a language such as "de-DE"; English for any other. */
export const textsFor = (language) => catalogs[codeFor(language)];
