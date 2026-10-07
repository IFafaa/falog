use crate::assistant::agent::registry::AgentConfig;
use crate::assistant::agent_settings::{self, AgentsForm};
use crate::assistant::voice::{self, VoiceLanguage};
use crate::calendar::CalendarState;
use crate::components::{
    ButtonStyle, Placement, button, icon_button, modal, modal_header, segmented, switch,
};
use crate::fonts;
use crate::icons::Icon;
use crate::platform::reveal;
use crate::prefs::Prefs;
use crate::theme::{Theme, ThemeMode};
mod calendar;

use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Frame, Layout, Margin, Rect, Response, RichText,
    Rounding, ScrollArea, Sense, Stroke, Ui, UiBuilder, pos2, vec2,
};
use falog_calendar::Client;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum SettingsEvent {
    SetAutostart(bool),
    SetTheme(ThemeMode),
    VoiceEngineChanged,
    AgentsChanged(Vec<AgentConfig>),
    Copied,
    SaveGoogleClient(Client),
    ConnectGoogle,
    CancelGoogleConnect,
    RemoveGoogleAccount(String),
}

#[derive(Debug)]
pub struct SettingsDialog {
    pub autostart: bool,
    google_id: String,
    google_secret: String,
    db_path: Option<PathBuf>,
    mcp_command: String,
    agents: AgentsForm,
    pub tab: SettingsTab,
}

impl SettingsDialog {
    pub fn new(
        autostart: bool,
        db_path: Option<&Path>,
        custom_agents: Vec<AgentConfig>,
        google: &Client,
    ) -> Self {
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
            google_id: google.id.clone(),
            google_secret: google.secret.clone(),
            db_path: db_path.map(Path::to_path_buf),
            mcp_command: format!("claude mcp add --scope user falog -- \"{mcp_exe}\""),
            agents: AgentsForm::new(custom_agents),
            tab: SettingsTab::default(),
        }
    }

    /// Returns the requested changes and whether the dialog should close.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        prefs: &mut Prefs,
        calendar: &CalendarState,
    ) -> (Vec<SettingsEvent>, bool) {
        const WIDTH: f32 = 780.0;
        const HEIGHT: f32 = 520.0;
        const NAV_WIDTH: f32 = 184.0;
        let theme = Theme::current(ctx);
        let mut events = Vec::new();
        let (closed, dismissed) = modal(ctx, "settings", WIDTH, Placement::Center, |ui| {
            let closed = modal_header(ui, "Settings");
            let (body, _) = ui.allocate_exact_size(vec2(WIDTH, HEIGHT), Sense::hover());
            let nav = Rect::from_min_size(body.min, vec2(NAV_WIDTH, HEIGHT));
            ui.painter().rect_filled(
                nav,
                Rounding {
                    sw: 8.0,
                    ..Rounding::ZERO
                },
                theme.panel,
            );
            ui.painter().vline(
                nav.right(),
                body.y_range(),
                Stroke::new(1.0_f32, theme.border_variant),
            );
            ui.allocate_new_ui(
                UiBuilder::new()
                    .max_rect(nav.shrink2(vec2(8.0, 10.0)))
                    .layout(Layout::top_down(Align::Min)),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for tab in SettingsTab::ALL {
                        if nav_row(ui, theme, tab, self.tab == tab).clicked() {
                            self.tab = tab;
                        }
                    }
                },
            );
            let content = Rect::from_min_max(pos2(nav.right() + 1.0, body.top()), body.max);
            ui.allocate_new_ui(
                UiBuilder::new()
                    .max_rect(content)
                    .layout(Layout::top_down(Align::Min)),
                |ui| {
                    ScrollArea::vertical()
                        .id_salt(("settings", self.tab))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            Frame::none()
                                .inner_margin(Margin {
                                    left: 22.0,
                                    right: 22.0,
                                    top: 14.0,
                                    bottom: 18.0,
                                })
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(self.tab.label())
                                            .font(fonts::semibold(16.0))
                                            .color(theme.text),
                                    );
                                    ui.add_space(6.0);
                                    match self.tab {
                                        SettingsTab::General => {
                                            self.general_tab(ui, theme, prefs, &mut events)
                                        }
                                        SettingsTab::Appearance => {
                                            appearance_tab(ui, theme, prefs, &mut events)
                                        }
                                        SettingsTab::Assistant => self.assistant_tab(ui, theme, &mut events),
                                        SettingsTab::Voice => voice_tab(ui, theme, prefs, &mut events),
                                        SettingsTab::Calendar => {
                                            self.calendar_tab(ui, theme, calendar, &mut events)
                                        }
                                        SettingsTab::Data => self.data_tab(ui, theme, &mut events),
                                    }
                                });
                        });
                },
            );
            closed
        });
        (events, closed || dismissed)
    }

    fn general_tab(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        prefs: &mut Prefs,
        events: &mut Vec<SettingsEvent>,
    ) {
        setting(
            ui,
            theme,
            "Launch at startup",
            "Open Falog when you sign in to this computer.",
            |ui| {
                if switch(ui, &mut self.autostart).changed() {
                    events.push(SettingsEvent::SetAutostart(self.autostart));
                }
            },
        );
        setting(
            ui,
            theme,
            "Older completed tasks",
            "Keep showing tasks done more than a week ago on the board.",
            |ui| {
                switch(ui, &mut prefs.show_old_completed);
            },
        );
    }

    fn assistant_tab(&mut self, ui: &mut Ui, theme: &Theme, events: &mut Vec<SettingsEvent>) {
        ui.label(
            RichText::new("The assistant dock talks to these agents; pick one per thread in the composer.")
                .size(13.0)
                .color(theme.text_muted),
        );
        section(ui, theme, "Agents");
        if agent_settings::show(ui, theme, &mut self.agents) {
            events.push(SettingsEvent::AgentsChanged(self.agents.custom.clone()));
        }
    }

    fn data_tab(&mut self, ui: &mut Ui, theme: &Theme, events: &mut Vec<SettingsEvent>) {
        section(ui, theme, "Database");
        let path = self
            .db_path
            .as_ref()
            .map_or("in memory".into(), |p| p.display().to_string());
        copy_row(ui, theme, Icon::Database, &path, events);
        if let Some(path) = &self.db_path
            && button(ui, ButtonStyle::Ghost, None, reveal::LABEL).clicked()
        {
            let _ = reveal::reveal(path);
        }

        section(ui, theme, "Other assistants (MCP)");
        ui.label(
            RichText::new("Use Falog from Claude Code or any MCP client. Register the server once:")
                .size(13.0)
                .color(theme.text_muted),
        );
        ui.add_space(4.0);
        copy_row(ui, theme, Icon::Server, &self.mcp_command, events);
    }
}

