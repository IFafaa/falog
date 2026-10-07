//! The Calendar tab: your own Google OAuth client, then the connected accounts.

use super::{SettingsDialog, SettingsEvent};
use crate::calendar::CalendarState;
use crate::components::{ButtonStyle, button};
use crate::icons::Icon;
use crate::theme::Theme;
use eframe::egui::{self, Align, Layout, RichText, TextEdit, Ui};
use falog_calendar::Client;

impl SettingsDialog {
    /// Your own OAuth client (Google does not let open source apps ship one), then the accounts.
    pub(super) fn calendar_tab(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        calendar: &CalendarState,
        events: &mut Vec<SettingsEvent>,
    ) {
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
