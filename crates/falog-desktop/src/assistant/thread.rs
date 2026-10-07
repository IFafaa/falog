//! One assistant conversation: what the user sees, plus the agent session behind it.

use super::agent::claude_code;
use super::agent::registry::{Agent, AgentId};
use super::agent::{
    self, AgentEvent, ConfigOption, Environment, OptionKind, Session, SlashCommand, StartOptions,
};
use eframe::egui;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

const SYSTEM_PROMPT: &str = include_str!("../../assets/assistant-prompt.md");
const TOOL_PREFIX: &str = "mcp__falog__";
const TITLE_CHARS: usize = 60;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Item {
    User(String),
    Reply(String),
    Tool(ToolCall),
    Error(String),
    Notice(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    /// Tool name without the MCP prefix (`create_task`).
    pub name: String,
    pub input: Value,
    pub result: Option<ToolOutcome>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolOutcome {
    pub text: String,
    pub is_error: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThreadId(pub u64);

/// Where and how agent sessions are started.
#[derive(Clone, Debug)]
pub struct Launcher {
    pub database: Option<PathBuf>,
}

impl Launcher {
    /// `<data dir>/assistant`: session working directory and thread history.
    pub fn workdir(&self) -> PathBuf {
        agent::default_workdir(self.database.as_deref())
    }

    fn environment(&self) -> Result<Environment, String> {
        let mcp_server = agent::find_mcp_server()
            .ok_or("falog-mcp was not found next to the app. Reinstall Falog to restore it.")?;
        Ok(Environment {
            mcp_server,
            database: self.database.clone(),
            workdir: self.workdir(),
            system_prompt: SYSTEM_PROMPT.to_owned(),
        })
    }

    fn start(
        &self,
        ctx: &egui::Context,
        agent: &Agent,
        options: &StartOptions,
    ) -> Result<Box<dyn Session>, String> {
        agent.start(&self.environment()?, options, ctx)
    }
}

/// Who a thread talks to and what it asks for. `None` leaves the choice to the agent.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Threads saved before agents existed talked to Claude Code.
    #[serde(default)]
    pub agent: AgentId,
    /// A `Choice::value` of the agent's model option.
    #[serde(default)]
    pub model: Option<String>,
    /// A `Choice::value` of the agent's effort option.
    #[serde(default)]
    pub effort: Option<String>,
}

impl Settings {
    fn get(&self, kind: OptionKind) -> Option<&str> {
        match kind {
            OptionKind::Model => self.model.as_deref(),
            OptionKind::Effort => self.effort.as_deref(),
            OptionKind::Other => None,
        }
    }
}

/// What is saved to disk for each thread.
#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadRecord {
    pub id: ThreadId,
    pub created_at: i64,
    pub updated_at: i64,
    /// The agent's session to resume, so the conversation keeps its context after a restart.
    pub session_id: Option<String>,
    pub items: Vec<Item>,
    #[serde(default)]
    pub draft: String,
    #[serde(default)]
    pub settings: Settings,
}

/// What a poll observed.
#[derive(Clone, Copy, Debug, Default)]
pub struct Activity {
    /// A tool finished: tasks may have changed.
    pub tasks_changed: bool,
    /// Something worth saving happened.
    pub changed: bool,
}

#[derive(Debug)]
pub struct Thread {
    pub id: ThreadId,
    pub items: Vec<Item>,
    /// Text of the reply being streamed.
    pub streaming: String,
    pub draft: String,
    pub busy: bool,
    pub model_name: Option<String>,
    /// What the agent accepts as `/name` messages, as last reported.
    pub commands: Vec<SlashCommand>,
    pub settings: Settings,
    /// The agent's display name, kept for when it is gone from the list.
    pub agent_name: String,
    /// The settings the agent offers (model, effort...).
    pub options: Vec<ConfigOption>,
    /// A setting changed during a turn; the session restarts once it ends to apply it.
    restart_pending: bool,
    pub created_at: i64,
    pub updated_at: i64,
    session: Option<Box<dyn Session>>,
    session_id: Option<String>,
}

impl Thread {
    pub fn new(id: ThreadId) -> Self {
        let now = now();
        Self {
            id,
            items: Vec::new(),
            streaming: String::new(),
            draft: String::new(),
            busy: false,
            model_name: None,
            commands: Vec::new(),
            settings: Settings::default(),
            agent_name: "Claude Code".into(),
            options: claude_code::options(),
            restart_pending: false,
            created_at: now,
            updated_at: now,
            session: None,
            session_id: None,
        }
    }

    pub fn from_record(record: ThreadRecord) -> Self {
        Self {
            items: record.items,
            draft: record.draft,
            created_at: record.created_at,
            updated_at: record.updated_at,
            session_id: record.session_id,
            settings: record.settings,
            ..Self::new(record.id)
        }
    }

    pub fn to_record(&self) -> ThreadRecord {
        ThreadRecord {
            id: self.id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            session_id: self.session_id.clone(),
            items: self.items.clone(),
            draft: self.draft.clone(),
            settings: self.settings.clone(),
        }
    }

    /// The first message, shortened; Zed-style threads are named after how they started.
    pub fn title(&self) -> String {
        let first = self.items.iter().find_map(|item| match item {
            Item::User(text) => Some(text.trim()),
            _ => None,
        });
        match first {
            Some(text) if text.chars().count() > TITLE_CHARS => {
                let cut: String = text.chars().take(TITLE_CHARS).collect();
                format!("{}…", cut.trim_end())
            }
            Some(text) => text.to_owned(),
            None => "New thread".to_owned(),
        }
    }

    /// Threads with no messages are not kept in the history.
    pub fn has_messages(&self) -> bool {
        self.items.iter().any(|item| matches!(item, Item::User(_)))
    }

    pub fn message_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| matches!(item, Item::User(_)))
            .count()
    }

    /// Sends a message, starting (or resuming) the agent if needed. Returns whether it was sent.
    pub fn send(
        &mut self,
        ctx: &egui::Context,
        text: String,
        launcher: &Launcher,
        agent: Option<&Agent>,
    ) -> bool {
        let text = text.trim().to_owned();
        if text.is_empty() || self.busy {
            return false;
        }
        self.items.push(Item::User(text.clone()));
        self.touch();
        if self.session.is_none() {
            let options = StartOptions {
                resume: self.session_id.clone(),
                model: self.settings.model.clone(),
                effort: self.settings.effort.clone(),
            };
            let started = match agent {
                Some(agent) => launcher.start(ctx, agent, &options),
                None => Err(format!(
                    "{} is no longer set up. Add it again in Settings, or start a new thread with another agent.",
                    self.agent_name
                )),
            };
            match started {
                Ok(session) => self.session = Some(session),
                Err(message) => {
                    self.items.push(Item::Error(message));
                    return true;
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
                    .push(Item::Error(format!("Could not reach {}: {err}", self.agent_name)));
            }
            None => {}
        }
        true
    }

    /// Applies the session's pending events.
    pub fn poll(&mut self) -> Activity {
        let mut activity = Activity::default();
        while let Some(event) = self.session.as_mut().and_then(|session| session.try_recv()) {
            activity.tasks_changed |= matches!(event, AgentEvent::ToolResult { .. });
            activity.changed |= !matches!(
                event,
                AgentEvent::TextDelta(_) | AgentEvent::Commands(_) | AgentEvent::Options(_)
            );
            self.apply(event);
        }
        activity
    }

    pub(super) fn apply(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::Ready {
                session_id,
                model,
                tools_connected,
            } => {
                self.session_id = Some(session_id);
                self.model_name = model;
                if !tools_connected {
                    self.items.push(Item::Error(
                        "Falog's task tools did not connect, so the assistant cannot change tasks.".into(),
                    ));
                }
            }
            AgentEvent::TextDelta(text) => self.streaming.push_str(&text),
            AgentEvent::Text(text) => {
                self.streaming.clear();
                if !text.trim().is_empty() {
                    self.items.push(Item::Reply(text));
                }
            }
            AgentEvent::ToolUse { id, name, input } => {
                let name = name.strip_prefix(TOOL_PREFIX).unwrap_or(&name).to_owned();
                // ACP agents report a call again once its arguments are known.
                match self.tool_call_mut(&id) {
                    Some(call) => {
                        call.name = name;
                        if !input.is_null() {
                            call.input = input;
                        }
                    }
                    None => self.items.push(Item::Tool(ToolCall {
                        id,
                        name,
                        input,
                        result: None,
                    })),
                }
            }
            AgentEvent::ToolResult { id, text, is_error } => {
                if let Some(call) = self.tool_call_mut(&id) {
                    call.result = Some(ToolOutcome { text, is_error });
                }
            }
            AgentEvent::TurnFinished { is_error, message } => {
                self.busy = false;
                if std::mem::take(&mut self.restart_pending) {
                    self.session = None;
                }
                self.flush_streaming();
                if is_error {
                    let message = message.unwrap_or_else(|| "The assistant stopped with an error.".into());
                    self.items.push(Item::Error(message));
                }
                self.touch();
            }
            AgentEvent::Commands(commands) => self.commands = commands,
            AgentEvent::Options(options) => self.options = options,
            AgentEvent::Notice(text) => self.items.push(Item::Notice(text)),
            AgentEvent::Exited { stderr } => {
                self.session = None;
                if self.busy {
                    self.busy = false;
                    self.flush_streaming();
                    self.items
                        .push(Item::Error(explain_exit(&self.agent_name, &stderr)));
                }
            }
        }
    }

    fn tool_call_mut(&mut self, id: &str) -> Option<&mut ToolCall> {
        self.items.iter_mut().rev().find_map(|item| match item {
            Item::Tool(call) if call.id == id => Some(call),
            _ => None,
        })
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
            if !self.session.as_mut().is_some_and(|session| session.cancel()) {
                self.session = None;
            }
            self.busy = false;
            self.flush_streaming();
            self.items.push(Item::Notice("Stopped.".into()));
        }
    }

    /// The option of `kind` the agent offers, and the value in effect for this thread.
    pub fn option(&self, kind: OptionKind) -> Option<(&ConfigOption, &str)> {
        let option = self.options.iter().find(|o| o.kind == kind)?;
        let value = self
            .settings
            .get(kind)
            .or(option.current.as_deref())
            .or_else(|| option.choices.first().map(|c| c.value.as_str()))?;
        Some((option, value))
    }

    /// Picks a model or effort for the rest of the conversation. Agents that cannot switch live are
    /// restarted (resuming the conversation) before the next message.
    pub fn choose(&mut self, kind: OptionKind, value: String) {
        let Some(id) = self.options.iter().find(|o| o.kind == kind).map(|o| o.id.clone()) else {
            return;
        };
        match kind {
            OptionKind::Model => self.settings.model = Some(value.clone()),
            OptionKind::Effort => self.settings.effort = Some(value.clone()),
            OptionKind::Other => return,
        }
        let applied = self
            .session
            .as_mut()
            .is_some_and(|session| session.set_option(&id, &value));
        if !applied && self.session.is_some() {
            if self.busy {
                self.restart_pending = true;
            } else {
                self.session = None;
            }
        }
    }

    /// Ends the agent process when idle; the next message resumes the conversation.
    pub fn release_session(&mut self) {
        if !self.busy {
            self.session = None;
        }
    }

    fn touch(&mut self) {
        self.updated_at = now();
    }
}

