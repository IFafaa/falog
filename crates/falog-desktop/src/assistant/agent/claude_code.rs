//! Claude Code in headless mode, the default agent.
//!
//! One long-lived `claude -p` process per conversation, speaking newline-delimited JSON on
//! stdin/stdout (`--input-format/--output-format stream-json`). All built-in tools are disabled;
//! the only tools available are the ones of the `falog` MCP server, so the assistant can manage
//! tasks and nothing else. It runs on the user's own Claude Code login.

use super::{
    AgentEvent, Choice, ConfigOption, Environment, OptionKind, Session, SlashCommand, StartOptions,
    StderrLog, Usage, hide_console, missing,
};
use eframe::egui;
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

/// The choice that leaves a setting to Claude Code (no flag is passed).
const DEFAULT: &str = "default";

/// Falog's own permission mode: falog tools run, anything else is refused without asking.
const FALOG_MODE: &str = "dontAsk";

/// Fast mode is turned on through settings in headless mode (`fastMode`).
pub const FAST_ON: &str = "on";
const FAST_OFF: &str = "off";

/// Models, effort levels and permission modes `claude --model/--effort/--permission-mode`
/// accept, plus fast mode. Aliases always mean the latest model of each family, so the list does
/// not go stale.
pub fn options() -> Vec<ConfigOption> {
    vec![
        ConfigOption {
            id: "model".into(),
            name: "Model".into(),
            kind: OptionKind::Model,
            current: Some(DEFAULT.into()),
            choices: vec![
                Choice::new(DEFAULT, "Default"),
                Choice::new("fable", "Fable"),
                Choice::new("opus", "Opus"),
                Choice::new("sonnet", "Sonnet"),
                Choice::new("haiku", "Haiku"),
            ],
            description: String::new(),
        },
        ConfigOption {
            id: "effort".into(),
            name: "Effort".into(),
            kind: OptionKind::Effort,
            current: Some(DEFAULT.into()),
            choices: vec![
                Choice::new(DEFAULT, "Default"),
                Choice::new("low", "Low"),
                Choice::new("medium", "Medium"),
                Choice::new("high", "High"),
                Choice::new("xhigh", "Extra high"),
                Choice::new("max", "Max"),
            ],
            description: String::new(),
        },
        ConfigOption {
            id: "mode".into(),
            name: "Mode".into(),
            kind: OptionKind::Mode,
            current: Some(DEFAULT.into()),
            choices: vec![
                Choice::new(DEFAULT, "Default"),
                Choice::new("acceptEdits", "Accept edits"),
                Choice::new("plan", "Plan"),
                Choice::new("auto", "Auto"),
                Choice::new("bypassPermissions", "Bypass permissions"),
            ],
            description: "Falog's tools are allowed in every mode; the others only matter for \
                          tools the assistant does not have."
                .into(),
        },
        ConfigOption {
            id: "fast".into(),
            name: "Fast mode".into(),
            kind: OptionKind::Fast,
            current: Some(FAST_OFF.into()),
            choices: vec![Choice::new(FAST_ON, "On"), Choice::new(FAST_OFF, "Off")],
            description: "Faster responses on Opus. It uses extra usage, which must be enabled for \
                          your account."
                .into(),
        },
    ]
}

