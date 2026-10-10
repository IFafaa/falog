use crate::action::{Action, Actions};
use crate::components::{icon_button, icon_toggle};
use crate::i18n::{tr, trf};
use crate::icons::Icon;
use crate::theme::Theme;
use eframe::egui::{
    self, Align, Color32, CursorIcon, FontId, Frame, Layout, Margin, Rect, Response, Sense, TopBottomPanel,
    Ui, pos2, vec2,
};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
}

/// A transient message shown at the right of the status bar.
#[derive(Debug)]
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    pub shown_at: Instant,
}

#[derive(Debug)]
pub struct StatusBar<'a> {
    pub date: String,
    pub sidebar_open: bool,
    pub assistant_open: bool,
    /// The microphone is recording for the assistant.
    pub listening: bool,
    pub toast: Option<&'a Toast>,
    pub update: Option<UpdateBadge>,
}

/// The update button at the right of the status bar, when a newer Falog is on its way.
#[derive(Clone, Debug, PartialEq)]
pub enum UpdateBadge {
    /// A newer version was found; clicking downloads it.
    Available(String),
    /// 0.0–1.0 once the size is known.
    Downloading(Option<f32>),
    /// Downloaded and verified; clicking installs it and restarts Falog.
    Ready(String),
}

pub fn show(ctx: &egui::Context, theme: &Theme, bar: StatusBar<'_>, actions: &mut Actions) {
    TopBottomPanel::bottom("status_bar")
        .exact_height(28.0)
        .show_separator_line(false)
        .frame(
            Frame::none()
                .fill(theme.status_bar)
                .inner_margin(Margin::symmetric(4.0, 0.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if icon_toggle(
                    ui,
                    Icon::SidebarLeft,
                    bar.sidebar_open,
                    tr("Toggle sidebar (Ctrl+B)"),
                )
                .clicked()
                {
                    actions.push(Action::ToggleSidebar);
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_toggle(
                        ui,
                        Icon::Sparkle,
                        bar.assistant_open,
                        tr("Assistant (Ctrl+Shift+A)"),
                    )
                    .clicked()
                    {
                        actions.push(Action::ToggleAssistant);
                    }
                    match &bar.update {
                        Some(UpdateBadge::Available(version)) => {
                            let label = trf("Update to {version}", &[("version", version)]);
                            let clicked = item(ui, theme, Icon::ArrowDown, theme.text_accent, &label)
                                .on_hover_text(tr("Download the new version of Falog"))
                                .clicked();
                            if clicked {
                                actions.push(Action::DownloadUpdate);
                            }
                        }
                        Some(UpdateBadge::Downloading(progress)) => {
                            let label = match progress {
                                Some(done) => {
                                    let percent = format!("{:.0}", done * 100.0);
                                    trf("Downloading update {percent}%", &[("percent", &percent)])
                                }
                                None => tr("Downloading update…").to_owned(),
                            };
                            item(ui, theme, Icon::ArrowDown, theme.icon_muted, &label);
                        }
                        Some(UpdateBadge::Ready(version)) => {
                            let clicked = item(
                                ui,
                                theme,
                                Icon::Refresh,
                                theme.text_accent,
                                tr("Restart to update"),
                            )
                            .on_hover_text(trf(
                                "Install Falog {version} and start it again",
                                &[("version", version)],
                            ))
                            .clicked();
                            if clicked {
                                actions.push(Action::InstallUpdate);
                            }
                        }
                        None => {}
                    }
                    if bar.listening && item(ui, theme, Icon::Mic, theme.error, tr("Listening")).clicked() {
                        actions.push(Action::ToggleDictation);
                    }
                    if icon_button(ui, Icon::Settings, tr("Settings (Ctrl+,)")).clicked() {
                        actions.push(Action::OpenSettings);
                    }
                    if icon_button(ui, Icon::Command, tr("Command palette (Ctrl+Shift+P)")).clicked() {
                        actions.push(Action::OpenCommandPalette);
                    }
                    item(ui, theme, Icon::Reader, theme.icon_muted, &bar.date);
                    if let Some(toast) = bar.toast {
                        let (icon, color) = match toast.kind {
                            ToastKind::Success => (Icon::Check, theme.success),
                            ToastKind::Error => (Icon::Warning, theme.error),
                        };
                        item(ui, theme, icon, color, &toast.message);
                    }
                });
            });
        });
}

/// A ghost status bar entry: small icon and label.
fn item(ui: &mut Ui, theme: &Theme, icon: Icon, icon_color: Color32, text: &str) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(12.5), theme.text_muted);
    let size = vec2(6.0 + 12.0 + 5.0 + galley.size().x + 6.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_hover);
    }
    let icon_rect = Rect::from_min_size(pos2(rect.left() + 6.0, rect.center().y - 6.0), vec2(12.0, 12.0));
    icon.paint(ui, icon_rect, 12.0, icon_color);
    ui.painter().galley(
        pos2(icon_rect.right() + 5.0, rect.center().y - galley.size().y / 2.0),
        galley,
        theme.text_muted,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}
