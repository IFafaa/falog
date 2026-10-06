use crate::theme::Theme;
use eframe::egui::{
    Color32, CursorIcon, FontId, Galley, Rect, Response, Sense, Stroke, Ui, lerp, pos2, vec2,
};
use std::sync::Arc;

/// On/off switch (Zed's `Switch`).
pub fn switch(ui: &mut Ui, on: &mut bool) -> Response {
    let theme = Theme::current(ui.ctx());
    let (rect, mut response) = ui.allocate_exact_size(vec2(30.0, 16.0), Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool_responsive(response.id, *on);
        let track = if *on {
            theme.text_accent
        } else {
            theme.element_selected
        };
        ui.painter().rect_filled(rect, rect.height() / 2.0, track);
        let radius = rect.height() / 2.0 - 2.0;
        let x = lerp((rect.left() + radius + 2.0)..=(rect.right() - radius - 2.0), t);
        let knob = if *on { theme.editor } else { theme.icon };
        ui.painter().circle_filled(pos2(x, rect.center().y), radius, knob);
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// A row of mutually exclusive options. Returns `true` when the value changed.
///
/// Painted as a single block so the options keep their order in any parent layout.
pub fn segmented<T: Copy + PartialEq>(ui: &mut Ui, value: &mut T, options: &[(T, &str)]) -> bool {
    const PADDING: f32 = 2.0;
    const HEIGHT: f32 = 20.0;
    let theme = Theme::current(ui.ctx());
    let segments: Vec<(T, bool, Arc<Galley>)> = options
        .iter()
        .map(|(option, label)| {
            let selected = *value == *option;
            let color = if selected { theme.text } else { theme.text_muted };
            let galley = ui
                .painter()
                .layout_no_wrap((*label).to_owned(), FontId::proportional(13.0), color);
            (*option, selected, galley)
        })
        .collect();
    let width = segments.iter().map(|(_, _, g)| g.size().x + 20.0).sum::<f32>() + PADDING * 2.0;
    let (outer, _) = ui.allocate_exact_size(vec2(width, HEIGHT + PADDING * 2.0), Sense::hover());
    ui.painter().rect(
        outer,
        4.0,
        theme.element,
        Stroke::new(1.0_f32, theme.border_variant),
    );

    let mut changed = false;
    let mut x = outer.left() + PADDING;
    for (index, (option, selected, galley)) in segments.into_iter().enumerate() {
        let rect = Rect::from_min_size(
            pos2(x, outer.top() + PADDING),
            vec2(galley.size().x + 20.0, HEIGHT),
        );
        x = rect.right();
        let response = ui.interact(rect, ui.id().with(("segment", index)), Sense::click());
        let fill = if selected {
            theme.element_selected
        } else if response.hovered() {
            theme.ghost_hover
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 3.0, fill);
        let color = if selected { theme.text } else { theme.text_muted };
        ui.painter()
            .galley(rect.center() - galley.size() / 2.0, galley, color);
        if response.on_hover_cursor(CursorIcon::PointingHand).clicked() && !selected {
            *value = option;
            changed = true;
        }
    }
    changed
}
