//! End-to-end checks against real agents, run by hand (they need the agent installed and signed in,
//! and cost a turn):
//!
//! ```powershell
//! cargo build -p falog-mcp
//! $env:FALOG_LIVE_AGENTS = "claude-code,claude"   # agent ids; default: claude-code
//! cargo test -p falog-desktop live -- --ignored --nocapture
//! ```

use super::registry;
use super::{AgentEvent, Environment, OptionKind, StartOptions};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const PROMPT: &str = "Call create_area to add an area named Live check, then call list_areas, then reply with only the number of areas.";

fn environment(name: &str) -> Environment {
    let deps = std::env::current_exe().unwrap();
    let target = deps.parent().unwrap().parent().unwrap();
    let mcp_server = target.join(format!("falog-mcp{}", std::env::consts::EXE_SUFFIX));
    assert!(
        mcp_server.is_file(),
        "build falog-mcp first: {}",
        mcp_server.display()
    );
    let dir: PathBuf = std::env::temp_dir().join(format!("falog-live-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    Environment {
        mcp_server,
        database: Some(dir.join("falog.db")),
        workdir: dir.join("assistant"),
        system_prompt: include_str!("../../../assets/assistant-prompt.md").to_owned(),
    }
}

/// Runs one turn and returns everything the agent reported.
fn turn(session: &mut dyn super::Session, text: &str) -> Vec<AgentEvent> {
    session.send(text).unwrap();
    let deadline = Instant::now() + Duration::from_secs(240);
    let mut events = Vec::new();
    while Instant::now() < deadline {
        match session.try_recv() {
            Some(event) => {
                println!("{event:?}");
                let done = matches!(event, AgentEvent::TurnFinished { .. } | AgentEvent::Exited { .. });
                events.push(event);
                if done {
                    return events;
                }
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    panic!("the turn did not finish: {events:?}");
}

#[test]
#[ignore = "talks to real agents"]
fn live_agents_answer_with_falog_tools() {
    let ids = std::env::var("FALOG_LIVE_AGENTS").unwrap_or_else(|_| "claude-code".into());
    for id in ids.split(',').map(str::trim) {
        println!("== {id}");
        let agent = registry::find(&[], &registry::AgentId(id.into())).expect("unknown agent id");
        let env = environment(id);
        let ctx = eframe::egui::Context::default();
        let options = StartOptions {
            effort: (id == registry::CLAUDE_CODE).then(|| "low".into()),
            model: (id == registry::CLAUDE_CODE).then(|| "haiku".into()),
            resume: None,
        };
        let mut session = agent.start(&env, &options, &ctx).unwrap();
        let events = turn(session.as_mut(), PROMPT);

        let session_id = events.iter().find_map(|e| match e {
            AgentEvent::Ready { session_id, .. } => Some(session_id.clone()),
            _ => None,
        });
        assert!(session_id.is_some(), "{id}: no session");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AgentEvent::ToolUse { name, .. } if name.ends_with("list_areas"))),
            "{id}: list_areas was not called"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AgentEvent::ToolUse { name, .. } if name.ends_with("create_area"))),
            "{id}: create_area was not called"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AgentEvent::ToolResult { is_error: false, .. })),
            "{id}: no tool result"
        );
        assert!(
            matches!(
                events.last(),
                Some(AgentEvent::TurnFinished { is_error: false, .. })
            ),
            "{id}: turn failed"
        );
        let commands = events.iter().rev().find_map(|e| match e {
            AgentEvent::Commands(commands) => Some(commands.len()),
            _ => None,
        });
        println!("{id}: {commands:?} commands");

        // Slash commands are sent as messages and run by the agent.
        let events = turn(session.as_mut(), "/context");
        assert!(
            matches!(
                events.last(),
                Some(AgentEvent::TurnFinished { is_error: false, .. })
            ),
            "{id}: /context failed"
        );

        // The conversation survives a restart.
        drop(session);
        let resumed = StartOptions {
            resume: session_id,
            ..options
        };
        let mut session = agent.start(&env, &resumed, &ctx).unwrap();
        let events = turn(
            session.as_mut(),
            "What number did you just tell me? Reply with only it.",
        );
        assert!(
            matches!(
                events.last(),
                Some(AgentEvent::TurnFinished { is_error: false, .. })
            ),
            "{id}: resumed turn failed"
        );
        if let Some(AgentEvent::Options(options)) =
            events.iter().find(|e| matches!(e, AgentEvent::Options(_)))
        {
            let kinds: Vec<OptionKind> = options.iter().map(|o| o.kind).collect();
            println!("{id}: options {kinds:?}");
        }
    }
}
