//! Brazilian Portuguese for every interface text, keyed by its English original. Write as a person
//! from Brazil would read it in an app: short, natural, "você" (implied, rarely written).

/// Month names, January first, as written inside a date ("6 de outubro").
pub const MONTHS: [&str; 12] = [
    "janeiro",
    "fevereiro",
    "março",
    "abril",
    "maio",
    "junho",
    "julho",
    "agosto",
    "setembro",
    "outubro",
    "novembro",
    "dezembro",
];

/// Weekday names, Monday first.
pub const WEEKDAYS: [&str; 7] = [
    "segunda-feira",
    "terça-feira",
    "quarta-feira",
    "quinta-feira",
    "sexta-feira",
    "sábado",
    "domingo",
];

/// The Portuguese for `english`, or `None` when there is no entry yet.
pub fn text(english: &str) -> Option<&'static str> {
    Some(match english {
        // ---- app ----
        "Could not load tasks: {err}" => "Não foi possível carregar as tarefas: {err}",
        "All areas" => "Todas as áreas",
        "Saved #{id}" => "#{id} salva",
        "Reloaded" => "Recarregado",
        "Add your Google OAuth client in Settings first" => {
            "Antes, adicione seu cliente OAuth do Google nas Configurações"
        }
        "#{id} moved to {status}" => "#{id} movida para {status}",
        "#{id} moved to {status}; the next one is due {date}" => {
            "#{id} movida para {status}; a próxima vence {date}"
        }
        "Saved #{id}; the next one is due {date}" => "#{id} salva; a próxima vence {date}",
        "No done tasks to archive" => "Nenhuma tarefa concluída para arquivar",
        "Archived #{id}; find it in the Archive (Ctrl+5)" => "#{id} arquivada; ela fica no Arquivo (Ctrl+5)",
        "Archived {n} tasks; find them in the Archive (Ctrl+5)" => {
            "{n} tarefas arquivadas; elas ficam no Arquivo (Ctrl+5)"
        }
        "Restored #{id} to Done" => "#{id} devolvida a Concluída",
        "Save or discard the open task before updating" => {
            "Salve ou descarte a tarefa aberta antes de atualizar"
        }
        "Added {name}" => "{name} adicionada",
        "Saved {name}" => "{name} salva",
        "Falog will open at startup" => "O Falog vai abrir ao ligar o computador",
        "Launch at startup disabled" => "O Falog não vai mais abrir ao ligar o computador",
        "Could not change launch at startup: {err}" => "Não foi possível mudar a abertura ao ligar: {err}",
        "Copied to clipboard" => "Copiado para a área de transferência",
        "Saved the Google client" => "Cliente do Google salvo",
        "Removed {name}" => "Removido: {name}",
        "Deleted #{id}" => "#{id} excluída",
        "Deleted {name}" => "{name} excluída",
        "Falog could not open its database" => "O Falog não conseguiu abrir o banco de dados",
        "Set {variable} to use another location." => "Defina {variable} para usar outro local.",
        // ---- assistant/agent/acp ----
        "The agent did not open a session." => "O agente não abriu uma sessão.",
        "{agent} could not reopen the earlier conversation, so this one starts fresh." => {
            "O {agent} não conseguiu reabrir a conversa anterior, então esta começa do zero."
        }
        "{agent} is not signed in. Run it once in a terminal to sign in, then try again." => {
            "O {agent} não está conectado. Abra-o uma vez num terminal para entrar e tente de novo."
        }
        "{agent} did not accept that setting: {err}" => "O {agent} não aceitou essa configuração: {err}",
        "{agent} failed: {err}" => "O {agent} falhou: {err}",
        "Could not start {agent}: {err}" => "Não foi possível iniciar o {agent}: {err}",
        "The reply was cut off: it reached the agent's length limit." => {
            "A resposta foi cortada: atingiu o limite de tamanho do agente."
        }
        "The agent stopped after too many steps in one turn." => {
            "O agente parou depois de passos demais numa só vez."
        }
        "The agent declined to continue." => "O agente se recusou a continuar.",
        // ---- assistant/agent/claude_code ----
        "it is not available on the free plan" => "não está disponível no plano gratuito",
        "it needs extra usage to be enabled for this account" => "precisa do uso extra ativado nesta conta",
        "it is not available for this model (Opus only)" => "não está disponível para este modelo (só Opus)",
        "it is not available on this API provider" => "não está disponível neste provedor de API",
        "it is disabled by the environment" => "está desativado pelo ambiente",
        "eligibility could not be checked (network error)" => {
            "não foi possível verificar a elegibilidade (erro de rede)"
        }
        "Fast mode is off: {reason}." => "Modo rápido desligado: {reason}.",
        // ---- assistant/agent/registry ----
        "Check the command of {agent} in Settings." => "Confira o comando do {agent} nas Configurações.",
        "{agent} was not found: `{command}` is not installed or not on PATH. {hint}" => {
            "O {agent} não foi encontrado: `{command}` não está instalado ou não está no PATH. {hint}"
        }
        "Install it from claude.com/claude-code, then run `claude` once in a terminal to sign in." => {
            "Instale em claude.com/claude-code e rode `claude` uma vez num terminal para entrar."
        }
        "It needs Node.js 22 or later (nodejs.org) and Claude Code signed in." => {
            "Precisa do Node.js 22 ou mais recente (nodejs.org) e do Claude Code conectado."
        }
        "Install it with `npm install -g @google/gemini-cli`, then run `gemini` once in a terminal to sign in." => {
            "Instale com `npm install -g @google/gemini-cli` e rode `gemini` uma vez num terminal para entrar."
        }
        "It needs Node.js (nodejs.org) and Codex signed in (`codex login`)." => {
            "Precisa do Node.js (nodejs.org) e do Codex conectado (`codex login`)."
        }
        // ---- assistant/agent_settings ----
        "Give the agent a name and the command that starts it in ACP mode." => {
            "Dê um nome ao agente e o comando que o inicia no modo ACP."
        }
        "\"{entry}\" is not a NAME=value pair." => "\"{entry}\" não é um par NOME=valor.",
        "Each thread talks to one agent, on your own subscription. Claude Code is built in; the others speak the Agent Client Protocol (ACP)." => {
            "Cada conversa fala com um agente, na sua própria assinatura. O Claude Code vem embutido; os outros falam o Agent Client Protocol (ACP)."
        }
        "Add agent" => "Adicionar agente",
        "Name" => "Nome",
        "My agent" => "Meu agente",
        "Command" => "Comando",
        "Environment" => "Ambiente",
        "Add" => "Adicionar",
        "Cancel" => "Cancelar",
        "Remove agent" => "Remover agente",
        "Installed" => "Instalado",
        "Not found" => "Não encontrado",
        // ---- assistant ----
        "Voice model ready. Press Ctrl+Space to talk." => {
            "Modelo de voz pronto. Tecle Ctrl+Space para falar."
        }
        "Didn't catch that." => "Não entendi.",
        // ---- assistant/panel ----
        "What's on my plate today?" => "O que tenho para hoje?",
        "Remind me to renew the car insurance by Friday, it's important" => {
            "Me lembre de renovar o seguro do carro até sexta, é importante"
        }
        "I finished the payments PR review" => "Terminei a revisão do PR de pagamentos",
        "History" => "Histórico",
        "Close (Ctrl+Shift+A)" => "Fechar (Ctrl+Shift+A)",
        "Zoom out (Shift+Esc)" => "Reduzir (Shift+Esc)",
        "Zoom in (Shift+Esc)" => "Ampliar (Shift+Esc)",
        "New thread" => "Nova conversa",
        "No threads yet" => "Nenhuma conversa ainda",
        "{agent} · 1 message · {when}" => "{agent} · 1 mensagem · {when}",
        "{agent} · {count} messages · {when}" => "{agent} · {count} mensagens · {when}",
        "Delete thread" => "Excluir conversa",
        "Thinking…" => "Pensando…",
        "Talk to your tasks" => "Converse com suas tarefas",
        "Create, update and review tasks in your own words. Hold Ctrl+Space or click the mic to speak." => {
            "Crie, atualize e revise tarefas com suas palavras. Segure Ctrl+Space ou clique no microfone para falar."
        }
        "Recent" => "Recentes",
        "View all ({total})" => "Ver todas ({total})",
        "Creating task…" => "Criando tarefa…",
        "Created task" => "Tarefa criada",
        "Updating task…" => "Atualizando tarefa…",
        "Updated task" => "Tarefa atualizada",
        "Adding note…" => "Adicionando nota…",
        "Added note" => "Nota adicionada",
        "Deleting task…" => "Excluindo tarefa…",
        "Deleted task" => "Tarefa excluída",
        "Archiving tasks…" => "Arquivando tarefas…",
        "Archived tasks" => "Tarefas arquivadas",
        "Restoring task…" => "Restaurando tarefa…",
        "Restored task" => "Tarefa restaurada",
        "Adding area…" => "Adicionando área…",
        "Added area" => "Área adicionada",
        "Read what needs attention" => "Leu o que precisa de atenção",
        "Searched tasks" => "Buscou tarefas",
        "Read task" => "Leu a tarefa",
        "Listed areas" => "Listou as áreas",
        "Updating area…" => "Atualizando área…",
        "Updated area" => "Área atualizada",
        "Reading calendars…" => "Lendo as agendas…",
        "Read calendars" => "Leu as agendas",
        "Booking event…" => "Marcando evento…",
        "Booked event" => "Evento marcado",
        "Updating event…" => "Atualizando evento…",
        "Updated event" => "Evento atualizado",
        "Deleting event…" => "Excluindo evento…",
        "Deleted event" => "Evento excluído",
        "Working…" => "Trabalhando…",
        "Done|tool" => "Concluído",
        "Listening…" => "Ouvindo…",
        "Finishing the transcript…" => "Finalizando a transcrição…",
        "Downloading the voice model… {size} MB" => "Baixando o modelo de voz… {size} MB",
        "Tell the assistant what changed…" => "Conte ao assistente o que mudou…",
        "Talk (Ctrl+Space)" => "Falar (Ctrl+Space)",
        "Stop" => "Parar",
        "Send (Enter)" => "Enviar (Enter)",
        "Fast mode is on" => "Modo rápido ligado",
        "Fast mode" => "Modo rápido",
        "Fast mode: {description}" => "Modo rápido: {description}",
        "Ultracode is not available here: it has Claude run multi-agent workflows with its built-in tools, and Falog's assistant runs with only its task tools." => {
            "O Ultracode não está disponível aqui: ele faz o Claude rodar fluxos com vários agentes usando as ferramentas embutidas, e o assistente do Falog roda só com as ferramentas de tarefas."
        }
        "Context: {used} of {size} tokens ({percent}%)" => "Contexto: {used} de {size} tokens ({percent}%)",
        "Each thread keeps its agent. Start a new thread (+) to talk to another one." => {
            "Cada conversa mantém seu agente. Comece uma nova conversa (+) para falar com outro."
        }
        "Agent" => "Agente",
        "Default model" => "Modelo padrão",
        "Default effort" => "Esforço padrão",
        "{name} effort" => "Esforço {name}",
        "Default mode" => "Modo padrão",
        "Listening  {time}" => "Ouvindo  {time}",
        "Enter to insert" => "Enter para inserir",
        "Stop and transcribe (Ctrl+Space)" => "Parar e transcrever (Ctrl+Space)",
        "Send" => "Enviar",
        "Done|dictation" => "Pronto",
        "Stop and send (Enter only inserts the text)" => "Parar e enviar (Enter só insere o texto)",
        "Stop and insert the text (Enter)" => "Parar e inserir o texto (Enter)",
        "Voice input runs Whisper on this computer. It needs a one-time download of about {size} MB." => {
            "A entrada de voz roda o Whisper neste computador. Ela precisa de um download único de cerca de {size} MB."
        }
        "Download" => "Baixar",
        "Not now" => "Agora não",
        // ---- assistant/slash ----
        "The agent lists its commands once it starts. Send a message first." => {
            "O agente lista seus comandos quando começa. Envie uma mensagem primeiro."
        }
        "No matching commands" => "Nenhum comando encontrado",
        // ---- assistant/thread ----
        "falog-mcp was not found next to the app. Reinstall Falog to restore it." => {
            "O falog-mcp não foi encontrado junto ao app. Reinstale o Falog para restaurá-lo."
        }
        "{agent} is no longer set up. Add it again in Settings, or start a new thread with another agent." => {
            "O {agent} não está mais configurado. Adicione-o de novo nas Configurações ou comece uma nova conversa com outro agente."
        }
        "Could not reach {agent}: {err}" => "Não foi possível falar com o {agent}: {err}",
        "Falog's task tools did not connect, so the assistant cannot change tasks." => {
            "As ferramentas de tarefas do Falog não conectaram, então o assistente não pode alterar tarefas."
        }
        "The assistant stopped with an error." => "O assistente parou com um erro.",
        "Interrupted" => "Interrompido",
        "Stopped." => "Parado.",
        "just now" => "agora mesmo",
        "{n}m ago" => "há {n} min",
        "{n}h ago" => "há {n} h",
        "yesterday" => "ontem",
        "{agent} exited unexpectedly." => "O {agent} fechou inesperadamente.",
        "{agent} exited unexpectedly: {detail}" => "O {agent} fechou inesperadamente: {detail}",
        // ---- assistant/voice ----
        "Auto (slower)" => "Automático (mais lento)",
        "No microphone was found." => "Nenhum microfone foi encontrado.",
        "Microphone unavailable: {err}" => "Microfone indisponível: {err}",
        "Unsupported microphone format {format}." => "Formato de microfone não suportado: {format}.",
        "Could not open the microphone: {err}" => "Não foi possível abrir o microfone: {err}",
        "Could not start the microphone: {err}" => "Não foi possível iniciar o microfone: {err}",
        "Could not load the Whisper model: {err}" => "Não foi possível carregar o modelo do Whisper: {err}",
        "Whisper failed to start: {err}" => "O Whisper não conseguiu iniciar: {err}",
        "Transcription failed: {err}" => "A transcrição falhou: {err}",
        "This build of Falog has no speech recognition (compiled without the `whisper` feature)." => {
            "Esta versão do Falog não tem reconhecimento de voz (compilada sem o recurso `whisper`)."
        }
        "This build of Falog has no speech recognition." => {
            "Esta versão do Falog não tem reconhecimento de voz."
        }
        // ---- calendar ----
        "Week" => "Semana",
        "Month" => "Mês",
        "This calendar is already linked" => "Esta agenda já está vinculada",
        // ---- components/modal ----
        "Close (Esc)" => "Fechar (Esc)",
        // ---- components/picker ----
        "No area" => "Sem área",
        // ---- download ----
        "Download failed: {err}" => "O download falhou: {err}",
        "the file was damaged on the way; try again" => "o arquivo chegou danificado; tente de novo",
        // ---- i18n/date ----
        "{month} {day}" => "{day} de {month}",
        "{month} {day}, {year}" => "{day} de {month} de {year}",
        "{weekday}, {month} {day}" => "{weekday}, {day} de {month}",
        "{month} {year}" => "{month} de {year}",
        "{weekday}, {month} {day}, {year}" => "{weekday}, {day} de {month} de {year}",
        "{days}d overdue" => "{days}d de atraso",
        "today" => "hoje",
        "tomorrow" => "amanhã",
        // ---- i18n ----
        "Delete" => "Excluir",
        "Archive|verb" => "Arquivar",
        "Archive" => "Arquivo",
        "To do" => "A fazer",
        "In progress" => "Em andamento",
        "Waiting" => "Aguardando",
        "Done" => "Concluída",
        "Low" => "Baixa",
        "Medium" => "Média",
        "High" => "Alta",
        "Urgent" => "Urgente",
        "Archived {n} tasks" => "{n} tarefas arquivadas",
        // ---- overlays/areas ----
        "Areas" => "Áreas",
        "The parts of your life you organize: work, personal, health, home... The color marks their tasks everywhere." => {
            "As partes da sua vida que você organiza: trabalho, pessoal, saúde, casa... A cor marca as tarefas delas em todo lugar."
        }
        "Save" => "Salvar",
        "Delete area" => "Excluir área",
        "New area" => "Nova área",
        // ---- overlays/command_palette ----
        "Execute a command…" => "Execute um comando…",
        "Search tasks by title, #id, area or requester…" => {
            "Busque tarefas por título, #id, área ou solicitante…"
        }
        "No matches" => "Nada encontrado",
        "task: new" => "tarefa: nova",
        "file finder: find task" => "busca: encontrar tarefa",
        "view: {view}" => "visualização: {view}",
        "task: archive done tasks" => "tarefa: arquivar concluídas",
        "workspace: toggle sidebar" => "área de trabalho: mostrar ou ocultar barra lateral",
        "assistant: toggle panel" => "assistente: mostrar ou ocultar painel",
        "assistant: toggle zoom" => "assistente: ampliar ou reduzir",
        "assistant: dictate" => "assistente: ditar",
        "assistant: new thread" => "assistente: nova conversa",
        "assistant: show history" => "assistente: mostrar histórico",
        "calendar: add calendar link" => "calendário: adicionar link de agenda",
        "calendar: connect google account" => "calendário: conectar conta do Google",
        "calendar: refresh" => "calendário: atualizar",
        "workspace: focus search" => "área de trabalho: ir para a busca",
        "areas: show all" => "áreas: mostrar todas",
        "areas: filter by {name}" => "áreas: filtrar por {name}",
        "areas: manage" => "áreas: gerenciar",
        "theme: {theme}" => "tema: {theme}",
        "settings: open" => "configurações: abrir",
        "data: reload" => "dados: recarregar",
        // ---- overlays/confirm ----
        "Delete task #{id}?" => "Excluir a tarefa #{id}?",
        "\"{title}\" and its activity log will be permanently deleted." => {
            "\"{title}\" e sua atividade serão excluídos para sempre."
        }
        "Delete \"{name}\"?" => "Excluir \"{name}\"?",
        "Its tasks are kept, without an area." => "Suas tarefas são mantidas, sem área.",
        "Discard unsaved changes?" => "Descartar as alterações não salvas?",
        "The changes in the task panel will be lost." => "As alterações no painel da tarefa serão perdidas.",
        "Discard" => "Descartar",
        // ---- overlays/settings/calendar ----
        "See your Google calendars next to your tasks. The assistant can also book meetings in signed-in accounts." => {
            "Veja suas agendas do Google ao lado das tarefas. O assistente também pode marcar reuniões nas contas conectadas."
        }
        "Paste a calendar link" => "Colar o link de uma agenda",
        "Recommended. Two minutes, no setup: copy a link from Google Calendar." => {
            "Recomendado. Dois minutos, sem configurar nada: copie um link do Google Calendar."
        }
        "Sign in with Google" => "Entrar com o Google",
        "Advanced. The assistant can book meetings; needs your own OAuth client." => {
            "Avançado. O assistente pode marcar reuniões; exige um cliente OAuth seu."
        }
        "Connected calendars" => "Agendas conectadas",
        "Calendar link" => "Link de agenda",
        "Google account" => "Conta do Google",
        "1. Open" => "1. Abra as configurações do",
        "settings (the gear, then Settings)." => "(a engrenagem, depois Configurações).",
        "2. On the left, under \"Settings for my calendars\", click the calendar." => {
            "2. À esquerda, em \"Configurações das minhas agendas\", clique na agenda."
        }
        "3. In \"Integrate calendar\", copy the \"Secret address in iCal format\"." => {
            "3. Em \"Integrar agenda\", copie o \"Endereço secreto no formato iCal\"."
        }
        "4. Paste it here and click Add. Do the same for each account (work, personal…)." => {
            "4. Cole aqui e clique em Adicionar. Faça o mesmo para cada conta (trabalho, pessoal…)."
        }
        "Work, Personal… (optional)" => "Trabalho, Pessoal… (opcional)",
        "Secret address" => "Endereço secreto",
        "Checking the link…" => "Verificando o link…",
        "The address works like a password: whoever has it can read the calendar. Falog keeps it on this computer only. Falog reads the calendars again every five minutes." => {
            "O endereço funciona como uma senha: quem o tiver pode ler a agenda. O Falog o guarda só neste computador e lê as agendas de novo a cada cinco minutos."
        }
        "Remove" => "Remover",
        "For accounts whose admin turned off secret addresses. Events show up right away, but you need your own Google OAuth client:" => {
            "Para contas cujo administrador desativou os endereços secretos. Os eventos aparecem na hora, mas você precisa do seu próprio cliente OAuth do Google:"
        }
        "1. In Google Cloud, create a project and enable the" => {
            "1. No Google Cloud, crie um projeto e ative a"
        }
        "2. In the" => "2. Na",
        "OAuth consent screen" => "tela de consentimento OAuth",
        "pick External, then publish the app (testing logins expire every 7 days)." => {
            "escolha Externo e publique o app (logins de teste expiram a cada 7 dias)."
        }
        "3. In" => "3. Em",
        "Credentials" => "Credenciais",
        "create an OAuth client ID of type Desktop app and paste it here." => {
            "crie um ID do cliente OAuth do tipo App para computador e cole aqui."
        }
        "Client ID" => "ID do cliente",
        "Client secret" => "Chave secreta",
        "Save client" => "Salvar cliente",
        "Finish signing in in your browser…" => "Termine de entrar no navegador…",
        "Connect Google account" => "Conectar conta do Google",
        "needs to sign in again" => "precisa entrar de novo",
        "Sign in" => "Entrar",
        "Area of each account" => "Área de cada conta",
        "Pick the area each email belongs to. An area can have several emails." => {
            "Escolha a área de cada e-mail. Uma área pode ter vários e-mails."
        }
        "read only" => "somente leitura",
        "Connected before Falog could create events. Reconnect it (Sign in with Google, below) to let the assistant book meetings here." => {
            "Conectada antes de o Falog poder criar eventos. Reconecte (Entrar com o Google, abaixo) para o assistente marcar reuniões aqui."
        }
        "Reconnect" => "Reconectar",
        // ---- overlays/settings ----
        "Settings" => "Configurações",
        "Language" => "Idioma",
        "The language of Falog's menus and messages." => "O idioma dos menus e mensagens do Falog.",
        "Launch at startup" => "Abrir ao ligar o computador",
        "Open Falog when you sign in to this computer." => "Abre o Falog quando você entra neste computador.",
        "Older completed tasks" => "Tarefas concluídas antigas",
        "Keep showing tasks done more than a week ago on the board." => {
            "Continua mostrando no quadro as tarefas concluídas há mais de uma semana."
        }
        "The assistant dock talks to these agents; pick one per thread in the composer." => {
            "O painel do assistente conversa com estes agentes; escolha um por conversa na caixa de mensagem."
        }
        "Agents" => "Agentes",
        "Database" => "Banco de dados",
        "in memory" => "na memória",
        "Other assistants (MCP)" => "Outros assistentes (MCP)",
        "Use Falog from Claude Code or any MCP client. Register the server once:" => {
            "Use o Falog pelo Claude Code ou qualquer cliente MCP. Registre o servidor uma vez:"
        }
        "Theme" => "Tema",
        "One Dark or One Light." => "One Dark ou One Light.",
        "Updates" => "Atualizações",
        "{current}. This build updates the way it was built, not from here." => {
            "{current}. Esta versão se atualiza do jeito como foi instalada, não por aqui."
        }
        "Check for updates" => "Procurar atualizações",
        "Checking for updates…" => "Procurando atualizações…",
        "{current} is the latest version." => "{current} é a versão mais recente.",
        "Check again" => "Procurar de novo",
        "Falog {version} is available. You have {installed}." => {
            "O Falog {version} está disponível. Você tem a {installed}."
        }
        "Downloading Falog {version}…" => "Baixando o Falog {version}…",
        "Falog {version} is ready to install." => "O Falog {version} está pronto para instalar.",
        "Restart to update" => "Reiniciar para atualizar",
        "Try again" => "Tentar de novo",
        "What's new" => "Novidades",
        "Check for updates automatically" => "Procurar atualizações automaticamente",
        "Look for a new version now and then, and show it in the status bar." => {
            "Procura uma nova versão de tempos em tempos e mostra na barra de status."
        }
        "Preview versions" => "Versões de prévia",
        "Also offer early versions of what comes next. They are less stable." => {
            "Oferece também versões antecipadas do que vem por aí. Elas são menos estáveis."
        }
        "Voice language" => "Idioma da voz",
        "Language you dictate in." => "O idioma em que você dita.",
        "Send after dictation" => "Enviar depois de ditar",
        "Send what you said right away instead of leaving it to edit." => {
            "Envia o que você disse na hora, em vez de deixar para editar."
        }
        "Speech recognition on GPU" => "Reconhecimento de voz na GPU",
        "Much faster when the graphics card is free; turn off while gaming." => {
            "Bem mais rápido quando a placa de vídeo está livre; desligue enquanto joga."
        }
        "Speech recognition: Whisper large-v3-turbo, on this computer." => {
            "Reconhecimento de voz: Whisper large-v3-turbo, neste computador."
        }
        "Speech recognition runs on this computer; its model ({size} MB) downloads the first time you dictate." => {
            "O reconhecimento de voz roda neste computador; o modelo ({size} MB) é baixado na primeira vez que você ditar."
        }
        "General" => "Geral",
        "Appearance" => "Aparência",
        "Assistant" => "Assistente",
        "Voice" => "Voz",
        "Calendar" => "Calendário",
        "Data" => "Dados",
        "Copy" => "Copiar",
        // ---- platform/reveal ----
        "Show in Explorer" => "Mostrar no Explorador de Arquivos",
        "Show in Finder" => "Mostrar no Finder",
        "Show in Files" => "Mostrar em Arquivos",
        // ---- theme ----
        "System" => "Sistema",
        "Dark" => "Escuro",
        "Light" => "Claro",
        // ---- update ----
        "Could not check for updates: {err}" => "Não foi possível procurar atualizações: {err}",
        "Could not download the update: {err}" => "Não foi possível baixar a atualização: {err}",
        "The release has no checksum for {name}" => "A versão não tem checksum para {name}",
        "No update is ready to install" => "Nenhuma atualização está pronta para instalar",
        "Could not install the update: {err}" => "Não foi possível instalar a atualização: {err}",
        // ---- views/archive ----
        "No matching archived tasks" => "Nenhuma tarefa arquivada encontrada",
        "Clear the search or pick another area in the sidebar." => {
            "Limpe a busca ou escolha outra área na barra lateral."
        }
        "Nothing archived yet" => "Nada arquivado ainda",
        "Archive finished tasks from the board's Done column, or right-click a done task." => {
            "Arquive tarefas finalizadas pela coluna Concluída do quadro ou clique com o botão direito numa tarefa concluída."
        }
        "1 archived task · restore one to put it back in Done" => {
            "1 tarefa arquivada · restaure para devolvê-la a Concluída"
        }
        "{count} archived tasks · restore one to put it back in Done" => {
            "{count} tarefas arquivadas · restaure uma para devolvê-la a Concluída"
        }
        "Restore to the board" => "Devolver ao quadro",
        // ---- i18n ----
        "Every day" => "Todo dia",
        "Every weekday" => "Dias úteis",
        "Weekly on {days}" => "Semanal: {days}",
        "Monthly on day {day}" => "Mensal, dia {day}",
        // ---- views/board ----
        "Completing it creates the next one" => "Ao concluir, a próxima é criada",
        "No tasks" => "Nenhuma tarefa",
        "1 completed more than a week ago" => "1 concluída há mais de uma semana",
        "{n} completed more than a week ago" => "{n} concluídas há mais de uma semana",
        "New task in {status}" => "Nova tarefa em {status}",
        "Archive the done task" => "Arquivar a tarefa concluída",
        "Archive the {n} done tasks" => "Arquivar as {n} tarefas concluídas",
        "1 note in the activity log" => "1 nota na atividade",
        "{n} notes in the activity log" => "{n} notas na atividade",
        "Open" => "Abrir",
        "Move to" => "Mover para",
        "Delete…" => "Excluir…",
        // ---- views/calendar ----
        "Today" => "Hoje",
        "Go to today (T)" => "Ir para hoje (T)",
        "Previous (K)" => "Anterior (K)",
        "Next (J)" => "Próximo (J)",
        "Refresh" => "Atualizar",
        "{month} {from} – {to}, {year}" => "{from} – {to} de {month} de {year}",
        "{from} – {to}, {year}" => "{from} – {to} de {year}",
        "Updated {time}" => "Atualizado às {time}",
        "See your Google Calendar meetings here: connect a calendar with its link or by signing in with Google." => {
            "Veja aqui suas reuniões do Google Calendar: conecte uma agenda pelo link dela ou entrando com o Google."
        }
        "Connect a calendar" => "Conectar uma agenda",
        "{date}, all day" => "{date}, o dia todo",
        "all-day" => "dia todo",
        "Open this week" => "Abrir esta semana",
        "+{n} more" => "+{n} mais",
        "Join" => "Participar",
        "Open in Google Calendar" => "Abrir no Google Calendar",
        // ---- views/focus ----
        "{open} open · {overdue} overdue · {today} due today · {week} more this week · {completed} completed in the last 7 days" => {
            "{open} em aberto · {overdue} em atraso · {today} para hoje · mais {week} nesta semana · concluídas nos últimos 7 dias: {completed}"
        }
        "All clear. Nothing open right now." => "Tudo em dia. Nada em aberto agora.",
        "Overdue" => "Atrasadas",
        "Due today" => "Para hoje",
        "Due this week" => "Para esta semana",
        "Waiting on others" => "Aguardando outras pessoas",
        "Upcoming" => "Próximas",
        "Backlog" => "Sem prazo",
        "Show only this area" => "Mostrar só esta área",
        "Completed in the last 7 days" => "Concluídas nos últimos 7 dias",
        // ---- views/list ----
        "Task" => "Tarefa",
        "Area" => "Área",
        "Priority" => "Prioridade",
        "Due" => "Prazo",
        "Requested by" => "Solicitado por",
        "Mark as done" => "Marcar como concluída",
        "Reopen" => "Reabrir",
        "No matching tasks" => "Nenhuma tarefa encontrada",
        "No tasks yet" => "Nenhuma tarefa ainda",
        "Tell your assistant about a request, or press Ctrl+N." => {
            "Conte um pedido ao assistente ou tecle Ctrl+N."
        }
        // ---- views ----
        "Board" => "Quadro",
        "List" => "Lista",
        "Focus" => "Foco",
        "Urgency" => "Urgência",
        "Due date" => "Prazo",
        "Newest" => "Mais recentes",
        // ---- workspace/sidebar ----
        "Manage areas…" => "Gerenciar áreas…",
        "Add area" => "Adicionar área",
        "No areas yet." => "Nenhuma área ainda.",
        "Calendars" => "Agendas",
        "Add a calendar" => "Adicionar uma agenda",
        "No calendars yet." => "Nenhuma agenda ainda.",
        "Google asked this account to sign in again. Click to reconnect." => {
            "O Google pediu para esta conta entrar de novo. Clique para reconectar."
        }
        // ---- workspace/status_bar ----
        "Toggle sidebar (Ctrl+B)" => "Mostrar ou ocultar a barra lateral (Ctrl+B)",
        "Assistant (Ctrl+Shift+A)" => "Assistente (Ctrl+Shift+A)",
        "Update to {version}" => "Atualizar para {version}",
        "Download the new version of Falog" => "Baixar a nova versão do Falog",
        "Downloading update {percent}%" => "Baixando atualização {percent}%",
        "Downloading update…" => "Baixando atualização…",
        "Install Falog {version} and start it again" => "Instalar o Falog {version} e abri-lo de novo",
        "Listening" => "Ouvindo",
        "Settings (Ctrl+,)" => "Configurações (Ctrl+,)",
        "Command palette (Ctrl+Shift+P)" => "Paleta de comandos (Ctrl+Shift+P)",
        // ---- workspace/tab_bar ----
        "New task (Ctrl+N)" => "Nova tarefa (Ctrl+N)",
        "Find task (Ctrl+P)" => "Buscar tarefa (Ctrl+P)",
        // ---- workspace/task_panel ----
        "Repeats" => "Repete",
        "Never" => "Nunca",
        "Every week" => "Toda semana",
        "Every month" => "Todo mês",
        "On day" => "No dia",
        "The title is required." => "O título é obrigatório.",
        "The due date is not a date." => "O prazo não é uma data.",
        "Created {at}" => "Criada em {at}",
        "updated {at}" => "atualizada em {at}",
        "completed {at}" => "concluída em {at}",
        "archived {at}" => "arquivada em {at}",
        "New task" => "Nova tarefa",
        "Task #{id}" => "Tarefa #{id}",
        "Task title" => "Título da tarefa",
        "Status" => "Status",
        "Who asked for it" => "Quem pediu",
        "Description" => "Descrição",
        "Context, links, acceptance criteria…" => "Contexto, links, critérios de aceite…",
        "Restore" => "Restaurar",
        "Put it back in the board's Done column" => "Devolver à coluna Concluída do quadro",
        "Put it away from the board, in the Archive (Ctrl+5)" => {
            "Tirar do quadro e guardar no Arquivo (Ctrl+5)"
        }
        "friday, 15/10, next week…" => "sexta, 15/10, semana que vem…",
        "not a date" => "não é uma data",
        "Tomorrow" => "Amanhã",
        "Friday" => "Sexta",
        "Next week" => "Semana que vem",
        "Clear" => "Limpar",
        "Activity ({count})" => "Atividade ({count})",
        "Delete note" => "Excluir nota",
        "Add a note and press Enter" => "Adicione uma nota e tecle Enter",
        // ---- workspace/toolbar ----
        "Show tasks completed more than a week ago" => "Mostrar tarefas concluídas há mais de uma semana",
        "Show completed tasks" => "Mostrar tarefas concluídas",
        "Sort: {order}" => "Ordem: {order}",
        "Search tasks…  Ctrl+F" => "Buscar tarefas…  Ctrl+F",
        "Clear search" => "Limpar busca",
        _ => return None,
    })
}
