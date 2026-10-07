//! The Calendar tab: calendar links first (paste a calendar's secret iCal address), then signing in
//! with Google through your own OAuth client, folded away for accounts that cannot use links.

use super::{SettingsDialog, SettingsEvent, section};
use crate::calendar::CalendarState;
use crate::components::{ButtonStyle, button};
use crate::icons::Icon;
use crate::theme::Theme;
use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Layout, Rect, RichText, Sense, Spinner, Stroke,
    TextEdit, Ui, pos2, vec2,
};
use falog_calendar::{Client, LINK_COLORS, ics};

/// What the user is typing on the Calendar page.
#[derive(Debug, Default)]
pub(super) struct CalendarForm {
    name: String,
    url: String,
    /// The link being renamed, and the name typed so far.
    renaming: Option<(String, String)>,
    show_oauth: bool,
}

impl CalendarForm {
    pub(super) fn new(calendar: &CalendarState) -> Self {
        Self {
            // Whoever already set up OAuth sees it without unfolding.
            show_oauth: calendar.has_client() || !calendar.config.accounts.is_empty(),
            ..Self::default()
        }
    }
}

/// A pasted calendar link. Its address is a credential, so `Debug` does not print it.
#[derive(PartialEq, Eq)]
pub struct NewLink {
    pub name: String,
    pub url: String,
}

impl std::fmt::Debug for NewLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewLink")
            .field("name", &self.name)
            .field("url", &ics::masked(&self.url))
            .finish()
    }
}

const LABEL_WIDTH: f32 = 96.0;

impl SettingsDialog {
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
        ui.label(
            RichText::new("See your Google calendars next to your tasks. Falog only reads them.")
                .size(13.0)
                .color(theme.text_muted),
        );

        section(ui, theme, "Add a calendar link");
        ui.horizontal_wrapped(|ui| {
            muted(ui, "1. Open");
            ui.hyperlink_to(
                RichText::new("Google Calendar").size(12.5),
                "https://calendar.google.com/calendar/r/settings",
            );
            muted(ui, "settings (the gear, then Settings).");
        });
        muted(
            ui,
            "2. On the left, under \"Settings for my calendars\", click the calendar.",
        );
        muted(
            ui,
            "3. In \"Integrate calendar\", copy the \"Secret address in iCal format\".",
        );
        muted(
            ui,
            "4. Paste it here and click Add. Do the same for each account (work, personal…).",
        );
        ui.add_space(6.0);

        // A successful add saves the address; then the form starts over.
        if !calendar.is_adding_link()
            && !self.calendar.url.is_empty()
            && calendar.config.has_link(&self.calendar.url)
        {
            self.calendar.name.clear();
            self.calendar.url.clear();
        }
        field(ui, theme, "Name", |ui| {
            ui.add(
                TextEdit::singleline(&mut self.calendar.name)
                    .hint_text("Work, Personal… (optional)")
                    .desired_width(ui.available_width()),
            );
        });
        let mut submitted = false;
        field(ui, theme, "Secret address", |ui| {
            let response = ui.add(
                TextEdit::singleline(&mut self.calendar.url)
                    .password(true)
                    .hint_text("https://calendar.google.com/calendar/ical/…/basic.ics")
                    .desired_width(ui.available_width()),
            );
            submitted = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(LABEL_WIDTH + ui.spacing().item_spacing.x);
            if calendar.is_adding_link() {
                ui.add(Spinner::new().size(14.0).color(theme.text_muted));
                muted(ui, "Checking the link…");
            } else {
                let ready = !self.calendar.url.trim().is_empty();
                let add = ui
                    .add_enabled_ui(ready, |ui| {
                        button(ui, ButtonStyle::Accent, Some(Icon::Plus), "Add")
                    })
                    .inner;
                if ready && (add.clicked() || submitted) {
                    events.push(SettingsEvent::AddCalendarLink(NewLink {
                        name: self.calendar.name.trim().to_owned(),
                        url: self.calendar.url.trim().to_owned(),
                    }));
                }
            }
        });
        if let Some(error) = &calendar.link_error {
            ui.label(RichText::new(error).size(12.5).color(theme.warning));
        }
        ui.label(
            RichText::new(
                "The address works like a password: whoever has it can read the calendar. Falog keeps it \
                 on this computer only. Falog reads the calendars again every five minutes.",
            )
            .size(12.0)
            .color(theme.text_placeholder),
        );

