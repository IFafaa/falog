//! Which agents exist and how to start them: built-in presets plus the user's custom ACP agents.

use super::acp::{AcpCommand, AcpSession};
use super::claude_code::{self, ClaudeSession};
use super::{Environment, Session, StartOptions, home_dir};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Identifies an agent: a preset (`claude-code`, `claude`, `gemini`, `codex`) or a custom one.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(pub String);

/// The default agent: Claude Code in headless mode.
pub const CLAUDE_CODE: &str = "claude-code";

impl Default for AgentId {
    fn default() -> Self {
        Self(CLAUDE_CODE.into())
    }
}

/// How Falog talks to an agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    /// `claude --print` with stream-json on stdin/stdout.
    ClaudeCode,
    /// The Agent Client Protocol.
    Acp,
}

/// How to run an agent. Custom agents are saved like this.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: AgentId,
    pub name: String,
    /// A program on `PATH` or a full path.
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Agent {
    pub config: AgentConfig,
    pub protocol: Protocol,
    /// Presets cannot be removed.
    pub builtin: bool,
    /// What to do when the command is missing or signed out.
    pub setup_hint: String,
}

impl Agent {
    pub fn id(&self) -> &AgentId {
        &self.config.id
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// The command line, for display (`npx -y @agentclientprotocol/codex-acp`).
    pub fn command_line(&self) -> String {
        std::iter::once(self.config.command.as_str())
            .chain(self.config.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn preset(id: &str, name: &str, protocol: Protocol, command: &str, args: &[&str], hint: &str) -> Self {
        Self {
            config: AgentConfig {
                id: AgentId(id.into()),
                name: name.into(),
                command: command.into(),
                args: args.iter().map(|a| (*a).to_owned()).collect(),
                env: BTreeMap::new(),
            },
            protocol,
            builtin: true,
            setup_hint: hint.into(),
        }
    }

    fn custom(config: AgentConfig) -> Self {
        let hint = format!("Check the command of {} in Settings.", config.name);
        Self {
            config,
            protocol: Protocol::Acp,
            builtin: false,
            setup_hint: hint,
        }
    }

    /// Starts a session. Errors are full sentences for the thread.
    pub fn start(
        &self,
        env: &Environment,
        options: &StartOptions,
        ctx: &egui::Context,
    ) -> Result<Box<dyn Session>, String> {
        let name = self.name();
        let program = self.program().ok_or_else(|| {
            format!(
                "{name} was not found: `{}` is not installed or not on PATH. {}",
                self.config.command, self.setup_hint
            )
        })?;
        let started = match self.protocol {
            Protocol::ClaudeCode => ClaudeSession::start(&program, env, options, ctx.clone())
                .map(|session| Box::new(session) as Box<dyn Session>),
            Protocol::Acp => {
                let command = AcpCommand {
                    program,
                    args: self.config.args.clone(),
                    env: self
                        .config
                        .env
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                };
                let ctx = ctx.clone();
                AcpSession::start(&command, env, options, name, move || ctx.request_repaint())
                    .map(|session| Box::new(session) as Box<dyn Session>)
            }
        };
        started.map_err(|err| format!("Could not start {name}: {err}"))
    }

    fn program(&self) -> Option<PathBuf> {
        match self.protocol {
            // The native installer puts `claude` in `~/.local/bin`, which may not be on PATH yet.
            Protocol::ClaudeCode if self.config.command == "claude" => claude_code::find_claude(),
            _ => find_executable(&self.config.command),
        }
    }
}

/// The built-in agents, Claude Code first. ACP adapters for Claude and Codex run through `npx`,
/// which downloads them on first use.
pub fn presets() -> Vec<Agent> {
    vec![
        Agent::preset(
            CLAUDE_CODE,
            "Claude Code",
            Protocol::ClaudeCode,
            "claude",
            &[],
            "Install it from claude.com/claude-code, then run `claude` once in a terminal to sign in.",
        ),
        Agent::preset(
            "claude",
            "Claude (ACP)",
            Protocol::Acp,
            "npx",
            &["-y", "@agentclientprotocol/claude-agent-acp"],
            "It needs Node.js 22 or later (nodejs.org) and Claude Code signed in.",
        ),
        Agent::preset(
            "gemini",
            "Gemini CLI",
            Protocol::Acp,
            "gemini",
            &["--acp"],
            "Install it with `npm install -g @google/gemini-cli`, then run `gemini` once in a terminal to sign in.",
        ),
        Agent::preset(
            "codex",
            "Codex",
            Protocol::Acp,
            "npx",
            &["-y", "@agentclientprotocol/codex-acp"],
            "It needs Node.js (nodejs.org) and Codex signed in (`codex login`).",
        ),
    ]
}

/// Presets followed by custom agents.
pub fn all(custom: &[AgentConfig]) -> Vec<Agent> {
    let mut agents = presets();
    agents.extend(
        custom
            .iter()
            .filter(|config| !agents_contain(&presets(), &config.id))
            .cloned()
            .map(Agent::custom),
    );
    agents
}

pub fn find(custom: &[AgentConfig], id: &AgentId) -> Option<Agent> {
    all(custom).into_iter().find(|agent| agent.id() == id)
}

fn agents_contain(agents: &[Agent], id: &AgentId) -> bool {
    agents.iter().any(|agent| agent.id() == id)
}

/// Resolves a command like a shell would: a path as is, otherwise each `PATH` directory, trying
/// the `PATHEXT` extensions on Windows (`npx` is `npx.cmd` there).
pub fn find_executable(command: &str) -> Option<PathBuf> {
    let path = Path::new(command);
    if path.components().count() > 1 || path.is_absolute() {
        return path.is_file().then(|| path.to_path_buf());
    }
    let extensions: Vec<String> = if cfg!(windows) {
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
        let mut list: Vec<String> = pathext.split(';').map(str::to_lowercase).collect();
        // A name with an extension is used as is; without one, only executable extensions count,
        // because npm also puts extensionless shell scripts next to the `.cmd` shims.
        if path.extension().is_some() {
            list.insert(0, String::new());
        }
        list
    } else {
        vec![String::new()]
    };
    let dirs = std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .unwrap_or_default();
    dirs.iter()
        .chain(home_dir().map(|home| home.join(".local").join("bin")).iter())
        .flat_map(|dir| {
            extensions
                .iter()
                .map(move |ext| dir.join(format!("{command}{ext}")))
        })
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_code_is_the_default_preset() {
        let agents = presets();
        assert_eq!(agents[0].id(), &AgentId::default());
        assert_eq!(agents[0].protocol, Protocol::ClaudeCode);
        assert!(agents.iter().skip(1).all(|a| a.protocol == Protocol::Acp));
        let gemini = agents.iter().find(|a| a.id().0 == "gemini").unwrap();
        assert_eq!(gemini.command_line(), "gemini --acp");
    }

    #[test]
    fn lists_custom_agents_after_the_presets() {
        let custom = vec![AgentConfig {
            id: AgentId("custom-goose".into()),
            name: "Goose".into(),
            command: "goose".into(),
            args: vec!["acp".into()],
            env: BTreeMap::new(),
        }];
        let agents = all(&custom);
        assert_eq!(agents.last().unwrap().name(), "Goose");
        assert!(!agents.last().unwrap().builtin);
        assert!(find(&custom, &AgentId("custom-goose".into())).is_some());
        assert!(find(&custom, &AgentId("missing".into())).is_none());
    }

    #[test]
    fn finds_executables_on_path() {
        let name = if cfg!(windows) { "cmd" } else { "sh" };
        assert!(find_executable(name).is_some());
        assert!(find_executable("surely-not-a-real-program-xyz").is_none());
    }
}
