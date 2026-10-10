// Brazilian Portuguese copy, same shape as en.ts. The app itself is in English, so view and button
// names that appear in the screenshots (Board, Focus, More info...) keep their English names where
// that helps the reader find them.

import type { Copy } from "./en";

const REPO = "https://github.com/IFafaa/falog";

const pt: Copy = {
  lang: "pt",
  htmlLang: "pt-BR",
  dateLocale: "pt-BR",
  languageName: "Português",

  meta: {
    home: {
      title: "Falog: seu organizador pessoal, é só falar",
      description:
        "O Falog guarda suas tarefas, pendências e compromissos num só lugar, no seu computador. Diga o que precisa fazer, por voz ou texto, e ele registra para você. Gratuito e de código aberto para Windows, macOS e Linux.",
    },
    download: {
      title: "Baixe o Falog para Windows, macOS e Linux",
      description: "Baixe a versão estável mais recente do Falog para Windows, macOS ou Linux, ou experimente uma prévia.",
    },
    releases: {
      title: "Versões do Falog e notas de cada uma",
      description: "Todas as versões do Falog, estáveis e prévias, com as notas de cada uma.",
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
    eyebrow: "Gratuito e de código aberto",
    title: "Seu organizador pessoal,<br>é só falar.",
    lede:
      "Tarefas, pendências e compromissos num só lugar, no seu computador. Diga o que precisa fazer, por voz ou texto, e o Falog registra tudo com a área, o prazo, a prioridade e quem pediu.",
    download: "Baixar o Falog",
    source: "Ver o código-fonte",
    also: "Windows · macOS · Linux · ",
    allDownloads: "todos os downloads",
    heroAlt: {
      dark: "O Falog no tema escuro: o quadro com as colunas To do, In progress e Waiting, e o painel do assistente, onde uma única mensagem falada virou duas tarefas.",
      light:
        "O Falog no tema claro: o quadro com as colunas To do, In progress e Waiting, e o painel do assistente, onde uma única mensagem falada virou duas tarefas.",
    },

    featuresLabel: "01 · Recursos",
    featuresTitle: "Tudo o que você tem para fazer, numa janela só",
    featuresLede:
      "Trabalho e vida pessoal lado a lado, sem ficar cada um num canto. Cinco jeitos de ver as mesmas tarefas, cada um a um atalho de distância.",
    viewsLabel: "Visões",
    views: [
      {
        id: "board",
        name: "Board",
        caption:
          "<b>Board.</b> Uma coluna para cada status: To do, In progress, Waiting e Done. Os cartões mostram a área, a prioridade, o prazo e quem pediu; arraste um cartão para mudar o status e clique nele para ver as anotações.",
        alt: "O quadro com uma tarefa aberta no painel lateral, mostrando a descrição e as anotações.",
      },
      {
        id: "list",
        name: "List",
        caption:
          "<b>List.</b> Todas as tarefas abertas numa tabela compacta, ordenada por urgência, prazo ou prioridade, com as concluídas a um clique.",
        alt: "A lista: uma linha por tarefa, com status, título, área, prioridade e prazo.",
      },
      {
        id: "focus",
        name: "Focus",
        caption:
          "<b>Focus.</b> O que precisa de você agora: o que está atrasado, vence hoje, vence esta semana, está em andamento, depende de alguém, vem depois e o resto. A resposta para “o que era mesmo que eu tinha que fazer?” em segundos.",
        alt: "A visão Focus: tarefas abertas agrupadas em atrasadas, para hoje, para esta semana e outras.",
      },
      {
        id: "calendar",
        name: "Calendar",
        caption:
          "<b>Calendar.</b> Suas reuniões por semana ou por mês, na cor de cada área, com as tarefas que vencem em cada dia na mesma grade.",
        alt: "O calendário do mês: reuniões das agendas de trabalho e pessoal e as tarefas que vencem em cada dia.",
      },
      {
        id: "archive",
        name: "Archive",
        caption:
          "<b>Archive.</b> As tarefas concluídas ficam guardadas pelo mês em que você terminou, e o quadro continua enxuto. Recupere qualquer uma com um clique.",
        alt: "A visão Archive: tarefas concluídas agrupadas pelo mês em que foram terminadas.",
      },
    ],
    themeAlt: { dark: "Tema escuro. ", light: "Tema claro. " },

    featuresList: "O que o Falog faz",
    features: [
      {
        icon: "sparkle",
        title: "Converse com suas tarefas",
        text: "O painel do assistente cria, atualiza e revisa tarefas a partir do que você diz. Uma mensagem pode trazer vários pedidos, e cada um vira uma tarefa com todo o contexto que você deu. Cada assunto pode ter sua própria conversa.",
      },
      {
        icon: "mic",
        title: "Sua voz não sai do computador",
        text: "Segure <kbd class=\"mod\">Ctrl+Space</kbd> e fale. O reconhecimento de voz roda na sua máquina e as palavras aparecem enquanto você fala; sua voz nunca é enviada para fora. O modelo de voz é baixado uma única vez, na primeira vez que você dita.",
      },
      {
        icon: "agents",
        title: "Funciona com o seu agente",
        text: "Use o agente que você já tem: Claude Code, Gemini CLI, Codex ou qualquer agente compatível com o Agent Client Protocol (ACP), inclusive um que você mesmo adicionar. Escolha o agente, o modelo e o nível de esforço em cada conversa. Qualquer cliente MCP também consegue criar e revisar suas tarefas.",
      },
      {
        icon: "folder",
        title: "Uma área para cada parte da vida",
        text: "Trabalho, Casa, Saúde, Estudos: dê nome às suas áreas e uma cor para cada uma. Filtre tudo por área, reuniões incluídas, sem perder a visão da semana inteira.",
      },
      {
        icon: "calendar",
        title: "Calendário",
        text: "O calendário do Falog mostra suas reuniões e as tarefas de cada dia numa visão por semana ou por mês, cada uma na cor da sua área. Hoje ele se conecta ao Google Agenda, colando o link privado de uma agenda ou entrando com a sua conta Google, o que também permite ao assistente marcar reuniões por você. Outros provedores estão a caminho.",
      },
      {
        icon: "lock",
        title: "Local e privado",
        text: "Suas tarefas ficam num único arquivo SQLite no seu computador. Sem conta, sem servidor, funciona offline. Quando você usa o assistente, o agente escolhido lê e altera as tarefas pelo servidor MCP do Falog, e mais nada tem acesso a elas.",
      },
      {
        icon: "monitor",
        title: "Windows, macOS e Linux",
        text: "Um app nativo nos três sistemas, escrito em Rust. Abre junto com o computador, aparece na hora numa janela só e acompanha o tema claro ou escuro do sistema.",
      },
      {
        icon: "code",
        title: "Código aberto",
        text: "Licença MIT, com as especificações e o roadmap no próprio repositório. Leia o código, abra uma issue ou compile você mesmo com poucos comandos.",
      },
    ],

    progressLabel: "02 · Progresso",
    progressTitle: "Acompanhe o projeto",
    progressLede: `O que já saiu, o que está sendo feito e o que vem por aí. <a href="${REPO}">Acompanhe o repositório</a> para saber das novas versões.`,
    latestTitle: "Versão mais recente",
    latestLoading: "Buscando a versão mais recente no GitHub…",
    allReleases: "Todas as versões e suas notas",
    roadmapTitle: "Roadmap",
    roadmapLink: "O roadmap no repositório ↗",
    stages: { doing: "Em andamento", next: "Próximos passos", done: "Já entregue", later: "Mais adiante" },
    moreShipped: (n: number) => `e mais ${n}`,
  },

  download: {
    label: "Windows · macOS · Linux",
    title: "Baixe o Falog",
    stableDefault: "Canal estável: a versão mais recente publicada no GitHub.",
    noRelease: `Ainda não há uma versão estável; a primeira está a caminho. Enquanto isso, dá para compilar o Falog a partir do código com o guia para <a href="${REPO}/blob/main/docs/install/windows.md">Windows</a>, <a href="${REPO}/blob/main/docs/install/macos.md">macOS</a> ou <a href="${REPO}/blob/main/docs/install/linux.md">Linux</a> (em inglês).`,
    yourSystem: "Seu sistema",
    windows: {
      sub: "x64 · instalador",
      button: "Baixar .exe",
      note: "Instala só para o seu usuário, sem precisar de administrador. Ainda não tem assinatura digital, então, se o SmartScreen disser que protegeu o seu computador, clique em <b>Mais informações › Executar assim mesmo</b>.",
    },
    macos: {
      sub: "Apple silicon e Intel · imagem de disco",
      arm: "Apple silicon",
      intel: "Intel",
      note: "Ainda não é notarizado pela Apple. Se o macOS disser que não consegue verificar o Falog, abra <b>Ajustes do Sistema › Privacidade e Segurança</b> e clique em <b>Abrir Mesmo Assim</b>.",
    },
    linux: {
      sub: "x86_64 · X11 e Wayland",
      copy: "Copiar o comando de instalação",
      button: "Baixar .tar.gz",
      note: "Rode o comando num terminal ou baixe o arquivo. Não precisa configurar mais nada.",
    },
    agents: `O assistente conversa por meio de um agente: Claude Code, Gemini CLI, Codex ou qualquer agente ACP. Instale o que você usa e faça login uma vez; o Falog encontra sozinho. Prefere compilar o Falog você mesmo? Siga os <a href="${REPO}/tree/main/docs/install">guias de instalação</a> (em inglês).`,
    previewBadge: "Prévia",
    previewText:
      "Uma versão antecipada do que vem por aí. É menos estável que a versão acima: pode ter arestas, então guarde uma cópia dos seus dados antes de testar.",
    older: "Procurando uma versão anterior? Veja <a href=\"{releases}\">todas as versões</a>.",
  },

  releases: {
    label: "Histórico de mudanças",
    title: "Versões",
    lede: "Todas as versões do Falog, da mais recente para a mais antiga. As estáveis são as oferecidas na página de download; as prévias antecipam o que vem por aí e são menos estáveis. As notas de cada versão são escritas em inglês.",
    onGithub: "Versões no GitHub ↗",
  },

  footer: {
    license: "Licença MIT",
    source: "Código",
    releases: "Versões",
    issues: "Issues",
    specs: "Especificações",
    credits:
      "Feito com as fontes IBM Plex Sans e Lilex (SIL Open Font License), nas cores One Dark e One Light. Sem cookies e sem rastreadores: a única requisição externa é ao GitHub, para listar as versões.",
  },

  js: {
    downloadFor: "Baixar para {os}",
    alsoFor: "Também para ",
    buildFor: "Compilar para {os}",
    howToInstall: "Como instalar",
    versionOut: "{version} já saiu · estável",
    comingSoon: "Primeira versão em breve",
    stableNone: "Canal estável: ainda sem versão publicada.",
    stable: "Estável",
    latestStable: "Estável mais recente",
    preview: "Prévia",
    releaseNotes: "Notas da versão",
    releaseNotesLink: "notas da versão ↗",
    releasePage: "Página da versão ↗",
    missingAsset: "Ainda não está na {version}; veja a página da versão",
    loading: "Carregando as versões do GitHub…",
    none: "Nenhuma versão publicada ainda. Acompanhe o repositório para saber da primeira.",
    failed: "O GitHub não respondeu agora. As versões estão a um clique, no link abaixo.",
    latestNone: "Ainda sem versão publicada: a primeira está a caminho.",
    latestFailed: "O GitHub não respondeu agora.",
    copied: "Copiado",
    themeToLight: "Mudar para o tema claro",
    themeToDark: "Mudar para o tema escuro",
    arm: "Apple silicon",
    intel: "Intel",
  },

  roadmap: {
    "System tray": "Ícone na bandeja do sistema",
    "Keep Falog running without a window in the taskbar": "Manter o Falog rodando sem uma janela na barra de tarefas",
    "Due date reminders": "Lembretes de prazo",
    "Get a Windows notification before things are late": "Receber uma notificação antes de algo atrasar",
    "App icon": "Ícone do app",
    "A distinctive icon in Explorer, the taskbar and the Start menu":
      "Um ícone próprio no Explorer, na barra de tarefas e no menu Iniciar",
    "Recurring tasks": "Tarefas recorrentes",
    "Weekly reports, monthly invoices, standing chores": "Relatórios semanais, notas fiscais mensais, tarefas de rotina",
    "Quick capture hotkey": "Atalho de captura rápida",
    "File a task from anywhere without the assistant": "Registrar uma tarefa de qualquer lugar, sem o assistente",
    "Assistant panel with voice": "Painel do assistente com voz",
    "Talk to Falog itself instead of a separate Claude window": "Falar com o próprio Falog em vez de uma janela separada",
    "Assistant threads": "Conversas do assistente",
    "Several conversations with their own context": "Várias conversas, cada uma com seu contexto",
    "Live transcription": "Transcrição ao vivo",
    "See the words while dictating": "Ver as palavras enquanto dita",
    "Areas for every part of life": "Áreas para cada parte da vida",
    "Personal tasks and appointments next to work": "Tarefas e compromissos pessoais ao lado do trabalho",
    "Calendar view with Google Calendar": "Calendário com o Google Agenda",
    "Meetings from several Google accounts next to the tasks": "Reuniões de várias contas Google ao lado das tarefas",
    "Assistant agents": "Agentes no assistente",
    "Claude Code, Gemini, Codex or any ACP agent; model, effort, mode, context and slash commands per thread":
      "Claude Code, Gemini, Codex ou qualquer agente ACP; modelo, esforço, modo, contexto e comandos por conversa",
    "macOS and Linux": "macOS e Linux",
    "The same board on the Mac and the Linux machine": "O mesmo quadro no Mac e no Linux",
    "Data folder outside AppData": "Pasta de dados fora do AppData",
    "Claude desktop's sandbox must not hide tasks from Falog":
      "Apps em sandbox não podem esconder tarefas do Falog",
    "Google Calendar through secret iCal links": "Google Agenda por link iCal privado",
    "Connect a calendar by pasting a link instead of creating an OAuth client":
      "Conectar uma agenda colando um link, sem criar um cliente OAuth",
    "Calendars by area, events by voice": "Agendas por área, eventos por voz",
    "Which meetings belong to which job, and booking them through the assistant":
      "Saber de qual trabalho é cada reunião e marcá-las pelo assistente",
    "CI quality gate": "Verificação de qualidade na CI",
    "One required check that says the code is correct before it lands on `main`":
      "Uma verificação obrigatória que confirma que o código está certo antes de entrar na `main`",
    "Public site": "Site público",
    "Three pages, in English and Portuguese: what Falog does, downloads and releases":
      "Três páginas, em inglês e português: o que o Falog faz, downloads e versões",
    "tags/labels": "etiquetas",
    "manual ordering within a column": "ordem manual dentro de uma coluna",
    "weekly report export (Markdown)": "relatório semanal exportado em Markdown",
    "undo for destructive actions": "desfazer ações destrutivas",
    "signed macOS builds and Linux packages (Flatpak, `.deb`)": "builds assinados para macOS e pacotes Linux (Flatpak, `.deb`)",
  },
};

export default pt;
