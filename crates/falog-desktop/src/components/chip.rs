use crate::icons::Icon;
use crate::theme::Theme;
use eframe::egui::{Color32, Frame, Margin, Response, RichText, Ui};

/// A small icon followed by text, used for task metadata (due date, requester, notes).
pub fn meta(ui: &mut Ui, icon: Icon, text: impl Into<String>, color: Color32) -> Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.add(icon.image(12.0, color));
        ui.label(RichText::new(text.into()).size(12.0).color(color));
    })
    .response
}

/// A pill-shaped counter.
pub fn badge(ui: &mut Ui, text: impl Into<String>) -> Response {
    let theme = Theme::current(ui.ctx());
    Frame::none()
        .fill(theme.element)
        .rounding(8.0)
        .inner_margin(Margin::symmetric(6.0, 0.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text.into()).size(11.5).color(theme.text_muted))
        })
        .response
}
