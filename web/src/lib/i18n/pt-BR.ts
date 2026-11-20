// The Brazilian Portuguese texts, in the shape of the English catalog (en.ts),
// addressing the reader with “você”. Browsers in any Portuguese get them. “Feed”
// stays as it is; an item is an “artigo”.

import type { Messages } from './en.ts';
import { formatNumber as n, plural } from './intl.ts';

const articles = (count: number) =>
	`${n(count)} ${plural(count, { one: 'artigo', other: 'artigos' })}`;

const minutes = (count: number) =>
	`${n(count)} ${plural(count, { one: 'minuto', other: 'minutos' })}`;

const ptBR: Messages = {
	languageName: 'Português (Brasil)',

	articles,

	common: {
		close: 'Fechar',
		cancel: 'Cancelar',
		save: 'Salvar',
		delete: 'Excluir',
		edit: 'Editar'
	},

	nav: {
		subscribe: 'Assinar',
		home: 'Início',
		all: 'Todos os artigos',
		unreadItems: 'Artigos não lidos',
		starred: 'Artigos com estrela',
		hidden: 'Ocultos por filtros',
		subscriptions: 'Assinaturas',
		newFolder: 'Nova pasta',
		groupedFolder: 'Nova pasta',
		folderName: 'Nome da pasta',
		renameFolder: 'Renomear',
		removeFolder: 'Remover a pasta e manter o que há nela',
		folderActions: (title) => `Ações da pasta ${title}`,
		feedName: 'Nome do feed',
		feedNameHint: 'Vazio para usar o nome do próprio feed',
		renameFeed: 'Renomear',
		unsubscribe: 'Cancelar assinatura',
		reportFeed: 'Denunciar o feed …',
		feedInfo: 'Informações do feed …',
		feedActions: (title) => `Ações do feed ${title}`,
		unsubscribeTitle: 'Cancelar assinatura?',
		unsubscribeText: (title) =>
			`Cancelar a assinatura de “${title}”? O que você leu e marcou com estrela lá será perdido.`,
		openFolder: 'Abrir a pasta',
		closeFolder: 'Fechar a pasta',
		openSidebar: 'Mostrar a barra lateral',
		closeSidebar: 'Ocultar a barra lateral',
		search: 'Pesquisar',
		searchPlaceholder: 'Pesquisar nestes artigos',
		clearSearch: 'Limpar a pesquisa',
		settings: 'Configurações',
		accountSettings: 'Configurações da conta',
		shortcuts: 'Atalhos de teclado',
		live: 'Ao vivo',
		liveOn: 'Artigos novos chegam sozinhos. Clique para pausar.',
		liveOff: 'Pausado. Clique para que artigos novos voltem a chegar.',
		paused: 'Pausado',
		account: (name) => `Conectado como ${name}`,
		logout: 'Sair',
		appSettings: 'Servidor e atualizações do app',
		install: 'Instalar como app',
		installSafari:
			'Para instalar o rueder como app: Arquivo › Adicionar ao Dock em um Mac, Compartilhar › Adicionar à Tela de Início em um iPhone ou iPad.',
		unread: (count) => `${n(count)} não ${plural(count, { one: 'lido', other: 'lidos' })}`,
		hiddenCount: (count) =>
			`${n(count)} ${plural(count, { one: 'oculto', other: 'ocultos' })} por filtros`,
		fetchFailed: (reason) => `A última busca falhou: ${reason}`,
		waiting: 'Ainda não buscado',
		blocked: 'Bloqueado por quem administra este rueder',
		moveHint: 'Arraste para ordenar, ou pressione Alt com as setas',
		resizeSidebar: 'Largura da barra lateral: arraste ou use as setas',
		tooDeep: (levels) => `As pastas podem ter no máximo ${n(levels)} níveis.`
	},

	toolbar: {
		refresh: 'Atualizar',
		showAll: 'Todos os artigos',
		showUnread: 'Só não lidos',
		show: 'Mostrar',
		newestFirst: 'Mais recentes primeiro',
		oldestFirst: 'Mais antigos primeiro',
		markAllRead: 'Marcar tudo como lido',
		markOptions: 'Mais formas de marcar como lido',
		phoneMenu: (shown) => `Visualização, mostrando: ${shown}`,
		markMenu: 'Marcar como lido …',
		markEverything: 'Todos os artigos',
		markOlderDay: 'Artigos com mais de um dia',
		markOlderWeek: 'Artigos com mais de uma semana',
		listView: 'Visualização em lista',
		expandedView: 'Visualização expandida',
		splitView: 'Três painéis: árvore, lista e artigo',
		magazineView: 'Revista',
		previous: 'Artigo anterior',
		next: 'Próximo artigo',
		newArticles: (count) =>
			`${n(count)} ${plural(count, { one: 'artigo novo', other: 'artigos novos' })}`
	},

	list: {
		empty: 'Não há artigos aqui.',
		emptyUnread: 'Não há nada não lido aqui.',
		emptySearch: 'Nenhum artigo corresponde à pesquisa.',
		found: (count, more, words) =>
			`${n(count)}${more ? '+' : ''} ${plural(count, { one: 'artigo', other: 'artigos' })} com “${words}”`,
		searchAll: 'Pesquisar em todos os feeds',
		feedFailing:
			'O rueder tenta de novo mais tarde e espera mais a cada falha, ou o tempo que o site pedir. Se continuar falhando, talvez o feed tenha mudado de endereço.',
		hiddenNote: 'Suas regras de filtro ocultaram estes artigos. Abra um para ver qual regra foi.',
		toRules: 'Regras de filtro',
		emptyFeed: 'Este feed ainda não tem artigos.',
		waitingFeed:
			'Este feed ainda não foi buscado. Os artigos chegam na próxima rodada de buscas, em poucos minutos.',
		showAll: 'Mostrar todos os artigos',
		more: 'Mostrar mais',
		star: 'Adicionar estrela',
		unstar: 'Remover estrela',
		keepUnread: 'Manter como não lido',
		openOriginal: 'Abrir o original',
		showFeed: 'Mostrar todos os artigos deste feed no rueder',
		from: 'de',
		by: (author) => `por ${author}`,
		updated: 'atualizado',
		updatedHeadline: 'nova manchete',
		updatedTitle: (count) =>
			`Mudou desde a primeira busca; ${n(count)} ${plural(count, { one: 'versão anterior', other: 'versões anteriores' })}`,
		updatedHeadlineTitle: (count) =>
			`A manchete foi reescrita, o texto continua igual; ${n(count)} ${plural(count, { one: 'versão anterior', other: 'versões anteriores' })}`,
		compare: 'Comparar versões',
		hiddenBy: (rule) => `Oculto pela regra: ${rule}`,
		textDropped: 'Este servidor não guarda mais o texto deste artigo, porque ele é antigo.',
		textDroppedOriginal: 'Leia na fonte',
		loading: 'Carregando o texto …',
		loadingList: 'Carregando …',
		toTop: 'Voltar ao topo',
		pick: 'Escolha um artigo para lê-lo aqui.',
		resizeList: 'Largura da lista: arraste ou use as setas',
		closeArticle: 'Fechar o artigo',
		blockedFeed:
			'Quem administra este rueder bloqueou este feed. Ele não é buscado e seus artigos não são mostrados.',
		articleActions: 'Mais ações para o artigo',
		articleInfo: 'Informações do artigo …',
		reportArticle: 'Denunciar o artigo …'
	},

	report: {
		feedTitle: 'Denunciar o feed',
		articleTitle: 'Denunciar o artigo',
		text: (title) =>
			`Quem administra este rueder vê que “${title}” foi denunciado, o motivo e o que você escrever, mas não quem denunciou. Essa pessoa decide se bloqueia para todos.`,
		reason: 'Motivo',
		reasons: {
			illegal: 'Contra a lei',
			copyright: 'Obra de outra pessoa',
			malicious: 'Malware, phishing ou fraude',
			spam: 'Spam',
			other: 'Outra coisa'
		},
		note: 'O que você gostaria de acrescentar (opcional)',
		left: (count) =>
			`${plural(count, { one: 'Resta', other: 'Restam' })} ${n(count)} ${plural(count, { one: 'caractere', other: 'caracteres' })}`,
		send: 'Enviar a denúncia',
		sending: 'Enviando …',
		sent: 'Obrigado. A denúncia chegou a quem administra este rueder.',
		tooMany: 'Você enviou muitas denúncias ultimamente. Tente de novo mais tarde.',
		unreachable: 'A denúncia não chegou ao servidor. Tente de novo mais tarde.'
	},

	feedInfo: {
		title: 'Informações do feed',
		text: (title) =>
			`O que este rueder sabe sobre “${title}” e como o busca. Se o feed não funcionar, copie isto na sua mensagem para quem administra.`,
		number: 'Número',
		name: 'Nome',
		address: 'Endereço do feed',
		site: 'Site',
		noSite: 'Nenhum',
		agent: 'Buscado como',
		agents: { rueder: 'rueder', browser: 'Navegador' },
		every: 'Buscado a cada',
		fetched: 'Última busca',
		succeeded: 'Última busca que funcionou',
		failed: 'Último erro',
		next: 'Próxima busca',
		blocked: 'Nenhuma, quem administra bloqueou o feed',
		articles: 'Artigos até agora',
		never: 'Nunca',
		fetchAs: 'Buscar como',
		fetchAsHint:
			'Para todos que leem o feed. Como navegador, para sites que recusam o rueder. Ele é buscado de novo na hora.',
		notSwitched: 'Não funcionou. Tente de novo mais tarde.',
		copy: 'Copiar',
		copied: 'Copiado',
		loading: 'Carregando …',
		unreachable: 'O servidor não respondeu. Tente de novo mais tarde.'
	},
	articleInfo: {
		title: 'Informações do artigo',
		text: (title) =>
			`O que este rueder sabe sobre “${title}” e suas versões. Se algo parecer errado, copie isto na sua mensagem para quem administra.`,
		feed: 'Feed',
		feedNumber: 'Número do feed',
		number: 'Número no feed',
		name: 'Título',
		id: 'Identificador no feed',
		link: 'Link',
		noLink: 'Nenhum',
		published: 'Publicado',
		fetched: 'Primeira busca',
		changed: 'Última alteração',
		never: 'Nunca',
		changedAt: (when, what) => `${when} (${what})`,
		what: { title: 'o título', text: 'o texto', both: 'o título e o texto' },
		version: 'Versão',
		kept: 'Versões guardadas',
		textLabel: 'Texto',
		textDropped: 'Não é mais guardado, por ser antigo',
		copy: 'Copiar',
		copied: 'Copiado',
		loading: 'Carregando …',
		unreachable: 'O servidor não respondeu. Tente de novo mais tarde.'
	},

	media: {
		title: 'Anexos',
		play: 'Reproduzir'
	},

	connection: {
		reconnecting: 'Reconectando …',
		unreachable: (since) => `O servidor não responde desde ${since}.`,
		offline: (since) => `Sem rede desde ${since}.`,
		retry: 'Tentar agora',
		unsent: (count) =>
			plural(count, {
				one: '1 alteração está aguardando.',
				other: `${n(count)} alterações estão aguardando.`
			})
	},

	notice: {
		unreachable: 'O servidor não responde. O que você vê pode estar desatualizado.',
		notSaved: 'O servidor não aceitou sua última alteração. Ela foi desfeita aqui.',
		tooManyUnsent: 'Há tantas alterações aguardando o servidor que esta foi desfeita.'
	},

	login: {
		title: 'Entrar no rueder',
		about:
			'Um leitor de feeds: ele busca o que os sites que você segue publicam e mostra o que você ainda não leu.',
		name: 'Nome',
		password: 'Senha',
		submit: 'Entrar',
		checking: 'Um momento …',
		wrong: 'Nome ou senha incorretos.',
		codeNeeded:
			'Esta conta tem um segundo fator. Digite o código que seu app mostra ou um código de recuperação, ou entre com uma chave de acesso.',
		wrongCode: 'Esse código não está certo. Os códigos mudam a cada 30 segundos.',
		code: 'Código',
		codeNote: 'Os seis dígitos do seu app autenticador, ou um dos seus códigos de recuperação.',
		tooMany: (count) => `Muitas tentativas erradas. Tente de novo em ${minutes(count)}.`,
		unreachable: 'O servidor não responde. Tente de novo em instantes.',
		retry: 'Tentar de novo',
		or: 'ou',
		oidc: 'Entrar com login único',
		passkey: 'Entrar com uma chave de acesso',
		passkeyCancelled:
			'Nenhuma chave de acesso foi usada. Tente de novo, ou entre com nome e senha.',
		passkeyFailed: 'Este navegador ou dispositivo não pode usar uma chave de acesso aqui.',
		passkeyWrong: 'Esta chave de acesso não pertence a nenhuma conta daqui.',
		oidcFailed: 'O login único não funcionou. Tente de novo.',
		oidcDenied: 'O login único foi cancelado.',
		oidcUnknown:
			'Nenhuma conta daqui pertence a esse login. Peça a quem administra este rueder para vinculá-lo.',
		disabled: 'Esta conta está desativada. Pergunte o motivo a quem administra este rueder.',
		unconfirmed:
			'Esta conta aguarda a confirmação do seu e-mail. Abra o link do e-mail, ou peça para enviá-lo de novo.',
		resend: 'Enviar o link de novo',
		resent: 'O link está a caminho de novo. Veja seu e-mail, e também a pasta de spam.',
		resendWrong: 'Digite o nome e a senha da conta para enviar o link de novo.',
		forgot: 'Esqueceu sua senha?',
		removed: 'Sua conta foi excluída, com tudo o que era seu.',
		note: 'Não há cadastro aberto. As contas são criadas por quem administra este rueder.'
	},

	legal: {
		imprint: 'Informações legais',
		privacy: 'Política de privacidade',
		terms: 'Termos de uso',
		agree: { before: 'Aceito os ', link: 'termos de uso', after: '.' },
		changed: 'Os termos de uso mudaram',
		changedIntro: 'Leia os novos termos de uso. Para continuar usando o rueder, aceite-os.',
		read: 'Ler os termos de uso',
		accept: 'Aceito',
		decline: 'Sair',
		notSaved: 'Não funcionou. Tente de novo em instantes.'
	},

	register: {
		title: 'Criar uma conta',
		invited: 'Você foi convidado para este rueder. Escolha um nome e uma senha.',
		again: 'A senha mais uma vez',
		passwordRule: (characters) => `Pelo menos ${n(characters)} caracteres.`,
		mismatch: 'As duas senhas são diferentes.',
		submit: 'Criar a conta',
		solving: 'Verificando este navegador …',
		toRegister: 'Criar uma conta',
		toLogin: 'Entrar em uma conta que você já tem',
		closed: 'Ninguém pode se cadastrar aqui sem convite.',
		inviteGone: 'Este convite não pode mais ser usado. Peça um novo.',
		full: 'Não há espaço para outra conta aqui.',
		today: 'Já foram criadas contas suficientes aqui hoje. Tente de novo amanhã.',
		captcha: 'A verificação deste navegador não foi concluída. Tente de novo.',
		taken: 'Já existe uma conta com este nome.',
		terms: 'Aceite os termos de uso para criar uma conta.',
		invalid: (reason) => `Isso não pode ser uma conta. O servidor diz: ${reason}`,
		email: 'E-mail',
		emailOptional: 'E-mail (opcional)',
		emailNote: 'Você recebe um link para confirmá-lo.',
		emailRequiredNote:
			'Você recebe um link para confirmá-lo; a conta pode entrar depois que você o abrir.',
		emailInvalid: 'Digite um endereço de e-mail que possa receber mensagens.',
		unsent:
			'Não foi possível enviar o link, então nenhuma conta foi criada. Tente de novo mais tarde.',
		tooMany: (count) => `Muitas tentativas. Tente de novo em ${minutes(count)}.`,
		awaitingTitle: 'Veja seu e-mail',
		awaiting: (address, hours) =>
			`A conta foi criada. Um link foi enviado para ${address}: abra-o em até ${n(hours)} horas e depois entre.`
	},

	forgot: {
		title: 'Esqueceu sua senha?',
		intro:
			'Informe seu nome ou e-mail. Um link para criar uma nova senha vai para o endereço confirmado.',
		field: 'Nome ou e-mail',
		submit: 'Enviar o link',
		asked: (count) =>
			`Se existir essa conta com um endereço confirmado, um link está a caminho dele, válido por ${minutes(count)}. Veja também a pasta de spam.`,
		tooMany: (count) => `Pedido vezes demais. Tente de novo em ${minutes(count)}.`,
		back: 'Voltar para o login'
	},

	reset: {
		title: 'Criar uma nova senha',
		intro: 'Todos os navegadores e dispositivos da conta serão desconectados.',
		fresh: 'Nova senha',
		again: 'Nova senha mais uma vez',
		submit: 'Salvar a senha',
		mismatch: 'As duas não coincidem.',
		gone: 'Este link não funciona mais. Peça um novo.',
		invalid: (reason) => `O servidor não aceitou essa senha. Ele diz: ${reason}`,
		done: 'A senha foi salva. Entre com ela.',
		cancel: 'Cancelar'
	},

	email: {
		title: 'E-mail',
		intro: 'O rueder só escreve para você sobre a sua conta.',
		current: 'Seu endereço',
		none: 'A conta não tem endereço.',
		pending: (address, hours) =>
			`Um link foi enviado para ${address}. O endereço passa a valer quando ele for aberto, em até ${n(hours)} horas; até lá, fica o anterior.`,
		address: 'Novo endereço',
		password: 'Sua senha',
		passwordNote: 'Para que ninguém mais que use este navegador desvie seus e-mails.',
		code: 'Código do segundo fator',
		submit: 'Enviar o link',
		sent: 'O link está a caminho. Veja seu e-mail, e também a pasta de spam.',
		remove: 'Remover o endereço',
		removeIntro: 'Sem endereço, nenhum e-mail é enviado a você. Fora isso, a conta continua igual.',
		removed: 'O endereço foi removido.',
		wrong: 'A senha ou o código não está certo.',
		codeNeeded: 'Esta conta tem um segundo fator. Digite também o código dele.',
		loginAgain:
			'Sua conta ainda não tem senha nem segundo fator. Entre de novo e informe o endereço em até dez minutos.',
		invalid: 'Digite um endereço de e-mail que possa receber mensagens.',
		same: 'Esse já é o endereço da conta.',
		required:
			'Toda conta deste rueder precisa de um endereço; ele pode ser trocado, mas não removido.',
		busy: (count) => `Muitas tentativas ou e-mails. Tente de novo em ${minutes(count)}.`,
		unsent: 'Não foi possível enviar o e-mail. Tente de novo mais tarde.',
		unreachable: 'O servidor não responde. Nada foi alterado.',
		askTitle: 'Adicione um e-mail',
		askIntro:
			'Agora toda conta deste rueder precisa de um e-mail. Informe o seu e abra o link enviado para ele.',
		askWaiting:
			'Quando você abrir o link, isto some sozinho. Não chegou nada? Envie de novo, ou para outro endereço.',
		logOut: 'Sair',
		confirmed: (address) => `O endereço ${address} está confirmado.`,
		confirmedLogIn: 'A conta está pronta: entre.',
		gone: 'Este link não pode mais ser usado: já foi usado, um mais novo foi enviado ou ele tem mais de um dia.',
		taken: 'Outra conta ficou com este endereço nesse meio-tempo.',
		confirmUnreachable: 'O servidor não responde. Abra o link de novo em instantes.',
		close: 'Fechar'
	},

	invites: {
		intro:
			'Um link permite que alguém crie uma conta neste rueder. Ele é mostrado uma única vez, aqui, quando você o cria.',
		left: (count) =>
			count === 0
				? 'Você não tem convites para dar.'
				: `Você pode convidar mais ${n(count)} ${plural(count, { one: 'pessoa', other: 'pessoas' })}.`,
		admin: 'Como admin, você sempre pode convidar.',
		uses: 'Para quantas contas',
		make: 'Criar um link',
		noneLeft: 'Você não tem convites suficientes para tantas.',
		failed: 'Não funcionou. Tente de novo em instantes.',
		link: 'O link, para repassar agora',
		copy: 'Copiar',
		copied: 'Copiado',
		goodFor: (days) =>
			`Um link vale por ${n(days)} ${plural(days, { one: 'dia', other: 'dias' })}.`,
		yours: 'Seus links',
		none: 'Você ainda não criou nenhum link.',
		used: (uses, of) => `${n(uses)} de ${n(of)} usados`,
		until: (when) => `válido até ${when}`,
		ran: (when) => `expirou ${when}`,
		usedUp: 'esgotado',
		revoked: 'retirado',
		revoke: 'Retirar',
		revokeLink: (made) => `Retirar o link criado ${made}`
	},

	home: {
		newIn: 'Novidades nas suas assinaturas',
		allRead: 'Você leu tudo. Artigos novos aparecem aqui assim que chegam.',
		recentlyStarred: 'Com estrela recentemente',
		noStars: 'Nada com estrela ainda. Pressione s em um artigo.',
		problems: 'Feeds com problemas',
		tips: 'Dicas',
		tipKeys: 'Pressione ? para ver todos os atalhos de teclado; j e k percorrem os artigos.',
		tipDrag: 'Arraste feeds e pastas na barra lateral para ordená-los.',
		tipFilters: 'As regras de filtro nas configurações ocultam artigos por palavra-chave ou autor.',
		showFeed: (title) => `Mostrar ${title}`
	},

	versions: {
		title: 'Versões deste artigo',
		current: 'Versão atual',
		version: (rev) => `Versão ${n(rev)}`,
		fetched: (when) => `buscada ${when}`,
		compareWith: 'Comparar a versão atual com',
		legendAdded: 'adicionado',
		legendRemoved: 'removido',
		titleWas: 'O título era:',
		same: 'O texto desta versão é igual ao atual.',
		sameWords: 'As palavras são as mesmas; mudaram imagens ou links.',
		markup: 'As palavras, as imagens e os links são os mesmos; só mudou a organização do texto.',
		pictures: 'Imagens que mudaram',
		noPicture: 'Nenhuma',
		links: 'Links que mudaram'
	},

	account: {
		title: 'Configurações da conta',
		tabs: {
			password: 'Senha',
			email: 'E-mail',
			security: 'Segurança',
			apps: 'Apps',
			invites: 'Convites',
			move: 'Mudança',
			privacy: 'Privacidade',
			remove: 'Excluir a conta'
		},
		password: {
			title: 'Alterar a senha',
			titleFirst: 'Criar uma senha',
			intro:
				'Uma nova senha desconecta todos os outros navegadores e dispositivos e revoga todas as senhas de app.',
			introFirst:
				'Com uma senha, esta conta também pode entrar com o nome, não só pelo login único.',
			current: 'Senha atual',
			fresh: 'Nova senha',
			again: 'Nova senha mais uma vez',
			submit: 'Alterar a senha',
			submitFirst: 'Salvar a senha',
			done: 'A senha foi alterada. Todos os outros navegadores e dispositivos foram desconectados, e as senhas de app foram revogadas.',
			doneFirst: 'A senha foi salva.',
			mismatch: 'As duas não coincidem.',
			code: 'Código do segundo fator',
			wrong: 'A senha atual ou o código não está certo.',
			codeNeeded:
				'Esta conta tem um segundo fator. Digite também um código do seu app autenticador ou um código de recuperação, ou confirme com uma chave de acesso.',
			loginAgain:
				'Sua conta ainda não tem senha nem segundo fator. Entre de novo e crie a senha em até dez minutos.',
			invalid: (characters) =>
				`O servidor não aceitou essa senha. Pelo menos ${n(characters)} caracteres.`,
			busy: (count) => `Muitas tentativas erradas. Tente de novo em ${minutes(count)}.`,
			unreachable: 'O servidor não responde. A senha não foi alterada.'
		},
		settingsNote: 'Aparência, leitura, regras de filtro e teclado ficam em Configurações.'
	},

	settings: {
		title: 'Configurações',
		tabs: {
			look: 'Aparência',
			reading: 'Leitura',
			rules: 'Regras de filtro',
			keys: 'Teclado'
		},
		accountNote:
			'Senha, segundo fator, apps e convites ficam em Configurações da conta, no menu da sua conta.',
		theme: 'Tema',
		themes: { system: 'Como o sistema', light: 'Claro', dark: 'Escuro' },
		accent: 'Cor de destaque',
		accents: {
			red: 'Vermelho',
			blue: 'Azul',
			green: 'Verde',
			orange: 'Laranja',
			purple: 'Roxo',
			graphite: 'Grafite'
		},
		density: 'Densidade da lista',
		densities: { compact: 'Compacta', comfortable: 'Confortável' },
		sidebarSide: 'Barra lateral',
		sidebarSides: { start: 'À esquerda', end: 'À direita' },
		textSize: 'Tamanho do texto dos artigos, neste dispositivo',
		textSizes: { small: 'Pequeno', medium: 'Médio', large: 'Grande' },
		readingFont: 'Fonte dos artigos',
		readingFonts: { sans: 'Sem serifa', serif: 'Com serifa' },
		counters: 'Contadores de não lidos',
		counterStyles: { parens: 'Entre parênteses', badge: 'Como selos' },
		language: 'Idioma',
		languageAuto: 'Como o navegador',
		view: 'Visualização',
		views: { list: 'Lista', expanded: 'Expandida', split: 'Três painéis', magazine: 'Revista' },
		markOnScroll: 'Marcar artigos como lidos ao rolar por eles na visualização expandida',
		teasers: 'Mostrar as primeiras palavras do texto depois do título na lista',
		wakeLock: 'Manter a tela ligada enquanto o rueder estiver aberto',
		wakeLockMissing: 'Este navegador não consegue manter a tela ligada.',
		note: 'As alterações valem na hora e são salvas com a sua conta, então acompanham você em todos os dispositivos.',
		noteDemo: 'As alterações valem na hora e ficam neste navegador.',
		apps: {
			intro:
				'Outros apps de leitura de feeds podem usar sua conta pela API do Google Reader. Cada um recebe sua própria senha de app, que você pode revogar a qualquer momento.',
			none: 'Nenhum app configurado ainda.',
			made: (when) => `configurado ${when}`,
			used: (when) => `último uso ${when}`,
			unused: 'ainda não usado',
			revoke: 'Revogar',
			revokeName: (name) => `Revogar a senha de app ${name}`,
			add: 'Configurar um app',
			full: (max) =>
				`Uma conta tem no máximo ${n(max)} senhas de app. Revogue uma para configurar outro app.`,
			choose: 'Qual app você quer usar?',
			website: (app) => `Site do ${app}`,
			other: 'Outro app',
			otherHint: 'Qualquer app que ofereça a API do Google Reader ou o FreshRSS.',
			platforms: {
				apple: 'Mac, iPhone e iPad',
				android: 'Android'
			},
			name: 'Nome da senha de app',
			password: 'Sua senha',
			code: 'Código do seu app autenticador, ou um código de recuperação',
			create: 'Criar a senha de app',
			creating: 'Um momento …',
			back: 'Voltar',
			how: {
				netnewswire:
					'No NetNewsWire, abra Ajustes e depois Contas, adicione uma conta (no Mac com +) e escolha FreshRSS. Digite o nome abaixo como Login (no iPhone e no iPad: Username or Email), a senha de app como Password e o endereço do servidor como API URL.',
				readyou:
					'No Read You, abra Configurações, Contas, Adicionar contas, e escolha Google Reader. Digite o endereço do servidor abaixo como URL do servidor, o nome como nome de usuário e a senha de app como senha.',
				other:
					'No seu app, adicione uma conta do tipo “Google Reader API” ou, se não houver, “FreshRSS”. Digite o endereço do servidor, o nome e a senha de app abaixo.'
			},
			ownIcons: (app) =>
				`Privacidade: o ${app} carrega sozinho os ícones dos seus feeds, a partir dos sites dos feeds, que veem seu endereço IP quando isso acontece. Os artigos e suas imagens chegam só pelo rueder.`,
			shotsTitle: 'Passo a passo, com imagens',
			shots: {
				readyou: [
					'Toque na engrenagem no canto superior esquerdo.',
					'Abra Contas.',
					'Toque em Adicionar contas.',
					'Escolha Google Reader.',
					'Digite o endereço do servidor como URL do servidor, o nome como nome de usuário e a senha de app como senha, e toque em Adicionar.'
				],
				netnewswire: [
					'No Mac: abra os Ajustes (⌘,), depois Contas, e clique em + no canto inferior esquerdo.',
					'Escolha FreshRSS e clique em Continuar.',
					'Digite o nome como Login, a senha de app como Password e o endereço do servidor como API URL, e clique em Criar.'
				]
			},
			shotAlt: (app, step) => `${app}, passo ${n(step)}`,
			server: 'Endereço do servidor',
			user: 'Nome',
			secret: 'Senha de app',
			copy: (what) => `Copiar ${what}`,
			copied: 'Copiado',
			once: 'A senha de app só é mostrada agora. Se você a perder, revogue-a e crie uma nova.',
			waiting: 'Aguardando o app entrar …',
			connected: 'Conectado: o app entrou.',
			done: 'Pronto',
			refusals: {
				wrongPassword: 'A senha não está certa.',
				codeNeeded:
					'Digite também um código do seu app autenticador ou um código de recuperação, ou confirme com uma chave de acesso.',
				wrongCode: 'Esse código não está certo. Os códigos mudam a cada 30 segundos.',
				loginAgain:
					'Sua conta ainda não tem senha nem segundo fator. Entre de novo e crie a senha de app em até dez minutos.',
				tooManyApps: 'Uma conta tem no máximo 20 senhas de app.',
				tooMany: 'Muitas tentativas erradas. Tente de novo em alguns minutos.',
				unreachable: 'O servidor não responde. Tente de novo em instantes.'
			}
		},
		secondFactor: {
			title: 'Segundo fator',
			intro:
				'Com um segundo fator, entrar também pede um código de um app autenticador, ou uma chave de acesso.',
			app: 'App autenticador',
			on: 'Ativado.',
			left: (count) =>
				`${plural(count, { one: 'Resta', other: 'Restam' })} ${n(count)} ${plural(count, { one: 'código de recuperação', other: 'códigos de recuperação' })}.`,
			off: 'Desativado.',
			byPasskeys: 'Não configurado. Suas chaves de acesso mantêm o segundo fator ativado.',
			adminNeeds:
				'Este servidor só trata como admin quem tem um segundo fator. Configure-o, ou entre de novo com o código dele, para ver o status do servidor.',
			password: 'Sua senha',
			start: 'Configurar',
			scan: 'Leia este código com seu app autenticador, ou digite a chave abaixo.',
			qr: 'Código QR para o app autenticador',
			larger: 'Mostrar o código QR maior',
			smaller: 'Mostrar o código QR menor',
			key: 'Chave',
			copyKey: 'Copiar a chave',
			copied: 'Copiado',
			code: 'O código que o app mostra agora',
			codeOrRecovery: 'Código do app, ou um código de recuperação',
			confirm: 'Confirmar',
			cancel: 'Cancelar',
			codesTitle: 'Seus códigos de recuperação',
			codesIntro:
				'Cada um permite entrar uma vez sem o seu celular. Guarde-os bem: eles só são mostrados agora.',
			copyCodes: 'Copiar os códigos',
			codesDone: 'Já guardei',
			switchOff: 'Desativar',
			switchOffNote: 'Depois disso, para entrar basta de novo a senha.',
			switchOffNoteKeys: 'Suas chaves de acesso mantêm o segundo fator ativado.',
			startWithPasskey: 'Configurar com uma chave de acesso',
			switchOffWithPasskey: 'Desativar com uma chave de acesso',
			refusals: {
				wrongPassword: 'A senha não está certa.',
				codeNeeded:
					'Suas chaves de acesso mantêm o segundo fator ativado. Digite também um código de recuperação.',
				wrongCode: 'Esse código não está certo. Os códigos mudam a cada 30 segundos.',
				loginAgain:
					'Sua conta ainda não tem senha nem segundo fator. Entre de novo e configure o app em até dez minutos.',
				tooMany: 'Muitas tentativas erradas. Tente de novo em alguns minutos.',
				unreachable: 'O servidor não responde. Tente de novo em instantes.'
			}
		},
		passkeys: {
			title: 'Chaves de acesso',
			intro:
				'Uma chave de acesso permite entrar sem nome e senha, com o PIN, a digital ou o rosto que seu dispositivo pedir. Ela conta como segundo fator.',
			once: 'Com uma chave de acesso, sua senha sozinha não abre mais sua conta.',
			none: 'Nenhuma chave de acesso ainda.',
			made: (when) => `adicionada ${when}`,
			used: (when) => `último uso ${when}`,
			unused: 'ainda não usada',
			add: 'Adicionar uma chave de acesso',
			name: 'Nome, por exemplo o do dispositivo',
			defaultName: 'Chave de acesso',
			password: 'Sua senha',
			code: 'Código do seu app autenticador, ou um código de recuperação',
			create: 'Criar a chave de acesso',
			working: 'Siga o seu navegador …',
			cancel: 'Cancelar',
			remove: 'Remover',
			removeName: (name) => `Remover a chave de acesso ${name}`,
			confirmRemove: 'Remover a chave de acesso',
			full: (max) => `Uma conta tem no máximo ${n(max)} chaves de acesso.`,
			or: 'ou',
			confirmWith: 'Confirmar com uma chave de acesso',
			codesTitle: 'Seus códigos de recuperação',
			codesIntro:
				'Sua primeira chave de acesso ativou o segundo fator. Cada código permite entrar uma vez se você perder seu dispositivo. Guarde-os bem: eles só são mostrados agora.',
			copyCodes: 'Copiar os códigos',
			copied: 'Copiado',
			codesDone: 'Já guardei',
			refusals: {
				cancelled: 'A janela do navegador foi fechada antes de a chave de acesso ser usada.',
				known: 'Este dispositivo já tem uma chave de acesso para a sua conta.',
				failed: 'Este navegador ou dispositivo não conseguiu criar nem usar uma chave de acesso.',
				wrong: 'A chave de acesso não foi aceita. Tente de novo.',
				wrongPassword: 'A senha não está certa.',
				codeNeeded:
					'Sua conta tem um segundo fator. Digite também um código do seu app ou um código de recuperação, ou confirme com uma chave de acesso.',
				wrongCode: 'Esse código não está certo. Os códigos mudam a cada 30 segundos.',
				tooManyPasskeys: 'Uma conta tem no máximo 10 chaves de acesso.',
				loginAgain:
					'Sua conta ainda não tem senha nem segundo fator. Entre de novo e adicione sua primeira chave de acesso em até dez minutos.',
				tooMany: 'Muitas tentativas erradas. Tente de novo em alguns minutos.',
				unreachable: 'O servidor não responde. Tente de novo em instantes.'
			}
		},
		reports: {
			send: 'Enviar a este servidor relatórios de erros deste navegador',
			intro: 'Quando algo dá errado na página, os admins deste servidor ficam sabendo.',
			what: 'Um relatório contém:',
			holds: [
				'a mensagem do erro, sem endereços e sem números que identifiquem algo',
				'em que lugar dos arquivos do rueder ele aconteceu',
				'a versão do rueder e o seu navegador com a versão principal dele'
			],
			never:
				'Nunca o que você lê, seus feeds, marcadores ou regras. Os relatórios somem depois de 30 dias.',
			off: 'Desativar isto exclui os relatórios que você enviou.'
		}
	},

	rules: {
		intro:
			'As regras ocultam artigos ou os marcam como lidos quando chegam. Artigos ocultos não contam como não lidos e ficam em “Ocultos por filtros”.',
		none: 'Nenhuma regra ainda.',
		add: 'Adicionar uma regra',
		new: 'Nova regra',
		on: 'Ativada',
		scope: 'Em',
		scopeAll: 'Todos os feeds',
		scopeFolder: (title) => `Pasta: ${title}`,
		scopeFeed: (title) => `Feed: ${title}`,
		field: 'Olhar em',
		fields: { any: 'Tudo', title: 'Título', content: 'Texto', author: 'Autor', tag: 'Tag' },
		match: 'Comparação',
		matches: {
			contains: 'contém',
			word: 'contém a palavra',
			starts: 'começa com',
			ends: 'termina com',
			regex: 'corresponde à expressão regular',
			all: 'cada artigo, diga o que disser'
		},
		pattern: 'Texto a procurar',
		action: 'Então',
		actions: { hide: 'ocultar o artigo', read: 'marcá-lo como lido' },
		preview: (count, total) => `Vale agora para ${n(count)} de ${n(total)} artigos daqui.`,
		regexTip:
			'As expressões regulares são lidas como no Rust, sem diferenciar maiúsculas de minúsculas.',
		regexBuilder: 'Montar e testar uma no regex101',
		invalid: 'Esta não é uma expressão regular válida.',
		empty: 'Digite o texto a procurar.',
		checking: 'Vendo o que ela pegaria …',
		hits: (count) => `${n(count)} ${plural(count, { one: 'ocorrência', other: 'ocorrências' })}`,
		summary: (field, match, pattern, action) => `${field} ${match} “${pattern}”: ${action}`,
		summaryAll: (action) => `Cada artigo: ${action}`
	},

	keys: {
		title: 'Atalhos de teclado',
		note: 'Suas teclas ficam salvas com as configurações. Enter, Esc e g continuam como estão.',
		change: (what) => `Mudar a tecla de: ${what}`,
		changeShort: 'Mudar',
		press: 'Pressione uma tecla …',
		taken: (key, other) => `${key} já é a tecla de: ${other}`,
		fixed: (key) => `${key} não pode ser atribuída a um atalho.`,
		reset: 'Voltar às teclas do rueder',
		groups: { articles: 'Artigos', lists: 'Listas', go: 'Ir para', app: 'Em qualquer lugar' },
		next: 'Próximo artigo, aberto',
		previous: 'Artigo anterior, aberto',
		nextCursor: 'Próximo artigo, sem abrir',
		previousCursor: 'Artigo anterior, sem abrir',
		toggle: 'Abrir ou fechar o artigo',
		star: 'Adicionar ou remover a estrela',
		read: 'Marcar como lido ou não lido',
		original: 'Abrir o original em uma nova aba',
		markAll: 'Marcar tudo como lido',
		refresh: 'Atualizar a lista',
		unreadOnly: 'Só não lidos, ou todos os artigos',
		listView: 'Visualização em lista',
		expandedView: 'Visualização expandida',
		splitView: 'Três painéis',
		magazineView: 'Revista',
		goHome: 'Início',
		goAll: 'Todos os artigos',
		goUnread: 'Artigos não lidos',
		goStarred: 'Artigos com estrela',
		search: 'Pesquisar',
		sidebar: 'Mostrar ou ocultar a barra lateral',
		help: 'Esta visão geral',
		then: 'depois'
	},

	labels: {
		title: 'Marcadores',
		add: 'Novo marcador',
		name: 'Nome do marcador',
		none: 'Nenhum marcador ainda.',
		actions: (title) => `Ações do marcador ${title}`,
		rename: 'Renomear',
		color: 'Cor',
		colors: ['Vermelho', 'Laranja', 'Amarelo', 'Verde', 'Turquesa', 'Azul', 'Roxo', 'Cinza'],
		remove: 'Excluir o marcador',
		show: 'Mostrar os marcadores',
		hide: 'Ocultar os marcadores',
		forArticle: 'Marcadores',
		set: (title) => `Marcador: ${title}`
	},

	subscribe: {
		title: 'Assinar um feed',
		address: 'Endereço do feed ou do site',
		find: 'Encontrar feeds',
		finding: 'Procurando …',
		note: 'O rascunho não consegue buscar nada, então o feed fica vazio.',
		found: (count) => (count === 1 ? 'Há um feed:' : `Há ${n(count)} feeds:`),
		none: 'Não há feed neste endereço.',
		movedTo: (url) => `O site encaminhou o pedido para ${url}.`,
		loginWall:
			'Alguns sites mandam quem não entrou para a página de login. Aí os feeds deles não podem ser lidos.',
		subscribed: 'Assinado',
		show: 'Mostrar',
		untitled: 'Sem nome',
		folder: 'Na pasta',
		topLevel: 'Sem pasta',
		add: 'Assinar',
		adding: 'Assinando …',
		refused: 'Não funcionou.',
		serverSays: (reason) => `O servidor diz: ${reason}`,
		busy: (seconds) =>
			seconds
				? `Muitas consultas ao mesmo tempo. Tente de novo em ${n(seconds)} ${plural(seconds, { one: 'segundo', other: 'segundos' })}.`
				: 'Muitas consultas ao mesmo tempo. Tente de novo em instantes.',
		noFetching: 'Este servidor não busca feeds.',
		tooLarge: (megabytes) => `O arquivo tem mais de ${n(megabytes)} MB.`,
		unreachable: 'O servidor não responde. Tente de novo em instantes.'
	},

	opml: {
		title: 'Importar e exportar',
		intro:
			'Traga suas assinaturas de outro leitor de feeds como arquivo OPML, ou leve-as com você.',
		import: 'Importar um arquivo OPML',
		importing: 'Importando …',
		imported: (feeds, folders) =>
			`${n(feeds)} ${plural(feeds, { one: 'feed novo', other: 'feeds novos' })} e ${n(folders)} ${plural(folders, { one: 'pasta nova', other: 'pastas novas' })}.`,
		known: (count) =>
			`${n(count)} ${plural(count, { one: 'feed já estava assinado', other: 'feeds já estavam assinados' })}.`,
		skipped: (count) =>
			`${n(count)} ${plural(count, { one: 'entrada ficou de fora, por não ser um feed legível', other: 'entradas ficaram de fora, por não serem feeds legíveis' })}.`,
		later: 'Os artigos deles chegam na próxima rodada de buscas.',
		export: 'Exportar como OPML'
	},

	removeAccount: {
		title: 'Excluir a conta',
		intro: 'Isto exclui sua conta neste rueder para sempre. Não dá para desfazer.',
		goes: 'O que vai junto: suas sessões em todos os dispositivos, suas assinaturas e pastas, o que você leu, estrelas, marcadores, regras de filtro, configurações, relatórios de erros, convites e o vínculo com o login único.',
		stays:
			'Os feeds ficam para os outros que os leem. Para levar sua conta para outro lugar antes, use “Mudança”.',
		password: 'Sua senha',
		name: (name) => `Digite seu nome, ${name}, para confirmar`,
		code: 'Código do seu app, ou um código de recuperação',
		submit: 'Excluir minha conta para sempre',
		wrong: 'Isso não está certo. Nada foi excluído.',
		codeNeeded:
			'Sua conta tem um segundo fator. Digite também um código do seu app ou um código de recuperação, ou confirme com uma chave de acesso.',
		busy: (count) => `Muitas tentativas erradas. Tente de novo em ${minutes(count)}.`,
		unreachable: 'O servidor não responde. Nada foi excluído.'
	},

	move: {
		intro: 'Leve sua conta para outro rueder, ou traga-a de um para cá, em um único arquivo.',
		howItWorks:
			'Uma importação acrescenta ao que está aqui e não tira nada. Suas configurações passam a ser as do arquivo.',
		notInIt: 'Não está no arquivo: sua senha, suas sessões e os textos dos artigos.',
		export: 'Exportar a conta',
		import: 'Importar um arquivo de conta',
		importing: 'Importando …',
		feeds: (feeds, known) =>
			`${n(feeds)} ${plural(feeds, { one: 'feed novo', other: 'feeds novos' })}, ${n(known)} já assinados.`,
		folders: (count) =>
			`${n(count)} ${plural(count, { one: 'pasta nova', other: 'pastas novas' })}.`,
		articles: (found) =>
			`${articles(found)} ${plural(found, { one: 'encontrado', other: 'encontrados' })} aqui.`,
		missing: (count) =>
			`${articles(count)} não ${plural(count, { one: 'foi encontrado', other: 'foram encontrados' })} aqui; o que você tinha lido, marcado com estrela ou com marcador neles se perdeu.`,
		marks: (labels, rules) =>
			`${n(labels)} ${plural(labels, { one: 'marcador novo', other: 'marcadores novos' })} e ${n(rules)} ${plural(rules, { one: 'regra de filtro nova', other: 'regras de filtro novas' })}.`,
		skipped: (count) =>
			`${n(count)} ${plural(count, { one: 'entrada do arquivo não pôde ser aproveitada', other: 'entradas do arquivo não puderam ser aproveitadas' })}.`,
		settings: 'Agora suas configurações são as do arquivo.',
		later:
			'Os feeds novos trazem seus artigos na próxima rodada de buscas. O que você tinha lido continua lido.',
		busy: (seconds) =>
			seconds
				? `Muitas importações. Tente de novo em ${minutes(Math.ceil(seconds / 60))}.`
				: 'Muitas importações. Tente de novo mais tarde.'
	},

	status: {
		title: 'Status do servidor',
		open: 'Status do servidor',
		loading: 'Consultando o servidor …',
		failed: 'O servidor não respondeu. Tente de novo em instantes.',
		refresh: 'Atualizar',
		asOf: (time) => `Às ${time}. O que se conta é a última hora.`,
		demo: 'Inventado, porque não há servidor aqui.',
		empty: '–',
		on: 'Ativado',
		off: 'Desativado',
		took: (median, p95) => `mediana ${median}, 95% em até ${p95}`,
		summary: {
			label: 'Num relance',
			hints: 'Avisos',
			failing: 'Feeds falhando',
			errors: 'Erros registrados',
			reported: 'Feeds denunciados'
		},
		hintsTitle: 'Vale uma olhada',
		allWell: 'Nada se destaca.',
		hints: {
			slowRoute: (route, took) =>
				`${route} está lenta: uma em cada vinte requisições leva mais de ${took}.`,
			serverErrors: (route, count) =>
				`${route} respondeu com um erro do servidor ${n(count)} ${plural(count, { one: 'vez', other: 'vezes' })}.`,
			poolBusy: (size) =>
				`Todas as ${n(size)} conexões com o banco de dados estão em uso, então as requisições esperam por uma.`,
			fetchBacklog: (count, late) =>
				`${n(count)} ${plural(count, { one: 'feed é buscado', other: 'feeds são buscados' })} com atraso, o mais atrasado ${late}.`,
			feedsFailing: (failing, subscribed) => `${n(failing)} de ${n(subscribed)} feeds falham.`,
			notFetching: 'O servidor não busca feeds. FETCH está em off?',
			fetchPaceFull: (most) =>
				`Cada rodada pega todos os feeds que pode, e ainda há feeds esperando: o ritmo permite ${n(most)} buscas por hora. FETCH_ROUND, FETCH_AT_ONCE e FETCH_EVERY definem isso.`,
			cacheMisses: (share) =>
				`O banco de dados encontra na memória só ${share} do que lê; mais shared_buffers ajudaria.`,
			databaseLarge: (size, warn) =>
				`O banco de dados chegou a ${size}; DATABASE_WARN_GB avisa a partir de ${warn}. “rueder retention set” remove os artigos antigos ou os textos deles.`,
			longQuery: (took) => `Uma consulta está rodando há ${took}.`,
			imageCacheFull: (share) => `O cache de imagens está ${share} cheio.`,
			errors: (count) =>
				`O servidor registrou ${n(count)} ${plural(count, { one: 'erro', other: 'erros' })} na última hora.`,
			limited: (count) =>
				`${n(count)} ${plural(count, { one: 'requisição foi recusada', other: 'requisições foram recusadas' })} por serem demais.`,
			browserErrors: (count) =>
				`Os navegadores relataram ${n(count)} ${plural(count, { one: 'erro', other: 'erros diferentes' })} no último dia.`,
			reportedFeeds: (count) =>
				`Leitores denunciaram ${n(count)} ${plural(count, { one: 'feed que não está bloqueado', other: 'feeds que não estão bloqueados' })}.`
		},
		build: {
			title: 'Build',
			commit: 'Commit',
			dirty: (commit) => `${commit}, com alterações próprias`,
			branch: 'Branch',
			tag: 'Tag',
			committed: 'Data do commit',
			where: 'Roda',
			container: 'em um contêiner',
			host: 'na própria máquina',
			target: 'Para',
			profile: 'Compilado',
			profileValue: (profile, level) => `${profile}, otimização ${level}`,
			ci: 'Compilado por',
			ciValue: 'uma execução da CI',
			byHand: 'à mão'
		},
		server: {
			title: 'Servidor',
			version: 'Versão',
			running: 'Rodando desde',
			memory: 'Memória',
			accounts: 'Contas',
			feeds: 'Feeds',
			feedsValue: (fetched, all) => `${n(fetched)} buscados, ${n(all)} no total`,
			articles: 'Artigos',
			articlesValue: (count) => `cerca de ${n(count)}`,
			live: 'Abas abertas'
		},
		database: {
			title: 'Banco de dados',
			pool: 'Conexões do rueder',
			poolValue: (size, idle, max) => `${n(size)} abertas, ${n(idle)} ociosas, no máximo ${n(max)}`,
			connections: 'Todas as conexões',
			size: 'Tamanho',
			sizeWarn: (size, warn) => `${size}, aviso a partir de ${warn}`,
			hitRatio: 'Lido da memória',
			deadlocks: 'Deadlocks',
			longest: 'Consulta mais longa em execução',
			tables: 'Maiores tabelas',
			table: 'Tabela',
			bytes: 'Tamanho',
			rows: 'Linhas, aprox.'
		},
		fetcher: {
			title: 'Busca de feeds',
			state: 'Busca',
			lastRound: 'Última rodada',
			lastRoundValue: (ago, feeds, took) =>
				`${ago}: ${n(feeds)} ${plural(feeds, { one: 'feed', other: 'feeds' })} em ${took}`,
			fetches: 'Buscas',
			fetchesValue: (fetched, unchanged, failed) =>
				`${n(fetched)} trouxeram o feed, ${n(unchanged)} sem mudanças, ${n(failed)} falharam`,
			broken: 'Falharam no banco de dados',
			newArticles: 'Artigos novos',
			took: 'Tempo por busca',
			overdue: 'Buscados com atraso',
			overdueValue: (count, late) => `${n(count)}, o mais atrasado ${late}`,
			waiting: 'Ainda não buscados',
			pace: 'Ritmo',
			paceValue: (round, every, atOnce, most) =>
				`${n(round)} ${plural(round, { one: 'feed', other: 'feeds' })} a cada ${every}, ${n(atOnce)} ao mesmo tempo: no máximo ${n(most)} por hora`,
			perHour: 'Buscas na última hora',
			behind: 'Atraso',
			behindValue: (late) => `o feed que espera há mais tempo está ${late} atrasado`,
			notBehind: 'Nada está esperando',
			failing: 'Feeds falhando, por motivo',
			noneFailing: 'Nenhum feed falha.',
			feeds: (count) => `${n(count)} ${plural(count, { one: 'feed', other: 'feeds' })}`,
			failures: {
				status: (code) => (code ? `O site responde ${code}` : 'O site responde com um erro'),
				timeout: 'O site demora demais',
				blocked: 'O endereço está numa rede da qual o rueder não busca',
				dns: 'O nome não tem endereço',
				network: 'Sem conexão, ou ela caiu',
				too_large: 'Grande demais',
				redirects: 'Redirecionamentos demais, ou um quebrado',
				unreadable: 'Não é um feed que o rueder consiga ler',
				other: 'Outra coisa'
			}
		},
		finder: {
			title: 'Descoberta de feeds',
			state: 'Análise de endereços',
			lookups: 'Consultas',
			lookupsValue: (found, none, failed) =>
				`${n(found)} encontraram feeds, ${n(none)} não encontraram nenhum, ${n(failed)} falharam`,
			limited: 'Acima do limite da conta',
			took: 'Tempo por consulta'
		},
		images: {
			title: 'Proxy de imagens',
			state: 'Proxy',
			cache: 'Cache',
			cacheValue: (used, max) => `${used} de ${max}`,
			served: 'Imagens',
			servedValue: (cached, fetched) => `${n(cached)} do cache, ${n(fetched)} buscadas`,
			refused: 'Recusadas',
			refusedValue: (source, notImage, tooLarge, notFound) =>
				`${n(source)} falha da fonte, ${n(notImage)} não são imagens, ${n(tooLarge)} grandes demais, ${n(notFound)} endereço errado`,
			took: 'Tempo por imagem buscada',
			failingSources: 'Fontes deixadas de lado por enquanto'
		},
		requests: {
			title: 'Requisições',
			all: 'Todas as requisições',
			allValue: (count, took) => `${n(count)}, ${took}`,
			answers: 'Respostas',
			answersValue: (fine, refused, errors) =>
				`${n(fine)} certas, ${n(refused)} recusadas, ${n(errors)} erros do servidor`,
			route: 'Rota',
			count: 'Quantidade',
			median: 'Mediana',
			p95: '95%',
			max: 'A mais longa',
			errors: 'Erros',
			none: 'Nenhuma requisição na última hora.'
		},
		problems: {
			title: 'Avisos e erros',
			counts: (errors, warnings) =>
				`Na última hora, ${n(errors)} ${plural(errors, { one: 'erro', other: 'erros' })} e ${n(warnings)} ${plural(warnings, { one: 'aviso', other: 'avisos' })}.`,
			none: 'Nada foi registrado desde o início.',
			error: 'Erro',
			warning: 'Aviso',
			times: (count, last) => (count > 1 ? `${n(count)} vezes, a última ${last}` : last)
		},
		browsers: {
			title: 'Erros nos navegadores',
			off: 'Este servidor não aceita relatórios de erros (CLIENT_REPORTS=off).',
			consenting: (count) =>
				`${n(count)} ${plural(count, { one: 'conta envia', other: 'contas enviam' })} relatórios. Os erros ficam 30 dias depois da última vez que aconteceram.`,
			none: 'Nenhum navegador relatou erro.',
			kinds: {
				error: 'Erro',
				rejection: 'Promise rejeitada',
				component: 'Parte da página',
				request: 'Requisição'
			},
			hits: (count, accounts, last) =>
				`${n(count)} ${plural(count, { one: 'vez', other: 'vezes' })} em ${n(accounts)} ${plural(accounts, { one: 'conta', other: 'contas' })}, a última ${last}`,
			where: (versions, browsers) => `Versão ${versions} · ${browsers}`
		},
		reported: {
			title: 'Feeds denunciados',
			text: 'Quem denunciou não aparece aqui. “rueder feed block <endereço>” bloqueia um feed para todos, “rueder feed unblock <endereço>” o desbloqueia.',
			none: 'Ninguém denunciou um feed.',
			reports: (count, last) =>
				`${n(count)} ${plural(count, { one: 'denúncia', other: 'denúncias' })}, a última ${last}`,
			blocked: 'Bloqueado'
		}
	},

	update: {
		ready: 'Uma nova versão do rueder está pronta.',
		reload: 'Recarregar',
		later: 'Mais tarde'
	},

	crash: {
		title: 'Algo deu errado nesta página.',
		text: 'Recarregue a página para continuar. O que você leu e alterou está salvo no servidor.',
		reload: 'Recarregar'
	},

	demo: {
		badge: 'Rascunho',
		note: 'Um rascunho com feeds inventados. Nada é salvo em um servidor.',
		arrive: 'Fazer chegar um artigo',
		revise: 'Alterar o artigo aberto',
		revised: 'Atualização: este parágrafo foi adicionado para testar as versões.',
		reviseHint: 'Abra um artigo primeiro.'
	}
};

export default ptBR;
