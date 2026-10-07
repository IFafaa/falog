//! The "Agents" section of Settings: the agents threads can talk to, and a form to add one.

use super::agent::registry::{self, Agent, AgentConfig};
use crate::components::{ButtonStyle, button, icon_button};
use crate::icons::Icon;
use crate::theme::Theme;
use eframe::egui::{self, Align, Frame, Layout, Margin, RichText, TextEdit, Ui};
use std::collections::BTreeMap;

/// The custom agents being edited and the add form.
#[derive(Debug, Default)]
pub struct AgentsForm {
    pub custom: Vec<AgentConfig>,
    /// Whether each agent's command was found, worked out once when Settings opens.
    installed: Vec<(registry::AgentId, bool)>,
    name: String,
    command: String,
    env: String,
    error: Option<String>,
    /// The add form is showing.
    open: bool,
}

impl AgentsForm {
    pub fn new(custom: Vec<AgentConfig>) -> Self {
        let installed = registry::all(&custom)
            .iter()
            .map(|agent| (agent.id().clone(), agent.is_installed()))
            .collect();
        Self {
            custom,
            installed,
            ..Self::default()
        }
    }

    fn installed(&self, agent: &Agent) -> Option<bool> {
        self.installed
            .iter()
            .find(|(id, _)| id == agent.id())
            .map(|(_, found)| *found)
    }

    /// Validates the form and adds the agent. Errors are shown under the form.
    fn add(&mut self) -> bool {
        let name = self.name.trim();
        let mut words = split_command(&self.command).into_iter();
        let (Some(command), false) = (words.next(), name.is_empty()) else {
            self.error = Some("Give the agent a name and the command that starts it in ACP mode.".into());
            return false;
        };
        let env = match parse_env(&self.env) {
            Ok(env) => env,
            Err(entry) => {
                self.error = Some(format!("\"{entry}\" is not a NAME=value pair."));
                return false;
            }
        };
        let config = AgentConfig {
            id: registry::custom_id(name, &self.custom),
            name: name.to_owned(),
            command,
            args: words.collect(),
            env,
        };
        let found = Agent::from_custom(config.clone()).is_installed();
        self.installed.push((config.id.clone(), found));
        self.custom.push(config);
        self.name.clear();
        self.command.clear();
        self.env.clear();
        self.error = None;
        true
    }
}

/// Draws the agent list and the add form. Returns whether the custom agents changed.
pub fn show(ui: &mut Ui, theme: &Theme, form: &mut AgentsForm) -> bool {
    let mut changed = false;
    ui.label(
        RichText::new(
            "Each thread talks to one agent, on your own subscription. Claude Code is built in; the others \
             speak the Agent Client Protocol (ACP).",
        )
        .size(12.5)
        .color(theme.text_muted),
    );
    ui.add_space(4.0);
    let mut remove = None;
    for agent in registry::all(&form.custom) {
        if agent_row(ui, theme, &agent, form.installed(&agent)) {
            remove = Some(agent.id().clone());
        }
    }
    if let Some(id) = remove {
        form.custom.retain(|config| config.id != id);
        changed = true;
    }

    ui.add_space(4.0);
    if !form.open {
        if button(ui, ButtonStyle::Ghost, Some(Icon::Plus), "Add agent").clicked() {
            form.open = true;
        }
        return changed;
    }
    Frame::none()
        .stroke((1.0, theme.border_variant))
        .rounding(6.0)
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            field(ui, theme, "Name", &mut form.name, "My agent");
            field(
                ui,
                theme,
                "Command",
                &mut form.command,
                "npx -y my-acp-agent --acp",
            );
            field(ui, theme, "Environment", &mut form.env, "API_KEY=...; OTHER=...");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if button(ui, ButtonStyle::Filled, Some(Icon::Plus), "Add").clicked() && form.add() {
                    form.open = false;
                    changed = true;
                }
                if button(ui, ButtonStyle::Ghost, None, "Cancel").clicked() {
                    form.open = false;
                    form.error = None;
                }
                if let Some(error) = &form.error {
                    ui.label(RichText::new(error).size(12.5).color(theme.error));
                }
            });
        });
    changed
}

