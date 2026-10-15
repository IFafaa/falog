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

impl UpdateBadge {
    /// What the status bar says.
    fn label(&self) -> String {
        match self {
            Self::Available(version) => trf("Update to {version}", &[("version", version)]),
            Self::Downloading(Some(done)) => {
                let percent = format!("{:.0}", done * 100.0);
                trf("Downloading update {percent}%", &[("percent", &percent)])
            }
            Self::Downloading(None) => tr("Downloading update…").to_owned(),
            Self::Ready(_) => tr("Restart to update").to_owned(),
        }
    }

    /// What clicking it does, on hover.
    fn tooltip(&self) -> Option<String> {
        match self {
            Self::Available(_) => Some(tr("Download the new version of Falog").to_owned()),
            Self::Downloading(_) => None,
            Self::Ready(version) => Some(trf(
                "Install Falog {version} and start it again",
                &[("version", version)],
            )),
        }
    }
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
                    if let Some(badge) = &bar.update {
                        let (icon, color, action) = match badge {
                            UpdateBadge::Available(_) => {
                                (Icon::ArrowDown, theme.text_accent, Some(Action::DownloadUpdate))
                            }
                            UpdateBadge::Downloading(_) => (Icon::ArrowDown, theme.icon_muted, None),
                            UpdateBadge::Ready(_) => {
                                (Icon::Refresh, theme.text_accent, Some(Action::InstallUpdate))
                            }
                        };
                        let mut response = item(ui, theme, icon, color, &badge.label());
                        if let Some(tooltip) = badge.tooltip() {
                            response = response.on_hover_text(tooltip);
                        }
                        if let (true, Some(action)) = (response.clicked(), action) {
                            actions.push(action);
                        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{Language, set};

    #[test]
    fn the_update_badge_says_where_the_update_is() {
        set(Language::English);
        let available = UpdateBadge::Available("0.4.0".into());
        assert_eq!(available.label(), "Update to 0.4.0");
        assert_eq!(
            available.tooltip().as_deref(),
            Some("Download the new version of Falog")
        );
        assert_eq!(UpdateBadge::Downloading(None).label(), "Downloading update…");
        assert_eq!(
            UpdateBadge::Downloading(Some(0.426)).label(),
            "Downloading update 43%"
        );
        assert_eq!(UpdateBadge::Downloading(None).tooltip(), None);
        let ready = UpdateBadge::Ready("0.4.0".into());
        assert_eq!(ready.label(), "Restart to update");
        assert_eq!(
            ready.tooltip().as_deref(),
            Some("Install Falog 0.4.0 and start it again")
        );
        set(Language::Portuguese);
        assert_eq!(available.label(), "Atualizar para 0.4.0");
        assert_eq!(
            UpdateBadge::Downloading(Some(1.0)).label(),
            "Baixando atualização 100%"
        );
        assert_eq!(ready.label(), "Reiniciar para atualizar");
        set(Language::English);
    }
}
