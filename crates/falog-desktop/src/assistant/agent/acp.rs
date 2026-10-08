//! A client for the [Agent Client Protocol](https://agentclientprotocol.com) (ACP, version 1).
//!
//! ACP is newline-delimited JSON-RPC 2.0 over the agent's stdin/stdout, the protocol editors use for
//! external agents (Claude through its adapter, Gemini CLI, Codex...). Falog is the client: it
//! opens one session per thread with the `falog` MCP server as the only tools, sends prompts and
//! turns the agent's `session/update` notifications into [`AgentEvent`]s. It advertises no file
//! system or terminal capabilities and only allows the agent to run falog tools.
//!
//! A reader thread owns the protocol: it runs the handshake (`initialize`, then `session/resume`,
//! `session/load` or `session/new`), answers the agent's requests and matches responses to the
//! requests that are waiting for them. Messages sent before the session is open are queued.

use super::{
    AgentEvent, Choice, ConfigOption, Environment, OptionKind, Session, SlashCommand, StartOptions,
    StderrLog, Usage, hide_console, missing,
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

/// The protocol version Falog speaks.
const PROTOCOL_VERSION: u64 = 1;

/// The tools of the `falog` MCP server; the only ones an agent may run.
const FALOG_TOOLS: [&str; 16] = [
    "create_task",
    "update_task",
    "add_note",
    "delete_task",
    "create_area",
    "get_focus",
    "list_tasks",
    "get_task",
    "list_areas",
    "update_area",
    "list_events",
    "create_event",
    "update_event",
    "delete_event",
    "archive_tasks",
    "restore_task",
];

/// The id under which an agent's older `modes` (switched with `session/set_mode`) are shown.
const LEGACY_MODE: &str = "__mode";

/// JSON-RPC error codes from the ACP schema.
const METHOD_NOT_FOUND: i64 = -32601;
const AUTH_REQUIRED: i64 = -32000;

/// How to run an ACP agent.
#[derive(Clone, Debug)]
pub struct AcpCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// What the session needs to open, independent of how the agent is reached.
#[derive(Clone, Debug)]
pub struct Setup {
    /// The agent's display name, for error messages.
    pub agent: String,
    pub cwd: PathBuf,
    pub mcp_server: PathBuf,
    pub database: Option<PathBuf>,
    pub system_prompt: String,
    pub options: StartOptions,
}

impl Setup {
    pub fn new(agent: &str, env: &Environment, options: &StartOptions) -> Self {
        Self {
            agent: agent.to_owned(),
            cwd: env.workdir.clone(),
            mcp_server: env.mcp_server.clone(),
            database: env.database.clone(),
            system_prompt: env.system_prompt.clone(),
            options: options.clone(),
        }
    }
}

/// `None` once closed: the agent sees the end of its input and exits.
type Writer = Arc<Mutex<Option<Box<dyn Write + Send>>>>;

#[derive(Debug)]
pub struct AcpSession {
    child: Option<Child>,
    connection: Connection,
    events: Receiver<AgentEvent>,
}

impl AcpSession {
    /// Starts the agent process and the handshake. Messages can be sent right away.
    pub fn start(
        command: &AcpCommand,
        env: &Environment,
        options: &StartOptions,
        agent: &str,
        notify: impl Fn() + Send + 'static,
    ) -> io::Result<Self> {
        std::fs::create_dir_all(&env.workdir)?;
        let mut process = Command::new(&command.program);
        process
            .args(&command.args)
            .envs(command.env.iter().map(|(k, v)| (k, v)))
            .current_dir(&env.workdir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_console(&mut process);
        let mut child = process.spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| missing("stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| missing("stdout"))?;
        let stderr = StderrLog::collect(child.stderr.take().ok_or_else(|| missing("stderr"))?);
        let mut session = Self::connect(
            BufReader::new(stdout),
            stdin,
            Setup::new(agent, env, options),
            move || stderr.text(),
            notify,
        );
        session.child = Some(child);
        Ok(session)
    }

    /// Speaks ACP over any pair of streams. `exit_reason` explains why the agent went away once its
    /// output ends; `notify` is called after each batch of events (to repaint).
    pub fn connect(
        reader: impl BufRead + Send + 'static,
        writer: impl Write + Send + 'static,
        setup: Setup,
        exit_reason: impl FnOnce() -> String + Send + 'static,
        notify: impl Fn() + Send + 'static,
    ) -> Self {
        let (sender, events) = mpsc::channel();
        let connection = Connection {
            writer: Arc::new(Mutex::new(Some(Box::new(writer)))),
            state: Arc::new(Mutex::new(State::new(setup))),
        };
        let reader_side = connection.clone();
        thread::spawn(move || {
            let _ = reader_side.request(Request::Initialize, "initialize", initialize_params());
            for line in reader.lines().map_while(Result::ok) {
                for event in reader_side.handle_line(&line) {
                    if sender.send(event).is_err() {
                        return;
                    }
                }
                notify();
            }
            let _ = sender.send(AgentEvent::Exited {
                stderr: exit_reason(),
            });
            notify();
        });
        Self {
            child: None,
            connection,
            events,
        }
    }
}

impl Session for AcpSession {
    fn send(&mut self, text: &str) -> io::Result<()> {
        let mut state = self.connection.state();
        if state.session_id.is_none() {
            state.queued.push(text.to_owned());
            return Ok(());
        }
        drop(state);
        self.connection.prompt(text)
    }

    fn try_recv(&mut self) -> Option<AgentEvent> {
        self.events.try_recv().ok()
    }

    /// Sends `session/cancel` and drops what the agent still streams for that turn. A session that
    /// is still opening cannot be cancelled; the caller starts over.
    fn cancel(&mut self) -> bool {
        let mut state = self.connection.state();
        let Some(session_id) = state.session_id.clone() else {
            return false;
        };
        state.cancelling = true;
        drop(state);
        self.connection
            .notify("session/cancel", json!({ "sessionId": session_id }))
            .is_ok()
    }

    fn set_option(&mut self, id: &str, value: &str) -> bool {
        let mut state = self.connection.state();
        let Some(session_id) = state.session_id.clone() else {
            // Still opening: applied as soon as the session is up.
            let kind = state.options.iter().find(|o| o.id == id).map(|o| o.kind);
            return kind.is_some_and(|kind| state.setup.options.set(kind, value.to_owned()));
        };
        drop(state);
        self.connection.set_option(&session_id, id, value).is_ok()
    }
}

impl Drop for AcpSession {
    /// Closes the agent's input first: on Windows `npx` agents run under `cmd.exe`, and killing
    /// it would leave the Node process behind, waiting for input that never comes.
    fn drop(&mut self) {
        self.connection.close();
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// What a request in flight was for, to know what to do with its response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Request {
    Initialize,
    NewSession,
    ResumeSession,
    LoadSession,
    Prompt,
    SetOption,
    SetMode,
}

#[derive(Debug)]
struct State {
    setup: Setup,
    next_id: u64,
    pending: HashMap<u64, Request>,
    session_id: Option<String>,
    /// Prompts sent before the session was open.
    queued: Vec<String>,
    can_resume: bool,
    can_load: bool,
    /// Whether the agent reads `_meta.systemPrompt` (Claude's adapter does); otherwise the
    /// instructions go in front of the first prompt of a new session.
    reads_system_prompt: bool,
    needs_instructions: bool,
    /// `session/load` replays the conversation as updates; the thread already shows it.
    replaying: bool,
    /// The turn was cancelled; whatever still streams for it is dropped.
    cancelling: bool,
    /// The latest `session/prompt`; answers to earlier (cancelled) ones are stale.
    current_prompt: Option<u64>,
    options: Vec<ConfigOption>,
    /// The mode asked for with `session/set_mode`, applied when the agent agrees.
    pending_mode: Option<String>,
    /// Tool names by call id, for permission requests that only carry the id.
    tools: HashMap<String, String>,
}

impl State {
    fn new(setup: Setup) -> Self {
        Self {
            setup,
            next_id: 1,
            pending: HashMap::new(),
            session_id: None,
            queued: Vec::new(),
            can_resume: false,
            can_load: false,
            reads_system_prompt: false,
            needs_instructions: false,
            replaying: false,
            cancelling: false,
            current_prompt: None,
            options: Vec::new(),
            pending_mode: None,
            tools: HashMap::new(),
        }
    }
}

/// The writing half plus the protocol state, shared by the session and its reader thread.
#[derive(Clone)]
struct Connection {
    writer: Writer,
    state: Arc<Mutex<State>>,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection").finish_non_exhaustive()
    }
}

impl Connection {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn write(&self, message: &Value) -> io::Result<()> {
        let mut writer = self.writer.lock().unwrap_or_else(PoisonError::into_inner);
        let writer = writer
            .as_mut()
            .ok_or_else(|| io::Error::from(io::ErrorKind::BrokenPipe))?;
        writeln!(writer, "{message}")?;
        writer.flush()
    }

    fn close(&self) {
        let mut writer = self.writer.lock().unwrap_or_else(PoisonError::into_inner);
        *writer = None;
    }

    /// Sends a request; its response is handled as `kind`. A failed write means the agent is gone,
    /// which the reader reports when its output ends.
    fn request(&self, kind: Request, method: &str, params: Value) -> io::Result<()> {
        let id = {
            let mut state = self.state();
            let id = state.next_id;
            state.next_id += 1;
            state.pending.insert(id, kind);
            if kind == Request::Prompt {
                state.current_prompt = Some(id);
            }
            id
        };
        self.write(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
    }

    fn notify(&self, method: &str, params: Value) -> io::Result<()> {
        self.write(&json!({ "jsonrpc": "2.0", "method": method, "params": params }))
    }

    fn respond(&self, id: &Value, result: Result<Value, (i64, &str)>) {
        let message = match result {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => {
                json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
            }
        };
        let _ = self.write(&message);
    }

    /// Changes a setting: `session/set_mode` for the older modes, else a config option.
    fn set_option(&self, session_id: &str, id: &str, value: &str) -> io::Result<()> {
        if id == LEGACY_MODE {
            self.state().pending_mode = Some(value.to_owned());
            return self.request(
                Request::SetMode,
                "session/set_mode",
                json!({ "sessionId": session_id, "modeId": value }),
            );
        }
        self.request(
            Request::SetOption,
            "session/set_config_option",
            json!({ "sessionId": session_id, "configId": id, "value": value }),
        )
    }

    /// Replaces the known options, keeping the older modes when the agent shows them that way
    /// (they are not config options, so config updates do not carry them).
    fn update_options(&self, mut options: Vec<ConfigOption>) -> Vec<ConfigOption> {
        let mut state = self.state();
        if !options.iter().any(|o| o.kind == OptionKind::Mode)
            && let Some(modes) = state.options.iter().find(|o| o.id == LEGACY_MODE)
        {
            options.push(modes.clone());
        }
        state.options.clone_from(&options);
        options
    }

    /// Marks the current mode of the older modes, and returns the options to show.
    fn mode_changed(&self, mode: &str) -> Vec<AgentEvent> {
        let mut state = self.state();
        let Some(option) = state.options.iter_mut().find(|o| o.id == LEGACY_MODE) else {
            return Vec::new();
        };
        option.current = Some(mode.to_owned());
        vec![AgentEvent::Options(state.options.clone())]
    }

    fn prompt(&self, text: &str) -> io::Result<()> {
        let (session_id, instructions) = {
            let mut state = self.state();
            let instructions =
                std::mem::take(&mut state.needs_instructions).then(|| state.setup.system_prompt.clone());
            state.cancelling = false;
            (state.session_id.clone().unwrap_or_default(), instructions)
        };
        let mut prompt = Vec::new();
        if let Some(instructions) = instructions {
            prompt.push(
                json!({ "type": "text", "text": format!("<instructions>\n{instructions}\n</instructions>") }),
            );
        }
        prompt.push(json!({ "type": "text", "text": text }));
        self.request(
            Request::Prompt,
            "session/prompt",
            json!({ "sessionId": session_id, "prompt": prompt }),
        )
    }

    /// Handles one line from the agent and returns what the thread should see.
    fn handle_line(&self, line: &str) -> Vec<AgentEvent> {
        let Ok(message) = serde_json::from_str::<Value>(line) else {
            return Vec::new();
        };
        match (&message["id"], message["method"].as_str()) {
            (Value::Null, Some(method)) => self.on_notification(method, &message["params"]),
            (id, Some(method)) => {
                self.on_request(id, method, &message["params"]);
                Vec::new()
            }
            (id, None) => {
                let kind = id.as_u64().and_then(|id| {
                    let mut state = self.state();
                    let kind = state.pending.remove(&id)?;
                    // A stopped turn answering after a new message was sent must not end the new one.
                    let stale = kind == Request::Prompt && state.current_prompt != Some(id);
                    (!stale).then_some(kind)
                });
                match (kind, message.get("error")) {
                    (Some(kind), Some(error)) => self.on_error(kind, error),
                    (Some(kind), None) => self.on_result(kind, &message["result"]),
                    (None, _) => Vec::new(),
                }
            }
        }
    }

    fn on_result(&self, kind: Request, result: &Value) -> Vec<AgentEvent> {
        match kind {
            Request::Initialize => {
                let version = result["protocolVersion"].as_u64().unwrap_or(0);
                if version != PROTOCOL_VERSION {
                    let agent = self.state().setup.agent.clone();
                    return vec![failure(format!(
                        "{agent} speaks version {version} of the Agent Client Protocol; Falog speaks version {PROTOCOL_VERSION}."
                    ))];
                }
                let capabilities = &result["agentCapabilities"];
                {
                    let mut state = self.state();
                    state.can_load = capabilities["loadSession"].as_bool().unwrap_or(false);
                    state.can_resume = capabilities["sessionCapabilities"]["resume"].is_object();
                    state.reads_system_prompt = result["agentInfo"]["name"]
                        .as_str()
                        .is_some_and(|name| name.contains("claude"));
                }
                self.open_session();
                Vec::new()
            }
            Request::NewSession | Request::ResumeSession | Request::LoadSession => {
                let session_id = match kind {
                    Request::NewSession => result["sessionId"].as_str().map(str::to_owned),
                    _ => self.state().setup.options.resume.clone(),
                };
                let Some(session_id) = session_id else {
                    return vec![failure("The agent did not open a session.".into())];
                };
                self.opened(kind, session_id, result)
            }
            Request::Prompt => {
                self.state().cancelling = false;
                vec![turn_end(result["stopReason"].as_str().unwrap_or("end_turn"))]
            }
            Request::SetOption => {
                let options = self.update_options(parse_options(&result["configOptions"]));
                vec![AgentEvent::Options(options)]
            }
            Request::SetMode => {
                let mode = self.state().pending_mode.take();
                mode.map(|mode| self.mode_changed(&mode)).unwrap_or_default()
            }
        }
    }

    fn on_error(&self, kind: Request, error: &Value) -> Vec<AgentEvent> {
        let message = error["message"].as_str().unwrap_or("unknown error").to_owned();
        let agent = self.state().setup.agent.clone();
        match kind {
            // The agent lost the conversation (or cannot reopen it): start a fresh one.
            Request::ResumeSession | Request::LoadSession => {
                self.state().replaying = false;
                self.new_session();
                vec![AgentEvent::Notice(format!(
                    "{agent} could not reopen the earlier conversation, so this one starts fresh."
                ))]
            }
            _ if error["code"].as_i64() == Some(AUTH_REQUIRED) => vec![failure(format!(
                "{agent} is not signed in. Run it once in a terminal to sign in, then try again."
            ))],
            Request::SetOption | Request::SetMode => vec![AgentEvent::Notice(format!(
                "{agent} did not accept that setting: {message}"
            ))],
            Request::Prompt => {
                self.state().cancelling = false;
                vec![failure(format!("{agent} failed: {message}"))]
            }
            Request::Initialize | Request::NewSession => {
                vec![failure(format!("Could not start {agent}: {message}"))]
            }
        }
    }

    /// Resumes the thread's conversation when the agent can, else starts a new one.
    fn open_session(&self) {
        let (resume, can_resume, can_load) = {
            let state = self.state();
            (
                state.setup.options.resume.clone(),
                state.can_resume,
                state.can_load,
            )
        };
        match resume {
            Some(id) if can_resume => {
                let params = self.session_params(Some(&id));
                let _ = self.request(Request::ResumeSession, "session/resume", params);
            }
            Some(id) if can_load => {
                self.state().replaying = true;
                let params = self.session_params(Some(&id));
                let _ = self.request(Request::LoadSession, "session/load", params);
            }
            _ => self.new_session(),
        }
    }

    fn new_session(&self) {
        let params = self.session_params(None);
        let _ = self.request(Request::NewSession, "session/new", params);
    }

    fn session_params(&self, session_id: Option<&str>) -> Value {
        let state = self.state();
        let setup = &state.setup;
        let env: Vec<Value> = setup
            .database
            .iter()
            .map(|db| json!({ "name": "FALOG_DB", "value": db }))
            .collect();
        let mut params = json!({
            "cwd": setup.cwd,
            "mcpServers": [{ "name": "falog", "command": setup.mcp_server, "args": [], "env": env }],
            // Read by Claude's adapter: Falog's instructions, no built-in tools, no user settings
            // (hooks, CLAUDE.md) and no MCP servers but falog's. Other agents ignore it.
            "_meta": {
                "systemPrompt": { "append": setup.system_prompt },
                "claudeCode": {
                    "options": { "tools": [], "settingSources": [], "strictMcpConfig": true },
                },
            },
        });
        if let Some(id) = session_id {
            params["sessionId"] = json!(id);
        }
        params
    }

    fn opened(&self, kind: Request, session_id: String, result: &Value) -> Vec<AgentEvent> {
        let mut options = parse_options(&result["configOptions"]);
        if !options.iter().any(|o| o.kind == OptionKind::Mode) {
            options.extend(legacy_modes(&result["modes"]));
        }
        let (queued, wanted) = {
            let mut state = self.state();
            state.session_id = Some(session_id.clone());
            state.replaying = false;
            state.needs_instructions = kind == Request::NewSession && !state.reads_system_prompt;
            state.options.clone_from(&options);
            let wanted: Vec<(OptionKind, Option<String>)> = OptionKind::PICKED
                .iter()
                .map(|kind| (*kind, state.setup.options.get(*kind).map(str::to_owned)))
                .collect();
            (std::mem::take(&mut state.queued), wanted)
        };
        let model = options
            .iter()
            .find(|o| o.kind == OptionKind::Model)
            .and_then(|o| o.current.as_deref().map(|value| o.label(value).to_owned()));
        let mut events = vec![
            AgentEvent::Ready {
                session_id: session_id.clone(),
                model,
                tools_connected: true,
            },
            AgentEvent::Options(options.clone()),
        ];
        for (kind, value) in wanted {
            let Some(value) = value else { continue };
            let Some(option) = options.iter().find(|o| o.kind == kind) else {
                continue;
            };
            if option.current.as_deref() != Some(&value) && option.choices.iter().any(|c| c.value == value) {
                let _ = self.set_option(&session_id, &option.id, &value);
            }
        }
        for text in queued {
            if let Err(err) = self.prompt(&text) {
                events.push(failure(err.to_string()));
            }
        }
        events
    }

    fn on_request(&self, id: &Value, method: &str, params: &Value) {
        match method {
            "session/request_permission" => {
                let outcome = self.permission(params);
                self.respond(id, Ok(json!({ "outcome": outcome })));
            }
            _ => self.respond(id, Err((METHOD_NOT_FOUND, "Falog does not support this method"))),
        }
    }

    /// Allows falog tools once; rejects everything else.
    fn permission(&self, params: &Value) -> Value {
        let call = &params["toolCall"];
        let known = call["toolCallId"]
            .as_str()
            .and_then(|id| self.state().tools.get(id).cloned());
        let allowed = [call["name"].as_str(), call["title"].as_str(), known.as_deref()]
            .into_iter()
            .flatten()
            .any(|name| falog_tool(name).is_some());
        let kinds: &[&str] = if allowed {
            &["allow_once", "allow_always"]
        } else {
            &["reject_once", "reject_always"]
        };
        let options = params["options"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        kinds
            .iter()
            .find_map(|kind| options.iter().find(|o| o["kind"] == *kind))
            .map_or_else(
                || json!({ "outcome": "cancelled" }),
                |option| json!({ "outcome": "selected", "optionId": option["optionId"] }),
            )
    }

    fn on_notification(&self, method: &str, params: &Value) -> Vec<AgentEvent> {
        if method != "session/update" {
            return Vec::new();
        }
        let update = &params["update"];
        let kind = update["sessionUpdate"].as_str().unwrap_or_default();
        match kind {
            "available_commands_update" => return vec![AgentEvent::Commands(parse_commands(update))],
            "config_option_update" => {
                let options = self.update_options(parse_options(&update["configOptions"]));
                return vec![AgentEvent::Options(options)];
            }
            "current_mode_update" => {
                return update["currentModeId"]
                    .as_str()
                    .map(|mode| self.mode_changed(mode))
                    .unwrap_or_default();
            }
            "usage_update" => {
                return match (update["used"].as_u64(), update["size"].as_u64()) {
                    (Some(used), Some(size)) => vec![AgentEvent::Usage(Usage { used, size })],
                    _ => Vec::new(),
                };
            }
            _ => {}
        }
        {
            let state = self.state();
            if state.replaying || state.cancelling {
                return Vec::new();
            }
        }
        match kind {
            "agent_message_chunk" => match update["content"]["text"].as_str() {
                Some(text) if update["content"]["type"] == "text" => {
                    vec![AgentEvent::TextDelta(text.to_owned())]
                }
                _ => Vec::new(),
            },
            "tool_call" | "tool_call_update" => self.tool_call(kind == "tool_call", update),
            _ => Vec::new(),
        }
    }

    fn tool_call(&self, new: bool, update: &Value) -> Vec<AgentEvent> {
        let id = update["toolCallId"].as_str().unwrap_or_default().to_owned();
        let mut events = Vec::new();
        let named = [update["name"].as_str(), update["title"].as_str()]
            .into_iter()
            .flatten()
            .find(|name| !name.is_empty());
        if new || !update["rawInput"].is_null() {
            let name = match named {
                Some(name) => tool_name(name),
                None => self.state().tools.get(&id).cloned().unwrap_or_default(),
            };
            self.state().tools.insert(id.clone(), name.clone());
            events.push(AgentEvent::ToolUse {
                id: id.clone(),
                name,
                input: update["rawInput"].clone(),
            });
        }
        if let Some(status @ ("completed" | "failed")) = update["status"].as_str() {
            events.push(AgentEvent::ToolResult {
                id,
                text: tool_output(update),
                is_error: status == "failed",
            });
        }
        events
    }
}

fn initialize_params() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "clientCapabilities": {
            "fs": { "readTextFile": false, "writeTextFile": false },
            "terminal": false,
        },
        "clientInfo": { "name": "falog", "title": "Falog", "version": env!("CARGO_PKG_VERSION") },
    })
}

/// A turn that ends with an error message.
fn failure(message: String) -> AgentEvent {
    AgentEvent::TurnFinished {
        is_error: true,
        message: Some(message),
    }
}

fn turn_end(stop_reason: &str) -> AgentEvent {
    let message = match stop_reason {
        "max_tokens" => Some("The reply was cut off: it reached the agent's length limit."),
        "max_turn_requests" => Some("The agent stopped after too many steps in one turn."),
        "refusal" => Some("The agent declined to continue."),
        _ => None,
    };
    AgentEvent::TurnFinished {
        is_error: message.is_some(),
        message: message.map(str::to_owned),
    }
}

/// The falog tool a name refers to, however the agent spells it: `create_task`,
/// `mcp__falog__create_task`, `falog.create_task`, `create_task (falog MCP Server)`... A name that
/// is qualified with another server is not a falog tool, even if the tool name matches.
pub fn falog_tool(name: &str) -> Option<&'static str> {
    let name = name.trim();
    let mentions_falog = name.to_lowercase().contains("falog");
    FALOG_TOOLS.into_iter().find(|tool| {
        let qualified = name
            .strip_suffix(tool)
            .is_some_and(|server| server.ends_with(['_', '.', '/', ':']));
        let described = name
            .strip_prefix(tool)
            .is_some_and(|rest| rest.starts_with([' ', '(', ':']));
        name == *tool || (mentions_falog && (qualified || described))
    })
}

/// Falog tools are shown by their bare name, so the thread labels them; others by their title.
fn tool_name(name: &str) -> String {
    falog_tool(name).map_or_else(|| name.to_owned(), str::to_owned)
}

/// The text of a finished tool call: its content blocks, else its raw output.
fn tool_output(update: &Value) -> String {
    let content: Vec<&str> = update["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| item["type"] == "content")
        .filter_map(|item| item["content"]["text"].as_str())
        .collect();
    if !content.is_empty() {
        return content.join("\n");
    }
    match &update["rawOutput"] {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        output => match output["content"].as_array() {
            Some(parts) => parts
                .iter()
                .filter_map(|p| p["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            None => output.to_string(),
        },
    }
}

fn parse_commands(update: &Value) -> Vec<SlashCommand> {
    update["availableCommands"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|command| {
            Some(SlashCommand {
                name: command["name"].as_str()?.to_owned(),
                description: command["description"].as_str().unwrap_or_default().to_owned(),
                hint: command["input"]["hint"].as_str().unwrap_or_default().to_owned(),
            })
        })
        .collect()
}

/// Select config options; grouped choices are flattened. Boolean options are not shown.
fn parse_options(options: &Value) -> Vec<ConfigOption> {
    options
        .as_array()
        .into_iter()
        .flatten()
        .filter(|option| option["type"] == "select")
        .filter_map(|option| {
            let choices = option["options"]
                .as_array()?
                .iter()
                .flat_map(|entry| match entry["options"].as_array() {
                    Some(group) => group.iter().collect::<Vec<_>>(),
                    None => vec![entry],
                })
                .filter_map(|choice| {
                    Some(Choice {
                        value: choice["value"].as_str()?.to_owned(),
                        name: choice["name"].as_str()?.to_owned(),
                    })
                })
                .collect();
            Some(ConfigOption {
                id: option["id"].as_str()?.to_owned(),
                name: option["name"].as_str().unwrap_or_default().to_owned(),
                kind: match (option["category"].as_str(), option["id"].as_str()) {
                    (Some("model"), _) => OptionKind::Model,
                    (Some("thought_level"), _) => OptionKind::Effort,
                    (Some("mode"), _) => OptionKind::Mode,
                    // Claude's adapter shows fast mode as an on/off `model_config` option.
                    (Some("model_config"), Some("fast")) => OptionKind::Fast,
                    _ => OptionKind::Other,
                },
                current: option["currentValue"].as_str().map(str::to_owned),
                choices,
                description: option["description"].as_str().unwrap_or_default().to_owned(),
            })
        })
        .collect()
}

/// An agent's older session modes (`modes`, switched with `session/set_mode`), as an option.
fn legacy_modes(modes: &Value) -> Option<ConfigOption> {
    let choices: Vec<Choice> = modes["availableModes"]
        .as_array()?
        .iter()
        .filter_map(|mode| {
            Some(Choice {
                value: mode["id"].as_str()?.to_owned(),
                name: mode["name"].as_str()?.to_owned(),
            })
        })
        .collect();
    (!choices.is_empty()).then(|| ConfigOption {
        id: LEGACY_MODE.into(),
        name: "Mode".into(),
        kind: OptionKind::Mode,
        current: modes["currentModeId"].as_str().map(str::to_owned),
        choices,
        description: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io::Read;
    use std::sync::mpsc::Sender;
    use std::time::{Duration, Instant};

    /// One direction of an in-memory stream.
    struct PipeReader {
        chunks: Receiver<Vec<u8>>,
        buffer: VecDeque<u8>,
    }

    impl Read for PipeReader {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if self.buffer.is_empty() {
                match self.chunks.recv() {
                    Ok(chunk) => self.buffer.extend(chunk),
                    Err(_) => return Ok(0),
                }
            }
            let n = out.len().min(self.buffer.len());
            for (slot, byte) in out.iter_mut().zip(self.buffer.drain(..n)) {
                *slot = byte;
            }
            Ok(n)
        }
    }

    struct PipeWriter(Sender<Vec<u8>>);

    impl Write for PipeWriter {
        fn write(&mut self, data: &[u8]) -> io::Result<usize> {
            self.0
                .send(data.to_vec())
                .map_err(|_| io::Error::from(io::ErrorKind::BrokenPipe))?;
            Ok(data.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn pipe() -> (PipeWriter, BufReader<PipeReader>) {
        let (sender, chunks) = mpsc::channel();
        let reader = PipeReader {
            chunks,
            buffer: VecDeque::new(),
        };
        (PipeWriter(sender), BufReader::new(reader))
    }

    /// The agent's side of the conversation, driven by the test.
    struct FakeAgent {
        input: std::io::Lines<BufReader<PipeReader>>,
        output: PipeWriter,
    }

    impl FakeAgent {
        fn next(&mut self) -> Value {
            let line = self.input.next().expect("client closed").unwrap();
            serde_json::from_str(&line).unwrap()
        }

        /// The next message from Falog, which must be a call to `method`.
        fn expect(&mut self, method: &str) -> Value {
            let message = self.next();
            assert_eq!(message["method"], method, "unexpected message {message}");
            message
        }

        /// The next message from Falog, which must answer the agent's request `id`.
        fn expect_response(&mut self, id: u64) -> Value {
            let message = self.next();
            assert_eq!(message["id"], id, "unexpected message {message}");
            message
        }

        fn send(&mut self, message: Value) {
            writeln!(self.output, "{message}").unwrap();
        }

        fn reply(&mut self, request: &Value, result: Value) {
            self.send(json!({ "jsonrpc": "2.0", "id": request["id"], "result": result }));
        }

        fn fail(&mut self, request: &Value, code: i64, message: &str) {
            self.send(json!({ "jsonrpc": "2.0", "id": request["id"], "error": { "code": code, "message": message } }));
        }

        fn update(&mut self, update: Value) {
            self.send(json!({
                "jsonrpc": "2.0",
                "method": "session/update",
                "params": { "sessionId": "s1", "update": update },
            }));
        }

        fn ask_permission(&mut self, id: u64, tool_call: Value) -> Value {
            let options = json!([
                { "optionId": "yes", "name": "Allow", "kind": "allow_once" },
                { "optionId": "always", "name": "Always allow", "kind": "allow_always" },
                { "optionId": "no", "name": "Reject", "kind": "reject_once" },
            ]);
            self.send(json!({
                "jsonrpc": "2.0", "id": id, "method": "session/request_permission",
                "params": { "sessionId": "s1", "toolCall": tool_call, "options": options },
            }));
            self.expect_response(id)["result"]["outcome"].clone()
        }
    }

    fn setup(resume: Option<&str>, model: Option<&str>) -> Setup {
        Setup {
            agent: "Gemini CLI".into(),
            cwd: PathBuf::from("work"),
            mcp_server: PathBuf::from("falog-mcp"),
            database: Some(PathBuf::from("falog.db")),
            system_prompt: "Be brief.".into(),
            options: StartOptions {
                resume: resume.map(str::to_owned),
                model: model.map(str::to_owned),
                ..StartOptions::default()
            },
        }
    }

    fn connect(setup: Setup) -> (AcpSession, FakeAgent) {
        let (client_out, agent_in) = pipe();
        let (agent_out, client_in) = pipe();
        let session = AcpSession::connect(client_in, client_out, setup, || "bye".into(), || ());
        let agent = FakeAgent {
            input: agent_in.lines(),
            output: agent_out,
        };
        (session, agent)
    }

    fn next_event(session: &mut AcpSession) -> AgentEvent {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(event) = session.try_recv() {
                return event;
            }
            assert!(Instant::now() < deadline, "no event");
            thread::sleep(Duration::from_millis(2));
        }
    }

    fn config_options() -> Value {
        json!([
            {
                "id": "model", "name": "Model", "category": "model", "type": "select",
                "currentValue": "pro",
                "options": [{ "value": "pro", "name": "Gemini Pro" }, { "value": "flash", "name": "Gemini Flash" }],
            },
            {
                "id": "thinking", "name": "Thinking", "category": "thought_level", "type": "select",
                "currentValue": "auto",
                "options": [{ "group": "levels", "name": "Levels", "options": [
                    { "value": "auto", "name": "Auto" }, { "value": "high", "name": "High" },
                ]}],
            },
            { "id": "yolo", "name": "YOLO", "type": "boolean", "currentValue": false },
        ])
    }

    fn handshake(agent: &mut FakeAgent, capabilities: Value, name: &str) {
        let init = agent.expect("initialize");
        assert_eq!(init["params"]["protocolVersion"], 1);
        assert_eq!(init["params"]["clientCapabilities"]["terminal"], false);
        let info = json!({ "name": name, "version": "1.0.0" });
        agent.reply(
            &init,
            json!({ "protocolVersion": 1, "agentCapabilities": capabilities, "agentInfo": info }),
        );
    }

    #[test]
    fn runs_a_turn_with_falog_tools() {
        let (mut session, mut agent) = connect(setup(None, None));
        session.send("hi").unwrap();
        handshake(&mut agent, json!({}), "gemini-cli");

        let new = agent.expect("session/new");
        let server = &new["params"]["mcpServers"][0];
        assert_eq!(server["name"], "falog");
        assert_eq!(server["command"], "falog-mcp");
        assert_eq!(
            server["env"][0],
            json!({ "name": "FALOG_DB", "value": "falog.db" })
        );
        assert_eq!(new["params"]["cwd"], "work");
        agent.reply(
            &new,
            json!({ "sessionId": "s1", "configOptions": config_options() }),
        );

        // The message typed while the agent was starting goes out now, after the instructions
        // (this agent does not read `_meta.systemPrompt`).
        let prompt = agent.expect("session/prompt");
        assert_eq!(prompt["params"]["sessionId"], "s1");
        let blocks = prompt["params"]["prompt"].as_array().unwrap();
        assert!(blocks[0]["text"].as_str().unwrap().contains("Be brief."));
        assert_eq!(blocks[1], json!({ "type": "text", "text": "hi" }));

        assert_eq!(
            next_event(&mut session),
            AgentEvent::Ready {
                session_id: "s1".into(),
                model: Some("Gemini Pro".into()),
                tools_connected: true,
            }
        );
        let AgentEvent::Options(options) = next_event(&mut session) else {
            panic!("expected options");
        };
        assert_eq!(options.len(), 2, "boolean options are not shown");
        assert_eq!(options[1].kind, OptionKind::Effort);
        assert_eq!(options[1].label("high"), "High");

        agent.update(
            json!({ "sessionUpdate": "agent_thought_chunk", "content": { "type": "text", "text": "hmm" } }),
        );
        agent.update(
            json!({ "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "On it" } }),
        );
        assert_eq!(next_event(&mut session), AgentEvent::TextDelta("On it".into()));

        agent.update(json!({
            "sessionUpdate": "tool_call", "toolCallId": "t1", "title": "create_task (falog MCP Server)",
            "kind": "other", "status": "pending", "rawInput": { "title": "Fix login" },
        }));
        assert_eq!(
            next_event(&mut session),
            AgentEvent::ToolUse {
                id: "t1".into(),
                name: "create_task".into(),
                input: json!({ "title": "Fix login" }),
            }
        );

        // Falog tools are allowed once (known by the id of the call seen before); others rejected.
        let allowed = agent.ask_permission(90, json!({ "toolCallId": "t1" }));
        assert_eq!(allowed, json!({ "outcome": "selected", "optionId": "yes" }));
        let rejected = agent.ask_permission(91, json!({ "toolCallId": "t2", "title": "Shell: del *" }));
        assert_eq!(rejected, json!({ "outcome": "selected", "optionId": "no" }));

        agent.update(json!({
            "sessionUpdate": "tool_call_update", "toolCallId": "t1", "status": "completed",
            "content": [{ "type": "content", "content": { "type": "text", "text": "Created #7" } }],
        }));
        assert_eq!(
            next_event(&mut session),
            AgentEvent::ToolResult {
                id: "t1".into(),
                text: "Created #7".into(),
                is_error: false,
            }
        );

        agent.update(
            json!({ "sessionUpdate": "available_commands_update", "availableCommands": [
                { "name": "compress", "description": "Summarize the chat", "input": { "hint": "focus" } },
            ]}),
        );
        assert_eq!(
            next_event(&mut session),
            AgentEvent::Commands(vec![SlashCommand {
                name: "compress".into(),
                description: "Summarize the chat".into(),
                hint: "focus".into(),
            }])
        );

        agent.reply(&prompt, json!({ "stopReason": "end_turn" }));
        assert_eq!(
            next_event(&mut session),
            AgentEvent::TurnFinished {
                is_error: false,
                message: None,
            }
        );

        // Later prompts carry only the message.
        session.send("thanks").unwrap();
        let prompt = agent.expect("session/prompt");
        assert_eq!(prompt["params"]["prompt"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn resumes_and_applies_the_chosen_model() {
        let (mut session, mut agent) = connect(setup(Some("s1"), Some("flash")));
        handshake(
            &mut agent,
            json!({ "sessionCapabilities": { "resume": {} } }),
            "@agentclientprotocol/claude-agent-acp",
        );
        let resume = agent.expect("session/resume");
        assert_eq!(resume["params"]["sessionId"], "s1");
        let options = &resume["params"]["_meta"]["claudeCode"]["options"];
        assert_eq!(options["tools"], json!([]));
        assert_eq!(options["strictMcpConfig"], true);
        agent.reply(&resume, json!({ "configOptions": config_options() }));

        let set = agent.expect("session/set_config_option");
        assert_eq!(
            set["params"],
            json!({ "sessionId": "s1", "configId": "model", "value": "flash" })
        );
        agent.reply(&set, json!({ "configOptions": [] }));
        assert!(
            matches!(next_event(&mut session), AgentEvent::Ready { session_id, .. } if session_id == "s1")
        );
        assert!(matches!(next_event(&mut session), AgentEvent::Options(_)));
        assert_eq!(next_event(&mut session), AgentEvent::Options(Vec::new()));

        // Claude's adapter reads the instructions from `_meta`, so prompts carry only the message.
        session.send("hi").unwrap();
        let prompt = agent.expect("session/prompt");
        assert_eq!(prompt["params"]["prompt"].as_array().unwrap().len(), 1);

        assert!(session.set_option("thinking", "high"), "switches live");
        let set = agent.expect("session/set_config_option");
        assert_eq!(set["params"]["value"], "high");
    }

    #[test]
    fn switches_older_modes_and_reports_usage() {
        let (mut session, mut agent) = connect(setup(None, None));
        handshake(&mut agent, json!({}), "gemini-cli");
        let new = agent.expect("session/new");
        let modes = json!({ "currentModeId": "default", "availableModes": [
            { "id": "default", "name": "Default" }, { "id": "yolo", "name": "YOLO" },
        ]});
        agent.reply(&new, json!({ "sessionId": "s1", "modes": modes }));
        assert!(matches!(next_event(&mut session), AgentEvent::Ready { .. }));
        let AgentEvent::Options(options) = next_event(&mut session) else {
            panic!("expected options");
        };
        assert_eq!(options[0].kind, OptionKind::Mode);
        assert_eq!(options[0].current.as_deref(), Some("default"));

        assert!(session.set_option(LEGACY_MODE, "yolo"));
        let set = agent.expect("session/set_mode");
        assert_eq!(set["params"], json!({ "sessionId": "s1", "modeId": "yolo" }));
        agent.reply(&set, json!({}));
        let AgentEvent::Options(options) = next_event(&mut session) else {
            panic!("expected options");
        };
        assert_eq!(options[0].current.as_deref(), Some("yolo"));

        agent.update(json!({ "sessionUpdate": "current_mode_update", "currentModeId": "default" }));
        assert!(
            matches!(next_event(&mut session), AgentEvent::Options(o) if o[0].current.as_deref() == Some("default"))
        );
        agent.update(json!({ "sessionUpdate": "usage_update", "used": 5300, "size": 1000000 }));
        assert_eq!(
            next_event(&mut session),
            AgentEvent::Usage(Usage {
                used: 5300,
                size: 1_000_000
            })
        );
    }

    #[test]
    fn reads_mode_and_fast_options() {
        let options = parse_options(&json!([
            { "id": "mode", "name": "Mode", "category": "mode", "type": "select", "currentValue": "default",
              "options": [{ "value": "default", "name": "Default" }, { "value": "plan", "name": "Plan" }] },
            { "id": "fast", "name": "Fast mode", "category": "model_config", "type": "select", "currentValue": "off",
              "description": "Faster responses — requires extra usage",
              "options": [{ "value": "on", "name": "On" }, { "value": "off", "name": "Off" }] },
        ]));
        assert_eq!(options[0].kind, OptionKind::Mode);
        assert_eq!(options[1].kind, OptionKind::Fast);
        assert!(options[1].description.contains("extra usage"));
    }

    #[test]
    fn starts_fresh_when_the_conversation_cannot_be_reopened() {
        let (mut session, mut agent) = connect(setup(Some("gone"), None));
        handshake(&mut agent, json!({ "loadSession": true }), "codex-acp");
        let load = agent.expect("session/load");
        // History replayed by session/load is not shown again.
        agent.update(
            json!({ "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "old" } }),
        );
        agent.fail(&load, -32002, "Resource not found");
        let new = agent.expect("session/new");
        assert!(matches!(next_event(&mut session), AgentEvent::Notice(_)));
        agent.reply(&new, json!({ "sessionId": "s2" }));
        assert!(
            matches!(next_event(&mut session), AgentEvent::Ready { session_id, .. } if session_id == "s2")
        );
    }

    #[test]
    fn drops_what_streams_after_a_cancel() {
        let (mut session, mut agent) = connect(setup(None, None));
        handshake(&mut agent, json!({}), "gemini-cli");
        let new = agent.expect("session/new");
        agent.reply(&new, json!({ "sessionId": "s1" }));
        assert!(matches!(next_event(&mut session), AgentEvent::Ready { .. }));
        assert!(matches!(next_event(&mut session), AgentEvent::Options(_)));

        session.send("long task").unwrap();
        let prompt = agent.expect("session/prompt");
        assert!(session.cancel());
        let cancel = agent.expect("session/cancel");
        assert_eq!(cancel["params"]["sessionId"], "s1");
        assert!(cancel.get("id").is_none(), "cancel is a notification");
        agent.update(
            json!({ "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "late" } }),
        );
        agent.reply(&prompt, json!({ "stopReason": "cancelled" }));
        assert_eq!(
            next_event(&mut session),
            AgentEvent::TurnFinished {
                is_error: false,
                message: None,
            }
        );
    }

    #[test]
    fn a_stopped_turn_does_not_end_the_next_one() {
        let (mut session, mut agent) = connect(setup(None, None));
        handshake(&mut agent, json!({}), "gemini-cli");
        let new = agent.expect("session/new");
        agent.reply(&new, json!({ "sessionId": "s1" }));
        assert!(matches!(next_event(&mut session), AgentEvent::Ready { .. }));
        assert!(matches!(next_event(&mut session), AgentEvent::Options(_)));

        session.send("first").unwrap();
        let first = agent.expect("session/prompt");
        assert!(session.cancel());
        agent.expect("session/cancel");
        session.send("second").unwrap();
        let second = agent.expect("session/prompt");
        agent.reply(&first, json!({ "stopReason": "cancelled" }));
        agent.update(
            json!({ "sessionUpdate": "agent_message_chunk", "content": { "type": "text", "text": "Hi" } }),
        );
        assert_eq!(next_event(&mut session), AgentEvent::TextDelta("Hi".into()));
        agent.reply(&second, json!({ "stopReason": "end_turn" }));
        assert_eq!(
            next_event(&mut session),
            AgentEvent::TurnFinished {
                is_error: false,
                message: None,
            }
        );
    }

    #[test]
    fn reports_sign_in_errors_unknown_methods_and_exits() {
        let (mut session, mut agent) = connect(setup(None, None));
        handshake(&mut agent, json!({}), "gemini-cli");
        let new = agent.expect("session/new");
        agent.send(json!({ "jsonrpc": "2.0", "id": 5, "method": "fs/read_text_file", "params": {} }));
        assert_eq!(agent.expect_response(5)["error"]["code"], METHOD_NOT_FOUND);
        agent.fail(&new, AUTH_REQUIRED, "Authentication required");
        assert!(matches!(next_event(&mut session),
            AgentEvent::TurnFinished { is_error: true, message: Some(text) } if text.contains("not signed in")));
        drop(agent);
        assert_eq!(
            next_event(&mut session),
            AgentEvent::Exited { stderr: "bye".into() }
        );
    }

    #[test]
    fn knows_every_tool_of_the_mcp_server() {
        // A tool missing here is refused when an ACP agent asks to run it.
        let catalog = include_str!("../../../../falog-mcp/src/tools/catalog.rs");
        let tools: Vec<&str> = catalog
            .split("\"name\": \"")
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .collect();
        assert!(tools.len() > 10, "{tools:?}");
        for tool in tools {
            assert_eq!(falog_tool(tool), Some(tool), "add {tool} to FALOG_TOOLS");
        }
    }

    #[test]
    fn recognizes_falog_tools_by_any_spelling() {
        assert_eq!(falog_tool("mcp__falog__create_event"), Some("create_event"));
        assert_eq!(falog_tool("create_task"), Some("create_task"));
        assert_eq!(falog_tool("mcp__falog__update_task"), Some("update_task"));
        assert_eq!(falog_tool("falog.get_focus"), Some("get_focus"));
        assert_eq!(falog_tool("list_tasks (falog MCP Server)"), Some("list_tasks"));
        assert_eq!(falog_tool("mcp__jira__create_task"), None);
        assert_eq!(falog_tool("list_tasks_by_owner"), None);
        assert_eq!(falog_tool("Shell"), None);
    }

    #[test]
    fn explains_why_a_turn_stopped() {
        assert!(matches!(
            turn_end("end_turn"),
            AgentEvent::TurnFinished { is_error: false, .. }
        ));
        assert!(matches!(
            turn_end("max_tokens"),
            AgentEvent::TurnFinished { is_error: true, .. }
        ));
    }
}
