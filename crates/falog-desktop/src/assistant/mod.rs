//! The assistant dock: a conversation with Claude that manages tasks, typed or dictated.
//!
//! [`claude`] drives Claude Code, [`voice`] records and transcribes speech, [`panel`] draws the
//! dock. [`Assistant`] owns the conversation state and ties them together.

pub mod claude;
pub mod voice;

use claude::{ClaudeEvent, ClaudeSession, SessionConfig};
use eframe::egui;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use voice::{Download, Recorder, Transcriber, VoiceLanguage};

const SYSTEM_PROMPT: &str = include_str!("../../assets/assistant-prompt.md");
const TOOL_PREFIX: &str = "mcp__falog__";

/// Which Claude model the assistant asks Claude Code for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssistantModel {
    /// Whatever Claude Code is configured to use.
    #[default]
    Default,
    Haiku,
    Sonnet,
    Opus,
}

impl AssistantModel {
    pub const ALL: [Self; 4] = [Self::Default, Self::Haiku, Self::Sonnet, Self::Opus];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Haiku => "Haiku",
            Self::Sonnet => "Sonnet",
            Self::Opus => "Opus",
        }
    }

    const fn alias(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::Haiku => Some("haiku"),
            Self::Sonnet => Some("sonnet"),
            Self::Opus => Some("opus"),
        }
    }
}

/// User-facing assistant options (stored in the app preferences).
#[derive(Clone, Copy, Debug)]
pub struct AssistantOptions {
    pub model: AssistantModel,
    pub language: VoiceLanguage,
    pub send_after_dictation: bool,
}

#[derive(Debug)]
pub enum Item {
    User(String),
    Reply(String),
    Tool(ToolCall),
    Error(String),
    Notice(String),
}

#[derive(Debug)]
pub struct ToolCall {
    pub id: String,
    /// Tool name without the MCP prefix (`create_task`).
    pub name: String,
    pub input: Value,
    pub result: Option<ToolOutcome>,
}

#[derive(Debug)]
pub struct ToolOutcome {
    pub text: String,
    pub is_error: bool,
}

#[derive(Debug, Default)]
pub enum VoiceState {
    #[default]
    Idle,
    Recording(Recorder),
    Transcribing,
    /// The user asked to dictate but the Whisper model is not downloaded yet.
    NeedsModel,
    Downloading(Download),
}

#[derive(Debug)]
pub struct Assistant {
    pub items: Vec<Item>,
    /// Text of the reply being streamed.
    pub streaming: String,
    pub draft: String,
    pub busy: bool,
    pub model_name: Option<String>,
    pub voice: VoiceState,
    pub focus_composer: bool,
    session: Option<ClaudeSession>,
    session_id: Option<String>,
    database: Option<PathBuf>,
    transcriber: Option<Transcriber>,
}

impl Assistant {
    pub fn new(database: Option<&Path>) -> Self {
        Self {
            items: Vec::new(),
            streaming: String::new(),
            draft: String::new(),
            busy: false,
            model_name: None,
            voice: VoiceState::Idle,
            focus_composer: false,
            session: None,
            session_id: None,
            database: database.map(Path::to_path_buf),
            transcriber: None,
        }
    }

    pub fn is_recording(&self) -> bool {
        matches!(self.voice, VoiceState::Recording(_))
    }

    /// Whether the UI should keep animating (spinners, level meter).
    pub fn is_active(&self) -> bool {
        self.busy || !matches!(self.voice, VoiceState::Idle | VoiceState::NeedsModel)
    }

    pub fn send(&mut self, ctx: &egui::Context, text: String, options: AssistantOptions) {
        let text = text.trim().to_owned();
        if text.is_empty() || self.busy {
            return;
        }
        self.items.push(Item::User(text.clone()));
        if self.session.is_none() {
            match self.start_session(ctx, options) {
                Ok(session) => self.session = Some(session),
                Err(message) => {
                    self.items.push(Item::Error(message));
                    return;
                }
            }
        }
        let sent = self.session.as_mut().map(|session| session.send(&text));
        match sent {
            Some(Ok(())) => {
                self.busy = true;
                self.streaming.clear();
            }
            Some(Err(err)) => {
                self.session = None;
                self.items
                    .push(Item::Error(format!("Could not reach Claude Code: {err}")));
            }
            None => {}
        }
    }