/// Why Claude Code turned fast mode off, in words (the codes are `fast_mode_disabled_reason`).
fn fast_mode_unavailable(reason: &str) -> Option<&'static str> {
    Some(match reason {
        "free" => "it is not available on the free plan",
        "extra_usage_disabled" => "it needs extra usage to be enabled for this account",
        "model_not_allowed" => "it is not available for this model (Opus only)",
        "not_first_party" => "it is not available on this API provider",
        "disabled_by_env" => "it is disabled by the environment",
        "network_error" => "eligibility could not be checked (network error)",
        _ => return None,
    })
}

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
            "--setting-sources",
            "",
        ]);
        let mode = options.mode.as_deref().filter(|m| *m != DEFAULT);
        command.args(["--permission-mode", mode.unwrap_or(FALOG_MODE)]);
        let fast = options.fast.as_deref() == Some(FAST_ON);
        if fast {
            command.args(["--settings", r#"{"fastMode":true}"#]);
        }
        command.arg("--mcp-config").arg(&mcp_config);
        command.arg("--append-system-prompt").arg(&env.system_prompt);
        if let Some(model) = options.model.as_deref().filter(|m| *m != DEFAULT) {
            command.args(["--model", model]);
        }
        if let Some(effort) = options.effort.as_deref().filter(|e| *e != DEFAULT) {
            command.args(["--effort", effort]);
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
            let mut fast_refused = false;
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let mut parsed = parse_line(&line);
                // Say once why fast mode was asked for but not used.
                if let (true, false, Some(reason)) = (fast, fast_refused, fast_mode_refusal(&line)) {
                    fast_refused = true;
                    parsed.push(AgentEvent::Notice(format!("Fast mode is off: {reason}.")));
                }
                for event in parsed {
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

/// Locates the `claude` executable: on `PATH`, then in the usual install folders (see
/// [`find_executable`](crate::platform::paths::find_executable)).
pub fn find_claude() -> Option<PathBuf> {
    crate::platform::paths::find_executable("claude")
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
            vec![
                AgentEvent::Ready {
                    session_id: text(&message["session_id"]),
                    model: message["model"].as_str().map(str::to_owned),
                    tools_connected,
                },
                AgentEvent::Commands(init_commands(&message)),
            ]
        }
        Some("system") if message["subtype"] == "commands_changed" => {
            let commands = message["commands"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|command| {
                    let name = command["name"].as_str()?;
                    usable(name).then(|| SlashCommand {
                        name: name.to_owned(),
                        description: text(&command["description"]),
                        hint: text(&command["argumentHint"]),
                    })
                })
                .collect();
            vec![AgentEvent::Commands(commands)]
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
            let finished = AgentEvent::TurnFinished {
                is_error,
                message: if is_error { detail } else { None },
            };
            match context_usage(&message) {
                Some(usage) => vec![AgentEvent::Usage(usage), finished],
                None => vec![finished],
            }
        }
        _ => Vec::new(),
    }
}

/// Tokens in context after the turn: everything sent to and produced by its last model call
/// (`usage.iterations`), against the context window of the model that answered.
fn context_usage(result: &Value) -> Option<Usage> {
    let count = |usage: &Value| -> u64 {
        [
            "input_tokens",
            "cache_creation_input_tokens",
            "cache_read_input_tokens",
            "output_tokens",
        ]
        .iter()
        .filter_map(|key| usage[*key].as_u64())
        .sum()
    };
    let last_call = result["usage"]["iterations"]
        .as_array()
        .and_then(|calls| calls.last())
        .unwrap_or(&result["usage"]);
    let size = result["modelUsage"]
        .as_object()?
        .values()
        .filter_map(|model| model["contextWindow"].as_u64())
        .max()?;
    // Local commands (`/context`) make no model call and report nothing in context.
    let used = count(last_call);
    (used > 0).then_some(Usage { used, size })
}

/// Why fast mode is off, when a `result` says it was refused for a reason worth telling.
fn fast_mode_refusal(line: &str) -> Option<&'static str> {
    let message: Value = serde_json::from_str(line).ok()?;
    if message["type"] != "result" || message["fast_mode_state"] == "on" {
        return None;
    }
    fast_mode_unavailable(message["fast_mode_disabled_reason"].as_str()?)
}

/// The `init` message only names the commands; descriptions follow in `commands_changed`.
/// Commands that only work in the interactive terminal are left out.
fn init_commands(init: &Value) -> Vec<SlashCommand> {
    let names = |key: &str| -> Vec<&str> {
        init[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect()
    };
    let terminal_only = names("terminal_slash_commands");
    names("slash_commands")
        .into_iter()
        .filter(|name| usable(name) && !terminal_only.contains(name))
        .map(|name| SlashCommand {
            name: name.to_owned(),
            description: String::new(),
            hint: String::new(),
        })
        .collect()
}

/// Names starting with `__` are Claude Code internals.
fn usable(name: &str) -> bool {
    !name.is_empty() && !name.starts_with("__")
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
            vec![
                AgentEvent::Ready {
                    session_id: "abc".into(),
                    model: Some("claude-haiku-4-5".into()),
                    tools_connected: true
                },
                AgentEvent::Commands(Vec::new())
            ]
        );
    }

    #[test]
    fn parses_slash_commands() {
        let init = r#"{"type":"system","subtype":"init","session_id":"s","slash_commands":["compact","doctor","__remote","context"],"terminal_slash_commands":["doctor"]}"#;
        let names: Vec<String> = match &parse_line(init)[1] {
            AgentEvent::Commands(commands) => commands.iter().map(|c| c.name.clone()).collect(),
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(names, vec!["compact", "context"]);

        let changed = r#"{"type":"system","subtype":"commands_changed","commands":[{"name":"design","description":"Make a design","argumentHint":"[what to design]","builtin":true},{"name":"__x","description":""}]}"#;
        assert_eq!(
            parse_line(changed),
            vec![AgentEvent::Commands(vec![SlashCommand {
                name: "design".into(),
                description: "Make a design".into(),
                hint: "[what to design]".into(),
            }])]
        );
    }

    #[test]
    fn offers_models_and_effort_levels() {
        let options = options();
        let model = options.iter().find(|o| o.kind == OptionKind::Model).unwrap();
        assert!(model.choices.iter().any(|c| c.value == "opus"));
        assert_eq!(model.label("haiku"), "Haiku");
        let effort = options.iter().find(|o| o.kind == OptionKind::Effort).unwrap();
        assert_eq!(effort.label("xhigh"), "Extra high");
    }

    #[test]
    fn parses_text_deltas_and_ignores_other_stream_events() {
        let delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Home has"}}}"#;
        assert_eq!(parse_line(delta), vec![AgentEvent::TextDelta("Home has".into())]);
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
    fn reads_context_usage_from_the_last_model_call() {
        let result = r#"{"type":"result","subtype":"success","is_error":false,
            "usage":{"input_tokens":20,"output_tokens":150,"iterations":[
                {"input_tokens":10,"cache_creation_input_tokens":6000,"cache_read_input_tokens":0,"output_tokens":50},
                {"input_tokens":10,"cache_creation_input_tokens":100,"cache_read_input_tokens":6000,"output_tokens":100}]},
            "modelUsage":{"claude-haiku-4-5":{"contextWindow":200000}}}"#
            .replace('\n', "");
        assert_eq!(
            parse_line(&result)[0],
            AgentEvent::Usage(Usage {
                used: 6210,
                size: 200_000
            })
        );
        assert!(matches!(parse_line(&result)[1], AgentEvent::TurnFinished { .. }));
        let local = r#"{"type":"result","is_error":false,"usage":{"input_tokens":0,"output_tokens":0},"modelUsage":{"m":{"contextWindow":200000}}}"#;
        assert_eq!(
            parse_line(local).len(),
            1,
            "a local command leaves the ring as it was"
        );
    }

    #[test]
    fn explains_why_fast_mode_was_refused() {
        let refused =
            r#"{"type":"result","fast_mode_state":"off","fast_mode_disabled_reason":"extra_usage_disabled"}"#;
        assert!(fast_mode_refusal(refused).unwrap().contains("extra usage"));
        let opt_in =
            r#"{"type":"result","fast_mode_state":"off","fast_mode_disabled_reason":"sdk_opt_in_required"}"#;
        assert_eq!(fast_mode_refusal(opt_in), None);
        let on = r#"{"type":"result","fast_mode_state":"on"}"#;
        assert_eq!(fast_mode_refusal(on), None);
    }

    #[test]
    fn ignores_garbage() {
        assert!(parse_line("not json").is_empty());
        assert!(parse_line(r#"{"type":"rate_limit_event"}"#).is_empty());
    }
}
