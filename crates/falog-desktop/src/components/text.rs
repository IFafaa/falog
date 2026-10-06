use crate::theme::Theme;
use eframe::egui::text::{LayoutJob, TextFormat, TextWrapping};
use eframe::egui::{Color32, FontId, Galley, Rect, Stroke, Ui, pos2, vec2};
use std::sync::Arc;

/// Lays out text on one line, cut with an ellipsis at `max_width`.
pub fn single_line(
    ui: &Ui,
    text: &str,
    font: FontId,
    color: Color32,
    max_width: f32,
    strikethrough: bool,
) -> Arc<Galley> {
    let format = TextFormat {
        font_id: font,
        color,
        strikethrough: if strikethrough {
            Stroke::new(1.0_f32, color)
        } else {
            Stroke::NONE
        },
        ..TextFormat::default()
    };
    let mut job = LayoutJob::single_section(text.to_owned(), format);
    job.wrap = TextWrapping {
        max_width: max_width.max(0.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.painter().layout_job(job)
}

/// Paints a key combination as separate key caps (`Ctrl` `Shift` `P`) ending at `right`,
/// vertically centered on `center_y`. Returns the width used.
pub fn paint_keybinding(ui: &Ui, keys: &str, right: f32, center_y: f32) -> f32 {
    let theme = Theme::current(ui.ctx());
    let caps: Vec<Arc<Galley>> = keys
        .split('+')
        .map(|key| {
            ui.painter()
                .layout_no_wrap(key.to_owned(), FontId::proportional(11.0), theme.text_muted)
        })
        .collect();
    let (padding, gap, height) = (5.0, 3.0, 18.0);
    let width: f32 =
        caps.iter().map(|g| g.size().x + padding * 2.0).sum::<f32>() + gap * (caps.len() - 1) as f32;
    let mut x = right - width;
    for galley in caps {
        let cap = Rect::from_min_size(
            pos2(x, center_y - height / 2.0),
            vec2(galley.size().x + padding * 2.0, height),
        );
        ui.painter().rect(
            cap,
            3.0,
            theme.element,
            Stroke::new(1.0_f32, theme.border_variant),
        );
        ui.painter().galley(
            cap.center() - galley.size() / 2.0,
            galley.clone(),
            theme.text_muted,
        );
        x = cap.right() + gap;
    }
    width
}
