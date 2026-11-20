// The French texts, in the shape of the English catalog (en.ts), addressing the
// reader with “vous”. A feed is a “flux”, an item an “article”, a starred one a
// “favori”. Before : ; ? ! and inside « » stands a no-break space, as French
// typography wants.

import type { Messages } from './en.ts';
import { formatNumber as n, plural } from './intl.ts';

const articles = (count: number) =>
	`${n(count)} ${plural(count, { one: 'article', other: 'articles' })}`;

const minutes = (count: number) =>
	`${n(count)} ${plural(count, { one: 'minute', other: 'minutes' })}`;

const fr: Messages = {
	languageName: 'Français',

	articles,

	common: {
		close: 'Fermer',
		cancel: 'Annuler',
		save: 'Enregistrer',
		delete: 'Supprimer',
		edit: 'Modifier'
	},

	nav: {
		subscribe: 'S’abonner',
		home: 'Accueil',
		all: 'Tous les articles',
		unreadItems: 'Articles non lus',
		starred: 'Articles favoris',
		hidden: 'Masqués par les filtres',
		subscriptions: 'Abonnements',
		newFolder: 'Nouveau dossier',
		groupedFolder: 'Nouveau dossier',
		folderName: 'Nom du dossier',
		renameFolder: 'Renommer',
		removeFolder: 'Supprimer le dossier, garder son contenu',
		folderActions: (title) => `Actions pour le dossier ${title}`,
		feedName: 'Nom du flux',
		feedNameHint: 'Vide pour le nom propre du flux',
		renameFeed: 'Renommer',
		unsubscribe: 'Se désabonner',
		reportFeed: 'Signaler le flux …',
		feedInfo: 'Infos sur le flux …',
		feedActions: (title) => `Actions pour le flux ${title}`,
		unsubscribeTitle: 'Se désabonner ?',
		unsubscribeText: (title) =>
			`Se désabonner de « ${title} » ? Ce que vous y avez lu et mis en favori sera perdu.`,
		openFolder: 'Ouvrir le dossier',
		closeFolder: 'Fermer le dossier',
		openSidebar: 'Afficher la barre latérale',
		closeSidebar: 'Masquer la barre latérale',
		search: 'Rechercher',
		searchPlaceholder: 'Rechercher dans ces articles',
		clearSearch: 'Effacer la recherche',
		settings: 'Paramètres',
		accountSettings: 'Paramètres du compte',
		shortcuts: 'Raccourcis clavier',
		live: 'En direct',
		liveOn: 'Les nouveaux articles arrivent tout seuls. Cliquez pour mettre en pause.',
		liveOff: 'En pause. Cliquez pour que les nouveaux articles arrivent de nouveau.',
		paused: 'En pause',
		account: (name) => `Connecté en tant que ${name}`,
		logout: 'Se déconnecter',
		appSettings: 'Serveur et mises à jour de l’app',
		install: 'Installer comme app',
		installSafari:
			'Pour installer rueder comme app : Fichier › Ajouter au Dock sur un Mac, Partager › Sur l’écran d’accueil sur un iPhone ou un iPad.',
		unread: (count) => `${n(count)} non ${plural(count, { one: 'lu', other: 'lus' })}`,
		hiddenCount: (count) =>
			`${n(count)} ${plural(count, { one: 'masqué', other: 'masqués' })} par les filtres`,
		fetchFailed: (reason) => `La dernière récupération a échoué : ${reason}`,
		waiting: 'Pas encore récupéré',
		blocked: 'Bloqué par l’administrateur de ce rueder',
		moveHint: 'Faites glisser pour trier, ou appuyez sur Alt avec les flèches',
		resizeSidebar: 'Largeur de la barre latérale : faites glisser, ou utilisez les flèches',
		tooDeep: (levels) => `Les dossiers s’imbriquent sur ${n(levels)} niveaux au plus.`
	},

	toolbar: {
		refresh: 'Actualiser',
		showAll: 'Tous les articles',
		showUnread: 'Non lus seulement',
		show: 'Afficher',
		newestFirst: 'Les plus récents d’abord',
		oldestFirst: 'Les plus anciens d’abord',
		markAllRead: 'Tout marquer comme lu',
		markOptions: 'Autres façons de marquer comme lu',
		phoneMenu: (shown) => `Affichage, montre : ${shown}`,
		markMenu: 'Marquer comme lu …',
		markEverything: 'Tous les articles',
		markOlderDay: 'Articles de plus d’un jour',
		markOlderWeek: 'Articles de plus d’une semaine',
		listView: 'Vue en liste',
		expandedView: 'Vue développée',
		splitView: 'Trois volets : arbre, liste et article',
		magazineView: 'Magazine',
		previous: 'Article précédent',
		next: 'Article suivant',
		newArticles: (count) =>
			`${n(count)} ${plural(count, { one: 'nouvel article', other: 'nouveaux articles' })}`
	},

	list: {
		empty: 'Aucun article ici.',
		emptyUnread: 'Rien de non lu ici.',
		emptySearch: 'Aucun article ne correspond à la recherche.',
		found: (count, more, words) =>
			`${n(count)}${more ? '+' : ''} ${plural(count, { one: 'article', other: 'articles' })} avec « ${words} »`,
		searchAll: 'Rechercher dans tous les flux',
		feedFailing:
			'rueder réessaie plus tard, et attend plus longtemps après chaque échec ou aussi longtemps que le site le demande. Si cela continue d’échouer, le flux a peut-être déménagé.',
		hiddenNote:
			'Vos règles de filtrage ont masqué ces articles. Ouvrez-en un pour voir quelle règle l’a fait.',
		toRules: 'Règles de filtrage',
		emptyFeed: 'Ce flux n’a pas encore d’articles.',
		waitingFeed:
			'Ce flux n’a pas encore été récupéré. Ses articles arrivent avec la prochaine tournée de récupération, d’ici quelques minutes.',
		showAll: 'Afficher tous les articles',
		more: 'Afficher plus',
		star: 'Ajouter aux favoris',
		unstar: 'Retirer des favoris',
		keepUnread: 'Garder non lu',
		openOriginal: 'Ouvrir l’original',
		showFeed: 'Afficher tous les articles de ce flux dans rueder',
		from: 'de',
		by: (author) => `par ${author}`,
		updated: 'mis à jour',
		updatedHeadline: 'nouveau titre',
		updatedTitle: (count) =>
			`Modifié depuis sa première récupération ; ${n(count)} ${plural(count, { one: 'version antérieure', other: 'versions antérieures' })}`,
		updatedHeadlineTitle: (count) =>
			`Son titre a été réécrit, son texte est resté le même ; ${n(count)} ${plural(count, { one: 'version antérieure', other: 'versions antérieures' })}`,
		compare: 'Comparer les versions',
		hiddenBy: (rule) => `Masqué par la règle : ${rule}`,
		textDropped: 'Ce serveur ne garde plus le texte de cet article, car il est ancien.',
		textDroppedOriginal: 'Le lire à sa source',
		loading: 'Chargement du texte …',
		loadingList: 'Chargement …',
		toTop: 'Revenir en haut',
		pick: 'Choisissez un article pour le lire ici.',
		resizeList: 'Largeur de la liste : faites glisser, ou utilisez les flèches',
		closeArticle: 'Fermer l’article',
		blockedFeed:
			'L’administrateur de ce rueder a bloqué ce flux. Il n’est pas récupéré, et ses articles ne sont pas affichés.',
		articleActions: 'Autres actions pour l’article',
		articleInfo: 'Infos sur l’article …',
		articleSource: 'Code source …',
		reportArticle: 'Signaler l’article …'
	},

	report: {
		feedTitle: 'Signaler le flux',
		articleTitle: 'Signaler l’article',
		text: (title) =>
			`L’administrateur de ce rueder voit que « ${title} » a été signalé, pourquoi et ce que vous écrivez, mais pas qui l’a signalé. Il décide de le bloquer ou non pour tout le monde.`,
		reason: 'Motif',
		reasons: {
			illegal: 'Contraire à la loi',
			copyright: 'L’œuvre de quelqu’un d’autre',
			malicious: 'Logiciel malveillant, hameçonnage ou fraude',
			spam: 'Spam',
			other: 'Autre chose'
		},
		note: 'Ce que vous aimeriez ajouter (facultatif)',
		left: (count) =>
			`${n(count)} ${plural(count, { one: 'caractère restant', other: 'caractères restants' })}`,
		send: 'Envoyer le signalement',
		sending: 'Envoi …',
		sent: 'Merci. Le signalement est parvenu à l’administrateur de ce rueder.',
		tooMany: 'Vous avez envoyé beaucoup de signalements ces derniers temps. Réessayez plus tard.',
		unreachable: 'Le signalement n’est pas parvenu au serveur. Réessayez plus tard.'
	},

	feedInfo: {
		title: 'Infos sur le flux',
		text: (title) =>
			`Ce que ce rueder sait de « ${title} » et comment il le récupère. Si le flux ne fonctionne pas, copiez ceci dans votre message à l’administrateur.`,
		number: 'Numéro',
		name: 'Nom',
		address: 'Adresse du flux',
		site: 'Site web',
		noSite: 'Aucun',
		agent: 'Récupéré en tant que',
		agents: { rueder: 'rueder', browser: 'Navigateur' },
		every: 'Récupéré toutes les',
		fetched: 'Dernière récupération',
		succeeded: 'Dernière récupération réussie',
		failed: 'Dernière erreur',
		next: 'Prochaine récupération',
		blocked: 'Aucune, l’administrateur a bloqué le flux',
		articles: 'Articles jusqu’ici',
		never: 'Jamais',
		fetchAs: 'Le récupérer en tant que',
		fetchAsHint:
			'Pour tous ceux qui lisent le flux. En tant que navigateur pour les sites qui refusent rueder. Il est récupéré de nouveau tout de suite.',
		notSwitched: 'Cela n’a pas fonctionné. Réessayez plus tard.',
		copy: 'Copier',
		copied: 'Copié',
		loading: 'Chargement …',
		unreachable: 'Le serveur n’a pas répondu. Réessayez plus tard.'
	},
	articleInfo: {
		title: 'Infos sur l’article',
		text: (title) =>
			`Ce que ce rueder sait de « ${title} » et de ses versions. Si quelque chose vous semble faux, copiez ceci dans votre message à l’administrateur.`,
		feed: 'Flux',
		feedNumber: 'Numéro du flux',
		number: 'Numéro dans le flux',
		name: 'Titre',
		id: 'Identifiant dans le flux',
		link: 'Lien',
		noLink: 'Aucun',
		published: 'Publié',
		fetched: 'Première récupération',
		changed: 'Dernière modification',
		never: 'Jamais',
		changedAt: (when, what) => `${when} (${what})`,
		what: { title: 'le titre', text: 'le texte', both: 'le titre et le texte' },
		version: 'Version',
		kept: 'Versions conservées',
		textLabel: 'Texte',
		textDropped: 'N’est plus conservé, car il est ancien',
		copy: 'Copier',
		copied: 'Copié',
		loading: 'Chargement …',
		unreachable: 'Le serveur n’a pas répondu. Réessayez plus tard.'
	},
	articleSource: {
		title: 'Code source de l’article',
		text: 'L’article en HTML, tel que ce rueder le conserve après l’avoir nettoyé, ou tel que le flux l’a envoyé. Les liens et les images gardent ici leurs propres adresses, si bien qu’un paramètre de suivi restant apparaît.',
		show: 'Afficher',
		kinds: { cleaned: 'Nettoyé', original: 'Tel que le flux l’a envoyé' },
		noText: 'Ce serveur ne conserve plus le texte de cet article.',
		noOriginal: 'Ce serveur ne conserve plus ce que le flux a envoyé.',
		copy: 'Copier',
		copied: 'Copié',
		loading: 'Chargement …',
		unreachable: 'Le serveur n’a pas répondu. Réessayez plus tard.'
	},

	media: {
		title: 'Pièces jointes',
		play: 'Lire'
	},

	connection: {
		reconnecting: 'Reconnexion …',
		unreachable: (since) => `Le serveur ne répond plus depuis ${since}.`,
		offline: (since) => `Pas de réseau depuis ${since}.`,
		retry: 'Réessayer maintenant',
		unsent: (count) =>
			plural(count, {
				one: '1 modification attend.',
				other: `${n(count)} modifications attendent.`
			})
	},

	notice: {
		unreachable: 'Le serveur ne répond pas. Ce que vous voyez n’est peut-être plus à jour.',
		notSaved: 'Le serveur n’a pas accepté votre dernière modification. Elle a été annulée ici.',
		tooManyUnsent: 'Tant de modifications attendent le serveur que celle-ci a été annulée.'
	},

	login: {
		title: 'Se connecter à rueder',
		about:
			'Un lecteur de flux : il récupère ce que publient les sites que vous suivez et montre ce que vous n’avez pas encore lu.',
		name: 'Nom',
		password: 'Mot de passe',
		submit: 'Se connecter',
		checking: 'Un instant …',
		wrong: 'Nom ou mot de passe incorrect.',
		codeNeeded:
			'Ce compte a un second facteur. Saisissez le code affiché par votre app ou un code de récupération, ou connectez-vous avec une clé d’accès.',
		wrongCode: 'Ce code n’est pas bon. Les codes changent toutes les 30 secondes.',
		code: 'Code',
		codeNote:
			'Les six chiffres de votre app d’authentification, ou l’un de vos codes de récupération.',
		tooMany: (count) => `Trop d’essais erronés. Réessayez dans ${minutes(count)}.`,
		unreachable: 'Le serveur ne répond pas. Réessayez dans un instant.',
		retry: 'Réessayer',
		or: 'ou',
		oidc: 'Se connecter avec l’authentification unique',
		passkey: 'Se connecter avec une clé d’accès',
		passkeyCancelled:
			'Aucune clé d’accès n’a été utilisée. Réessayez, ou connectez-vous avec nom et mot de passe.',
		passkeyFailed: 'Ce navigateur ou cet appareil ne peut pas utiliser de clé d’accès ici.',
		passkeyWrong: 'Cette clé d’accès n’appartient à aucun compte ici.',
		oidcFailed: 'La connexion par authentification unique n’a pas fonctionné. Réessayez.',
		oidcDenied: 'La connexion par authentification unique a été annulée.',
		oidcUnknown:
			'Aucun compte ici n’appartient à cette connexion. Demandez à l’administrateur de ce rueder de la lier.',
		disabled: 'Ce compte est désactivé. Demandez pourquoi à l’administrateur de ce rueder.',
		unconfirmed:
			'Ce compte attend que son adresse e-mail soit confirmée. Ouvrez le lien de l’e-mail, ou faites-le renvoyer.',
		resend: 'Renvoyer le lien',
		resent: 'Le lien est reparti. Regardez dans votre messagerie, et aussi dans les spams.',
		resendWrong: 'Saisissez le nom et le mot de passe du compte pour faire renvoyer le lien.',
		forgot: 'Mot de passe oublié ?',
		removed: 'Votre compte est supprimé, avec tout ce qui vous appartenait.',
		note: 'Il n’y a pas d’inscription. Les comptes sont créés par l’administrateur de ce rueder.'
	},

	legal: {
		imprint: 'Mentions légales',
		privacy: 'Politique de confidentialité',
		terms: 'Conditions d’utilisation',
		agree: { before: 'J’accepte les ', link: 'conditions d’utilisation', after: '.' },
		changed: 'Les conditions d’utilisation ont changé',
		changedIntro:
			'Veuillez lire les nouvelles conditions d’utilisation. Pour continuer à utiliser rueder, acceptez-les.',
		read: 'Lire les conditions d’utilisation',
		accept: 'J’accepte',
		decline: 'Se déconnecter',
		notSaved: 'Cela n’a pas fonctionné. Réessayez dans un instant.'
	},

	register: {
		title: 'Créer un compte',
		invited: 'Vous êtes invité sur ce rueder. Choisissez un nom et un mot de passe.',
		again: 'Le mot de passe encore une fois',
		passwordRule: (characters) => `Au moins ${n(characters)} caractères.`,
		mismatch: 'Les deux mots de passe diffèrent.',
		submit: 'Créer le compte',
		solving: 'Vérification de ce navigateur …',
		toRegister: 'Créer un compte',
		toLogin: 'Se connecter à un compte existant',
		closed: 'Personne ne peut s’inscrire ici sans invitation.',
		inviteGone: 'Cette invitation n’est plus utilisable. Demandez-en une nouvelle.',
		full: 'Il n’y a plus de place pour un autre compte ici.',
		today: 'Assez de comptes ont été créés ici pour aujourd’hui. Réessayez demain.',
		captcha: 'La vérification de ce navigateur n’a pas abouti. Réessayez.',
		taken: 'Un compte porte déjà ce nom.',
		terms: 'Acceptez les conditions d’utilisation pour créer un compte.',
		invalid: (reason) => `Ce ne peut pas être un compte. Le serveur dit : ${reason}`,
		email: 'Adresse e-mail',
		emailOptional: 'Adresse e-mail (facultative)',
		emailNote: 'Vous recevrez un lien pour la confirmer.',
		emailRequiredNote:
			'Vous recevrez un lien pour la confirmer ; le compte pourra se connecter une fois le lien ouvert.',
		emailInvalid: 'Saisissez une adresse e-mail qui peut recevoir des messages.',
		unsent: 'Le lien n’a pas pu être envoyé, aucun compte n’a donc été créé. Réessayez plus tard.',
		tooMany: (count) => `Trop d’essais. Réessayez dans ${minutes(count)}.`,
		awaitingTitle: 'Regardez vos e-mails',
		awaiting: (address, hours) =>
			`Le compte est créé. Un lien est parti vers ${address} : ouvrez-le dans les ${n(hours)} heures, puis connectez-vous.`
	},

	forgot: {
		title: 'Mot de passe oublié ?',
		intro:
			'Indiquez votre nom ou votre adresse e-mail. Un lien pour choisir un nouveau mot de passe part vers l’adresse confirmée.',
		field: 'Nom ou adresse e-mail',
		submit: 'Envoyer le lien',
		asked: (count) =>
			`S’il existe un tel compte avec une adresse confirmée, un lien est en route vers elle, valable ${minutes(count)}. Regardez aussi dans les spams.`,
		tooMany: (count) => `Demandé trop souvent. Réessayez dans ${minutes(count)}.`,
		back: 'Retour à la connexion'
	},

	reset: {
		title: 'Choisir un nouveau mot de passe',
		intro: 'Tous les navigateurs et appareils du compte seront déconnectés.',
		fresh: 'Nouveau mot de passe',
		again: 'Nouveau mot de passe encore une fois',
		submit: 'Enregistrer le mot de passe',
		mismatch: 'Les deux ne correspondent pas.',
		gone: 'Ce lien ne fonctionne plus. Demandez-en un nouveau.',
		invalid: (reason) => `Le serveur n’a pas accepté ce mot de passe. Il dit : ${reason}`,
		done: 'Le mot de passe est enregistré. Connectez-vous avec.',
		cancel: 'Annuler'
	},

	email: {
		title: 'Adresse e-mail',
		intro: 'rueder ne vous écrit qu’au sujet de votre compte.',
		current: 'Votre adresse',
		none: 'Le compte n’a pas d’adresse.',
		pending: (address, hours) =>
			`Un lien est parti vers ${address}. L’adresse compte une fois qu’il est ouvert, dans les ${n(hours)} heures ; jusque-là, l’ancienne reste.`,
		address: 'Nouvelle adresse',
		password: 'Votre mot de passe',
		passwordNote: 'Pour que personne d’autre ayant accès à ce navigateur ne détourne vos e-mails.',
		code: 'Code du second facteur',
		submit: 'Envoyer le lien',
		sent: 'Le lien est en route. Regardez dans votre messagerie, et aussi dans les spams.',
		remove: 'Supprimer l’adresse',
		removeIntro:
			'Sans adresse, vous ne recevez aucun e-mail. Pour le reste, le compte reste tel quel.',
		removed: 'L’adresse est supprimée.',
		wrong: 'Le mot de passe ou le code n’est pas bon.',
		codeNeeded: 'Ce compte a un second facteur. Saisissez aussi son code.',
		loginAgain:
			'Votre compte n’a encore ni mot de passe ni second facteur. Reconnectez-vous, puis indiquez l’adresse dans les dix minutes.',
		invalid: 'Saisissez une adresse e-mail qui peut recevoir des messages.',
		same: 'C’est déjà l’adresse du compte.',
		required:
			'Chaque compte de ce rueder a besoin d’une adresse ; on peut la changer, pas la supprimer.',
		busy: (count) => `Trop d’essais ou d’e-mails. Réessayez dans ${minutes(count)}.`,
		unsent: 'L’e-mail n’a pas pu être envoyé. Réessayez plus tard.',
		unreachable: 'Le serveur ne répond pas. Rien n’a été modifié.',
		askTitle: 'Ajoutez une adresse e-mail',
		askIntro:
			'Chaque compte de ce rueder a désormais besoin d’une adresse e-mail. Indiquez la vôtre, et ouvrez le lien qui y est envoyé.',
		askWaiting:
			'Une fois le lien ouvert, ce message disparaît tout seul. Rien reçu ? Renvoyez-le, ou envoyez-le à une autre adresse.',
		logOut: 'Se déconnecter',
		confirmed: (address) => `L’adresse ${address} est confirmée.`,
		confirmedLogIn: 'Le compte est prêt : connectez-vous.',
		gone: 'Ce lien n’est plus utilisable : il a servi, un plus récent a été envoyé, ou il a plus d’un jour.',
		taken: 'Un autre compte a pris cette adresse entre-temps.',
		confirmUnreachable: 'Le serveur ne répond pas. Rouvrez le lien dans un instant.',
		close: 'Fermer'
	},

	invites: {
		intro:
			'Un lien permet à quelqu’un de créer un compte sur ce rueder. Il n’est affiché qu’une fois, ici, quand vous le créez.',
		left: (count) =>
			count === 0
				? 'Vous n’avez aucune invitation à donner.'
				: `Vous pouvez encore inviter ${n(count)} ${plural(count, { one: 'personne', other: 'personnes' })}.`,
		admin: 'En tant qu’admin, vous pouvez toujours inviter.',
		uses: 'Pour combien de comptes',
		make: 'Créer un lien',
		noneLeft: 'Il ne vous reste pas assez d’invitations pour autant.',
		failed: 'Cela n’a pas fonctionné. Réessayez dans un instant.',
		link: 'Le lien, à transmettre maintenant',
		copy: 'Copier',
		copied: 'Copié',
		goodFor: (days) =>
			`Un lien est valable ${n(days)} ${plural(days, { one: 'jour', other: 'jours' })}.`,
		yours: 'Vos liens',
		none: 'Vous n’avez encore créé aucun lien.',
		used: (uses, of) => `${n(uses)} sur ${n(of)} utilisés`,
		until: (when) => `valable jusqu’au ${when}`,
		ran: (when) => `expiré ${when}`,
		usedUp: 'épuisé',
		revoked: 'retiré',
		revoke: 'Retirer',
		revokeLink: (made) => `Retirer le lien créé ${made}`
	},

	home: {
		newIn: 'Nouveautés dans vos abonnements',
		allRead: 'Vous avez tout lu. Les nouveaux articles s’affichent ici dès qu’ils arrivent.',
		recentlyStarred: 'Ajoutés récemment aux favoris',
		noStars: 'Aucun favori pour l’instant. Appuyez sur s sur un article.',
		problems: 'Flux qui posent problème',
		tips: 'Astuces',
		tipKeys: 'Appuyez sur ? pour tous les raccourcis clavier ; j et k parcourent les articles.',
		tipDrag: 'Faites glisser les flux et les dossiers dans la barre latérale pour les trier.',
		tipFilters:
			'Les règles de filtrage, dans les paramètres, masquent des articles par mot-clé ou par auteur.',
		showFeed: (title) => `Afficher ${title}`
	},

	versions: {
		title: 'Versions de cet article',
		current: 'Version actuelle',
		version: (rev) => `Version ${n(rev)}`,
		fetched: (when) => `récupérée ${when}`,
		compareWith: 'Comparer la version actuelle avec',
		legendAdded: 'ajouté',
		legendRemoved: 'supprimé',
		titleWas: 'Le titre était :',
		same: 'Le texte de cette version est identique à celui de la version actuelle.',
		sameWords: 'Les mots sont les mêmes ; des images ou des liens ont changé.',
		markup:
			'Les mots, les images et les liens sont les mêmes ; seule la mise en forme du texte a changé.',
		pictures: 'Images modifiées',
		noPicture: 'Aucune',
		links: 'Liens modifiés'
	},

	account: {
		title: 'Paramètres du compte',
		tabs: {
			password: 'Mot de passe',
			email: 'E-mail',
			security: 'Sécurité',
			apps: 'Apps',
			invites: 'Invitations',
			move: 'Déménagement',
			privacy: 'Confidentialité',
			remove: 'Supprimer le compte'
		},
		password: {
			title: 'Changer le mot de passe',
			titleFirst: 'Choisir un mot de passe',
			intro:
				'Un nouveau mot de passe déconnecte tous les autres navigateurs et appareils et révoque tous les mots de passe d’app.',
			introFirst:
				'Avec un mot de passe, ce compte peut aussi se connecter avec son nom, pas seulement par authentification unique.',
			current: 'Mot de passe actuel',
			fresh: 'Nouveau mot de passe',
			again: 'Nouveau mot de passe encore une fois',
			submit: 'Changer le mot de passe',
			submitFirst: 'Enregistrer le mot de passe',
			done: 'Le mot de passe est changé. Tous les autres navigateurs et appareils sont déconnectés, et les mots de passe d’app sont révoqués.',
			doneFirst: 'Le mot de passe est enregistré.',
			mismatch: 'Les deux ne correspondent pas.',
			code: 'Code du second facteur',
			wrong: 'Le mot de passe actuel ou le code n’est pas bon.',
			codeNeeded:
				'Ce compte a un second facteur. Saisissez aussi un code de votre app d’authentification ou un code de récupération, ou confirmez avec une clé d’accès.',
			loginAgain:
				'Votre compte n’a encore ni mot de passe ni second facteur. Reconnectez-vous, puis choisissez le mot de passe dans les dix minutes.',
			invalid: (characters) =>
				`Le serveur n’a pas accepté ce mot de passe. Au moins ${n(characters)} caractères.`,
			busy: (count) => `Trop d’essais erronés. Réessayez dans ${minutes(count)}.`,
			unreachable: 'Le serveur ne répond pas. Le mot de passe n’a pas changé.'
		},
		settingsNote:
			'L’apparence, la lecture, les règles de filtrage et le clavier se trouvent dans les Paramètres.'
	},

	settings: {
		title: 'Paramètres',
		tabs: {
			look: 'Apparence',
			reading: 'Lecture',
			rules: 'Règles de filtrage',
			keys: 'Clavier'
		},
		accountNote:
			'Le mot de passe, le second facteur, les apps et les invitations se trouvent dans les Paramètres du compte, dans le menu de votre compte.',
		theme: 'Thème',
		themes: { system: 'Comme le système', light: 'Clair', dark: 'Sombre' },
		accent: 'Couleur d’accent',
		accents: {
			red: 'Rouge',
			blue: 'Bleu',
			green: 'Vert',
			orange: 'Orange',
			purple: 'Violet',
			graphite: 'Graphite'
		},
		density: 'Densité de la liste',
		densities: { compact: 'Compacte', comfortable: 'Aérée' },
		sidebarSide: 'Barre latérale',
		sidebarSides: { start: 'À gauche', end: 'À droite' },
		textSize: 'Taille du texte des articles, sur cet appareil',
		textSizes: { small: 'Petite', medium: 'Moyenne', large: 'Grande' },
		readingFont: 'Police des articles',
		readingFonts: { sans: 'Sans empattement', serif: 'Avec empattement' },
		counters: 'Compteurs de non lus',
		counterStyles: { parens: 'Entre parenthèses', badge: 'En pastilles' },
		language: 'Langue',
		languageAuto: 'Comme le navigateur',
		view: 'Affichage',
		views: { list: 'Liste', expanded: 'Développé', split: 'Trois volets', magazine: 'Magazine' },
		markOnScroll: 'Marquer les articles comme lus en les faisant défiler dans la vue développée',
		teasers: 'Afficher les premiers mots du texte après le titre dans la liste',
		wakeLock: 'Garder l’écran allumé tant que rueder est ouvert',
		wakeLockMissing: 'Ce navigateur ne peut pas garder l’écran allumé.',
		note: 'Les modifications s’appliquent tout de suite et sont enregistrées avec votre compte : elles vous suivent sur chaque appareil.',
		noteDemo: 'Les modifications s’appliquent tout de suite et restent dans ce navigateur.',
		apps: {
			intro:
				'D’autres apps de lecture de flux peuvent utiliser votre compte via l’API Google Reader. Chacune reçoit son propre mot de passe d’app, que vous pouvez révoquer à tout moment.',
			none: 'Aucune app n’est encore configurée.',
			made: (when) => `configurée ${when}`,
			used: (when) => `dernière utilisation ${when}`,
			unused: 'pas encore utilisée',
			revoke: 'Révoquer',
			revokeName: (name) => `Révoquer le mot de passe d’app ${name}`,
			add: 'Configurer une app',
			full: (max) =>
				`Un compte a au plus ${n(max)} mots de passe d’app. Révoquez-en un pour configurer une autre app.`,
			choose: 'Quelle app voulez-vous utiliser ?',
			website: (app) => `Site web de ${app}`,
			other: 'Une autre app',
			otherHint: 'Toute app qui propose l’API Google Reader ou FreshRSS.',
			platforms: {
				apple: 'Mac, iPhone et iPad',
				android: 'Android'
			},
			name: 'Nom du mot de passe d’app',
			password: 'Votre mot de passe',
			code: 'Code de votre app d’authentification, ou un code de récupération',
			create: 'Créer le mot de passe d’app',
			creating: 'Un instant …',
			back: 'Retour',
			how: {
				netnewswire:
					'Dans NetNewsWire, ouvrez les Réglages, puis Comptes, ajoutez un compte (sur Mac avec +) et choisissez FreshRSS. Saisissez le nom ci-dessous comme Login (sur iPhone et iPad : Username or Email), le mot de passe d’app comme Password et l’adresse du serveur comme API URL.',
				readyou:
					'Dans Read You, ouvrez Paramètres, Comptes, Ajouter des comptes, et choisissez Google Reader. Saisissez l’adresse du serveur ci-dessous comme URL du serveur, le nom comme nom d’utilisateur et le mot de passe d’app comme mot de passe.',
				other:
					'Dans votre app, ajoutez un compte du type « Google Reader API » ou, à défaut, « FreshRSS ». Saisissez l’adresse du serveur, le nom et le mot de passe d’app ci-dessous.'
			},
			ownIcons: (app) =>
				`Confidentialité : ${app} charge elle-même les icônes de vos flux, depuis les sites web des flux, qui voient alors votre adresse IP. Les articles et leurs images passent uniquement par rueder.`,
			shotsTitle: 'Pas à pas, en images',
			shots: {
				readyou: [
					'Touchez l’engrenage en haut à gauche.',
					'Ouvrez Comptes.',
					'Touchez Ajouter des comptes.',
					'Choisissez Google Reader.',
					'Saisissez l’adresse du serveur comme URL du serveur, le nom comme nom d’utilisateur et le mot de passe d’app comme mot de passe, puis touchez Ajouter.'
				],
				netnewswire: [
					'Sur Mac : ouvrez les Réglages (⌘,), puis Comptes, et cliquez sur + en bas à gauche.',
					'Choisissez FreshRSS et cliquez sur Continuer.',
					'Saisissez le nom comme Login, le mot de passe d’app comme Password et l’adresse du serveur comme API URL, puis cliquez sur Créer.'
				]
			},
			shotAlt: (app, step) => `${app}, étape ${n(step)}`,
			server: 'Adresse du serveur',
			user: 'Nom',
			secret: 'Mot de passe d’app',
			copy: (what) => `Copier : ${what}`,
			copied: 'Copié',
			once: 'Le mot de passe d’app n’est affiché que maintenant. Si vous le perdez, révoquez-le et créez-en un nouveau.',
			waiting: 'En attente de la connexion de l’app …',
			connected: 'Connectée : l’app s’est connectée.',
			done: 'Terminé',
			refusals: {
				wrongPassword: 'Le mot de passe n’est pas bon.',
				codeNeeded:
					'Saisissez aussi un code de votre app d’authentification ou un code de récupération, ou confirmez avec une clé d’accès.',
				wrongCode: 'Ce code n’est pas bon. Les codes changent toutes les 30 secondes.',
				loginAgain:
					'Votre compte n’a encore ni mot de passe ni second facteur. Reconnectez-vous, puis créez le mot de passe d’app dans les dix minutes.',
				tooManyApps: 'Un compte a au plus 20 mots de passe d’app.',
				tooMany: 'Trop d’essais erronés. Réessayez dans quelques minutes.',
				unreachable: 'Le serveur ne répond pas. Réessayez dans un instant.'
			}
		},
		secondFactor: {
			title: 'Second facteur',
			intro:
				'Avec un second facteur, la connexion demande aussi un code d’une app d’authentification, ou une clé d’accès.',
			app: 'App d’authentification',
			on: 'Activé.',
			left: (count) =>
				`${n(count)} ${plural(count, { one: 'code de récupération restant', other: 'codes de récupération restants' })}.`,
			off: 'Désactivé.',
			byPasskeys: 'Non configurée. Vos clés d’accès gardent le second facteur activé.',
			adminNeeds:
				'Ce serveur ne fait des admins des admins qu’avec un second facteur. Configurez-le, ou reconnectez-vous avec son code, pour voir l’état du serveur.',
			password: 'Votre mot de passe',
			start: 'Configurer',
			scan: 'Scannez ce code avec votre app d’authentification, ou saisissez la clé ci-dessous.',
			qr: 'Code QR pour l’app d’authentification',
			larger: 'Afficher le code QR en plus grand',
			smaller: 'Afficher le code QR en plus petit',
			key: 'Clé',
			copyKey: 'Copier la clé',
			copied: 'Copié',
			code: 'Le code que l’app affiche maintenant',
			codeOrRecovery: 'Code de l’app, ou un code de récupération',
			confirm: 'Confirmer',
			cancel: 'Annuler',
			codesTitle: 'Vos codes de récupération',
			codesIntro:
				'Chacun vous connecte une fois sans votre téléphone. Gardez-les en lieu sûr : ils ne sont affichés que maintenant.',
			copyCodes: 'Copier les codes',
			codesDone: 'Je les ai gardés',
			switchOff: 'Désactiver',
			switchOffNote: 'La connexion ne demande alors de nouveau que le mot de passe.',
			switchOffNoteKeys: 'Vos clés d’accès gardent le second facteur activé.',
			startWithPasskey: 'Configurer avec une clé d’accès',
			switchOffWithPasskey: 'Désactiver avec une clé d’accès',
			refusals: {
				wrongPassword: 'Le mot de passe n’est pas bon.',
				codeNeeded:
					'Vos clés d’accès ont activé le second facteur. Saisissez aussi un code de récupération.',
				wrongCode: 'Ce code n’est pas bon. Les codes changent toutes les 30 secondes.',
				loginAgain:
					'Votre compte n’a encore ni mot de passe ni second facteur. Reconnectez-vous, puis configurez l’app dans les dix minutes.',
				tooMany: 'Trop d’essais erronés. Réessayez dans quelques minutes.',
				unreachable: 'Le serveur ne répond pas. Réessayez dans un instant.'
			}
		},
		passkeys: {
			title: 'Clés d’accès',
			intro:
				'Une clé d’accès vous connecte sans nom ni mot de passe, avec le code, l’empreinte ou le visage que demande votre appareil. Elle compte comme second facteur.',
			once: 'Avec une clé d’accès, votre mot de passe seul n’ouvre plus votre compte.',
			none: 'Aucune clé d’accès pour l’instant.',
			made: (when) => `ajoutée ${when}`,
			used: (when) => `dernière utilisation ${when}`,
			unused: 'pas encore utilisée',
			add: 'Ajouter une clé d’accès',
			name: 'Nom, par exemple celui de l’appareil',
			defaultName: 'Clé d’accès',
			password: 'Votre mot de passe',
			code: 'Code de votre app d’authentification, ou un code de récupération',
			create: 'Créer la clé d’accès',
			working: 'Suivez votre navigateur …',
			cancel: 'Annuler',
			remove: 'Supprimer',
			removeName: (name) => `Supprimer la clé d’accès ${name}`,
			confirmRemove: 'Supprimer la clé d’accès',
			full: (max) => `Un compte a au plus ${n(max)} clés d’accès.`,
			or: 'ou',
			confirmWith: 'Confirmer avec une clé d’accès',
			codesTitle: 'Vos codes de récupération',
			codesIntro:
				'Votre première clé d’accès a activé le second facteur. Chaque code vous connecte une fois si vous perdez votre appareil. Gardez-les en lieu sûr : ils ne sont affichés que maintenant.',
			copyCodes: 'Copier les codes',
			copied: 'Copié',
			codesDone: 'Je les ai gardés',
			refusals: {
				cancelled: 'La fenêtre du navigateur a été fermée avant que la clé d’accès ne serve.',
				known: 'Cet appareil a déjà une clé d’accès pour votre compte.',
				failed: 'Ce navigateur ou cet appareil n’a pas pu créer ni utiliser de clé d’accès.',
				wrong: 'La clé d’accès n’a pas été acceptée. Réessayez.',
				wrongPassword: 'Le mot de passe n’est pas bon.',
				codeNeeded:
					'Votre compte a un second facteur. Saisissez aussi un code de votre app ou un code de récupération, ou confirmez avec une clé d’accès.',
				wrongCode: 'Ce code n’est pas bon. Les codes changent toutes les 30 secondes.',
				tooManyPasskeys: 'Un compte a au plus 10 clés d’accès.',
				loginAgain:
					'Votre compte n’a encore ni mot de passe ni second facteur. Reconnectez-vous, puis ajoutez votre première clé d’accès dans les dix minutes.',
				tooMany: 'Trop d’essais erronés. Réessayez dans quelques minutes.',
				unreachable: 'Le serveur ne répond pas. Réessayez dans un instant.'
			}
		},
		reports: {
			send: 'Envoyer à ce serveur les rapports d’erreurs de ce navigateur',
			intro: 'Quand quelque chose tourne mal sur la page, les admins de ce serveur l’apprennent.',
			what: 'Un rapport contient :',
			holds: [
				'le message de l’erreur, sans adresses et sans nombres qui désignent quelque chose',
				'l’endroit des fichiers de rueder où elle s’est produite',
				'la version de rueder et votre navigateur avec sa version majeure'
			],
			never:
				'Jamais ce que vous lisez, ni vos flux, libellés ou règles. Les rapports disparaissent au bout de 30 jours.',
			off: 'Désactiver ceci supprime les rapports que vous avez envoyés.'
		}
	},

	rules: {
		intro:
			'Les règles masquent des articles ou les marquent comme lus à leur arrivée. Les articles masqués ne comptent pas comme non lus et restent sous « Masqués par les filtres ».',
		none: 'Aucune règle pour l’instant.',
		add: 'Ajouter une règle',
		new: 'Nouvelle règle',
		on: 'Active',
		scope: 'Dans',
		scopeAll: 'Tous les flux',
		scopeFolder: (title) => `Dossier : ${title}`,
		scopeFeed: (title) => `Flux : ${title}`,
		field: 'Regarder',
		fields: { any: 'Partout', title: 'Titre', content: 'Texte', author: 'Auteur', tag: 'Mot-clé' },
		match: 'Comparaison',
		matches: {
			contains: 'contient',
			word: 'contient le mot',
			starts: 'commence par',
			ends: 'se termine par',
			regex: 'correspond à l’expression régulière',
			all: 'chaque article, quoi qu’il dise'
		},
		pattern: 'Texte recherché',
		action: 'Alors',
		actions: { hide: 'masquer l’article', read: 'le marquer comme lu' },
		preview: (count, total) =>
			`S’applique maintenant à ${n(count)} des ${n(total)} articles d’ici.`,
		regexTip:
			'Les expressions régulières sont lues comme en Rust, sans tenir compte des majuscules.',
		regexBuilder: 'En construire et en tester une sur regex101',
		invalid: 'Ce n’est pas une expression régulière valide.',
		empty: 'Saisissez le texte recherché.',
		checking: 'Recherche de ce qu’elle attraperait …',
		hits: (count) => `${n(count)} ${plural(count, { one: 'résultat', other: 'résultats' })}`,
		summary: (field, match, pattern, action) => `${field} ${match} « ${pattern} » : ${action}`,
		summaryAll: (action) => `Chaque article : ${action}`
	},

	keys: {
		title: 'Raccourcis clavier',
		note: 'Vos touches sont gardées avec les paramètres. Entrée, Échap et g restent comme elles sont.',
		change: (what) => `Changer la touche pour : ${what}`,
		changeShort: 'Changer',
		press: 'Appuyez sur une touche …',
		taken: (key, other) => `${key} est déjà la touche pour : ${other}`,
		fixed: (key) => `${key} ne peut pas être attribuée à un raccourci.`,
		reset: 'Revenir aux touches de rueder',
		groups: { articles: 'Articles', lists: 'Listes', go: 'Aller à', app: 'Partout' },
		next: 'Article suivant, ouvert',
		previous: 'Article précédent, ouvert',
		nextCursor: 'Article suivant, sans l’ouvrir',
		previousCursor: 'Article précédent, sans l’ouvrir',
		toggle: 'Ouvrir ou fermer l’article',
		star: 'Ajouter aux favoris ou en retirer',
		read: 'Marquer comme lu ou non lu',
		original: 'Ouvrir l’original dans un nouvel onglet',
		markAll: 'Tout marquer comme lu',
		refresh: 'Actualiser la liste',
		unreadOnly: 'Non lus seulement, ou tous les articles',
		listView: 'Vue en liste',
		expandedView: 'Vue développée',
		splitView: 'Trois volets',
		magazineView: 'Magazine',
		goHome: 'Accueil',
		goAll: 'Tous les articles',
		goUnread: 'Articles non lus',
		goStarred: 'Articles favoris',
		search: 'Rechercher',
		sidebar: 'Afficher ou masquer la barre latérale',
		help: 'Cet aperçu',
		then: 'puis'
	},

	labels: {
		title: 'Libellés',
		add: 'Nouveau libellé',
		name: 'Nom du libellé',
		none: 'Aucun libellé pour l’instant.',
		actions: (title) => `Actions pour le libellé ${title}`,
		rename: 'Renommer',
		color: 'Couleur',
		colors: ['Rouge', 'Orange', 'Jaune', 'Vert', 'Turquoise', 'Bleu', 'Violet', 'Gris'],
		remove: 'Supprimer le libellé',
		show: 'Afficher les libellés',
		hide: 'Masquer les libellés',
		forArticle: 'Libellés',
		set: (title) => `Libellé : ${title}`
	},

	subscribe: {
		title: 'S’abonner à un flux',
		address: 'Adresse du flux ou du site web',
		find: 'Trouver des flux',
		finding: 'Recherche …',
		note: 'Le brouillon ne peut rien récupérer, le flux reste donc vide.',
		found: (count) => (count === 1 ? 'Il y a un flux :' : `Il y a ${n(count)} flux :`),
		none: 'Il n’y a pas de flux à cette adresse.',
		movedTo: (url) => `Le site a renvoyé la demande vers ${url}.`,
		loginWall:
			'Certains sites renvoient vers leur page de connexion quiconque n’est pas connecté. Leurs flux ne peuvent alors pas être lus.',
		subscribed: 'Abonné',
		show: 'Afficher',
		untitled: 'Sans nom',
		folder: 'Dans le dossier',
		topLevel: 'Aucun dossier',
		add: 'S’abonner',
		adding: 'Abonnement …',
		refused: 'Cela n’a pas fonctionné.',
		serverSays: (reason) => `Le serveur dit : ${reason}`,
		busy: (seconds) =>
			seconds
				? `Trop de recherches à la fois. Réessayez dans ${n(seconds)} ${plural(seconds, { one: 'seconde', other: 'secondes' })}.`
				: 'Trop de recherches à la fois. Réessayez dans un instant.',
		noFetching: 'Ce serveur ne récupère pas de flux.',
		tooLarge: (megabytes) => `Le fichier dépasse ${n(megabytes)} Mo.`,
		unreachable: 'Le serveur ne répond pas. Réessayez dans un instant.'
	},

	opml: {
		title: 'Importer et exporter',
		intro:
			'Apportez vos abonnements d’un autre lecteur de flux sous forme de fichier OPML, ou emportez-les.',
		import: 'Importer un fichier OPML',
		importing: 'Importation …',
		imported: (feeds, folders) =>
			`${n(feeds)} ${plural(feeds, { one: 'nouveau flux', other: 'nouveaux flux' })} et ${n(folders)} ${plural(folders, { one: 'nouveau dossier', other: 'nouveaux dossiers' })}.`,
		known: (count) =>
			`${n(count)} ${plural(count, { one: 'flux était déjà suivi', other: 'flux étaient déjà suivis' })}.`,
		skipped: (count) =>
			`${n(count)} ${plural(count, { one: 'entrée a été laissée de côté, n’étant pas un flux lisible', other: 'entrées ont été laissées de côté, n’étant pas des flux lisibles' })}.`,
		later: 'Leurs articles arrivent avec la prochaine tournée de récupération.',
		export: 'Exporter en OPML'
	},

	removeAccount: {
		title: 'Supprimer le compte',
		intro: 'Ceci supprime définitivement votre compte sur ce rueder. C’est irréversible.',
		goes: 'Ce qui part avec : vos sessions sur tous les appareils, vos abonnements et dossiers, ce que vous avez lu, les favoris, libellés, règles de filtrage, paramètres, rapports d’erreurs, invitations et le lien avec l’authentification unique.',
		stays:
			'Les flux restent pour les autres qui les lisent. Pour emmener d’abord votre compte ailleurs, utilisez « Déménagement ».',
		password: 'Votre mot de passe',
		name: (name) => `Tapez votre nom, ${name}, pour confirmer`,
		code: 'Code de votre app, ou un code de récupération',
		submit: 'Supprimer définitivement mon compte',
		wrong: 'Ce n’est pas bon. Rien n’a été supprimé.',
		codeNeeded:
			'Votre compte a un second facteur. Tapez aussi un code de votre app ou un code de récupération, ou confirmez avec une clé d’accès.',
		busy: (count) => `Trop d’essais erronés. Réessayez dans ${minutes(count)}.`,
		unreachable: 'Le serveur ne répond pas. Rien n’a été supprimé.'
	},

	move: {
		intro:
			'Emmenez votre compte vers un autre rueder, ou amenez-le ici depuis un autre, en un seul fichier.',
		howItWorks:
			'Une importation ajoute à ce qui est ici et n’enlève rien. Vos paramètres deviennent ceux du fichier.',
		notInIt: 'Pas dans le fichier : votre mot de passe, vos sessions et les textes des articles.',
		export: 'Exporter le compte',
		import: 'Importer un fichier de compte',
		importing: 'Importation …',
		feeds: (feeds, known) =>
			`${n(feeds)} ${plural(feeds, { one: 'nouveau flux', other: 'nouveaux flux' })}, ${n(known)} déjà suivis.`,
		folders: (count) =>
			`${n(count)} ${plural(count, { one: 'nouveau dossier', other: 'nouveaux dossiers' })}.`,
		articles: (found) =>
			`${articles(found)} ${plural(found, { one: 'trouvé', other: 'trouvés' })} ici.`,
		missing: (count) =>
			`${articles(count)} ${plural(count, { one: 'introuvable', other: 'introuvables' })} ici ; ce que vous en aviez lu, mis en favori ou étiqueté est perdu.`,
		marks: (labels, rules) =>
			`${n(labels)} ${plural(labels, { one: 'nouveau libellé', other: 'nouveaux libellés' })} et ${n(rules)} ${plural(rules, { one: 'nouvelle règle de filtrage', other: 'nouvelles règles de filtrage' })}.`,
		skipped: (count) =>
			`${n(count)} ${plural(count, { one: 'entrée du fichier n’a pas pu être reprise', other: 'entrées du fichier n’ont pas pu être reprises' })}.`,
		settings: 'Vos paramètres sont désormais ceux du fichier.',
		later:
			'Les nouveaux flux apportent leurs articles avec la prochaine tournée de récupération. Ce que vous aviez lu reste lu.',
		busy: (seconds) =>
			seconds
				? `Trop d’importations. Réessayez dans ${minutes(Math.ceil(seconds / 60))}.`
				: 'Trop d’importations. Réessayez plus tard.'
	},

	status: {
		title: 'État du serveur',
		open: 'État du serveur',
		loading: 'Interrogation du serveur …',
		failed: 'Le serveur n’a pas répondu. Réessayez dans un instant.',
		refresh: 'Actualiser',
		asOf: (time) => `À ${time}. Ce qui est compté, c’est la dernière heure.`,
		demo: 'Inventé, car il n’y a pas de serveur ici.',
		empty: '–',
		on: 'Activé',
		off: 'Désactivé',
		took: (median, p95) => `médiane ${median}, 95 % en moins de ${p95}`,
		summary: {
			label: 'En un coup d’œil',
			hints: 'Remarques',
			failing: 'Flux en échec',
			errors: 'Erreurs journalisées',
			reported: 'Flux signalés'
		},
		hintsTitle: 'À regarder',
		allWell: 'Rien ne ressort.',
		hints: {
			slowRoute: (route, took) =>
				`${route} est lente : une requête sur vingt prend plus de ${took}.`,
			serverErrors: (route, count) =>
				`${route} a répondu par une erreur du serveur ${n(count)} fois.`,
			poolBusy: (size) =>
				`Les ${n(size)} connexions à la base de données sont toutes occupées, les requêtes en attendent donc une.`,
			fetchBacklog: (count, late) =>
				`${n(count)} flux ${plural(count, { one: 'est récupéré', other: 'sont récupérés' })} en retard, le plus en retard de ${late}.`,
			feedsFailing: (failing, subscribed) => `${n(failing)} des ${n(subscribed)} flux échouent.`,
			notFetching: 'Le serveur ne récupère pas de flux. FETCH est-il sur off ?',
			fetchPaceFull: (most) =>
				`Chaque tournée prend autant de flux qu’elle le peut, et des flux attendent encore : le rythme permet ${n(most)} récupérations par heure. FETCH_ROUND, FETCH_AT_ONCE et FETCH_EVERY le fixent.`,
			cacheMisses: (share) =>
				`La base de données ne trouve en mémoire que ${share} de ce qu’elle lit ; davantage de shared_buffers aiderait.`,
			databaseLarge: (size, warn) =>
				`La base de données a atteint ${size} ; DATABASE_WARN_GB avertit à partir de ${warn}. « rueder retention set » supprime les anciens articles ou leurs textes.`,
			longQuery: (took) => `Une requête tourne depuis ${took}.`,
			imageCacheFull: (share) => `Le cache d’images est plein à ${share}.`,
			errors: (count) =>
				`Le serveur a journalisé ${n(count)} ${plural(count, { one: 'erreur', other: 'erreurs' })} dans la dernière heure.`,
			limited: (count) =>
				`${n(count)} ${plural(count, { one: 'requête a été refusée', other: 'requêtes ont été refusées' })} comme trop nombreuses.`,
			browserErrors: (count) =>
				`Les navigateurs ont signalé ${n(count)} ${plural(count, { one: 'erreur', other: 'erreurs différentes' })} le dernier jour.`,
			reportedFeeds: (count) =>
				`Des lecteurs ont signalé ${n(count)} flux qui ${plural(count, { one: 'n’est pas bloqué', other: 'ne sont pas bloqués' })}.`
		},
		build: {
			title: 'Build',
			commit: 'Commit',
			dirty: (commit) => `${commit}, avec des modifications propres`,
			branch: 'Branche',
			tag: 'Tag',
			committed: 'Date du commit',
			where: 'Tourne',
			container: 'dans un conteneur',
			host: 'sur la machine elle-même',
			target: 'Pour',
			profile: 'Compilé',
			profileValue: (profile, level) => `${profile}, optimisation ${level}`,
			ci: 'Construit par',
			ciValue: 'une exécution de la CI',
			byHand: 'à la main'
		},
		server: {
			title: 'Serveur',
			version: 'Version',
			running: 'En marche depuis',
			memory: 'Mémoire',
			accounts: 'Comptes',
			feeds: 'Flux',
			feedsValue: (fetched, all) => `${n(fetched)} récupérés, ${n(all)} en tout`,
			articles: 'Articles',
			articlesValue: (count) => `environ ${n(count)}`,
			live: 'Onglets ouverts'
		},
		database: {
			title: 'Base de données',
			pool: 'Connexions de rueder',
			poolValue: (size, idle, max) =>
				`${n(size)} ouvertes, ${n(idle)} inactives, au plus ${n(max)}`,
			connections: 'Toutes les connexions',
			size: 'Taille',
			sizeWarn: (size, warn) => `${size}, avertissement à partir de ${warn}`,
			hitRatio: 'Lu depuis la mémoire',
			deadlocks: 'Interblocages',
			longest: 'Requête la plus longue en cours',
			tables: 'Plus grandes tables',
			table: 'Table',
			bytes: 'Taille',
			rows: 'Lignes, environ'
		},
		fetcher: {
			title: 'Récupération des flux',
			state: 'Récupération',
			lastRound: 'Dernière tournée',
			lastRoundValue: (ago, feeds, took) => `${ago} : ${n(feeds)} flux en ${took}`,
			fetches: 'Récupérations',
			fetchesValue: (fetched, unchanged, failed) =>
				`${n(fetched)} ont apporté le flux, ${n(unchanged)} inchangés, ${n(failed)} en échec`,
			broken: 'Échec à la base de données',
			newArticles: 'Nouveaux articles',
			took: 'Durée par récupération',
			overdue: 'Récupérés en retard',
			overdueValue: (count, late) => `${n(count)}, le plus en retard de ${late}`,
			waiting: 'Pas encore récupérés',
			pace: 'Rythme',
			paceValue: (round, every, atOnce, most) =>
				`${n(round)} flux toutes les ${every}, ${n(atOnce)} à la fois : au plus ${n(most)} par heure`,
			perHour: 'Récupérations dans la dernière heure',
			behind: 'Retard',
			behindValue: (late) => `le flux qui attend le plus a ${late} de retard`,
			notBehind: 'Rien n’attend',
			failing: 'Flux en échec, par cause',
			noneFailing: 'Aucun flux n’échoue.',
			feeds: (count) => `${n(count)} flux`,
			failures: {
				status: (code) => (code ? `Le site répond ${code}` : 'Le site répond par une erreur'),
				timeout: 'Le site met trop de temps',
				blocked: 'L’adresse est dans un réseau d’où rueder ne récupère rien',
				dns: 'Le nom n’a pas d’adresse',
				network: 'Pas de connexion, ou elle a été coupée',
				too_large: 'Trop volumineux',
				redirects: 'Trop de redirections, ou une redirection cassée',
				unreadable: 'Pas un flux que rueder sait lire',
				other: 'Autre chose'
			}
		},
		finder: {
			title: 'Recherche de flux',
			state: 'Examen des adresses',
			lookups: 'Recherches',
			lookupsValue: (found, none, failed) =>
				`${n(found)} ont trouvé des flux, ${n(none)} aucun, ${n(failed)} en échec`,
			limited: 'Au-delà de la limite du compte',
			took: 'Durée par recherche'
		},
		images: {
			title: 'Proxy d’images',
			state: 'Proxy',
			cache: 'Cache',
			cacheValue: (used, max) => `${used} sur ${max}`,
			served: 'Images',
			servedValue: (cached, fetched) => `${n(cached)} du cache, ${n(fetched)} récupérées`,
			refused: 'Refusées',
			refusedValue: (source, notImage, tooLarge, notFound) =>
				`${n(source)} échec de la source, ${n(notImage)} pas une image, ${n(tooLarge)} trop volumineuses, ${n(notFound)} mauvaise adresse`,
			took: 'Durée par image récupérée',
			failingSources: 'Sources laissées de côté pour l’instant'
		},
		requests: {
			title: 'Requêtes',
			all: 'Toutes les requêtes',
			allValue: (count, took) => `${n(count)}, ${took}`,
			answers: 'Réponses',
			answersValue: (fine, refused, errors) =>
				`${n(fine)} correctes, ${n(refused)} refusées, ${n(errors)} erreurs du serveur`,
			route: 'Route',
			count: 'Nombre',
			median: 'Médiane',
			p95: '95 %',
			max: 'La plus longue',
			errors: 'Erreurs',
			none: 'Aucune requête dans la dernière heure.'
		},
		problems: {
			title: 'Avertissements et erreurs',
			counts: (errors, warnings) =>
				`Dans la dernière heure, ${n(errors)} ${plural(errors, { one: 'erreur', other: 'erreurs' })} et ${n(warnings)} ${plural(warnings, { one: 'avertissement', other: 'avertissements' })}.`,
			none: 'Rien n’a été journalisé depuis le démarrage.',
			error: 'Erreur',
			warning: 'Avertissement',
			times: (count, last) => (count > 1 ? `${n(count)} fois, la dernière ${last}` : last)
		},
		browsers: {
			title: 'Erreurs dans les navigateurs',
			off: 'Ce serveur n’accepte pas de rapports d’erreurs (CLIENT_REPORTS=off).',
			consenting: (count) =>
				`${n(count)} ${plural(count, { one: 'compte envoie', other: 'comptes envoient' })} des rapports. Les erreurs restent 30 jours après leur dernière occurrence.`,
			none: 'Aucun navigateur n’a signalé d’erreur.',
			kinds: {
				error: 'Erreur',
				rejection: 'Promesse rejetée',
				component: 'Partie de la page',
				request: 'Requête'
			},
			hits: (count, accounts, last) =>
				`${n(count)} fois dans ${n(accounts)} ${plural(accounts, { one: 'compte', other: 'comptes' })}, la dernière ${last}`,
			where: (versions, browsers) => `Version ${versions} · ${browsers}`
		},
		reported: {
			title: 'Flux signalés',
			text: 'Qui a signalé n’est pas affiché ici. « rueder feed block <adresse> » bloque un flux pour tout le monde, « rueder feed unblock <adresse> » le débloque.',
			none: 'Personne n’a signalé de flux.',
			reports: (count, last) =>
				`${n(count)} ${plural(count, { one: 'signalement', other: 'signalements' })}, le dernier ${last}`,
			blocked: 'Bloqué'
		}
	},

	update: {
		ready: 'Une nouvelle version de rueder est prête.',
		reload: 'Recharger',
		later: 'Plus tard'
	},

	crash: {
		title: 'Un problème est survenu sur cette page.',
		text: 'Rechargez la page pour continuer. Ce que vous avez lu et modifié est enregistré sur le serveur.',
		reload: 'Recharger'
	},

	demo: {
		badge: 'Brouillon',
		note: 'Un brouillon avec des flux inventés. Rien n’est enregistré sur un serveur.',
		arrive: 'Faire arriver un article',
		revise: 'Modifier l’article ouvert',
		revised: 'Mise à jour : ce paragraphe a été ajouté pour essayer les versions.',
		reviseHint: 'Ouvrez d’abord un article.'
	}
};

export default fr;