fn appearance_tab(ui: &mut Ui, theme: &Theme, prefs: &mut Prefs, events: &mut Vec<SettingsEvent>) {
    setting(ui, theme, "Theme", "One Dark and One Light, from Zed.", |ui| {
        let options: Vec<(ThemeMode, &str)> = ThemeMode::ALL.iter().map(|m| (*m, m.label())).collect();
        if segmented(ui, &mut prefs.theme, &options) {
            events.push(SettingsEvent::SetTheme(prefs.theme));
        }
    });
}

fn voice_tab(ui: &mut Ui, theme: &Theme, prefs: &mut Prefs, events: &mut Vec<SettingsEvent>) {
    setting(ui, theme, "Voice language", "Language you dictate in.", |ui| {
        let options: Vec<(VoiceLanguage, &str)> =
            VoiceLanguage::ALL.iter().map(|l| (*l, l.label())).collect();
        segmented(ui, &mut prefs.voice_language, &options);
    });
    setting(
        ui,
        theme,
        "Send after dictation",
        "Send what you said right away instead of leaving it to edit.",
        |ui| {
            switch(ui, &mut prefs.send_after_dictation);
        },
    );
    if cfg!(feature = "gpu") {
        setting(
            ui,
            theme,
            "Speech recognition on GPU",
            "Much faster when the graphics card is free; turn off while gaming.",
            |ui| {
                if switch(ui, &mut prefs.voice_gpu).changed() {
                    events.push(SettingsEvent::VoiceEngineChanged);
                }
            },
        );
    }
    let model_status = if voice::model_path().is_file() {
        "Speech recognition: Whisper large-v3-turbo, on this computer.".to_owned()
    } else {
        format!(
            "Speech recognition runs on this computer; its model ({} MB) downloads the first time you dictate.",
            voice::MODEL_SIZE_MB
        )
    };
    ui.label(
        RichText::new(model_status)
            .size(12.5)
            .color(theme.text_placeholder),
    );
}

/// A settings page in the left column, like the sections of Zed's settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SettingsTab {
    #[default]
    General,
    Appearance,
    Assistant,
    Voice,
    Calendar,
    Data,
}

impl SettingsTab {
    pub const ALL: [Self; 6] = [
        Self::General,
        Self::Appearance,
        Self::Assistant,
        Self::Voice,
        Self::Calendar,
        Self::Data,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Appearance => "Appearance",
            Self::Assistant => "Assistant",
            Self::Voice => "Voice",
            Self::Calendar => "Calendar",
            Self::Data => "Data",
        }
    }

    const fn icon(self) -> Icon {
        match self {
            Self::General => Icon::Settings,
            Self::Appearance => Icon::Circle,
            Self::Assistant => Icon::Sparkle,
            Self::Voice => Icon::Mic,
            Self::Calendar => Icon::Calendar,
            Self::Data => Icon::Database,
        }
    }
}

fn nav_row(ui: &mut Ui, theme: &Theme, tab: SettingsTab, selected: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
    let fill = if selected {
        theme.ghost_selected
    } else if response.hovered() {
        theme.ghost_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    let icon = Rect::from_center_size(pos2(rect.left() + 16.0, rect.center().y), vec2(14.0, 14.0));
    let (text, tint) = if selected {
        (theme.text, theme.icon)
    } else {
        (theme.text_muted, theme.icon_muted)
    };
    tab.icon().paint(ui, icon, 14.0, tint);
    ui.painter().text(
        pos2(icon.right() + 9.0, rect.center().y),
        Align2::LEFT_CENTER,
        tab.label(),
        FontId::proportional(13.5),
        text,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
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
