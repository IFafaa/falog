use super::icon_button;
use crate::fonts;
use crate::icons::Icon;
use crate::theme::Theme;
use eframe::egui::{
    self, Align, Align2, Color32, Frame, Id, Key, Layout, Margin, Order, RichText, Sense, Stroke, Ui, pos2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Near the top, for the command palette and pickers.
    Top,
    Center,
}

/// Shows a modal surface over a dimmed backdrop.
///
/// Returns the content's result and whether the user dismissed the modal
/// (Escape or a click on the backdrop).
pub fn modal<R>(
    ctx: &egui::Context,
    id: &str,
    width: f32,
    placement: Placement,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> (R, bool) {
    let theme = Theme::current(ctx);
    let screen = ctx.screen_rect();

    let backdrop_clicked = egui::Area::new(Id::new((id, "backdrop")))
        .order(Order::Middle)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(screen.size(), Sense::click());
            let dim = if theme.dark { 100 } else { 40 };
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_black_alpha(dim));
            response.clicked()
        })
        .inner;

    let (anchor, pivot) = match placement {
        Placement::Top => (
            pos2(screen.center().x, screen.top() + screen.height() * 0.12),
            Align2::CENTER_TOP,
        ),
        Placement::Center => (screen.center(), Align2::CENTER_CENTER),
    };
    let inner = egui::Area::new(Id::new(id))
        .order(Order::Foreground)
        .fixed_pos(anchor)
        .pivot(pivot)
        .show(ctx, |ui| {
            Frame::none()
                .fill(theme.elevated_surface)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .rounding(8.0)
                .shadow(theme.popover_shadow())
                .show(ui, |ui| {
                    ui.set_width(width);
                    // Tall dialogs (Settings) scroll instead of running off a small window.
                    egui::ScrollArea::vertical()
                        .max_height(screen.height() - 48.0)
                        .min_scrolled_height(screen.height() - 48.0)
                        .auto_shrink([false, true])
                        .show(ui, add_contents)
                        .inner
                })
                .inner
        })
        .inner;

    let escape = ctx.input(|i| i.key_pressed(Key::Escape));
    (inner, backdrop_clicked || escape)
}

/// Title row with a close button for dialogs. Returns whether close was clicked.
pub fn modal_header(ui: &mut Ui, title: &str) -> bool {
    let theme = Theme::current(ui.ctx());
    let mut closed = false;
    Frame::none()
        .inner_margin(Margin {
            left: 16.0,
            right: 8.0,
            top: 8.0,
            bottom: 8.0,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).font(fonts::semibold(15.0)).color(theme.text));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    closed = icon_button(ui, Icon::Close, "Close (Esc)").clicked();
                });
            });
        });
    ui.painter().hline(
        ui.min_rect().x_range(),
        ui.cursor().top(),
        Stroke::new(1.0_f32, theme.border_variant),
    );
    closed
}
