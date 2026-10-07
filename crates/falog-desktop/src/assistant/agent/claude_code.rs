//! Claude Code in headless mode, the default agent.
//!
//! One long-lived `claude -p` process per conversation, speaking newline-delimited JSON on
//! stdin/stdout (`--input-format/--output-format stream-json`). All built-in tools are disabled;
//! the only tools available are the ones of the `falog` MCP server, so the assistant can manage
//! tasks and nothing else. It runs on the user's own Claude Code login.

use super::{AgentEvent, Environment, Session, StartOptions, StderrLog, hide_console, home_dir, missing};
use eframe::egui;
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

#[derive(Debug)]
pub struct ClaudeSession {
    child: Child,
    stdin: ChildStdin,
    events: Receiver<AgentEvent>,
}

impl ClaudeSession {
    pub fn start(
        claude: &Path,
        env: &Environment,
        options: &StartOptions,
        ctx: egui::Context,
    ) -> io::Result<Self> {
        std::fs::create_dir_all(&env.workdir)?;
        let mcp_config = env.workdir.join("mcp.json");
        std::fs::write(&mcp_config, mcp_config_json(env).to_string())?;

        let mut command = Command::new(claude);
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
        command.arg("--append-system-prompt").arg(&env.system_prompt);
        if let Some(model) = &options.model {
            command.args(["--model", model]);
        }
        if let Some(session) = &options.resume {
            command.args(["--resume", session]);
        }
        command
            .current_dir(&env.workdir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_console(&mut command);

        let mut child = command.spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| missing("stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| missing("stdout"))?;
        let stderr = StderrLog::collect(child.stderr.take().ok_or_else(|| missing("stderr"))?);

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
            let _ = sender.send(AgentEvent::Exited {
                stderr: stderr.text(),
            });
            ctx.request_repaint();
        });

        Ok(Self { child, stdin, events })
    }
}

impl Session for ClaudeSession {
    fn send(&mut self, text: &str) -> io::Result<()> {
        let message = json!({
            "type": "user",
            "message": { "role": "user", "content": [{ "type": "text", "text": text }] },
        });
        writeln!(self.stdin, "{message}")?;
        self.stdin.flush()
    }

    fn try_recv(&mut self) -> Option<AgentEvent> {
        self.events.try_recv().ok()
    }

    /// Headless Claude Code cannot interrupt a turn over stdin, so the process is ended; the next
    /// message resumes the conversation with `--resume`.
    fn cancel(&mut self) -> bool {
        false
    }
}

impl Drop for ClaudeSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn mcp_config_json(env: &Environment) -> Value {
    let mut vars = serde_json::Map::new();
    if let Some(database) = &env.database {
        vars.insert("FALOG_DB".into(), json!(database));
    }
    json!({
        "mcpServers": {
            "falog": { "type": "stdio", "command": env.mcp_server, "args": [], "env": vars }
        }
    })
}

/// Locates the native `claude` executable: on `PATH`, then in `~/.local/bin`.
pub fn find_claude() -> Option<PathBuf> {
    let name = format!("claude{}", std::env::consts::EXE_SUFFIX);
    let on_path = std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join(&name))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let home = home_dir().map(|home| home.join(".local").join("bin").join(&name));
    on_path
        .into_iter()
        .chain(home)
        .find(|candidate| candidate.is_file())
}
/// Maps one stream-json line to zero or more events. Unknown messages are ignored.
pub fn parse_line(line: &str) -> Vec<AgentEvent> {
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
            vec![AgentEvent::Ready {
                session_id: text(&message["session_id"]),
                model: message["model"].as_str().map(str::to_owned),
                tools_connected,
            }]
        }
        Some("stream_event") => {
            let event = &message["event"];
            if event["type"] == "content_block_delta" && event["delta"]["type"] == "text_delta" {
                vec![AgentEvent::TextDelta(text(&event["delta"]["text"]))]
            } else {
                Vec::new()
            }
        }
        Some("assistant") => blocks(&message)
            .filter_map(|block| match block["type"].as_str() {
                Some("text") => Some(AgentEvent::Text(text(&block["text"]))),
                Some("tool_use") => Some(AgentEvent::ToolUse {
                    id: text(&block["id"]),
                    name: text(&block["name"]),
                    input: block["input"].clone(),
                }),
                _ => None,
            })
            .collect(),
        Some("user") => blocks(&message)
            .filter(|block| block["type"] == "tool_result")
            .map(|block| AgentEvent::ToolResult {
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
            vec![AgentEvent::TurnFinished {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_session_init() {
        let line = r#"{"type":"system","subtype":"init","session_id":"abc","model":"claude-haiku-4-5","mcp_servers":[{"name":"falog","status":"connected"}]}"#;
        assert_eq!(
            parse_line(line),
            vec![AgentEvent::Ready {
                session_id: "abc".into(),
                model: Some("claude-haiku-4-5".into()),
                tools_connected: true
            }]
        );
    }

    #[test]
    fn parses_text_deltas_and_ignores_other_stream_events() {
        let delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Globex has"}}}"#;
        assert_eq!(
            parse_line(delta),
            vec![AgentEvent::TextDelta("Globex has".into())]
        );
        let thinking = r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"..."}}}"#;
        assert!(parse_line(thinking).is_empty());
    }

    #[test]
    fn parses_tool_use_and_result() {
        let tool_use = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"mcp__falog__add_note","input":{"id":4,"note":"x"}}]}}"#;
        assert_eq!(
            parse_line(tool_use),
            vec![AgentEvent::ToolUse {
                id: "t1".into(),
                name: "mcp__falog__add_note".into(),
                input: json!({"id": 4, "note": "x"})
            }]
        );
        let result = r#"{"type":"user","message":{"role":"user","content":[{"tool_use_id":"t1","type":"tool_result","content":[{"type":"text","text":"Note added to #4"}]}]}}"#;
        assert_eq!(
            parse_line(result),
            vec![AgentEvent::ToolResult {
                id: "t1".into(),
                text: "Note added to #4".into(),
                is_error: false
            }]
        );
    }

    #[test]
    fn parses_assistant_text_and_turn_end() {
        let text = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Done."}]}}"#;
        assert_eq!(parse_line(text), vec![AgentEvent::Text("Done.".into())]);
        let ok = r#"{"type":"result","subtype":"success","is_error":false,"result":"Done."}"#;
        assert_eq!(
            parse_line(ok),
            vec![AgentEvent::TurnFinished {
                is_error: false,
                message: None
            }]
        );
        let failed = r#"{"type":"result","subtype":"error_during_execution","is_error":true}"#;
        assert_eq!(
            parse_line(failed),
            vec![AgentEvent::TurnFinished {
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