    fn start_session(&self, ctx: &egui::Context, options: AssistantOptions) -> Result<ClaudeSession, String> {
        let claude = claude::find_claude().ok_or(
            "Claude Code was not found. Install it from claude.com/claude-code, then run `claude` once in a \
             terminal to sign in.",
        )?;
        let mcp_server = claude::find_mcp_server()
            .ok_or("falog-mcp was not found next to the app. Reinstall Falog to restore it.")?;
        let config = SessionConfig {
            claude,
            mcp_server,
            database: self.database.clone(),
            model: options.model.alias().map(str::to_owned),
            workdir: claude::default_workdir(self.database.as_deref()),
            system_prompt: SYSTEM_PROMPT.to_owned(),
            resume: self.session_id.clone(),
        };
        ClaudeSession::start(&config, ctx.clone())
            .map_err(|err| format!("Could not start Claude Code: {err}"))
    }

    /// Processes pending events. Returns `true` when a tool finished, so tasks may have changed.
    pub fn poll(&mut self, ctx: &egui::Context, options: AssistantOptions) -> bool {
        let mut tasks_changed = false;
        while let Some(event) = self.session.as_ref().and_then(ClaudeSession::try_recv) {
            tasks_changed |= matches!(event, ClaudeEvent::ToolResult { .. });
            self.apply(event);
        }

        if matches!(self.voice, VoiceState::Transcribing)
            && let Some(result) = self.transcriber.as_ref().and_then(Transcriber::try_recv)
        {
            self.voice = VoiceState::Idle;
            match result {
                Ok(text) if text.trim().is_empty() => {
                    self.items.push(Item::Notice("Didn't catch that.".into()))
                }
                Ok(text) if options.send_after_dictation => self.send(ctx, text, options),
                Ok(text) => {
                    self.draft = text;
                    self.focus_composer = true;
                }
                Err(message) => self.items.push(Item::Error(message)),
            }
        }

        if let VoiceState::Downloading(download) = &self.voice
            && let Some(result) = download.try_finish()
        {
            self.voice = VoiceState::Idle;
            match result {
                Ok(()) => self.items.push(Item::Notice(
                    "Voice model ready. Press Ctrl+Space to talk.".into(),
                )),
                Err(message) => self.items.push(Item::Error(message)),
            }
        }
        tasks_changed
    }

    fn apply(&mut self, event: ClaudeEvent) {
        match event {
            ClaudeEvent::Ready {
                session_id,
                model,
                tools_connected,
            } => {
                self.session_id = Some(session_id);
                self.model_name = Some(model);
                if !tools_connected {
                    self.items.push(Item::Error(
                        "Falog's task tools did not connect, so the assistant cannot change tasks.".into(),
                    ));
                }
            }
            ClaudeEvent::TextDelta(text) => self.streaming.push_str(&text),
            ClaudeEvent::Text(text) => {
                self.streaming.clear();
                if !text.trim().is_empty() {
                    self.items.push(Item::Reply(text));
                }
            }
            ClaudeEvent::ToolUse { id, name, input } => {
                let name = name.strip_prefix(TOOL_PREFIX).unwrap_or(&name).to_owned();
                self.items.push(Item::Tool(ToolCall {
                    id,
                    name,
                    input,
                    result: None,
                }));
            }
            ClaudeEvent::ToolResult { id, text, is_error } => {
                let call = self.items.iter_mut().rev().find_map(|item| match item {
                    Item::Tool(call) if call.id == id => Some(call),
                    _ => None,
                });
                if let Some(call) = call {
                    call.result = Some(ToolOutcome { text, is_error });
                }
            }
            ClaudeEvent::TurnFinished { is_error, message } => {
                self.busy = false;
                self.flush_streaming();
                if is_error {
                    let message = message.unwrap_or_else(|| "The assistant stopped with an error.".into());
                    self.items.push(Item::Error(message));
                }
            }
            ClaudeEvent::Exited { stderr } => {
                self.session = None;
                if self.busy {
                    self.busy = false;
                    self.flush_streaming();
                    self.items.push(Item::Error(explain_exit(&stderr)));
                }
            }
        }
    }

    fn flush_streaming(&mut self) {
        let text = std::mem::take(&mut self.streaming);
        if !text.trim().is_empty() {
            self.items.push(Item::Reply(text));
        }
    }

    /// Interrupts the current turn. The next message resumes the same conversation.
    pub fn stop(&mut self) {
        if self.busy {
            self.session = None;
            self.busy = false;
            self.flush_streaming();
            self.items.push(Item::Notice("Stopped.".into()));
        }
    }

    /// Restarts Claude Code on the next message (e.g. after a model change), keeping the context.
    pub fn restart_session(&mut self) {
        if !self.busy {
            self.session = None;
        }
    }

