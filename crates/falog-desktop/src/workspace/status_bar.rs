use crate::action::{Action, Actions};
use crate::components::{icon_button, icon_toggle};
use crate::icons::Icon;
use crate::theme::Theme;
use crate::views::View;
use eframe::egui::{
    self, Align, Color32, CursorIcon, FontId, Frame, Layout, Margin, Rect, Response, Sense, TopBottomPanel,
    Ui, pos2, vec2,
};
use falog_core::agenda::Summary;
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
    pub summary: Summary,
    pub date: String,
    pub sidebar_open: bool,
    pub toast: Option<&'a Toast>,
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
                if icon_toggle(ui, Icon::SidebarLeft, bar.sidebar_open, "Toggle sidebar (Ctrl+B)").clicked() {
                    actions.push(Action::ToggleSidebar);
                }
                let summary = bar.summary;
                if item(
                    ui,
                    theme,
                    Icon::Circle,
                    theme.icon_muted,
                    &format!("{} open", summary.open),
                )
                .clicked()
                {
                    actions.push(Action::SetView(View::List));
                }
                if summary.overdue > 0
                    && item(
                        ui,
                        theme,
                        Icon::Warning,
                        theme.error,
                        &format!("{} overdue", summary.overdue),
                    )
                    .clicked()
                {
                    actions.push(Action::SetView(View::Agenda));
                }
                if summary.due_today > 0
                    && item(
                        ui,
                        theme,
                        Icon::Clock,
                        theme.warning,
                        &format!("{} due today", summary.due_today),
                    )
                    .clicked()
                {
                    actions.push(Action::SetView(View::Agenda));
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, Icon::Settings, "Settings (Ctrl+,)").clicked() {
                        actions.push(Action::OpenSettings);
                    }
                    if icon_button(ui, Icon::Command, "Command palette (Ctrl+Shift+P)").clicked() {
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
