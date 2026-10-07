//! Agents the assistant talks to.
//!
//! A thread never talks to a program directly: it starts a [`Session`] and reads [`AgentEvent`]s
//! from it. [`claude_code`] drives Claude Code in headless mode. Every session gets the same
//! [`Environment`]: the `falog` MCP server as its only tools, the assistant prompt and an isolated
//! working directory.

pub mod acp;
pub mod claude_code;
pub mod registry;

#[cfg(test)]
mod live_tests;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{ChildStderr, Command};
use std::sync::{Arc, Mutex};
use std::thread;

/// What an agent reports while it works, in the order it happens.
#[derive(Clone, Debug, PartialEq)]
pub enum AgentEvent {
    /// The session is up. `session_id` resumes it later.
    Ready {
        session_id: String,
        model: Option<String>,
        tools_connected: bool,
    },
    /// A piece of the reply being streamed.
    TextDelta(String),
    /// A whole reply block; replaces what was streamed so far.
    Text(String),
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        id: String,
        text: String,
        is_error: bool,
    },
    TurnFinished {
        is_error: bool,
        message: Option<String>,
    },
    /// The commands the agent accepts as `/name` messages (replaces earlier lists).
    Commands(Vec<SlashCommand>),
    /// The settings the agent offers for this session, such as model and effort.
    Options(Vec<ConfigOption>),
    /// Something the user should know that is not an error.
    Notice(String),
    /// How full the context window is, in tokens.
    Usage(Usage),
    /// The process ended; `stderr` may explain why.
    Exited {
        stderr: String,
    },
}

/// Tokens in the conversation's context, and the model's context window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub used: u64,
    pub size: u64,
}

impl Usage {
    /// How full the window is, from 0 to 1.
    pub fn fraction(self) -> f32 {
        if self.size == 0 {
            0.0
        } else {
            (self.used as f64 / self.size as f64).clamp(0.0, 1.0) as f32
        }
    }
}

/// A command the agent runs when sent `/name [input]`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashCommand {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// What to type after the name, if it takes input (`[what to design]`).
    #[serde(default)]
    pub hint: String,
}

/// What a session setting controls; the composer has a picker for models and effort.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionKind {
    Model,
    Effort,
    /// Permission mode (default, accept edits, plan, auto, bypass permissions...).
    Mode,
    /// Fast mode, `on` or `off`.
    Fast,
    Other,
}

impl OptionKind {
    /// The kinds a thread remembers and the composer shows, in footer order.
    pub const PICKED: [Self; 4] = [Self::Model, Self::Effort, Self::Mode, Self::Fast];
}

/// A session setting with a closed set of values (ACP's select config options).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigOption {
    pub id: String,
    pub name: String,
    pub kind: OptionKind,
    /// The value in effect, when the agent says.
    pub current: Option<String>,
    pub choices: Vec<Choice>,
    /// What it does, or why it is unavailable.
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    pub value: String,
    pub name: String,
}

impl Choice {
    fn new(value: &str, name: &str) -> Self {
        Self {
            value: value.into(),
            name: name.into(),
        }
    }
}

impl ConfigOption {
    /// The display name of `value`, or the value itself when it is not one of the choices.
    pub fn label<'a>(&'a self, value: &'a str) -> &'a str {
        self.choices
            .iter()
            .find(|choice| choice.value == value)
            .map_or(value, |choice| choice.name.as_str())
    }
}

/// A running conversation with an agent.
pub trait Session: std::fmt::Debug {
    /// Sends a user message (or a slash command) to the agent.
    fn send(&mut self, text: &str) -> io::Result<()>;
    /// The next pending event, if any. Never blocks.
    fn try_recv(&mut self) -> Option<AgentEvent>;
    /// Stops the current turn. Returns whether the session is still usable afterwards; when it is
    /// not, the caller drops it and resumes the conversation in a new one.
    fn cancel(&mut self) -> bool;
    /// Changes a setting (`ConfigOption::id`) for the rest of the conversation. Returns whether
    /// it applied right away; when it did not, the caller restarts the session to apply it.
    fn set_option(&mut self, _id: &str, _value: &str) -> bool {
        false
    }
}

/// What every agent session gets, whichever the agent.
#[derive(Clone, Debug)]
pub struct Environment {
    pub mcp_server: PathBuf,
    /// Database the MCP server should use (the one the app has open).
    pub database: Option<PathBuf>,
    /// Isolated working directory, so no project instructions leak into the conversation.
    pub workdir: PathBuf,
    pub system_prompt: String,
}

/// How to start one session.
#[derive(Clone, Debug, Default)]
pub struct StartOptions {
    /// Continue an earlier conversation (after a stop, a restart or a model change).
    pub resume: Option<String>,
    /// Model to ask for; `None` uses the agent's default.
    pub model: Option<String>,
    /// Effort (thinking) level to ask for; `None` uses the agent's default.
    pub effort: Option<String>,
    /// Permission mode; `None` keeps Falog's default.
    pub mode: Option<String>,
    /// Fast mode (`on`/`off`); `None` leaves it off.
    pub fast: Option<String>,
}

impl StartOptions {
    /// The value asked for a kind of option.
    pub fn get(&self, kind: OptionKind) -> Option<&str> {
        match kind {
            OptionKind::Model => self.model.as_deref(),
            OptionKind::Effort => self.effort.as_deref(),
            OptionKind::Mode => self.mode.as_deref(),
            OptionKind::Fast => self.fast.as_deref(),
            OptionKind::Other => None,
        }
    }

    /// Asks for a value of a kind of option. Returns false for kinds that are not remembered.
    pub fn set(&mut self, kind: OptionKind, value: String) -> bool {
        let slot = match kind {
            OptionKind::Model => &mut self.model,
            OptionKind::Effort => &mut self.effort,
            OptionKind::Mode => &mut self.mode,
            OptionKind::Fast => &mut self.fast,
            OptionKind::Other => return false,
        };
        *slot = Some(value);
        true
    }
}

/// The `falog-mcp` executable shipped next to the app.
pub fn find_mcp_server() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let candidate = exe
        .parent()?
        .join(format!("falog-mcp{}", std::env::consts::EXE_SUFFIX));
    candidate.is_file().then_some(candidate)
}

/// Default working directory for assistant sessions: `<data dir>/Falog/assistant`.
pub fn default_workdir(database: Option<&Path>) -> PathBuf {
    database
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(std::env::temp_dir)
        .join("assistant")
}

pub(crate) fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

/// Last lines of stderr, enough to explain a failure in the UI.
pub fn summarize_stderr(stderr: &str) -> String {
    let lines: Vec<&str> = stderr.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(3)..].join("\n")
}

/// Keeps agents from flashing a console window on Windows.
fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}

/// Collects a child's stderr in the background, to explain why it exited.
#[derive(Clone, Debug, Default)]
struct StderrLog(Arc<Mutex<String>>);

impl StderrLog {
    fn collect(stderr: ChildStderr) -> Self {
        let log = Self::default();
        let sink = Arc::clone(&log.0);
        thread::spawn(move || {
            let mut text = String::new();
            let _ = BufReader::new(stderr).read_to_string(&mut text);
            if let Ok(mut shared) = sink.lock() {
                *shared = text;
            }
        });
        log
    }

    /// What was written, once the process has had a moment to finish writing it.
    fn text(&self) -> String {
        thread::sleep(std::time::Duration::from_millis(200));
        self.0.lock().map(|s| s.trim().to_owned()).unwrap_or_default()
    }
}

fn missing(what: &str) -> io::Error {
    io::Error::other(format!("no {what}"))
}