/// `Name  command line ... Installed 🗑`, on one line. Returns whether remove was clicked.
fn agent_row(ui: &mut Ui, theme: &Theme, agent: &Agent, installed: Option<bool>) -> bool {
    let mut remove = false;
    ui.horizontal(|ui| {
        ui.set_min_height(24.0);
        ui.label(RichText::new(agent.name()).size(13.5).color(theme.text));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if !agent.builtin && icon_button(ui, Icon::Trash, "Remove agent").clicked() {
                remove = true;
            }
            let (text, color) = match installed {
                Some(true) => ("Installed", theme.success),
                Some(false) => ("Not found", theme.text_placeholder),
                None => ("", theme.text_placeholder),
            };
            ui.label(RichText::new(text).size(12.0).color(color))
                .on_hover_text(&agent.setup_hint);
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let command = RichText::new(agent.command_line())
                    .monospace()
                    .size(11.5)
                    .color(theme.text_muted);
                ui.add(egui::Label::new(command).truncate())
                    .on_hover_text(agent.command_line());
            });
        });
    });
    remove
}

fn field(ui: &mut Ui, theme: &Theme, label: &str, value: &mut String, hint: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(90.0, 20.0), egui::Sense::hover());
        ui.painter().text(
            rect.left_center(),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(13.0),
            theme.text_muted,
        );
        ui.add(
            TextEdit::singleline(value)
                .desired_width(f32::INFINITY)
                .hint_text(RichText::new(hint).color(theme.text_placeholder)),
        );
    });
}

/// Splits a command line into words; double quotes keep spaces (`"C:\My Tools\agent.exe" --acp`).
pub fn split_command(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let (mut quoted, mut started) = (false, false);
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            c => {
                word.push(c);
                started = true;
            }
        }
    }
    if started {
        words.push(word);
    }
    words
}

/// `NAME=value` pairs separated by `;` or new lines. Returns the first malformed entry on error.
pub fn parse_env(text: &str) -> Result<BTreeMap<String, String>, String> {
    text.split([';', '\n'])
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| match entry.split_once('=') {
            Some((name, value)) if !name.trim().is_empty() => {
                Ok((name.trim().to_owned(), value.trim().to_owned()))
            }
            _ => Err(entry.to_owned()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_command_lines() {
        assert_eq!(split_command("npx -y  pkg"), vec!["npx", "-y", "pkg"]);
        assert_eq!(
            split_command(r#""C:\My Tools\agent.exe" --acp """#),
            vec![r"C:\My Tools\agent.exe", "--acp", ""]
        );
        assert!(split_command("   ").is_empty());
    }

    #[test]
    fn parses_environment_pairs() {
        let env = parse_env("A=1; B = two words\nC=").unwrap();
        assert_eq!(env["A"], "1");
        assert_eq!(env["B"], "two words");
        assert_eq!(env["C"], "");
        assert_eq!(parse_env("oops"), Err("oops".to_owned()));
        assert!(parse_env("").unwrap().is_empty());
    }

    #[test]
    fn adds_and_validates_custom_agents() {
        let mut form = AgentsForm::new(Vec::new());
        assert!(!form.add());
        assert!(form.error.is_some());
        form.name = "Goose".into();
        form.command = "goose acp --verbose".into();
        form.env = "GOOSE_MODE=chat".into();
        assert!(form.add());
        let added = &form.custom[0];
        assert_eq!(added.id.0, "custom-goose");
        assert_eq!(added.command, "goose");
        assert_eq!(added.args, vec!["acp", "--verbose"]);
        assert_eq!(added.env["GOOSE_MODE"], "chat");
        assert!(form.name.is_empty(), "the form is cleared");
    }
}