    pub fn new_thread(&mut self) {
        self.session = None;
        self.session_id = None;
        self.items.clear();
        self.streaming.clear();
        self.busy = false;
        self.focus_composer = true;
    }

    /// Push-to-talk: starts recording, or stops and transcribes.
    pub fn toggle_dictation(&mut self, ctx: &egui::Context, options: AssistantOptions) {
        match std::mem::take(&mut self.voice) {
            VoiceState::Idle | VoiceState::NeedsModel if !voice::model_path().is_file() => {
                self.voice = VoiceState::NeedsModel;
            }
            VoiceState::Idle | VoiceState::NeedsModel => match Recorder::start(ctx.clone()) {
                Ok(recorder) => self.voice = VoiceState::Recording(recorder),
                Err(message) => self.items.push(Item::Error(message)),
            },
            VoiceState::Recording(recorder) => match recorder.finish() {
                Some(audio) => {
                    let transcriber = self
                        .transcriber
                        .get_or_insert_with(|| Transcriber::new(voice::model_path(), ctx.clone()));
                    transcriber.submit(audio, options.language);
                    self.voice = VoiceState::Transcribing;
                }
                None => self.voice = VoiceState::Idle,
            },
            busy @ (VoiceState::Transcribing | VoiceState::Downloading(_)) => self.voice = busy,
        }
    }

    pub fn cancel_dictation(&mut self) {
        if matches!(self.voice, VoiceState::Recording(_) | VoiceState::NeedsModel) {
            self.voice = VoiceState::Idle;
        }
    }

    pub fn download_model(&mut self, ctx: &egui::Context) {
        self.voice = VoiceState::Downloading(Download::start(voice::model_path(), ctx.clone()));
    }
}

fn explain_exit(stderr: &str) -> String {
    let lower = stderr.to_lowercase();
    if lower.contains("login") || lower.contains("logged in") || lower.contains("authenticat") {
        return "Claude Code is not signed in. Run `claude` once in a terminal to sign in, then try again."
            .into();
    }
    match claude::summarize_stderr(stderr) {
        detail if detail.is_empty() => "Claude Code exited unexpectedly.".into(),
        detail => format!("Claude Code exited unexpectedly: {detail}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn assistant() -> Assistant {
        Assistant::new(None)
    }

    #[test]
    fn builds_the_thread_from_events() {
        let mut a = assistant();
        a.busy = true;
        a.apply(ClaudeEvent::Ready {
            session_id: "s1".into(),
            model: "m".into(),
            tools_connected: true,
        });
        a.apply(ClaudeEvent::ToolUse {
            id: "t1".into(),
            name: "mcp__falog__create_task".into(),
            input: json!({}),
        });
        a.apply(ClaudeEvent::ToolResult {
            id: "t1".into(),
            text: "Created #7".into(),
            is_error: false,
        });
        a.apply(ClaudeEvent::TextDelta("Do".into()));
        a.apply(ClaudeEvent::TextDelta("ne".into()));
        assert_eq!(a.streaming, "Done");
        a.apply(ClaudeEvent::Text("Done.".into()));
        a.apply(ClaudeEvent::TurnFinished {
            is_error: false,
            message: None,
        });

        assert!(!a.busy);
        assert_eq!(a.session_id.as_deref(), Some("s1"));
        assert!(
            matches!(&a.items[0], Item::Tool(call) if call.name == "create_task"
            && call.result.as_ref().is_some_and(|r| r.text == "Created #7"))
        );
        assert!(matches!(&a.items[1], Item::Reply(text) if text == "Done."));
    }

    #[test]
    fn reports_missing_tools_and_crashes() {
        let mut a = assistant();
        a.apply(ClaudeEvent::Ready {
            session_id: "s".into(),
            model: "m".into(),
            tools_connected: false,
        });
        assert!(matches!(a.items.last(), Some(Item::Error(_))));

        a.busy = true;
        a.apply(ClaudeEvent::TextDelta("partial".into()));
        a.apply(ClaudeEvent::Exited {
            stderr: "Invalid API key · Please run /login".into(),
        });
        assert!(!a.busy);
        assert!(matches!(&a.items[a.items.len() - 2], Item::Reply(text) if text == "partial"));
        assert!(matches!(a.items.last(), Some(Item::Error(text)) if text.contains("not signed in")));
    }

    #[test]
    fn new_thread_forgets_the_conversation() {
        let mut a = assistant();
        a.session_id = Some("s".into());
        a.items.push(Item::User("hi".into()));
        a.new_thread();
        assert!(a.items.is_empty());
        assert!(a.session_id.is_none());
    }
}
