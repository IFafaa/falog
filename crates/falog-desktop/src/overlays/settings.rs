use crate::assistant::AssistantModel;
use crate::assistant::voice::{self, VoiceLanguage};
use crate::calendar::CalendarState;
use crate::components::{
    ButtonStyle, Placement, button, icon_button, modal, modal_header, segmented, switch,
};
use crate::fonts;
use crate::icons::Icon;
use crate::prefs::Prefs;
use crate::theme::{Theme, ThemeMode};
use eframe::egui::{self, Align, Frame, Layout, Margin, RichText, TextEdit, Ui};
use falog_calendar::Client;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum SettingsEvent {
    SetAutostart(bool),
    SetTheme(ThemeMode),
    AssistantModelChanged,
    VoiceEngineChanged,
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
}

impl SettingsDialog {
    pub fn new(autostart: bool, db_path: Option<&Path>, google: &Client) -> Self {
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
        }
    }

    /// Returns the requested changes and whether the dialog should close.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        prefs: &mut Prefs,
        calendar: &CalendarState,
    ) -> (Vec<SettingsEvent>, bool) {
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

                    self.google(ui, theme, calendar, &mut events);

                    section(ui, theme, "Assistant");
                    setting(ui, theme, "Model", "Claude model used in the assistant panel.", |ui| {
                        let options: Vec<(AssistantModel, &str)> =
                            AssistantModel::ALL.iter().map(|m| (*m, m.label())).collect();
                        if segmented(ui, &mut prefs.assistant_model, &options) {
                            events.push(SettingsEvent::AssistantModelChanged);
                        }
                    });
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
                    ui.label(RichText::new(model_status).size(12.5).color(theme.text_placeholder));

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

                    section(ui, theme, "Other assistants (MCP)");
                    ui.label(
                        RichText::new("Use Falog from Claude Code or any MCP client. Register the server once:")
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

impl SettingsDialog {
    /// Your own OAuth client (Google does not let open source apps ship one), then the accounts.
    fn google(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        calendar: &CalendarState,
        events: &mut Vec<SettingsEvent>,
    ) {
        section(ui, theme, "Google Calendar");
        let muted = |ui: &mut Ui, text: &str| {
            ui.label(RichText::new(text).size(12.5).color(theme.text_muted));
        };
        if !calendar.has_client() {
            muted(
                ui,
                "Falog reads your meetings through your own Google OAuth client. Create it once:",
            );
            ui.horizontal_wrapped(|ui| {
                muted(ui, "1. In Google Cloud, create a project and enable the");
                ui.hyperlink_to(
                    RichText::new("Google Calendar API").size(12.5),
                    "https://console.cloud.google.com/apis/library/calendar-json.googleapis.com",
                );
            });
            ui.horizontal_wrapped(|ui| {
                muted(ui, "2. In the");
                ui.hyperlink_to(
                    RichText::new("OAuth consent screen").size(12.5),
                    "https://console.cloud.google.com/auth/overview",
                );
                muted(
                    ui,
                    "pick External, then publish the app (testing logins expire every 7 days).",
                );
            });
            ui.horizontal_wrapped(|ui| {
                muted(ui, "3. In");
                ui.hyperlink_to(
                    RichText::new("Credentials").size(12.5),
                    "https://console.cloud.google.com/apis/credentials",
                );
                muted(
                    ui,
                    "create an OAuth client ID of type Desktop app and paste it here.",
                );
            });
            ui.add_space(6.0);
        }
        let field = |ui: &mut Ui, label: &str, value: &mut String, password: bool| {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [96.0, 24.0],
                    egui::Label::new(RichText::new(label).size(13.0).color(theme.text)),
                );
                ui.add(
                    TextEdit::singleline(value)
                        .password(password)
                        .desired_width(ui.available_width()),
                );
            });
        };
        field(ui, "Client ID", &mut self.google_id, false);
        field(ui, "Client secret", &mut self.google_secret, true);
        let edited = Client {
            id: self.google_id.trim().to_owned(),
            secret: self.google_secret.trim().to_owned(),
        };
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let changed = edited != calendar.config.client;
            if changed && button(ui, ButtonStyle::Filled, None, "Save client").clicked() {
                events.push(SettingsEvent::SaveGoogleClient(edited.clone()));
            }
            if calendar.is_connecting() {
                if button(ui, ButtonStyle::Ghost, None, "Cancel").clicked() {
                    events.push(SettingsEvent::CancelGoogleConnect);
                }
                muted(ui, "Finish signing in in your browser…");
            } else if calendar.has_client()
                && !changed
                && button(
                    ui,
                    ButtonStyle::Accent,
                    Some(Icon::Plus),
                    "Connect Google account",
                )
                .clicked()
            {
                events.push(SettingsEvent::ConnectGoogle);
            }
        });
        if let Some(error) = &calendar.error {
            ui.label(RichText::new(error).size(12.5).color(theme.warning));
        }
        for account in &calendar.config.accounts {
            ui.horizontal(|ui| {
                ui.add(Icon::Calendar.image(13.0, theme.icon_muted));
                ui.label(RichText::new(&account.email).size(13.0).color(theme.text));
                if account.needs_sign_in {
                    ui.label(
                        RichText::new("needs to sign in again")
                            .size(12.5)
                            .color(theme.warning),
                    );
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button(ui, ButtonStyle::Danger, None, "Remove").clicked() {
                        events.push(SettingsEvent::RemoveGoogleAccount(account.email.clone()));
                    }
                    if account.needs_sign_in && button(ui, ButtonStyle::Filled, None, "Sign in").clicked() {
                        events.push(SettingsEvent::ConnectGoogle);
                    }
                });
            });
        }
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
