//! Drives Claude Code in headless mode.
//!
//! One long-lived `claude -p` process per conversation, speaking newline-delimited JSON on
//! stdin/stdout (`--input-format/--output-format stream-json`). All built-in tools are disabled;
//! the only tools available are the ones of the `falog` MCP server, so the assistant can manage
//! tasks and nothing else. It runs on the user's own Claude Code login.

use eframe::egui;
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

/// Everything needed to start a session.
#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub claude: PathBuf,
    pub mcp_server: PathBuf,
    /// Database the MCP server should use (the one the app has open).
    pub database: Option<PathBuf>,
    /// Model alias (`haiku`, `sonnet`, `opus`); `None` uses the Claude Code default.
    pub model: Option<String>,
    /// Isolated working directory, so no project `CLAUDE.md` leaks into the conversation.
    pub workdir: PathBuf,
    pub system_prompt: String,
    /// Continue an earlier conversation (after a stop or a model change).
    pub resume: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClaudeEvent {
    Ready {
        session_id: String,
        model: String,
        tools_connected: bool,
    },
    TextDelta(String),
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
    Exited {
        stderr: String,
    },
}

#[derive(Debug)]
pub struct ClaudeSession {
    child: Child,
    stdin: ChildStdin,
    events: Receiver<ClaudeEvent>,
}

impl ClaudeSession {
    pub fn start(config: &SessionConfig, ctx: egui::Context) -> io::Result<Self> {
        std::fs::create_dir_all(&config.workdir)?;
        let mcp_config = config.workdir.join("mcp.json");
        std::fs::write(&mcp_config, mcp_config_json(config).to_string())?;

        let mut command = Command::new(&config.claude);
        command.args([
            "--print",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--tools",
            "",
            "--strict-mcp-config",
            "--allowedTools",
            "mcp__falog",
            "--permission-mode",
            "dontAsk",
            "--setting-sources",
            "",
        ]);
        command.arg("--mcp-config").arg(&mcp_config);
        command.arg("--append-system-prompt").arg(&config.system_prompt);
        if let Some(model) = &config.model {
            command.args(["--model", model]);
        }
        if let Some(session) = &config.resume {
            command.args(["--resume", session]);
        }
        command
            .current_dir(&config.workdir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = command.spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| io::Error::other("no stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| io::Error::other("no stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| io::Error::other("no stderr"))?;

        let stderr_text = Arc::new(Mutex::new(String::new()));
        let sink = Arc::clone(&stderr_text);
        thread::spawn(move || {
            let mut text = String::new();
            let _ = BufReader::new(stderr).read_to_string(&mut text);
            if let Ok(mut shared) = sink.lock() {
                *shared = text;
            }
        });

        let (sender, events) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                for event in parse_line(&line) {
                    if sender.send(event).is_err() {
                        return;
                    }
                }
                ctx.request_repaint();
            }
            // Give the stderr reader a moment to collect the reason the process ended.
            thread::sleep(std::time::Duration::from_millis(200));
            let stderr = stderr_text
                .lock()
                .map(|s| s.trim().to_owned())
                .unwrap_or_default();
            let _ = sender.send(ClaudeEvent::Exited { stderr });
            ctx.request_repaint();
        });

        Ok(Self { child, stdin, events })
    }

    pub fn send(&mut self, text: &str) -> io::Result<()> {
        let message = json!({
            "type": "user",
            "message": { "role": "user", "content": [{ "type": "text", "text": text }] },
        });
        writeln!(self.stdin, "{message}")?;
        self.stdin.flush()
    }

    pub fn try_recv(&self) -> Option<ClaudeEvent> {
        self.events.try_recv().ok()
    }
}

impl Drop for ClaudeSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn mcp_config_json(config: &SessionConfig) -> Value {
    let mut env = serde_json::Map::new();
    if let Some(database) = &config.database {
        env.insert("FALOG_DB".into(), json!(database));
    }
    json!({
        "mcpServers": {
            "falog": { "type": "stdio", "command": config.mcp_server, "args": [], "env": env }
        }
    })
}

/// Locates the `claude` executable: on `PATH`, then in the usual install folders (see
/// [`find_executable`](crate::platform::paths::find_executable)).
pub fn find_claude() -> Option<PathBuf> {
    crate::platform::paths::find_executable("claude")
}

/// The `falog-mcp` executable shipped next to the app.
pub fn find_mcp_server() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let candidate = exe
        .parent()?
        .join(format!("falog-mcp{}", std::env::consts::EXE_SUFFIX));
    candidate.is_file().then_some(candidate)
}

