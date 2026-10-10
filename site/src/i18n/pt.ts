// Brazilian Portuguese copy, same shape as en.ts. The app itself is in English, so the view names that
// appear in the screenshots (Board, Focus...) keep their English names.

import type { Copy } from "./en";

const REPO = "https://github.com/IFafaa/falog";

const pt: Copy = {
  lang: "pt",
  htmlLang: "pt-BR",
  dateLocale: "pt-BR",
  languageName: "Português",

  meta: {
    home: {
      title: "Falog: organize sua vida só falando",
      description:
        "Diga o que precisa fazer e o Falog coloca na sua lista, ao lado das suas reuniões. Grátis para Windows, macOS e Linux.",
    },
    download: {
      title: "Baixe o Falog para Windows, macOS e Linux",
      description: "Baixe a versão mais recente do Falog para Windows, macOS ou Linux.",
    },
    releases: {
      title: "Versões do Falog",
      description: "Todas as versões do Falog e o que mudou em cada uma.",
    },
  },

  nav: {
    skip: "Pular para o conteúdo",
    sections: "Páginas",
    home: "Início",
    download: "Download",
    releases: "Versões",
    github: "GitHub",
    switchLanguage: "View in English",
    themeToLight: "Mudar para o tema claro",
    themeToDark: "Mudar para o tema escuro",
    brandHome: "Falog, página inicial",
  },

  suggest: {
    text: "Este site também está em português.",
    action: "Ver em português",
    dismiss: "Agora não",
  },

  home: {
    title: "Organize sua vida<br>só falando.",
    lede: "O Falog é um organizador pessoal que você usa pela voz. Diga o que precisa fazer e pronto: está na sua lista, ao lado das suas reuniões.",
    download: "Baixar o Falog",
    tour: "Veja o que ele faz",
    also: "Windows · macOS · Linux · ",
    allDownloads: "todos os downloads",
    heroAlt: {
      dark: "O quadro de tarefas do Falog no tema escuro, com o assistente transformando uma mensagem falada em duas tarefas.",
      light: "O quadro de tarefas do Falog no tema claro, com o assistente transformando uma mensagem falada em duas tarefas.",
    },

    featuresLabel: "01 · Recursos",
    featuresTitle: "Tudo o que você tem para fazer, num só lugar",
    featuresLede: "Trabalho e vida pessoal lado a lado. Veja suas tarefas em quadro, em lista, por urgência ou no calendário.",
    viewsLabel: "Visões",
    views: [
      {
        id: "board",
        name: "Board",
        caption: "<b>Board.</b> Suas tarefas em colunas, por status. Arraste um cartão para avançar.",
        alt: "O quadro, com uma tarefa aberta ao lado.",
      },
      {
        id: "list",
        name: "List",
        caption: "<b>List.</b> Todas as tarefas numa lista compacta, na ordem que você precisar.",
        alt: "A lista: uma linha por tarefa.",
      },
      {
        id: "focus",
        name: "Focus",
        caption: "<b>Focus.</b> O que precisa de você agora, do que está atrasado ao que vence esta semana.",
        alt: "Focus: tarefas abertas agrupadas pelo prazo.",
      },
      {
        id: "calendar",
        name: "Calendar",
        caption: "<b>Calendar.</b> Suas reuniões e tarefas lado a lado, por semana ou por mês.",
        alt: "O calendário: um mês de reuniões e tarefas.",
      },
      {
        id: "archive",
        name: "Archive",
        caption: "<b>Archive.</b> Tarefas concluídas fora do caminho, a um clique de voltar.",
        alt: "O arquivo: tarefas concluídas agrupadas por mês.",
      },
    ],
    themeAlt: { dark: "Tema escuro. ", light: "Tema claro. " },

    featuresList: "O que o Falog faz",
    features: [
      {
        icon: "sparkle",
        title: "Converse com suas tarefas",
        text: "Diga ao assistente o que precisa, do seu jeito. Ele cria, atualiza e revisa suas tarefas por você, várias de uma vez se quiser.",
      },
      {
        icon: "mic",
        title: "Ditado seguro e offline",
        text: "Segure <kbd class=\"mod\">Ctrl+Space</kbd> e fale. O ditado funciona sem internet e fica no seu computador: é totalmente seguro.",
      },
      {
        icon: "agents",
        title: "Funciona com a sua IA",
        text: "Use o assistente que preferir: Claude Code, Gemini CLI, Codex ou qualquer agente ACP. Dá até para adicionar o seu.",
      },
      {
        icon: "folder",
        title: "Cada contexto no seu lugar",
        text: "Trabalho, casa, estudos: cada parte da sua vida com sua própria área e cor. Foque numa só ou veja tudo junto.",
      },
      {
        icon: "calendar",
        title: "Calendário",
        text: "Suas reuniões ao lado das suas tarefas, por semana ou por mês.",
      },
      {
        icon: "lock",
        title: "Seus dados ficam com você",
        text: "Sem criar conta. O Falog só se conecta à internet para o assistente de IA que você escolher e as agendas que você conectar; todo o resto fica com você.",
      },
      {
        icon: "monitor",
        title: "Windows, macOS e Linux",
        text: "Disponível nos três. Abre junto com o computador e acompanha o seu tema claro ou escuro.",
      },
      {
        icon: "code",
        title: "Gratuito e de código aberto",
        text: "Grátis para usar e aberto para qualquer pessoa ler e melhorar.",
      },
    ],

    progressLabel: "02 · Progresso",
    progressTitle: "Acompanhe o projeto",
    progressLede: `O que há de novo e o que vem por aí. <a href="${REPO}">Acompanhe o Falog no GitHub</a>.`,
    latestTitle: "Versão mais recente",
    latestNone: "A primeira versão está a caminho.",
    allReleases: "Todas as versões",
    roadmapTitle: "Roadmap",
    roadmapLink: "Roadmap completo no GitHub ↗",
    stages: { doing: "Em andamento", next: "Próximos passos", done: "Já entregue" },
    moreShipped: (n: number) => `e mais ${n}`,
  },

  download: {
    label: "Windows · macOS · Linux",
    title: "Baixe o Falog",
    stableDefault: "A versão estável mais recente.",
    stableNone: "Ainda não há versão estável.",
    notesLink: "o que mudou ↗",
    previewPage: "Página da versão ↗",
    missingAsset: "Ainda não está na {version}; veja a página da versão",
    noRelease: `A primeira versão está quase pronta. Enquanto isso, você pode compilar o Falog no <a href="${REPO}/blob/main/docs/install/windows.md">Windows</a>, no <a href="${REPO}/blob/main/docs/install/macos.md">macOS</a> ou no <a href="${REPO}/blob/main/docs/install/linux.md">Linux</a> (guias em inglês).`,
    yourSystem: "Seu sistema",
    windows: {
      sub: "Instalador",
      button: "Baixar .exe",
      note: "Se o Windows mostrar um aviso do SmartScreen, clique em <b>Mais informações › Executar assim mesmo</b>.",
    },
    macos: {
      sub: "Apple silicon e Intel",
      arm: "Apple silicon",
      intel: "Intel",
      note: "Se o macOS disser que não consegue verificar o Falog, abra <b>Ajustes do Sistema › Privacidade e Segurança</b> e clique em <b>Abrir Mesmo Assim</b>.",
    },
    linux: {
      sub: "Um comando ou o arquivo",
      copy: "Copiar o comando de instalação",
      button: "Baixar .tar.gz",
      note: "Cole o comando num terminal ou baixe o arquivo.",
    },
    agents: "Para conversar com o Falog, instale um assistente de IA: Claude Code, Gemini CLI, Codex ou qualquer agente ACP.",
    previewBadge: "Prévia",
    previewText: "Uma prévia do que vem por aí. Menos estável, para quem gosta de testar primeiro.",
    older: "Procurando uma versão anterior? Veja <a href=\"{releases}\">todas as versões</a>.",
  },

  releases: {
    label: "Histórico de mudanças",
    title: "Versões",
    lede: "Todas as versões do Falog, da mais nova para a mais antiga. As prévias são versões antecipadas e menos estáveis. As notas estão em inglês.",
    onGithub: "Versões no GitHub ↗",
    stable: "Estável",
    latestStable: "Estável mais recente",
    preview: "Prévia",
    notes: "O que mudou",
    none: "Nenhuma versão ainda. A primeira está a caminho.",
    unknown: `Não dá para mostrar as versões agora. <a href="${REPO}/releases">Veja todas as versões no GitHub</a>.`,
  },

  footer: {
    license: "Licença MIT",
    source: "Código",
    releases: "Versões",
    issues: "Issues",
    specs: "Especificações",
  },

  js: {
    downloadFor: "Baixar para {os}",
    alsoFor: "Também para ",
    installOn: "Instalar no {os}",
    howToInstall: "Como instalar",
    copied: "Copiado",
    themeToLight: "Mudar para o tema claro",
    themeToDark: "Mudar para o tema escuro",
    arm: "Apple silicon",
    intel: "Intel",
  },

  roadmap: {
    "Accounts and sync across devices": "Conta e sincronização entre dispositivos",
    "Sign in with your email and keep the same tasks on every device": "Entre com o seu e-mail e tenha as mesmas tarefas em todos os seus dispositivos",
    "System tray": "Rodar em segundo plano",
    "Keep Falog running without a window in the taskbar": "Feche a janela e o Falog continua funcionando",
    "Due date reminders": "Lembretes de prazo",
    "Get a Windows notification before things are late": "Um aviso antes de algo atrasar",
    "Recurring tasks": "Tarefas recorrentes",
    "Weekly reports, monthly invoices, standing chores": "Relatórios semanais, contas do mês, tarefas de rotina",
    "Quick capture hotkey": "Captura rápida",
    "File a task from anywhere without the assistant": "Anote uma tarefa de qualquer lugar, com um atalho",
    "Release downloads": "Downloads para todos os sistemas",
    "Calendars by area, events by voice": "Agendas por área, eventos por voz",
    "Google Calendar through secret iCal links": "Google Agenda conectado por um link",
    "macOS and Linux": "macOS e Linux",
    "Assistant agents": "Escolha o seu assistente de IA",
    "Calendar view with Google Calendar": "Um calendário com as suas reuniões",
    "Areas for every part of life": "Áreas para cada parte da vida",
    "Live transcription": "Veja as palavras enquanto fala",
    "Assistant threads": "Conversas separadas",
    "Assistant panel with voice": "Converse com o Falog",
    "App icon": "Ícone do app",
  },
};

export default pt;
