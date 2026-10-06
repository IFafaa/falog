use crate::components::{
    ButtonStyle, Placement, button, icon_button, modal, modal_header, segmented, switch,
};
use crate::fonts;
use crate::icons::Icon;
use crate::prefs::Prefs;
use crate::theme::{Theme, ThemeMode};
use eframe::egui::{self, Align, Frame, Layout, Margin, RichText, Ui};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum SettingsEvent {
    SetAutostart(bool),
    SetTheme(ThemeMode),
    Copied,
}

#[derive(Debug)]
pub struct SettingsDialog {
    pub autostart: bool,
    db_path: Option<PathBuf>,
    mcp_command: String,
}

impl SettingsDialog {
    pub fn new(autostart: bool, db_path: Option<&Path>) -> Self {
        let mcp_exe = std::env::current_exe()
            .ok()
            .and_then(|exe| {
                exe.parent()
                    .map(|dir| dir.join(format!("falog-mcp{}", std::env::consts::EXE_SUFFIX)))
            })
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "falog-mcp".into());
        Self {
            autostart,
            db_path: db_path.map(Path::to_path_buf),
            mcp_command: format!("claude mcp add --scope user falog -- \"{mcp_exe}\""),
        }
    }

    /// Returns the requested changes and whether the dialog should close.
    pub fn show(&mut self, ctx: &egui::Context, prefs: &mut Prefs) -> (Vec<SettingsEvent>, bool) {
        let theme = Theme::current(ctx);
        let mut events = Vec::new();
        let (closed, dismissed) = modal(ctx, "settings", 540.0, Placement::Center, |ui| {
            let closed = modal_header(ui, "Settings");
            Frame::none()
                .inner_margin(Margin::symmetric(16.0, 12.0))
                .show(ui, |ui| {
                    section(ui, theme, "General");
                    setting(
                        ui,
                        theme,
                        "Launch at startup",
                        "Open Falog when you sign in to Windows.",
                        |ui| {
                            if switch(ui, &mut self.autostart).changed() {
                                events.push(SettingsEvent::SetAutostart(self.autostart));
                            }
                        },
                    );

                    section(ui, theme, "Appearance");
                    setting(ui, theme, "Theme", "One Dark and One Light, from Zed.", |ui| {
                        let options: Vec<(ThemeMode, &str)> =
                            ThemeMode::ALL.iter().map(|m| (*m, m.label())).collect();
                        if segmented(ui, &mut prefs.theme, &options) {
                            events.push(SettingsEvent::SetTheme(prefs.theme));
                        }
                    });
                    setting(
                        ui,
                        theme,
                        "Older completed tasks",
                        "Keep showing tasks done more than a week ago on the board.",
                        |ui| {
                            switch(ui, &mut prefs.show_old_completed);
                        },
                    );

                    section(ui, theme, "Data");
                    let path = self
                        .db_path
                        .as_ref()
                        .map_or("in memory".into(), |p| p.display().to_string());
                    copy_row(ui, theme, Icon::Database, &path, &mut events);
                    if let Some(path) = &self.db_path
                        && button(ui, ButtonStyle::Ghost, None, "Show in Explorer").clicked()
                    {
                        let _ = std::process::Command::new("explorer")
                            .arg(format!("/select,{}", path.display()))
                            .spawn();
                    }

                    section(ui, theme, "Assistant (MCP)");
                    ui.label(
                        RichText::new("Let Claude create and update tasks. Register the server once:")
                            .size(13.0)
                            .color(theme.text_muted),
                    );
                    ui.add_space(4.0);
                    copy_row(ui, theme, Icon::Server, &self.mcp_command, &mut events);
                });
            closed
        });
        (events, closed || dismissed)
    }
}

fn section(ui: &mut Ui, theme: &Theme, title: &str) {
    ui.add_space(10.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .font(fonts::semibold(11.5))
            .color(theme.text_placeholder),
    );
    ui.add_space(4.0);
}

fn setting(ui: &mut Ui, theme: &Theme, title: &str, description: &str, add_control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(14.0).color(theme.text));
            ui.label(RichText::new(description).size(12.5).color(theme.text_muted));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), add_control);
    });
    ui.add_space(6.0);
}

fn copy_row(ui: &mut Ui, theme: &Theme, icon: Icon, text: &str, events: &mut Vec<SettingsEvent>) {
    Frame::none()
        .fill(theme.editor)
        .stroke((1.0, theme.border_variant))
        .rounding(4.0)
        .inner_margin(Margin::symmetric(8.0, 4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(icon.image(13.0, theme.icon_muted));
                // Copy button on the right; the text is truncated to the space left over.
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, Icon::Copy, "Copy").clicked() {
                        ui.ctx().copy_text(text.to_owned());
                        events.push(SettingsEvent::Copied);
                    }
                    let label = RichText::new(text).monospace().size(12.0).color(theme.text);
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        ui.add(egui::Label::new(label).truncate()).on_hover_text(text);
                    });
                });
            });
        });
}
