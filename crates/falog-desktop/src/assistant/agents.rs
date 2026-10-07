//! What Falog remembers about agents, in `<data dir>/assistant/agents.json`: the user's custom
//! agents, the last agent/model/effort picked (for new threads), and what each agent reported the
//! last time it ran (its settings and commands), so the pickers and `/` work before it starts.

use super::agent::registry::{AgentConfig, AgentId};
use super::agent::{ConfigOption, SlashCommand};
use super::thread::Settings;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SavedAgents {
    #[serde(default)]
    pub custom: Vec<AgentConfig>,
    #[serde(default)]
    pub last: Option<Settings>,
    #[serde(default)]
    pub known: BTreeMap<AgentId, Known>,
}

/// What an agent reported last time.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Known {
    #[serde(default)]
    pub options: Vec<ConfigOption>,
    #[serde(default)]
    pub commands: Vec<SlashCommand>,
}

impl SavedAgents {
    /// A missing or unreadable file means nothing is remembered yet.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Writes atomically (temporary file, then rename).
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let partial = path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        std::fs::write(&partial, json)?;
        std::fs::rename(&partial, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::agent::{Choice, OptionKind};

    #[test]
    fn saves_and_loads() {
        let dir = std::env::temp_dir().join(format!("falog-agents-{}", std::process::id()));
        let path = dir.join("agents.json");
        assert_eq!(SavedAgents::load(&path), SavedAgents::default());

        let mut saved = SavedAgents {
            last: Some(Settings {
                agent: AgentId("gemini".into()),
                model: Some("flash".into()),
                effort: None,
            }),
            ..SavedAgents::default()
        };
        saved.known.insert(
            AgentId("gemini".into()),
            Known {
                options: vec![ConfigOption {
                    id: "model".into(),
                    name: "Model".into(),
                    kind: OptionKind::Model,
                    current: Some("pro".into()),
                    choices: vec![Choice {
                        value: "flash".into(),
                        name: "Flash".into(),
                    }],
                }],
                commands: Vec::new(),
            },
        );
        saved.save(&path).unwrap();
        assert_eq!(SavedAgents::load(&path), saved);
        let _ = std::fs::remove_dir_all(dir);
    }
}