pub fn now() -> i64 {
    chrono::Local::now().timestamp()
}

/// `just now`, `5m ago`, `3h ago`, `yesterday`, `Oct 3`.
pub fn relative_time(timestamp: i64, now: i64) -> String {
    let seconds = (now - timestamp).max(0);
    match seconds {
        0..60 => "just now".into(),
        60..3_600 => format!("{}m ago", seconds / 60),
        3_600..86_400 => format!("{}h ago", seconds / 3_600),
        86_400..172_800 => "yesterday".into(),
        _ => chrono::DateTime::from_timestamp(timestamp, 0)
            .map(|utc| utc.with_timezone(&chrono::Local).format("%b %-d").to_string())
            .unwrap_or_default(),
    }
}

fn explain_exit(agent: &str, stderr: &str) -> String {
    let lower = stderr.to_lowercase();
    if lower.contains("login") || lower.contains("logged in") || lower.contains("authenticat") {
        return format!("{agent} is not signed in. Run it once in a terminal to sign in, then try again.");
    }
    match agent::summarize_stderr(stderr) {
        detail if detail.is_empty() => format!("{agent} exited unexpectedly."),
        detail => format!("{agent} exited unexpectedly: {detail}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_the_conversation_from_events() {
        let mut thread = Thread::new(ThreadId(1));
        thread.busy = true;
        thread.apply(AgentEvent::Ready {
            session_id: "s1".into(),
            model: Some("m".into()),
            tools_connected: true,
        });
        thread.apply(AgentEvent::ToolUse {
            id: "t1".into(),
            name: "mcp__falog__create_task".into(),
            input: json!({}),
        });
        thread.apply(AgentEvent::ToolResult {
            id: "t1".into(),
            text: "Created #7".into(),
            is_error: false,
        });
        thread.apply(AgentEvent::TextDelta("Do".into()));
        thread.apply(AgentEvent::TextDelta("ne".into()));
        assert_eq!(thread.streaming, "Done");
        thread.apply(AgentEvent::Text("Done.".into()));
        thread.apply(AgentEvent::TurnFinished {
            is_error: false,
            message: None,
        });

        assert!(!thread.busy);
        assert_eq!(thread.session_id.as_deref(), Some("s1"));
        assert!(
            matches!(&thread.items[0], Item::Tool(call) if call.name == "create_task"
            && call.result.as_ref().is_some_and(|r| r.text == "Created #7"))
        );
        assert!(matches!(&thread.items[1], Item::Reply(text) if text == "Done."));
    }

    #[test]
    fn reports_missing_tools_and_crashes() {
        let mut thread = Thread::new(ThreadId(1));
        thread.apply(AgentEvent::Ready {
            session_id: "s".into(),
            model: None,
            tools_connected: false,
        });
        assert!(matches!(thread.items.last(), Some(Item::Error(_))));

        thread.busy = true;
        thread.apply(AgentEvent::TextDelta("partial".into()));
        thread.apply(AgentEvent::Exited {
            stderr: "Invalid API key · Please run /login".into(),
        });
        assert!(!thread.busy);
        assert!(matches!(&thread.items[thread.items.len() - 2], Item::Reply(text) if text == "partial"));
        assert!(matches!(thread.items.last(), Some(Item::Error(text)) if text.contains("not signed in")));
    }

    #[test]
    fn is_titled_after_the_first_message() {
        let mut thread = Thread::new(ThreadId(1));
        assert_eq!(thread.title(), "New thread");
        thread.items.push(Item::User("  Review the payments PR  ".into()));
        thread.items.push(Item::User("and another".into()));
        assert_eq!(thread.title(), "Review the payments PR");
        let long = "word ".repeat(30);
        thread.items[0] = Item::User(long);
        assert!(thread.title().ends_with('…'));
        assert!(thread.title().chars().count() <= TITLE_CHARS + 1);
    }

    #[test]
    fn records_round_trip() {
        let mut thread = Thread::new(ThreadId(4));
        thread.session_id = Some("abc".into());
        thread.items.push(Item::User("hi".into()));
        thread.draft = "unsent".into();
        thread.settings.effort = Some("high".into());
        let restored = Thread::from_record(thread.to_record());
        assert_eq!(restored.settings.effort.as_deref(), Some("high"));
        assert_eq!(restored.id, ThreadId(4));
        assert_eq!(restored.session_id.as_deref(), Some("abc"));
        assert_eq!(restored.draft, "unsent");
        assert_eq!(restored.message_count(), 1);
    }

    #[test]
    fn records_without_settings_still_load() {
        let json = r#"{"id":3,"created_at":1,"updated_at":2,"session_id":null,"items":[]}"#;
        let record: ThreadRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.settings, Settings::default());
    }

    #[test]
    fn picks_model_and_effort() {
        let mut thread = Thread::new(ThreadId(1));
        let (_, model) = thread.option(OptionKind::Model).unwrap();
        assert_eq!(model, "default");
        thread.choose(OptionKind::Model, "opus".into());
        thread.choose(OptionKind::Effort, "max".into());
        let (option, model) = thread.option(OptionKind::Model).unwrap();
        assert_eq!(option.label(model), "Opus");
        assert_eq!(thread.settings.effort.as_deref(), Some("max"));
    }

    /// A session that records what it is asked to do.
    #[derive(Debug, Default)]
    struct FakeSession {
        sent: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
        live_options: bool,
    }

    impl Session for FakeSession {
        fn send(&mut self, text: &str) -> std::io::Result<()> {
            self.sent.borrow_mut().push(text.to_owned());
            Ok(())
        }
        fn try_recv(&mut self) -> Option<AgentEvent> {
            None
        }
        fn cancel(&mut self) -> bool {
            self.live_options
        }
        fn set_option(&mut self, id: &str, value: &str) -> bool {
            self.sent.borrow_mut().push(format!("{id}={value}"));
            self.live_options
        }
    }

    #[test]
    fn restarts_agents_that_cannot_switch_live_after_the_turn() {
        let mut thread = Thread::new(ThreadId(1));
        thread.session = Some(Box::new(FakeSession::default()));
        thread.busy = true;
        thread.choose(OptionKind::Model, "haiku".into());
        assert!(thread.session.is_some(), "the running turn is not cut short");
        thread.apply(AgentEvent::TurnFinished {
            is_error: false,
            message: None,
        });
        assert!(
            thread.session.is_none(),
            "restarted with --resume on the next message"
        );

        thread.session = Some(Box::new(FakeSession::default()));
        thread.choose(OptionKind::Effort, "low".into());
        assert!(thread.session.is_none(), "idle sessions restart right away");
    }

    #[test]
    fn switches_live_when_the_agent_can() {
        let sent = std::rc::Rc::default();
        let mut thread = Thread::new(ThreadId(1));
        thread.session = Some(Box::new(FakeSession {
            sent: std::rc::Rc::clone(&sent),
            live_options: true,
        }));
        thread.choose(OptionKind::Model, "opus".into());
        assert!(thread.session.is_some());
        assert_eq!(*sent.borrow(), vec!["model=opus".to_owned()]);
    }

    #[test]
    fn describes_relative_times() {
        let now = 1_000_000;
        assert_eq!(relative_time(now - 5, now), "just now");
        assert_eq!(relative_time(now - 300, now), "5m ago");
        assert_eq!(relative_time(now - 7_200, now), "2h ago");
        assert_eq!(relative_time(now - 90_000, now), "yesterday");
    }
}
