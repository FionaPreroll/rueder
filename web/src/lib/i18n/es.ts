// The Spanish texts, in the shape of the English catalog (en.ts): neutral Spanish,
// without what only Spain or Mexico says, addressing the reader with “tú”. “Feed”
// stays as it is; an item is an “artículo”.

import type { Messages } from './en.ts';
import { formatNumber as n, plural } from './intl.ts';

const articles = (count: number) =>
	`${n(count)} ${plural(count, { one: 'artículo', other: 'artículos' })}`;

const minutes = (count: number) =>
	`${n(count)} ${plural(count, { one: 'minuto', other: 'minutos' })}`;

const es: Messages = {
	languageName: 'Español',

	articles,

	common: {
		close: 'Cerrar',
		cancel: 'Cancelar',
		save: 'Guardar',
		delete: 'Eliminar',
		edit: 'Editar'
	},

	nav: {
		subscribe: 'Suscribirse',
		home: 'Inicio',
		all: 'Todos los artículos',
		unreadItems: 'Artículos sin leer',
		starred: 'Artículos destacados',
		hidden: 'Ocultos por filtros',
		subscriptions: 'Suscripciones',
		newFolder: 'Nueva carpeta',
		groupedFolder: 'Nueva carpeta',
		folderName: 'Nombre de la carpeta',
		renameFolder: 'Cambiar nombre',
		removeFolder: 'Quitar la carpeta y conservar lo que contiene',
		folderActions: (title) => `Acciones de la carpeta ${title}`,
		feedName: 'Nombre del feed',
		feedNameHint: 'Vacío para usar el nombre propio del feed',
		renameFeed: 'Cambiar nombre',
		unsubscribe: 'Cancelar la suscripción',
		reportFeed: 'Denunciar el feed …',
		feedInfo: 'Información del feed …',
		feedActions: (title) => `Acciones del feed ${title}`,
		unsubscribeTitle: '¿Cancelar la suscripción?',
		unsubscribeText: (title) =>
			`¿Cancelar la suscripción a “${title}”? Se pierde lo que leíste y destacaste allí.`,
		openFolder: 'Abrir la carpeta',
		closeFolder: 'Cerrar la carpeta',
		openSidebar: 'Mostrar la barra lateral',
		closeSidebar: 'Ocultar la barra lateral',
		search: 'Buscar',
		searchPlaceholder: 'Buscar en estos artículos',
		clearSearch: 'Borrar la búsqueda',
		settings: 'Ajustes',
		accountSettings: 'Ajustes de la cuenta',
		shortcuts: 'Atajos de teclado',
		live: 'En vivo',
		liveOn: 'Los artículos nuevos llegan solos. Haz clic para pausar.',
		liveOff: 'En pausa. Haz clic para que vuelvan a llegar artículos nuevos.',
		paused: 'En pausa',
		account: (name) => `Sesión iniciada como ${name}`,
		logout: 'Cerrar sesión',
		appSettings: 'Servidor y actualizaciones de la app',
		install: 'Instalar como app',
		installSafari:
			'Para instalar rueder como app: Archivo › Agregar al Dock en una Mac, Compartir › Agregar a inicio en un iPhone o iPad.',
		unread: (count) => `${n(count)} sin leer`,
		hiddenCount: (count) =>
			`${n(count)} ${plural(count, { one: 'oculto', other: 'ocultos' })} por filtros`,
		fetchFailed: (reason) => `La última descarga falló: ${reason}`,
		waiting: 'Aún no descargado',
		blocked: 'Bloqueado por quien administra este rueder',
		moveHint: 'Arrastra para ordenar, o pulsa Alt con las flechas',
		resizeSidebar: 'Ancho de la barra lateral: arrastra o usa las flechas',
		tooDeep: (levels) => `Las carpetas se anidan como máximo ${n(levels)} niveles.`
	},

	toolbar: {
		refresh: 'Actualizar',
		showAll: 'Todos los artículos',
		showUnread: 'Solo sin leer',
		show: 'Mostrar',
		newestFirst: 'Más nuevos primero',
		oldestFirst: 'Más antiguos primero',
		markAllRead: 'Marcar todo como leído',
		markOptions: 'Más formas de marcar como leído',
		phoneMenu: (shown) => `Vista, se muestra: ${shown}`,
		markMenu: 'Marcar como leído …',
		markEverything: 'Todos los artículos',
		markOlderDay: 'Artículos de hace más de un día',
		markOlderWeek: 'Artículos de hace más de una semana',
		listView: 'Vista de lista',
		expandedView: 'Vista expandida',
		splitView: 'Tres paneles: árbol, lista y artículo',
		magazineView: 'Revista',
		previous: 'Artículo anterior',
		next: 'Artículo siguiente',
		newArticles: (count) =>
			`${n(count)} ${plural(count, { one: 'artículo nuevo', other: 'artículos nuevos' })}`
	},

	list: {
		empty: 'No hay artículos aquí.',
		emptyUnread: 'No hay nada sin leer aquí.',
		emptySearch: 'Ningún artículo coincide con la búsqueda.',
		found: (count, more, words) =>
			`${n(count)}${more ? '+' : ''} ${plural(count, { one: 'artículo', other: 'artículos' })} con “${words}”`,
		searchAll: 'Buscar en todos los feeds',
		feedFailing:
			'rueder vuelve a intentarlo más tarde y espera más tras cada fallo, o tanto como pida el sitio. Si sigue fallando, puede que el feed se haya mudado.',
		hiddenNote: 'Tus reglas de filtro ocultaron estos artículos. Abre uno para ver qué regla fue.',
		toRules: 'Reglas de filtro',
		emptyFeed: 'Este feed aún no tiene artículos.',
		waitingFeed:
			'Este feed aún no se ha descargado. Sus artículos llegan con la próxima ronda de descargas, en pocos minutos.',
		showAll: 'Mostrar todos los artículos',
		more: 'Mostrar más',
		star: 'Destacar',
		unstar: 'Quitar de destacados',
		keepUnread: 'Mantener sin leer',
		openOriginal: 'Abrir el original',
		from: 'de',
		by: (author) => `por ${author}`,
		updated: 'actualizado',
		updatedHeadline: 'nuevo titular',
		updatedTitle: (count) =>
			`Cambió desde la primera descarga; ${n(count)} ${plural(count, { one: 'versión anterior', other: 'versiones anteriores' })}`,
		updatedHeadlineTitle: (count) =>
			`Se reescribió el titular, el texto sigue igual; ${n(count)} ${plural(count, { one: 'versión anterior', other: 'versiones anteriores' })}`,
		compare: 'Comparar versiones',
		hiddenBy: (rule) => `Oculto por la regla: ${rule}`,
		textDropped: 'Este servidor ya no guarda el texto de este artículo, porque es antiguo.',
		textDroppedOriginal: 'Léelo en su fuente',
		loading: 'Cargando el texto …',
		loadingList: 'Cargando …',
		toTop: 'Volver arriba',
		pick: 'Elige un artículo para leerlo aquí.',
		resizeList: 'Ancho de la lista: arrastra o usa las flechas',
		closeArticle: 'Cerrar el artículo',
		blockedFeed:
			'Quien administra este rueder bloqueó este feed. No se descarga y sus artículos no se muestran.',
		articleActions: 'Más acciones para el artículo',
		reportArticle: 'Denunciar el artículo …'
	},

	report: {
		feedTitle: 'Denunciar el feed',
		articleTitle: 'Denunciar el artículo',
		text: (title) =>
			`Quien administra este rueder ve que se denunció “${title}”, el motivo y lo que escribas, pero no quién lo denunció. Decide si lo bloquea para todos.`,
		reason: 'Motivo',
		reasons: {
			illegal: 'Va contra la ley',
			copyright: 'Es obra de otra persona',
			malicious: 'Malware, phishing o fraude',
			spam: 'Spam',
			other: 'Otra cosa'
		},
		note: 'Lo que quieras añadir (opcional)',
		left: (count) =>
			`${plural(count, { one: 'Queda', other: 'Quedan' })} ${n(count)} ${plural(count, { one: 'carácter', other: 'caracteres' })}`,
		send: 'Enviar la denuncia',
		sending: 'Enviando …',
		sent: 'Gracias. La denuncia llegó a quien administra este rueder.',
		tooMany: 'Enviaste muchas denuncias últimamente. Inténtalo de nuevo más tarde.',
		unreachable: 'La denuncia no llegó al servidor. Inténtalo de nuevo más tarde.'
	},

	feedInfo: {
		title: 'Información del feed',
		text: (title) =>
			`Lo que este rueder sabe de “${title}” y cómo lo descarga. Si el feed no funciona, cópialo en tu mensaje a quien lo administra.`,
		number: 'Número',
		name: 'Nombre',
		address: 'Dirección del feed',
		site: 'Sitio web',
		noSite: 'Ninguno',
		agent: 'Se descarga como',
		agents: { rueder: 'rueder', browser: 'Navegador' },
		every: 'Se descarga cada',
		fetched: 'Última descarga',
		succeeded: 'Última descarga correcta',
		failed: 'Último error',
		next: 'Próxima descarga',
		blocked: 'Ninguna, quien administra bloqueó el feed',
		articles: 'Artículos hasta ahora',
		never: 'Nunca',
		fetchAs: 'Descargarlo como',
		fetchAsHint:
			'Para todos los que leen el feed. Como navegador, para los sitios que rechazan a rueder. Se vuelve a descargar enseguida.',
		notSwitched: 'No funcionó. Inténtalo de nuevo más tarde.',
		copy: 'Copiar',
		copied: 'Copiado',
		loading: 'Cargando …',
		unreachable: 'El servidor no respondió. Inténtalo de nuevo más tarde.'
	},

	media: {
		title: 'Adjuntos',
		play: 'Reproducir'
	},

	connection: {
		reconnecting: 'Reconectando …',
		unreachable: (since) => `El servidor no responde desde ${since}.`,
		offline: (since) => `Sin red desde ${since}.`,
		retry: 'Intentar ahora',
		unsent: (count) =>
			plural(count, { one: 'Hay 1 cambio en espera.', other: `Hay ${n(count)} cambios en espera.` })
	},

	notice: {
		unreachable: 'El servidor no responde. Puede que lo que ves no esté al día.',
		notSaved: 'El servidor no aceptó tu último cambio. Se deshizo aquí.',
		tooManyUnsent: 'Hay tantos cambios esperando al servidor que este se deshizo.'
	},

	login: {
		title: 'Iniciar sesión en rueder',
		about:
			'Un lector de feeds: descarga lo que publican los sitios que sigues y te muestra lo que aún no leíste.',
		name: 'Nombre',
		password: 'Contraseña',
		submit: 'Iniciar sesión',
		checking: 'Un momento …',
		wrong: 'Nombre o contraseña incorrectos.',
		codeNeeded:
			'Esta cuenta tiene un segundo factor. Escribe el código que muestra tu app o un código de recuperación, o inicia sesión con una llave de acceso.',
		wrongCode: 'Ese código no es correcto. Los códigos cambian cada 30 segundos.',
		code: 'Código',
		codeNote: 'Los seis dígitos de tu app de autenticación, o uno de tus códigos de recuperación.',
		tooMany: (count) => `Demasiados intentos fallidos. Inténtalo de nuevo en ${minutes(count)}.`,
		unreachable: 'El servidor no responde. Inténtalo de nuevo en un momento.',
		retry: 'Intentar de nuevo',
		or: 'o',
		oidc: 'Iniciar sesión con inicio de sesión único',
		passkey: 'Iniciar sesión con una llave de acceso',
		passkeyCancelled:
			'No se usó ninguna llave de acceso. Inténtalo de nuevo, o inicia sesión con nombre y contraseña.',
		passkeyFailed: 'Este navegador o dispositivo no puede usar aquí una llave de acceso.',
		passkeyWrong: 'Esta llave de acceso no pertenece a ninguna cuenta de aquí.',
		oidcFailed: 'El inicio de sesión único no funcionó. Inténtalo de nuevo.',
		oidcDenied: 'Se canceló el inicio de sesión único.',
		oidcUnknown:
			'Ninguna cuenta de aquí corresponde a ese inicio de sesión. Pide a quien administra este rueder que lo vincule.',
		disabled: 'Esta cuenta está desactivada. Pregunta a quien administra este rueder por qué.',
		unconfirmed:
			'Esta cuenta espera a que se confirme su correo electrónico. Abre el enlace del correo o pide que se envíe otra vez.',
		resend: 'Enviar el enlace otra vez',
		resent: 'El enlace va en camino otra vez. Revisa tu correo, también la carpeta de spam.',
		resendWrong:
			'Escribe el nombre y la contraseña de la cuenta para que se envíe el enlace otra vez.',
		forgot: '¿Olvidaste tu contraseña?',
		removed: 'Tu cuenta se eliminó, con todo lo que era tuyo.',
		note: 'No hay registro abierto. Las cuentas las crea quien administra este rueder.'
	},

	legal: {
		imprint: 'Aviso legal',
		privacy: 'Política de privacidad',
		terms: 'Condiciones de uso',
		agree: { before: 'Acepto las ', link: 'condiciones de uso', after: '.' },
		changed: 'Las condiciones de uso cambiaron',
		changedIntro: 'Lee las nuevas condiciones de uso. Para seguir usando rueder, acéptalas.',
		read: 'Leer las condiciones de uso',
		accept: 'Acepto',
		decline: 'Cerrar sesión',
		notSaved: 'No funcionó. Inténtalo de nuevo en un momento.'
	},

	register: {
		title: 'Crear una cuenta',
		invited: 'Te invitaron a este rueder. Elige un nombre y una contraseña.',
		again: 'La contraseña otra vez',
		passwordRule: (characters) => `Al menos ${n(characters)} caracteres.`,
		mismatch: 'Las dos contraseñas no coinciden.',
		submit: 'Crear la cuenta',
		solving: 'Comprobando este navegador …',
		toRegister: 'Crear una cuenta',
		toLogin: 'Iniciar sesión en una cuenta que ya tienes',
		closed: 'Aquí nadie puede registrarse sin una invitación.',
		inviteGone: 'Esta invitación ya no se puede usar. Pide una nueva.',
		full: 'Aquí no hay lugar para otra cuenta.',
		today: 'Hoy ya se crearon suficientes cuentas aquí. Inténtalo de nuevo mañana.',
		captcha: 'La comprobación de este navegador no se completó. Inténtalo de nuevo.',
		taken: 'Ya existe una cuenta con este nombre.',
		terms: 'Acepta las condiciones de uso para crear una cuenta.',
		invalid: (reason) => `Eso no puede ser una cuenta. El servidor dice: ${reason}`,
		email: 'Correo electrónico',
		emailOptional: 'Correo electrónico (opcional)',
		emailNote: 'Recibirás un enlace para confirmarlo.',
		emailRequiredNote:
			'Recibirás un enlace para confirmarlo; la cuenta podrá iniciar sesión cuando lo hayas abierto.',
		emailInvalid: 'Escribe una dirección de correo electrónico que pueda recibir correo.',
		unsent:
			'No se pudo enviar el enlace, así que no se creó ninguna cuenta. Inténtalo de nuevo más tarde.',
		tooMany: (count) => `Demasiados intentos. Inténtalo de nuevo en ${minutes(count)}.`,
		awaitingTitle: 'Revisa tu correo',
		awaiting: (address, hours) =>
			`La cuenta está creada. Se envió un enlace a ${address}: ábrelo en las próximas ${n(hours)} horas y luego inicia sesión.`
	},

	forgot: {
		title: '¿Olvidaste tu contraseña?',
		intro:
			'Indica tu nombre o tu correo electrónico. Se enviará a la dirección confirmada un enlace para elegir una contraseña nueva.',
		field: 'Nombre o correo electrónico',
		submit: 'Enviar el enlace',
		asked: (count) =>
			`Si existe esa cuenta con una dirección confirmada, se le está enviando un enlace, válido durante ${minutes(count)}. Revisa también la carpeta de spam.`,
		tooMany: (count) => `Se pidió demasiadas veces. Inténtalo de nuevo en ${minutes(count)}.`,
		back: 'Volver al inicio de sesión'
	},

	reset: {
		title: 'Elegir una contraseña nueva',
		intro: 'Se cerrará la sesión en todos los navegadores y dispositivos de la cuenta.',
		fresh: 'Contraseña nueva',
		again: 'Contraseña nueva otra vez',
		submit: 'Guardar la contraseña',
		mismatch: 'Las dos no coinciden.',
		gone: 'Este enlace ya no funciona. Pide uno nuevo.',
		invalid: (reason) => `El servidor no aceptó esa contraseña. Dice: ${reason}`,
		done: 'La contraseña está lista. Inicia sesión con ella.',
		cancel: 'Cancelar'
	},

	email: {
		title: 'Correo electrónico',
		intro: 'rueder solo te escribe sobre tu cuenta.',
		current: 'Tu dirección',
		none: 'La cuenta no tiene dirección.',
		pending: (address, hours) =>
			`Se envió un enlace a ${address}. La dirección cuenta una vez abierto, dentro de ${n(hours)} horas; hasta entonces sigue la anterior.`,
		address: 'Dirección nueva',
		password: 'Tu contraseña',
		passwordNote: 'Para que nadie más que use este navegador pueda desviar tu correo.',
		code: 'Código del segundo factor',
		submit: 'Enviar el enlace',
		sent: 'El enlace va en camino. Revisa tu correo, también la carpeta de spam.',
		remove: 'Quitar la dirección',
		removeIntro: 'Sin dirección no se te envía ningún correo. Por lo demás, la cuenta sigue igual.',
		removed: 'La dirección se quitó.',
		wrong: 'La contraseña o el código no son correctos.',
		codeNeeded: 'Esta cuenta tiene un segundo factor. Escribe también su código.',
		loginAgain:
			'Tu cuenta aún no tiene contraseña ni segundo factor. Inicia sesión otra vez y luego indica la dirección en los diez minutos siguientes.',
		invalid: 'Escribe una dirección de correo electrónico que pueda recibir correo.',
		same: 'Esa es la dirección que ya tiene la cuenta.',
		required:
			'Toda cuenta de este rueder necesita una dirección; se puede cambiar, pero no quitar.',
		busy: (count) => `Demasiados intentos o correos. Inténtalo de nuevo en ${minutes(count)}.`,
		unsent: 'No se pudo enviar el correo. Inténtalo de nuevo más tarde.',
		unreachable: 'El servidor no responde. No se cambió nada.',
		askTitle: 'Agrega un correo electrónico',
		askIntro:
			'Ahora toda cuenta de este rueder necesita un correo electrónico. Indica el tuyo y abre el enlace que se te envíe.',
		askWaiting:
			'Cuando abras el enlace, esto desaparece solo. ¿No llegó nada? Envíalo otra vez, o a otra dirección.',
		logOut: 'Cerrar sesión',
		confirmed: (address) => `La dirección ${address} está confirmada.`,
		confirmedLogIn: 'La cuenta está lista: inicia sesión.',
		gone: 'Este enlace ya no se puede usar: ya se usó, se envió uno más nuevo o tiene más de un día.',
		taken: 'Mientras tanto, otra cuenta tomó esta dirección.',
		confirmUnreachable: 'El servidor no responde. Abre el enlace otra vez en un momento.',
		close: 'Cerrar'
	},

	invites: {
		intro:
			'Un enlace permite a alguien crear una cuenta en este rueder. Se muestra una sola vez, aquí, al crearlo.',
		left: (count) =>
			count === 0
				? 'No tienes invitaciones para dar.'
				: `Puedes invitar a ${n(count)} ${plural(count, { one: 'persona más', other: 'personas más' })}.`,
		admin: 'Como admin, siempre puedes invitar.',
		uses: 'Para cuántas cuentas',
		make: 'Crear un enlace',
		noneLeft: 'No te quedan invitaciones para tantas.',
		failed: 'No funcionó. Inténtalo de nuevo en un momento.',
		link: 'El enlace, para compartirlo ahora',
		copy: 'Copiar',
		copied: 'Copiado',
		goodFor: (days) =>
			`Un enlace es válido durante ${n(days)} ${plural(days, { one: 'día', other: 'días' })}.`,
		yours: 'Tus enlaces',
		none: 'Aún no creaste ningún enlace.',
		used: (uses, of) => `${n(uses)} de ${n(of)} usados`,
		until: (when) => `válido hasta ${when}`,
		ran: (when) => `venció ${when}`,
		usedUp: 'agotado',
		revoked: 'retirado',
		revoke: 'Retirar',
		revokeLink: (made) => `Retirar el enlace creado ${made}`
	},

	home: {
		newIn: 'Novedades en tus suscripciones',
		allRead: 'Leíste todo. Los artículos nuevos aparecen aquí cuando llegan.',
		recentlyStarred: 'Destacados recientemente',
		noStars: 'Aún no hay nada destacado. Pulsa s en un artículo.',
		problems: 'Feeds con problemas',
		tips: 'Consejos',
		tipKeys: 'Pulsa ? para ver todos los atajos de teclado; j y k recorren los artículos.',
		tipDrag: 'Arrastra feeds y carpetas en la barra lateral para ordenarlos.',
		tipFilters: 'Las reglas de filtro de los ajustes ocultan artículos por palabra clave o autor.',
		showFeed: (title) => `Mostrar ${title}`
	},

	versions: {
		title: 'Versiones de este artículo',
		current: 'Versión actual',
		version: (rev) => `Versión ${n(rev)}`,
		fetched: (when) => `descargada ${when}`,
		compareWith: 'Comparar la versión actual con',
		legendAdded: 'añadido',
		legendRemoved: 'quitado',
		titleWas: 'El título era:',
		same: 'El texto de esta versión es igual al actual.'
	},

	account: {
		title: 'Ajustes de la cuenta',
		tabs: {
			password: 'Contraseña',
			email: 'Correo',
			security: 'Seguridad',
			apps: 'Apps',
			invites: 'Invitaciones',
			move: 'Mudanza',
			privacy: 'Privacidad',
			remove: 'Eliminar la cuenta'
		},
		password: {
			title: 'Cambiar la contraseña',
			titleFirst: 'Elegir una contraseña',
			intro:
				'Una contraseña nueva cierra la sesión en todos los demás navegadores y dispositivos y revoca todas las contraseñas de apps.',
			introFirst:
				'Con una contraseña, esta cuenta también puede iniciar sesión con su nombre, no solo con el inicio de sesión único.',
			current: 'Contraseña actual',
			fresh: 'Contraseña nueva',
			again: 'Contraseña nueva otra vez',
			submit: 'Cambiar la contraseña',
			submitFirst: 'Guardar la contraseña',
			done: 'La contraseña cambió. Se cerró la sesión en todos los demás navegadores y dispositivos, y se revocaron las contraseñas de apps.',
			doneFirst: 'La contraseña está lista.',
			mismatch: 'Las dos no coinciden.',
			code: 'Código del segundo factor',
			wrong: 'La contraseña actual o el código no son correctos.',
			codeNeeded:
				'Esta cuenta tiene un segundo factor. Escribe también un código de tu app de autenticación o un código de recuperación, o confirma con una llave de acceso.',
			loginAgain:
				'Tu cuenta aún no tiene contraseña ni segundo factor. Inicia sesión otra vez y luego elige la contraseña en los diez minutos siguientes.',
			invalid: (characters) =>
				`El servidor no aceptó esa contraseña. Al menos ${n(characters)} caracteres.`,
			busy: (count) => `Demasiados intentos fallidos. Inténtalo de nuevo en ${minutes(count)}.`,
			unreachable: 'El servidor no responde. La contraseña no cambió.'
		},
		settingsNote: 'El aspecto, la lectura, las reglas de filtro y el teclado están en Ajustes.'
	},

	settings: {
		title: 'Ajustes',
		tabs: {
			look: 'Aspecto',
			reading: 'Lectura',
			rules: 'Reglas de filtro',
			keys: 'Teclado'
		},
		accountNote:
			'La contraseña, el segundo factor, las apps y las invitaciones están en Ajustes de la cuenta, en el menú de tu cuenta.',
		theme: 'Tema',
		themes: { system: 'Como el sistema', light: 'Claro', dark: 'Oscuro' },
		accent: 'Color de acento',
		accents: {
			red: 'Rojo',
			blue: 'Azul',
			green: 'Verde',
			orange: 'Naranja',
			purple: 'Morado',
			graphite: 'Grafito'
		},
		density: 'Densidad de la lista',
		densities: { compact: 'Compacta', comfortable: 'Cómoda' },
		sidebarSide: 'Barra lateral',
		sidebarSides: { start: 'Izquierda', end: 'Derecha' },
		textSize: 'Tamaño del texto de los artículos, en este dispositivo',
		textSizes: { small: 'Pequeño', medium: 'Mediano', large: 'Grande' },
		readingFont: 'Fuente de los artículos',
		readingFonts: { sans: 'Sin serifa', serif: 'Con serifa' },
		counters: 'Contadores de no leídos',
		counterStyles: { parens: 'Entre paréntesis', badge: 'Como insignias' },
		language: 'Idioma',
		languageAuto: 'Como el navegador',
		view: 'Vista',
		views: { list: 'Lista', expanded: 'Expandida', split: 'Tres paneles', magazine: 'Revista' },
		markOnScroll: 'Marcar los artículos como leídos al pasar por ellos en la vista expandida',
		teasers: 'Mostrar las primeras palabras del texto tras el título en la lista',
		wakeLock: 'Mantener la pantalla encendida mientras rueder está abierto',
		wakeLockMissing: 'Este navegador no puede mantener la pantalla encendida.',
		note: 'Los cambios se aplican al instante y se guardan con tu cuenta, así que te siguen a cada dispositivo.',
		noteDemo: 'Los cambios se aplican al instante y se quedan en este navegador.',
		apps: {
			intro:
				'Otras apps de lectura de feeds pueden usar tu cuenta mediante la API de Google Reader. Cada una recibe su propia contraseña de app, que puedes revocar en cualquier momento.',
			none: 'Aún no hay ninguna app configurada.',
			made: (when) => `configurada ${when}`,
			used: (when) => `último uso ${when}`,
			unused: 'aún sin usar',
			revoke: 'Revocar',
			revokeName: (name) => `Revocar la contraseña de app ${name}`,
			add: 'Configurar una app',
			full: (max) =>
				`Una cuenta tiene como máximo ${n(max)} contraseñas de app. Revoca una para configurar otra app.`,
			choose: '¿Qué app quieres usar?',
			website: (app) => `Sitio web de ${app}`,
			other: 'Otra app',
			otherHint: 'Cualquier app que ofrezca la API de Google Reader o FreshRSS.',
			platforms: {
				apple: 'Mac, iPhone y iPad',
				android: 'Android'
			},
			name: 'Nombre de la contraseña de app',
			password: 'Tu contraseña',
			code: 'Código de tu app de autenticación, o un código de recuperación',
			create: 'Crear la contraseña de app',
			creating: 'Un momento …',
			back: 'Atrás',
			how: {
				netnewswire:
					'En NetNewsWire, abre Ajustes y luego Cuentas, agrega una cuenta (en la Mac con +) y elige FreshRSS. Escribe el nombre de abajo como Login (en iPhone y iPad: Username or Email), la contraseña de app como Password y la dirección del servidor como API URL.',
				readyou:
					'En Read You, abre Ajustes, Cuentas, Agregar cuentas, y elige Google Reader. Escribe la dirección del servidor de abajo como URL del servidor, el nombre como nombre de usuario y la contraseña de app como contraseña.',
				other:
					'En tu app, agrega una cuenta del tipo “Google Reader API” o, si no la hay, “FreshRSS”. Escribe la dirección del servidor, el nombre y la contraseña de app de abajo.'
			},
			ownIcons: (app) =>
				`Privacidad: ${app} carga por sí misma los íconos de tus feeds desde los sitios web de los feeds, que ven tu dirección IP al hacerlo. Los artículos y sus imágenes llegan solo a través de rueder.`,
			shotsTitle: 'Paso a paso, con imágenes',
			shots: {
				readyou: [
					'Toca el engranaje arriba a la izquierda.',
					'Abre Cuentas.',
					'Toca Agregar cuentas.',
					'Elige Google Reader.',
					'Escribe la dirección del servidor como URL del servidor, el nombre como nombre de usuario y la contraseña de app como contraseña, y toca Agregar.'
				],
				netnewswire: [
					'En la Mac: abre Ajustes (⌘,), luego Cuentas, y haz clic en + abajo a la izquierda.',
					'Elige FreshRSS y haz clic en Continuar.',
					'Escribe el nombre como Login, la contraseña de app como Password y la dirección del servidor como API URL, y haz clic en Crear.'
				]
			},
			shotAlt: (app, step) => `${app}, paso ${n(step)}`,
			server: 'Dirección del servidor',
			user: 'Nombre',
			secret: 'Contraseña de app',
			copy: (what) => `Copiar ${what}`,
			copied: 'Copiado',
			once: 'La contraseña de app solo se muestra ahora. Si la pierdes, revócala y crea una nueva.',
			waiting: 'Esperando a que la app inicie sesión …',
			connected: 'Conectada: la app inició sesión.',
			done: 'Listo',
			refusals: {
				wrongPassword: 'La contraseña no es correcta.',
				codeNeeded:
					'Escribe también un código de tu app de autenticación o un código de recuperación, o confirma con una llave de acceso.',
				wrongCode: 'Ese código no es correcto. Los códigos cambian cada 30 segundos.',
				loginAgain:
					'Tu cuenta aún no tiene contraseña ni segundo factor. Inicia sesión otra vez y luego crea la contraseña de app en los diez minutos siguientes.',
				tooManyApps: 'Una cuenta tiene como máximo 20 contraseñas de app.',
				tooMany: 'Demasiados intentos fallidos. Inténtalo de nuevo en unos minutos.',
				unreachable: 'El servidor no responde. Inténtalo de nuevo en un momento.'
			}
		},
		secondFactor: {
			title: 'Segundo factor',
			intro:
				'Con un segundo factor, iniciar sesión pide además un código de una app de autenticación, o una llave de acceso.',
			app: 'App de autenticación',
			on: 'Activado.',
			left: (count) =>
				`${plural(count, { one: 'Queda', other: 'Quedan' })} ${n(count)} ${plural(count, { one: 'código de recuperación', other: 'códigos de recuperación' })}.`,
			off: 'Desactivado.',
			byPasskeys: 'Sin configurar. Tus llaves de acceso mantienen activado el segundo factor.',
			adminNeeds:
				'Este servidor solo trata como admins a quienes tienen un segundo factor. Configúralo, o inicia sesión otra vez con su código, para ver el estado del servidor.',
			password: 'Tu contraseña',
			start: 'Configurar',
			scan: 'Escanea este código con tu app de autenticación, o escribe la clave de abajo.',
			qr: 'Código QR para la app de autenticación',
			larger: 'Mostrar el código QR más grande',
			smaller: 'Mostrar el código QR más pequeño',
			key: 'Clave',
			copyKey: 'Copiar la clave',
			copied: 'Copiado',
			code: 'El código que muestra la app ahora',
			codeOrRecovery: 'Código de la app, o un código de recuperación',
			confirm: 'Confirmar',
			cancel: 'Cancelar',
			codesTitle: 'Tus códigos de recuperación',
			codesIntro:
				'Cada uno te permite iniciar sesión una vez sin tu teléfono. Guárdalos bien: solo se muestran ahora.',
			copyCodes: 'Copiar los códigos',
			codesDone: 'Ya los guardé',
			switchOff: 'Desactivar',
			switchOffNote: 'Después, para iniciar sesión basta otra vez con la contraseña.',
			switchOffNoteKeys: 'Tus llaves de acceso mantienen activado el segundo factor.',
			startWithPasskey: 'Configurar con una llave de acceso',
			switchOffWithPasskey: 'Desactivar con una llave de acceso',
			refusals: {
				wrongPassword: 'La contraseña no es correcta.',
				codeNeeded:
					'Tus llaves de acceso tienen activado el segundo factor. Escribe también un código de recuperación.',
				wrongCode: 'Ese código no es correcto. Los códigos cambian cada 30 segundos.',
				loginAgain:
					'Tu cuenta aún no tiene contraseña ni segundo factor. Inicia sesión otra vez y luego configura la app en los diez minutos siguientes.',
				tooMany: 'Demasiados intentos fallidos. Inténtalo de nuevo en unos minutos.',
				unreachable: 'El servidor no responde. Inténtalo de nuevo en un momento.'
			}
		},
		passkeys: {
			title: 'Llaves de acceso',
			intro:
				'Una llave de acceso te permite iniciar sesión sin nombre ni contraseña, con el PIN, la huella o la cara que pida tu dispositivo. Cuenta como segundo factor.',
			once: 'Con una llave de acceso, tu contraseña sola ya no abre tu cuenta.',
			none: 'Aún no hay ninguna llave de acceso.',
			made: (when) => `agregada ${when}`,
			used: (when) => `último uso ${when}`,
			unused: 'aún sin usar',
			add: 'Agregar una llave de acceso',
			name: 'Nombre, por ejemplo el del dispositivo',
			defaultName: 'Llave de acceso',
			password: 'Tu contraseña',
			code: 'Código de tu app de autenticación, o un código de recuperación',
			create: 'Crear la llave de acceso',
			working: 'Sigue a tu navegador …',
			cancel: 'Cancelar',
			remove: 'Quitar',
			removeName: (name) => `Quitar la llave de acceso ${name}`,
			confirmRemove: 'Quitar la llave de acceso',
			full: (max) => `Una cuenta tiene como máximo ${n(max)} llaves de acceso.`,
			or: 'o',
			confirmWith: 'Confirmar con una llave de acceso',
			codesTitle: 'Tus códigos de recuperación',
			codesIntro:
				'Tu primera llave de acceso activó el segundo factor. Cada código te permite iniciar sesión una vez si pierdes tu dispositivo. Guárdalos bien: solo se muestran ahora.',
			copyCodes: 'Copiar los códigos',
			copied: 'Copiado',
			codesDone: 'Ya los guardé',
			refusals: {
				cancelled: 'La ventana del navegador se cerró antes de usar la llave de acceso.',
				known: 'Este dispositivo ya tiene una llave de acceso para tu cuenta.',
				failed: 'Este navegador o dispositivo no pudo crear ni usar una llave de acceso.',
				wrong: 'La llave de acceso no se aceptó. Inténtalo de nuevo.',
				wrongPassword: 'La contraseña no es correcta.',
				codeNeeded:
					'Tu cuenta tiene un segundo factor. Escribe también un código de tu app o un código de recuperación, o confirma con una llave de acceso.',
				wrongCode: 'Ese código no es correcto. Los códigos cambian cada 30 segundos.',
				tooManyPasskeys: 'Una cuenta tiene como máximo 10 llaves de acceso.',
				loginAgain:
					'Tu cuenta aún no tiene contraseña ni segundo factor. Inicia sesión otra vez y luego agrega tu primera llave de acceso en los diez minutos siguientes.',
				tooMany: 'Demasiados intentos fallidos. Inténtalo de nuevo en unos minutos.',
				unreachable: 'El servidor no responde. Inténtalo de nuevo en un momento.'
			}
		},
		reports: {
			send: 'Enviar a este servidor los informes de errores de este navegador',
			intro: 'Cuando algo falla en la página, los admins de este servidor se enteran.',
			what: 'Un informe contiene:',
			holds: [
				'el mensaje del error, sin direcciones y sin números que identifiquen algo',
				'en qué parte de los archivos de rueder ocurrió',
				'la versión de rueder y tu navegador con su versión principal'
			],
			never:
				'Nunca lo que lees, tus feeds, etiquetas ni reglas. Los informes se borran a los 30 días.',
			off: 'Desactivar esto elimina los informes que enviaste.'
		}
	},

	rules: {
		intro:
			'Las reglas ocultan artículos o los marcan como leídos al llegar. Los artículos ocultos no cuentan como sin leer y quedan en “Ocultos por filtros”.',
		none: 'Aún no hay reglas.',
		add: 'Agregar una regla',
		new: 'Regla nueva',
		on: 'Activada',
		scope: 'En',
		scopeAll: 'Todos los feeds',
		scopeFolder: (title) => `Carpeta: ${title}`,
		scopeFeed: (title) => `Feed: ${title}`,
		field: 'Mirar en',
		fields: { any: 'Todo', title: 'Título', content: 'Texto', author: 'Autor', tag: 'Etiqueta' },
		match: 'Comparación',
		matches: {
			contains: 'contiene',
			word: 'contiene la palabra',
			starts: 'empieza con',
			ends: 'termina con',
			regex: 'coincide con la expresión regular',
			all: 'cada artículo, diga lo que diga'
		},
		pattern: 'Texto a buscar',
		action: 'Entonces',
		actions: { hide: 'ocultar el artículo', read: 'marcarlo como leído' },
		preview: (count, total) => `Se aplica ahora a ${n(count)} de ${n(total)} artículos de aquí.`,
		regexTip:
			'Las expresiones regulares se leen como en Rust, sin distinguir mayúsculas de minúsculas.',
		regexBuilder: 'Crear y probar una en regex101',
		invalid: 'Esta no es una expresión regular válida.',
		empty: 'Escribe el texto a buscar.',
		checking: 'Viendo qué atraparía …',
		hits: (count) =>
			`${n(count)} ${plural(count, { one: 'coincidencia', other: 'coincidencias' })}`,
		summary: (field, match, pattern, action) => `${field} ${match} “${pattern}”: ${action}`,
		summaryAll: (action) => `Cada artículo: ${action}`
	},

	keys: {
		title: 'Atajos de teclado',
		note: 'Tus teclas se guardan con los ajustes. Enter, Escape y g se quedan como están.',
		change: (what) => `Cambiar la tecla de: ${what}`,
		changeShort: 'Cambiar',
		press: 'Pulsa una tecla …',
		taken: (key, other) => `${key} ya es la tecla de: ${other}`,
		fixed: (key) => `${key} no se puede asignar a un atajo.`,
		reset: 'Volver a las teclas de rueder',
		groups: { articles: 'Artículos', lists: 'Listas', go: 'Ir a', app: 'En todas partes' },
		next: 'Artículo siguiente, abierto',
		previous: 'Artículo anterior, abierto',
		nextCursor: 'Artículo siguiente, sin abrir',
		previousCursor: 'Artículo anterior, sin abrir',
		toggle: 'Abrir o cerrar el artículo',
		star: 'Destacar o quitar de destacados',
		read: 'Marcar como leído o sin leer',
		original: 'Abrir el original en una pestaña nueva',
		markAll: 'Marcar todo como leído',
		refresh: 'Actualizar la lista',
		unreadOnly: 'Solo sin leer, o todos los artículos',
		listView: 'Vista de lista',
		expandedView: 'Vista expandida',
		splitView: 'Tres paneles',
		magazineView: 'Revista',
		goHome: 'Inicio',
		goAll: 'Todos los artículos',
		goUnread: 'Artículos sin leer',
		goStarred: 'Artículos destacados',
		search: 'Buscar',
		sidebar: 'Mostrar u ocultar la barra lateral',
		help: 'Este resumen',
		then: 'luego'
	},

	labels: {
		title: 'Etiquetas',
		add: 'Etiqueta nueva',
		name: 'Nombre de la etiqueta',
		none: 'Aún no hay etiquetas.',
		actions: (title) => `Acciones de la etiqueta ${title}`,
		rename: 'Cambiar nombre',
		color: 'Color',
		colors: ['Rojo', 'Naranja', 'Amarillo', 'Verde', 'Turquesa', 'Azul', 'Morado', 'Gris'],
		remove: 'Eliminar la etiqueta',
		show: 'Mostrar las etiquetas',
		hide: 'Ocultar las etiquetas',
		forArticle: 'Etiquetas',
		set: (title) => `Etiqueta: ${title}`
	},

	subscribe: {
		title: 'Suscribirse a un feed',
		address: 'Dirección del feed o del sitio web',
		find: 'Buscar feeds',
		finding: 'Buscando …',
		note: 'El borrador no puede descargar nada, así que el feed queda vacío.',
		found: (count) => (count === 1 ? 'Hay un feed:' : `Hay ${n(count)} feeds:`),
		none: 'No hay ningún feed en esta dirección.',
		movedTo: (url) => `El sitio reenvió la solicitud a ${url}.`,
		loginWall:
			'Algunos sitios envían a su página de inicio de sesión a quien no ha iniciado sesión. Entonces sus feeds no se pueden leer.',
		subscribed: 'Suscrito',
		show: 'Mostrar',
		untitled: 'Sin nombre',
		folder: 'En la carpeta',
		topLevel: 'Sin carpeta',
		add: 'Suscribirse',
		adding: 'Suscribiendo …',
		refused: 'No funcionó.',
		serverSays: (reason) => `El servidor dice: ${reason}`,
		busy: (seconds) =>
			seconds
				? `Demasiadas búsquedas a la vez. Inténtalo de nuevo en ${n(seconds)} ${plural(seconds, { one: 'segundo', other: 'segundos' })}.`
				: 'Demasiadas búsquedas a la vez. Inténtalo de nuevo en un momento.',
		noFetching: 'Este servidor no descarga feeds.',
		tooLarge: (megabytes) => `El archivo pesa más de ${n(megabytes)} MB.`,
		unreachable: 'El servidor no responde. Inténtalo de nuevo en un momento.'
	},

	opml: {
		title: 'Importar y exportar',
		intro:
			'Trae tus suscripciones de otro lector de feeds como archivo OPML, o llévatelas contigo.',
		import: 'Importar un archivo OPML',
		importing: 'Importando …',
		imported: (feeds, folders) =>
			`${n(feeds)} ${plural(feeds, { one: 'feed nuevo', other: 'feeds nuevos' })} y ${n(folders)} ${plural(folders, { one: 'carpeta nueva', other: 'carpetas nuevas' })}.`,
		known: (count) =>
			`${n(count)} ${plural(count, { one: 'feed ya estaba suscrito', other: 'feeds ya estaban suscritos' })}.`,
		skipped: (count) =>
			`Se omitieron ${n(count)} ${plural(count, { one: 'entrada que no es un feed legible', other: 'entradas que no son feeds legibles' })}.`,
		later: 'Sus artículos llegan con la próxima ronda de descargas.',
		export: 'Exportar como OPML'
	},

	removeAccount: {
		title: 'Eliminar la cuenta',
		intro: 'Esto elimina tu cuenta en este rueder para siempre. No se puede deshacer.',
		goes: 'Lo que se va con ella: tus sesiones en todos los dispositivos, tus suscripciones y carpetas, lo que leíste, los destacados, etiquetas, reglas de filtro, ajustes, informes de errores, invitaciones y el vínculo con el inicio de sesión único.',
		stays:
			'Los feeds se quedan para quienes los leen. Para llevar tu cuenta a otro lugar primero, usa “Mudanza”.',
		password: 'Tu contraseña',
		name: (name) => `Escribe tu nombre, ${name}, para confirmar`,
		code: 'Código de tu app, o un código de recuperación',
		submit: 'Eliminar mi cuenta para siempre',
		wrong: 'Eso no es correcto. No se eliminó nada.',
		codeNeeded:
			'Tu cuenta tiene un segundo factor. Escribe también un código de tu app o un código de recuperación, o confirma con una llave de acceso.',
		busy: (count) => `Demasiados intentos fallidos. Inténtalo de nuevo en ${minutes(count)}.`,
		unreachable: 'El servidor no responde. No se eliminó nada.'
	},

	move: {
		intro: 'Lleva tu cuenta a otro rueder, o tráela aquí desde uno, en un solo archivo.',
		howItWorks:
			'Una importación añade a lo que hay aquí y no quita nada. Tus ajustes pasan a ser los del archivo.',
		notInIt: 'No está en el archivo: tu contraseña, tus sesiones y los textos de los artículos.',
		export: 'Exportar la cuenta',
		import: 'Importar un archivo de cuenta',
		importing: 'Importando …',
		feeds: (feeds, known) =>
			`${n(feeds)} ${plural(feeds, { one: 'feed nuevo', other: 'feeds nuevos' })}, ${n(known)} ya suscritos.`,
		folders: (count) =>
			`${n(count)} ${plural(count, { one: 'carpeta nueva', other: 'carpetas nuevas' })}.`,
		articles: (found) =>
			`${articles(found)} ${plural(found, { one: 'encontrado', other: 'encontrados' })} aquí.`,
		missing: (count) =>
			`${articles(count)} no ${plural(count, { one: 'se encontró', other: 'se encontraron' })} aquí; se pierde lo que habías leído, destacado o etiquetado de ellos.`,
		marks: (labels, rules) =>
			`${n(labels)} ${plural(labels, { one: 'etiqueta nueva', other: 'etiquetas nuevas' })} y ${n(rules)} ${plural(rules, { one: 'regla de filtro nueva', other: 'reglas de filtro nuevas' })}.`,
		skipped: (count) =>
			`${n(count)} ${plural(count, { one: 'entrada del archivo no se pudo tomar', other: 'entradas del archivo no se pudieron tomar' })}.`,
		settings: 'Ahora tus ajustes son los del archivo.',
		later:
			'Los feeds nuevos traen sus artículos con la próxima ronda de descargas. Lo que habías leído sigue leído.',
		busy: (seconds) =>
			seconds
				? `Demasiadas importaciones. Inténtalo de nuevo en ${minutes(Math.ceil(seconds / 60))}.`
				: 'Demasiadas importaciones. Inténtalo de nuevo más tarde.'
	},

	status: {
		title: 'Estado del servidor',
		open: 'Estado del servidor',
		loading: 'Consultando al servidor …',
		failed: 'El servidor no respondió. Inténtalo de nuevo en un momento.',
		refresh: 'Actualizar',
		asOf: (time) => `A las ${time}. Lo que se cuenta es la última hora.`,
		demo: 'Inventado, porque aquí no hay servidor.',
		empty: '–',
		on: 'Activado',
		off: 'Desactivado',
		took: (median, p95) => `mediana ${median}, el 95 % en menos de ${p95}`,
		summary: {
			label: 'De un vistazo',
			hints: 'Avisos',
			failing: 'Feeds que fallan',
			errors: 'Errores registrados',
			reported: 'Feeds denunciados'
		},
		hintsTitle: 'Para revisar',
		allWell: 'Nada llama la atención.',
		hints: {
			slowRoute: (route, took) =>
				`${route} es lenta: una de cada veinte solicitudes tarda más de ${took}.`,
			serverErrors: (route, count) =>
				`${route} respondió con un error del servidor ${n(count)} ${plural(count, { one: 'vez', other: 'veces' })}.`,
			poolBusy: (size) =>
				`Las ${n(size)} conexiones a la base de datos están en uso, así que las solicitudes esperan una.`,
			fetchBacklog: (count, late) =>
				`${n(count)} ${plural(count, { one: 'feed se descarga', other: 'feeds se descargan' })} con retraso, el más atrasado ${late}.`,
			feedsFailing: (failing, subscribed) => `Fallan ${n(failing)} de ${n(subscribed)} feeds.`,
			notFetching: 'El servidor no descarga feeds. ¿FETCH está en off?',
			fetchPaceFull: (most) =>
				`Cada ronda toma todos los feeds que puede, y aun así hay feeds esperando: el ritmo permite ${n(most)} descargas por hora. Lo fijan FETCH_ROUND, FETCH_AT_ONCE y FETCH_EVERY.`,
			cacheMisses: (share) =>
				`La base de datos encuentra en memoria solo el ${share} de lo que lee; más shared_buffers ayudaría.`,
			databaseLarge: (size, warn) =>
				`La base de datos ha crecido hasta ${size}; DATABASE_WARN_GB avisa a partir de ${warn}. “rueder retention set” elimina los artículos antiguos o sus textos.`,
			longQuery: (took) => `Hay una consulta en marcha desde hace ${took}.`,
			imageCacheFull: (share) => `La caché de imágenes está llena al ${share}.`,
			errors: (count) =>
				`El servidor registró ${n(count)} ${plural(count, { one: 'error', other: 'errores' })} en la última hora.`,
			limited: (count) =>
				`Se ${plural(count, { one: 'rechazó', other: 'rechazaron' })} ${n(count)} ${plural(count, { one: 'solicitud', other: 'solicitudes' })} por ser demasiadas.`,
			browserErrors: (count) =>
				`Los navegadores informaron ${n(count)} ${plural(count, { one: 'error', other: 'errores distintos' })} en el último día.`,
			reportedFeeds: (count) =>
				`Los lectores denunciaron ${n(count)} ${plural(count, { one: 'feed que no está bloqueado', other: 'feeds que no están bloqueados' })}.`
		},
		build: {
			title: 'Compilación',
			commit: 'Commit',
			dirty: (commit) => `${commit}, con cambios propios`,
			branch: 'Rama',
			tag: 'Tag',
			committed: 'Fecha del commit',
			where: 'Se ejecuta',
			container: 'en un contenedor',
			host: 'en la máquina misma',
			target: 'Para',
			profile: 'Compilado',
			profileValue: (profile, level) => `${profile}, optimización ${level}`,
			ci: 'Compilado por',
			ciValue: 'una ejecución de la CI',
			byHand: 'a mano'
		},
		server: {
			title: 'Servidor',
			version: 'Versión',
			running: 'En marcha desde',
			memory: 'Memoria',
			accounts: 'Cuentas',
			feeds: 'Feeds',
			feedsValue: (fetched, all) => `${n(fetched)} descargados, ${n(all)} en total`,
			articles: 'Artículos',
			articlesValue: (count) => `unos ${n(count)}`,
			live: 'Pestañas abiertas'
		},
		database: {
			title: 'Base de datos',
			pool: 'Conexiones de rueder',
			poolValue: (size, idle, max) =>
				`${n(size)} abiertas, ${n(idle)} libres, como máximo ${n(max)}`,
			connections: 'Todas las conexiones',
			size: 'Tamaño',
			sizeWarn: (size, warn) => `${size}, aviso a partir de ${warn}`,
			hitRatio: 'Leído de la memoria',
			deadlocks: 'Bloqueos mutuos',
			longest: 'Consulta más larga en marcha',
			tables: 'Tablas más grandes',
			table: 'Tabla',
			bytes: 'Tamaño',
			rows: 'Filas, aprox.'
		},
		fetcher: {
			title: 'Descarga de feeds',
			state: 'Descarga',
			lastRound: 'Última ronda',
			lastRoundValue: (ago, feeds, took) =>
				`${ago}: ${n(feeds)} ${plural(feeds, { one: 'feed', other: 'feeds' })} en ${took}`,
			fetches: 'Descargas',
			fetchesValue: (fetched, unchanged, failed) =>
				`${n(fetched)} trajeron el feed, ${n(unchanged)} sin cambios, ${n(failed)} fallaron`,
			broken: 'Fallaron en la base de datos',
			newArticles: 'Artículos nuevos',
			took: 'Tiempo por descarga',
			overdue: 'Descargados con retraso',
			overdueValue: (count, late) => `${n(count)}, el más atrasado ${late}`,
			waiting: 'Aún no descargados',
			pace: 'Ritmo',
			paceValue: (round, every, atOnce, most) =>
				`${n(round)} ${plural(round, { one: 'feed', other: 'feeds' })} cada ${every}, ${n(atOnce)} a la vez: como máximo ${n(most)} por hora`,
			perHour: 'Descargas en la última hora',
			behind: 'Retraso',
			behindValue: (late) => `el feed que más espera lleva ${late} de retraso`,
			notBehind: 'No hay nada esperando',
			failing: 'Feeds que fallan, por motivo',
			noneFailing: 'Ningún feed falla.',
			feeds: (count) => `${n(count)} ${plural(count, { one: 'feed', other: 'feeds' })}`,
			failures: {
				status: (code) => (code ? `El sitio responde ${code}` : 'El sitio responde con un error'),
				timeout: 'El sitio tarda demasiado',
				blocked: 'La dirección está en una red de la que rueder no descarga',
				dns: 'El nombre no tiene dirección',
				network: 'Sin conexión, o se cortó',
				too_large: 'Demasiado grande',
				redirects: 'Demasiadas redirecciones, o una rota',
				unreadable: 'No es un feed que rueder pueda leer',
				other: 'Otra cosa'
			}
		},
		finder: {
			title: 'Búsqueda de feeds',
			state: 'Revisión de direcciones',
			lookups: 'Búsquedas',
			lookupsValue: (found, none, failed) =>
				`${n(found)} encontraron feeds, ${n(none)} no encontraron ninguno, ${n(failed)} fallaron`,
			limited: 'Por encima del límite de la cuenta',
			took: 'Tiempo por búsqueda'
		},
		images: {
			title: 'Proxy de imágenes',
			state: 'Proxy',
			cache: 'Caché',
			cacheValue: (used, max) => `${used} de ${max}`,
			served: 'Imágenes',
			servedValue: (cached, fetched) => `${n(cached)} de la caché, ${n(fetched)} descargadas`,
			refused: 'Rechazadas',
			refusedValue: (source, notImage, tooLarge, notFound) =>
				`${n(source)} fallo de la fuente, ${n(notImage)} no son imágenes, ${n(tooLarge)} demasiado grandes, ${n(notFound)} dirección errónea`,
			took: 'Tiempo por imagen descargada',
			failingSources: 'Fuentes que se dejan en paz por ahora'
		},
		requests: {
			title: 'Solicitudes',
			all: 'Todas las solicitudes',
			allValue: (count, took) => `${n(count)}, ${took}`,
			answers: 'Respuestas',
			answersValue: (fine, refused, errors) =>
				`${n(fine)} correctas, ${n(refused)} rechazadas, ${n(errors)} errores del servidor`,
			route: 'Ruta',
			count: 'Cantidad',
			median: 'Mediana',
			p95: '95 %',
			max: 'La más larga',
			errors: 'Errores',
			none: 'No hubo solicitudes en la última hora.'
		},
		problems: {
			title: 'Advertencias y errores',
			counts: (errors, warnings) =>
				`En la última hora, ${n(errors)} ${plural(errors, { one: 'error', other: 'errores' })} y ${n(warnings)} ${plural(warnings, { one: 'advertencia', other: 'advertencias' })}.`,
			none: 'No se registró nada desde el inicio.',
			error: 'Error',
			warning: 'Advertencia',
			times: (count, last) => (count > 1 ? `${n(count)} veces, la última ${last}` : last)
		},
		browsers: {
			title: 'Errores en navegadores',
			off: 'Este servidor no acepta informes de errores (CLIENT_REPORTS=off).',
			consenting: (count) =>
				`${n(count)} ${plural(count, { one: 'cuenta envía', other: 'cuentas envían' })} informes. Los errores se guardan 30 días desde la última vez que ocurrieron.`,
			none: 'Ningún navegador informó un error.',
			kinds: {
				error: 'Error',
				rejection: 'Promesa fallida',
				component: 'Parte de la página',
				request: 'Solicitud'
			},
			hits: (count, accounts, last) =>
				`${n(count)} ${plural(count, { one: 'vez', other: 'veces' })} en ${n(accounts)} ${plural(accounts, { one: 'cuenta', other: 'cuentas' })}, la última ${last}`,
			where: (versions, browsers) => `Versión ${versions} · ${browsers}`
		},
		reported: {
			title: 'Feeds denunciados',
			text: 'Aquí no se muestra quién denunció. “rueder feed block <dirección>” bloquea un feed para todos, “rueder feed unblock <dirección>” lo desbloquea.',
			none: 'Nadie denunció ningún feed.',
			reports: (count, last) =>
				`${n(count)} ${plural(count, { one: 'denuncia', other: 'denuncias' })}, la última ${last}`,
			blocked: 'Bloqueado'
		}
	},

	update: {
		ready: 'Hay una versión nueva de rueder lista.',
		reload: 'Recargar',
		later: 'Más tarde'
	},

	crash: {
		title: 'Algo salió mal en esta página.',
		text: 'Recarga la página para seguir. Lo que leíste y cambiaste está guardado en el servidor.',
		reload: 'Recargar'
	},

	demo: {
		badge: 'Borrador',
		note: 'Un borrador con feeds inventados. Nada se guarda en un servidor.',
		arrive: 'Hacer llegar un artículo',
		revise: 'Cambiar el artículo abierto',
		revised: 'Actualización: este párrafo se añadió para probar las versiones.',
		reviseHint: 'Abre primero un artículo.'
	}
};

export default es;
