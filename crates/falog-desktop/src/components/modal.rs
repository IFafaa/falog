use super::icon_button;
use crate::fonts;
use crate::i18n::tr;
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
                    closed = icon_button(ui, Icon::Close, tr("Close (Esc)")).clicked();
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

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Context, RawInput, Rect, vec2};

    /// The dialog's rect after a frame in an 800 px wide window `height` tall, with contents
    /// `content_height` tall.
    fn shown_rect(ctx: &Context, height: f32, placement: Placement, content_height: f32) -> Rect {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, height))),
            ..RawInput::default()
        };
        let _ = ctx.run(input, |ctx| {
            modal(ctx, "tall", 400.0, placement, |ui| {
                ui.allocate_space(vec2(400.0, content_height));
            });
        });
        ctx.memory(|m| m.area_rect(Id::new("tall"))).unwrap()
    }

    // Regression: a dialog taller than the window (Settings on a small screen) ran off its bottom.
    #[test]
    fn a_dialog_taller_than_the_window_fits_in_it() {
        for placement in [Placement::Center, Placement::Top] {
            let ctx = Context::default();
            // Two frames: an area is placed with the size measured on the frame before.
            shown_rect(&ctx, 400.0, placement, 1_000.0);
            let rect = shown_rect(&ctx, 400.0, placement, 1_000.0);
            assert!(rect.height() <= 400.0, "{placement:?}: {rect:?}");
        }
    }

    #[test]
    fn a_short_dialog_keeps_its_own_height() {
        let ctx = Context::default();
        shown_rect(&ctx, 800.0, Placement::Center, 120.0);
        let rect = shown_rect(&ctx, 800.0, Placement::Center, 120.0);
        assert!(rect.height() < 200.0, "{rect:?}");
    }
}
