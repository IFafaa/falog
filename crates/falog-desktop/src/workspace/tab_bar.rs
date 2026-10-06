use super::BAR_HEIGHT;
use crate::action::{Action, Actions};
use crate::components::icon_button;
use crate::icons::Icon;
use crate::theme::Theme;
use crate::views::View;
use eframe::egui::{
    self, Align, CursorIcon, FontId, Frame, Layout, Margin, Rect, Response, Sense, Stroke, TopBottomPanel,
    Ui, pos2, vec2,
};

pub fn show(ctx: &egui::Context, theme: &Theme, current: View, actions: &mut Actions) {
    TopBottomPanel::top("tab_bar")
        .exact_height(BAR_HEIGHT)
        .show_separator_line(false)
        .frame(Frame::none().fill(theme.tab_bar))
        .show(ctx, |ui| {
            let bar = ui.max_rect();
            ui.painter().hline(
                bar.x_range(),
                bar.bottom() - 0.5,
                Stroke::new(1.0_f32, theme.border),
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for view in View::ALL {
                    let tooltip = format!("{} ({})", view.label(), view.shortcut());
                    if tab(ui, theme, view, view == current)
                        .on_hover_text(tooltip)
                        .clicked()
                    {
                        actions.push(Action::SetView(view));
                    }
                }
                Frame::none()
                    .inner_margin(Margin::symmetric(6.0, 0.0))
                    .show(ui, |ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            if icon_button(ui, Icon::Plus, "New task (Ctrl+N)").clicked() {
                                actions.push(Action::NewTask(Default::default()));
                            }
                            if icon_button(ui, Icon::Search, "Find task (Ctrl+P)").clicked() {
                                actions.push(Action::OpenTaskFinder);
                            }
                        });
                    });
            });
        });
}

/// A Zed tab: the active one takes the editor background and opens the bottom border.
fn tab(ui: &mut Ui, theme: &Theme, view: View, active: bool) -> Response {
    let font = FontId::proportional(14.0);
    let galley = ui
        .painter()
        .layout_no_wrap(view.label().to_owned(), font, theme.text);
    let size = vec2(12.0 + 14.0 + 6.0 + galley.size().x + 14.0, BAR_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    let color = if active || response.hovered() {
        theme.text
    } else {
        theme.text_muted
    };
    if active {
        ui.painter().rect_filled(rect, 0.0, theme.tab_active);
    }
    let border = Stroke::new(1.0_f32, theme.border);
    ui.painter().vline(rect.right() - 0.5, rect.y_range(), border);
    if active && rect.left() > ui.max_rect().left() {
        ui.painter().vline(rect.left() + 0.5, rect.y_range(), border);
    }

    let icon_rect = Rect::from_min_size(pos2(rect.left() + 12.0, rect.center().y - 7.0), vec2(14.0, 14.0));
    view.icon().paint(
        ui,
        icon_rect,
        14.0,
        if active { theme.icon } else { theme.icon_muted },
    );
    let text_pos = pos2(icon_rect.right() + 6.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(text_pos, galley, color);
    response.on_hover_cursor(CursorIcon::PointingHand)
}