        if !calendar.config.links.is_empty() {
            section(ui, theme, "Linked calendars");
            for link in &calendar.config.links {
                let id = &link.calendar.id;
                ui.horizontal(|ui| {
                    let editing = self
                        .calendar
                        .renaming
                        .as_ref()
                        .filter(|(renaming, _)| renaming == id)
                        .map(|(_, name)| name.clone());
                    let mut name = editing.unwrap_or_else(|| link.calendar.name.clone());
                    let response = ui.add(TextEdit::singleline(&mut name).desired_width(170.0));
                    if response.has_focus() {
                        self.calendar.renaming = Some((id.clone(), name.clone()));
                    }
                    if response.lost_focus() {
                        let name = name.trim();
                        if !name.is_empty() && name != link.calendar.name {
                            events.push(SettingsEvent::RenameCalendarLink {
                                id: id.clone(),
                                name: name.to_owned(),
                            });
                        }
                        self.calendar.renaming = None;
                    }
                    ui.add_space(4.0);
                    if let Some(color) = swatches(ui, theme, &link.calendar.color) {
                        events.push(SettingsEvent::SetCalendarLinkColor {
                            id: id.clone(),
                            color: color.to_owned(),
                        });
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if button(ui, ButtonStyle::Danger, None, "Remove").clicked() {
                            events.push(SettingsEvent::RemoveCalendarLink(id.clone()));
                        }
                    });
                });
                ui.label(
                    RichText::new(ics::masked(&link.url))
                        .monospace()
                        .size(11.5)
                        .color(theme.text_placeholder),
                );
                ui.add_space(4.0);
            }
        }
        if let Some(error) = &calendar.error {
            ui.label(RichText::new(error).size(12.5).color(theme.warning));
        }

        ui.add_space(12.0);
        if disclosure(
            ui,
            theme,
            "Sign in with Google instead (advanced)",
            self.calendar.show_oauth,
        )
        .clicked()
        {
            self.calendar.show_oauth = !self.calendar.show_oauth;
        }
        if self.calendar.show_oauth {
            ui.add_space(4.0);
            self.oauth(ui, theme, calendar, events);
        }
    }

    /// Your own OAuth client (Google does not let open source apps ship one), then the accounts.
    fn oauth(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        calendar: &CalendarState,
        events: &mut Vec<SettingsEvent>,
    ) {
        let muted = |ui: &mut Ui, text: &str| {
            ui.label(RichText::new(text).size(12.5).color(theme.text_muted));
        };
        muted(
            ui,
            "For accounts whose admin turned off secret addresses. Events show up right away, but you \
             need your own Google OAuth client:",
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
        field(ui, theme, "Client ID", |ui| {
            ui.add(TextEdit::singleline(&mut self.google_id).desired_width(ui.available_width()));
        });
        field(ui, theme, "Client secret", |ui| {
            ui.add(
                TextEdit::singleline(&mut self.google_secret)
                    .password(true)
                    .desired_width(ui.available_width()),
            );
        });
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

/// A label column of fixed width, then the control filling the row.
fn field(ui: &mut Ui, theme: &Theme, label: &str, add_control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [LABEL_WIDTH, 24.0],
            egui::Label::new(RichText::new(label).size(13.0).color(theme.text)),
        );
        add_control(ui);
    });
}

/// Google's calendar colors as small squares; returns the one clicked.
fn swatches(ui: &mut Ui, theme: &Theme, current: &str) -> Option<&'static str> {
    let mut picked = None;
    ui.spacing_mut().item_spacing.x = 4.0;
    for color in LINK_COLORS {
        let (rect, response) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::click());
        ui.painter().rect_filled(rect, 3.0, hex(color));
        if color.eq_ignore_ascii_case(current) {
            ui.painter()
                .rect_stroke(rect.expand(2.0), 4.0, Stroke::new(1.5_f32, theme.text));
        } else if response.hovered() {
            ui.painter()
                .rect_stroke(rect.expand(2.0), 4.0, Stroke::new(1.0_f32, theme.border));
        }
        if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
            picked = Some(color);
        }
    }
    picked
}

/// `#rrggbb` as a color; the palette is known-good, so a typo shows as gray.
fn hex(color: &str) -> Color32 {
    let channel = |i: usize| u8::from_str_radix(color.get(i..i + 2).unwrap_or("80"), 16).unwrap_or(0x80);
    Color32::from_rgb(channel(1), channel(3), channel(5))
}

/// A row that folds a section open or closed, with a chevron like Zed's settings.
fn disclosure(ui: &mut Ui, theme: &Theme, label: &str, open: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_hover);
    }
    let icon = if open {
        Icon::ChevronDown
    } else {
        Icon::ChevronRight
    };
    let chevron = Rect::from_center_size(pos2(rect.left() + 10.0, rect.center().y), vec2(12.0, 12.0));
    icon.paint(ui, chevron, 12.0, theme.icon_muted);
    ui.painter().text(
        pos2(chevron.right() + 6.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        theme.text,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}