/// Maps one stream-json line to zero or more events. Unknown messages are ignored.
pub fn parse_line(line: &str) -> Vec<ClaudeEvent> {
    let Ok(message) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    let text = |value: &Value| value.as_str().unwrap_or_default().to_owned();
    match message["type"].as_str() {
        Some("system") if message["subtype"] == "init" => {
            let tools_connected = message["mcp_servers"].as_array().is_some_and(|servers| {
                servers
                    .iter()
                    .any(|s| s["name"] == "falog" && s["status"] == "connected")
            });
            vec![ClaudeEvent::Ready {
                session_id: text(&message["session_id"]),
                model: text(&message["model"]),
                tools_connected,
            }]
        }
        Some("stream_event") => {
            let event = &message["event"];
            if event["type"] == "content_block_delta" && event["delta"]["type"] == "text_delta" {
                vec![ClaudeEvent::TextDelta(text(&event["delta"]["text"]))]
            } else {
                Vec::new()
            }
        }
        Some("assistant") => blocks(&message)
            .filter_map(|block| match block["type"].as_str() {
                Some("text") => Some(ClaudeEvent::Text(text(&block["text"]))),
                Some("tool_use") => Some(ClaudeEvent::ToolUse {
                    id: text(&block["id"]),
                    name: text(&block["name"]),
                    input: block["input"].clone(),
                }),
                _ => None,
            })
            .collect(),
        Some("user") => blocks(&message)
            .filter(|block| block["type"] == "tool_result")
            .map(|block| ClaudeEvent::ToolResult {
                id: text(&block["tool_use_id"]),
                text: tool_result_text(&block["content"]),
                is_error: block["is_error"].as_bool().unwrap_or(false),
            })
            .collect(),
        Some("result") => {
            let is_error = message["is_error"].as_bool().unwrap_or(false);
            let detail = message["result"]
                .as_str()
                .or(message["subtype"].as_str())
                .map(str::to_owned);
            vec![ClaudeEvent::TurnFinished {
                is_error,
                message: if is_error { detail } else { None },
            }]
        }
        _ => Vec::new(),
    }
}

fn blocks(message: &Value) -> impl Iterator<Item = &Value> {
    message["message"]["content"].as_array().into_iter().flatten()
}

fn tool_result_text(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// Last lines of stderr, enough to explain a failure in the UI.
pub fn summarize_stderr(stderr: &str) -> String {
    let lines: Vec<&str> = stderr.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(3)..].join("\n")
}

/// Default working directory for assistant sessions: `<data dir>/Falog/assistant`.
pub fn default_workdir(database: Option<&Path>) -> PathBuf {
    database
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(std::env::temp_dir)
        .join("assistant")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_session_init() {
        let line = r#"{"type":"system","subtype":"init","session_id":"abc","model":"claude-haiku-4-5","mcp_servers":[{"name":"falog","status":"connected"}]}"#;
        assert_eq!(
            parse_line(line),
            vec![ClaudeEvent::Ready {
                session_id: "abc".into(),
                model: "claude-haiku-4-5".into(),
                tools_connected: true
            }]
        );
    }

    #[test]
    fn parses_text_deltas_and_ignores_other_stream_events() {
        let delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Globex has"}}}"#;
        assert_eq!(
            parse_line(delta),
            vec![ClaudeEvent::TextDelta("Globex has".into())]
        );
        let thinking = r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"..."}}}"#;
        assert!(parse_line(thinking).is_empty());
    }

    #[test]
    fn parses_tool_use_and_result() {
        let tool_use = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"mcp__falog__add_note","input":{"id":4,"note":"x"}}]}}"#;
        assert_eq!(
            parse_line(tool_use),
            vec![ClaudeEvent::ToolUse {
                id: "t1".into(),
                name: "mcp__falog__add_note".into(),
                input: json!({"id": 4, "note": "x"})
            }]
        );
        let result = r#"{"type":"user","message":{"role":"user","content":[{"tool_use_id":"t1","type":"tool_result","content":[{"type":"text","text":"Note added to #4"}]}]}}"#;
        assert_eq!(
            parse_line(result),
            vec![ClaudeEvent::ToolResult {
                id: "t1".into(),
                text: "Note added to #4".into(),
                is_error: false
            }]
        );
    }

    #[test]
    fn parses_assistant_text_and_turn_end() {
        let text = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Done."}]}}"#;
        assert_eq!(parse_line(text), vec![ClaudeEvent::Text("Done.".into())]);
        let ok = r#"{"type":"result","subtype":"success","is_error":false,"result":"Done."}"#;
        assert_eq!(
            parse_line(ok),
            vec![ClaudeEvent::TurnFinished {
                is_error: false,
                message: None
            }]
        );
        let failed = r#"{"type":"result","subtype":"error_during_execution","is_error":true}"#;
        assert_eq!(
            parse_line(failed),
            vec![ClaudeEvent::TurnFinished {
                is_error: true,
                message: Some("error_during_execution".into())
            }]
        );
    }

    #[test]
    fn ignores_garbage() {
        assert!(parse_line("not json").is_empty());
        assert!(parse_line(r#"{"type":"rate_limit_event"}"#).is_empty());
    }
}
