//! The Calendar tab: the calendars already connected, then the two ways to connect one, each a card
//! that folds open: pasting a calendar's secret iCal address, or signing in with Google through your
//! own OAuth client.

use super::{SettingsDialog, SettingsEvent, section};
use crate::calendar::CalendarState;
use crate::components::{ButtonStyle, area_picker, button, single_line};
use crate::fonts;
use crate::i18n::tr;
use crate::icons::Icon;
use crate::theme::Theme;
use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Frame, Layout, Margin, Rect, RichText, Sense, Spinner,
    Stroke, TextEdit, Ui, pos2, vec2,
};
use falog_calendar::{Client, LINK_COLORS, ics};
use falog_core::domain::{Area, AreaId};

/// What the user is typing on the Calendar page.
#[derive(Debug, Default)]
pub(super) struct CalendarForm {
    name: String,
    url: String,
    /// The link being renamed, and the name typed so far.
    renaming: Option<(String, String)>,
    /// Which ways of connecting are unfolded.
    show_link: bool,
    show_oauth: bool,
}

impl CalendarForm {
    pub(super) fn new(calendar: &CalendarState) -> Self {
        let connected = !calendar.config.links.is_empty() || !calendar.config.accounts.is_empty();
        Self {
            // With nothing connected yet, the simple way starts open; whoever saved an OAuth client but
            // has not signed in finds the rest of that setup open.
            show_link: !connected,
            show_oauth: calendar.has_client() && calendar.config.accounts.is_empty(),
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

const LABEL_WIDTH: f32 = 120.0;

impl SettingsDialog {
    pub(super) fn calendar_tab(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        calendar: &CalendarState,
        areas: &[Area],
        events: &mut Vec<SettingsEvent>,
    ) {
        ui.label(
            RichText::new(tr("See your Google calendars next to your tasks. The assistant can also book meetings in signed-in accounts."))
                .size(13.0)
                .color(theme.text_muted),
        );

        self.connected_section(ui, theme, calendar, events);
        areas_section(ui, theme, calendar, areas, events);

        section(ui, theme, tr("Connect a calendar"));
        let mut open = self.calendar.show_link;
        connect_option(
            ui,
            theme,
            ConnectOption {
                icon: Icon::Copy,
                title: tr("Paste a calendar link"),
                summary: tr("Recommended. Two minutes, no setup: copy a link from Google Calendar."),
                open: &mut open,
            },
            |ui| self.link_form(ui, theme, calendar, events),
        );
        self.calendar.show_link = open;
        ui.add_space(8.0);
        let mut open = self.calendar.show_oauth;
        connect_option(
            ui,
            theme,
            ConnectOption {
                icon: Icon::Person,
                title: tr("Sign in with Google"),
                summary: tr("Advanced. The assistant can book meetings; needs your own OAuth client."),
                open: &mut open,
            },
            |ui| self.oauth(ui, theme, calendar, events),
        );
        self.calendar.show_oauth = open;
        if let Some(error) = &calendar.error {
            ui.add_space(6.0);
            ui.label(RichText::new(error).size(12.5).color(theme.warning));
        }
    }

    /// Every calendar shown in Falog, links and signed-in accounts alike, with what can be done to it.
    fn connected_section(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        calendar: &CalendarState,
        events: &mut Vec<SettingsEvent>,
    ) {
        if calendar.config.links.is_empty() && calendar.config.accounts.is_empty() {
            return;
        }
        section(ui, theme, tr("Connected calendars"));
        for link in &calendar.config.links {
            let id = &link.calendar.id;
            ui.horizontal(|ui| {
                ui.add(Icon::Copy.image(13.0, theme.icon_muted))
                    .on_hover_text(tr("Calendar link"));
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
                    if button(ui, ButtonStyle::Danger, None, tr("Remove")).clicked() {
                        events.push(SettingsEvent::RemoveCalendarLink(id.clone()));
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.add_space(13.0 + ui.spacing().item_spacing.x);
                ui.label(
                    RichText::new(ics::masked(&link.url))
                        .monospace()
                        .size(11.5)
                        .color(theme.text_placeholder),
                );
            });
            ui.add_space(4.0);
        }
        for account in &calendar.config.accounts {
            ui.horizontal(|ui| {
                ui.set_min_height(24.0);
                ui.add(Icon::Person.image(13.0, theme.icon_muted))
                    .on_hover_text(tr("Google account"));
                ui.label(RichText::new(&account.email).size(13.0).color(theme.text));
                if account.needs_sign_in {
                    ui.label(
                        RichText::new(tr("needs to sign in again"))
                            .size(12.5)
                            .color(theme.warning),
                    );
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button(ui, ButtonStyle::Danger, None, tr("Remove")).clicked() {
                        events.push(SettingsEvent::RemoveGoogleAccount(account.email.clone()));
                    }
                    if account.needs_sign_in && button(ui, ButtonStyle::Filled, None, tr("Sign in")).clicked()
                    {
                        events.push(SettingsEvent::ConnectGoogle);
                    }
                });
            });
            ui.add_space(4.0);
        }
    }

    /// Steps to copy a calendar's secret iCal address, and the form to paste it.
    fn link_form(
        &mut self,
        ui: &mut Ui,
        theme: &Theme,
        calendar: &CalendarState,
        events: &mut Vec<SettingsEvent>,
    ) {
        let muted = |ui: &mut Ui, text: &str| {
            ui.label(RichText::new(text).size(12.5).color(theme.text_muted));
        };
        ui.horizontal_wrapped(|ui| {
            muted(ui, tr("1. Open"));
            ui.hyperlink_to(
                RichText::new("Google Calendar").size(12.5),
                "https://calendar.google.com/calendar/r/settings",
            );
            muted(ui, tr("settings (the gear, then Settings)."));
        });
        muted(
            ui,
            tr("2. On the left, under \"Settings for my calendars\", click the calendar."),
        );
        muted(
            ui,
            tr("3. In \"Integrate calendar\", copy the \"Secret address in iCal format\"."),
        );
        muted(
            ui,
            tr("4. Paste it here and click Add. Do the same for each account (work, personal…)."),
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
        field(ui, theme, tr("Name"), |ui| {
            ui.add(
                TextEdit::singleline(&mut self.calendar.name)
                    .hint_text(tr("Work, Personal… (optional)"))
                    .desired_width(ui.available_width()),
            );
        });
        let mut submitted = false;
        field(ui, theme, tr("Secret address"), |ui| {
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
                muted(ui, tr("Checking the link…"));
            } else {
                let ready = !self.calendar.url.trim().is_empty();
                let add = ui
                    .add_enabled_ui(ready, |ui| {
                        button(ui, ButtonStyle::Accent, Some(Icon::Plus), tr("Add"))
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
            RichText::new(tr(
                "The address works like a password: whoever has it can read the calendar. Falog keeps it on this computer only. Falog reads the calendars again every five minutes.",
            ))
            .size(12.0)
            .color(theme.text_placeholder),
        );
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
            tr(
                "For accounts whose admin turned off secret addresses. Events show up right away, but you need your own Google OAuth client:",
            ),
        );
        ui.horizontal_wrapped(|ui| {
            muted(ui, tr("1. In Google Cloud, create a project and enable the"));
            ui.hyperlink_to(
                RichText::new("Google Calendar API").size(12.5),
                "https://console.cloud.google.com/apis/library/calendar-json.googleapis.com",
            );
        });
        ui.horizontal_wrapped(|ui| {
            muted(ui, tr("2. In the"));
            ui.hyperlink_to(
                RichText::new(tr("OAuth consent screen")).size(12.5),
                "https://console.cloud.google.com/auth/overview",
            );
            muted(
                ui,
                tr("pick External, then publish the app (testing logins expire every 7 days)."),
            );
        });
        ui.horizontal_wrapped(|ui| {
            muted(ui, tr("3. In"));
            ui.hyperlink_to(
                RichText::new(tr("Credentials")).size(12.5),
                "https://console.cloud.google.com/apis/credentials",
            );
            muted(
                ui,
                tr("create an OAuth client ID of type Desktop app and paste it here."),
            );
        });
        ui.add_space(6.0);
        field(ui, theme, tr("Client ID"), |ui| {
            ui.add(TextEdit::singleline(&mut self.google_id).desired_width(ui.available_width()));
        });
        field(ui, theme, tr("Client secret"), |ui| {
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
            if changed && button(ui, ButtonStyle::Filled, None, tr("Save client")).clicked() {
                events.push(SettingsEvent::SaveGoogleClient(edited.clone()));
            }
            if calendar.is_connecting() {
                if button(ui, ButtonStyle::Ghost, None, tr("Cancel")).clicked() {
                    events.push(SettingsEvent::CancelGoogleConnect);
                }
                if button(ui, ButtonStyle::Filled, None, tr("Open again"))
                    .on_hover_text(tr("Closed the browser tab? Start the sign-in over"))
                    .clicked()
                {
                    events.push(SettingsEvent::ConnectGoogle);
                }
                muted(ui, tr("Finish signing in in your browser…"));
            } else if calendar.has_client()
                && !changed
                && button(
                    ui,
                    ButtonStyle::Accent,
                    Some(Icon::Plus),
                    tr("Connect Google account"),
                )
                .clicked()
            {
                events.push(SettingsEvent::ConnectGoogle);
            }
        });
    }
}

/// One row per email (and per calendar link) with a select for its area. An email belongs to one
/// area; an area may have several emails. The sidebar's area filter then shows that area's meetings,
/// and the assistant books an area's meetings in its account.
fn areas_section(
    ui: &mut Ui,
    theme: &Theme,
    calendar: &CalendarState,
    areas: &[Area],
    events: &mut Vec<SettingsEvent>,
) {
    // (source, icon, label, area, read only), with the icons of the connected list.
    let sources: Vec<(&str, Icon, &str, Option<i64>, bool)> = calendar
        .config
        .accounts
        .iter()
        .map(|a| {
            (
                a.email.as_str(),
                Icon::Person,
                a.email.as_str(),
                a.area,
                !a.can_write,
            )
        })
        .chain(calendar.config.links.iter().map(|l| {
            (
                l.calendar.id.as_str(),
                Icon::Copy,
                l.calendar.name.as_str(),
                l.area,
                false,
            )
        }))
        .collect();
    if sources.is_empty() || areas.is_empty() {
        return;
    }
    section(ui, theme, tr("Area of each account"));
    ui.label(
        RichText::new(tr(
            "Pick the area each email belongs to. An area can have several emails.",
        ))
        .size(12.5)
        .color(theme.text_muted),
    );
    ui.add_space(4.0);
    const PICKER_WIDTH: f32 = 200.0;
    for (source, icon, label, area, read_only) in sources {
        ui.horizontal(|ui| {
            ui.add(icon.image(13.0, theme.icon_muted));
            // The label takes what the picker leaves, so the row stays as wide as the page.
            let label_width = (ui.available_width() - PICKER_WIDTH - ui.spacing().item_spacing.x).max(120.0);
            ui.allocate_ui_with_layout(
                vec2(label_width, 22.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.set_width(label_width);
                    ui.add(egui::Label::new(RichText::new(label).size(13.0).color(theme.text)).truncate());
                },
            );
            let mut value = area.map(AreaId);
            let before = value;
            area_picker(
                ui,
                &format!("calendar-area-{source}"),
                &mut value,
                areas,
                PICKER_WIDTH,
            );
            if value != before {
                events.push(SettingsEvent::SetCalendarArea {
                    source: source.to_owned(),
                    area: value,
                });
            }
        });
        if read_only {
            ui.horizontal(|ui| {
                ui.add_space(13.0 + ui.spacing().item_spacing.x);
                ui.label(RichText::new(tr("read only")).size(12.0).color(theme.warning)).on_hover_text(
                    tr("Connected before Falog could create events. Reconnect it (Sign in with Google, below) to let the assistant book meetings here."),
                );
                if button(ui, ButtonStyle::Ghost, None, tr("Reconnect")).clicked() {
                    events.push(SettingsEvent::ConnectGoogle);
                }
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

/// The header of one way to connect a calendar.
struct ConnectOption<'a> {
    icon: Icon,
    title: &'a str,
    summary: &'a str,
    open: &'a mut bool,
}

/// A bordered card for one way to connect: icon, title and a one-line summary in a header that folds
/// the card open or closed, then the steps when open. Both ways look alike, so neither reads as the
/// only one.
fn connect_option(ui: &mut Ui, theme: &Theme, option: ConnectOption<'_>, add_body: impl FnOnce(&mut Ui)) {
    Frame::none()
        .stroke(Stroke::new(1.0_f32, theme.border_variant))
        .rounding(6.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::click());
            if response.hovered() {
                ui.painter().rect_filled(rect, 6.0, theme.ghost_hover);
            }
            let chevron = if *option.open {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            };
            let chevron_rect =
                Rect::from_center_size(pos2(rect.right() - 18.0, rect.center().y), vec2(12.0, 12.0));
            chevron.paint(ui, chevron_rect, 12.0, theme.icon_muted);
            let icon_rect =
                Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), vec2(16.0, 16.0));
            option.icon.paint(ui, icon_rect, 16.0, theme.icon);
            let left = icon_rect.right() + 12.0;
            ui.painter().text(
                pos2(left, rect.center().y - 9.0),
                Align2::LEFT_CENTER,
                option.title,
                fonts::semibold(13.5),
                theme.text,
            );
            // Cut with an ellipsis rather than run past the border on a narrow page.
            let summary = single_line(
                ui,
                option.summary,
                FontId::proportional(12.0),
                theme.text_muted,
                chevron_rect.left() - 8.0 - left,
                false,
            );
            let summary_top = rect.center().y + 9.0 - summary.size().y / 2.0;
            ui.painter()
                .galley(pos2(left, summary_top), summary, theme.text_muted);
            if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                *option.open = !*option.open;
            }
            if *option.open {
                ui.painter().hline(
                    rect.x_range(),
                    rect.bottom(),
                    Stroke::new(1.0_f32, theme.border_variant),
                );
                Frame::none()
                    .inner_margin(Margin {
                        left: 14.0,
                        right: 14.0,
                        top: 10.0,
                        bottom: 12.0,
                    })
                    .show(ui, add_body);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use falog_calendar::Account;

    const SECRET_LINK: &str =
        "https://calendar.google.com/calendar/ical/me%40work.example/private-SECRET/basic.ics";

    fn account() -> Account {
        Account {
            email: "me@work.example".into(),
            refresh_token: "token".into(),
            calendars: Vec::new(),
            needs_sign_in: false,
            can_write: false,
            area: None,
        }
    }

    fn client() -> Client {
        Client {
            id: "falog-test.apps.example".into(),
            secret: "not-a-secret".into(),
        }
    }

    /// Which ways to connect start unfolded: the link card, then the Google card.
    fn unfolded(calendar: &CalendarState) -> (bool, bool) {
        let form = CalendarForm::new(calendar);
        (form.show_link, form.show_oauth)
    }

    #[test]
    fn with_nothing_connected_the_link_card_starts_open() {
        assert_eq!(unfolded(&CalendarState::new(None)), (true, false));
    }

    #[test]
    fn a_saved_client_without_an_account_opens_the_rest_of_the_google_setup() {
        let mut calendar = CalendarState::new(None);
        calendar.config.client = client();
        assert_eq!(unfolded(&calendar), (true, true));

        calendar.config.add_link("Team", SECRET_LINK, None).unwrap();
        assert_eq!(unfolded(&calendar), (false, true), "a link is connected");

        calendar.config.upsert(account());
        assert_eq!(unfolded(&calendar), (false, false), "signed in");
    }

    #[test]
    fn a_connected_link_folds_both_ways_away() {
        let mut calendar = CalendarState::new(None);
        calendar.config.add_link("Team", SECRET_LINK, None).unwrap();
        assert_eq!(unfolded(&calendar), (false, false));
    }

    #[test]
    fn a_pasted_link_never_prints_its_address() {
        let link = NewLink {
            name: "Team".into(),
            url: SECRET_LINK.into(),
        };
        let printed = format!("{link:?}");
        assert!(printed.contains("Team"), "{printed}");
        assert!(
            !printed.contains("SECRET") && !printed.contains("me%40"),
            "{printed}"
        );
    }
}
