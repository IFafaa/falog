use crate::icons::Icon;
use crate::theme::Theme;
use eframe::egui::{Color32, CursorIcon, FontId, Rect, Response, Sense, Stroke, Ui, pos2, vec2};

const HEIGHT: f32 = 24.0;
const ICON: f32 = 14.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonStyle {
    /// Transparent until hovered; the default in toolbars.
    Ghost,
    /// Element background with a border.
    Filled,
    /// Accent tint, for the primary action of a surface.
    Accent,
    /// Error tint, for destructive actions.
    Danger,
}

pub fn button(ui: &mut Ui, style: ButtonStyle, icon: Option<Icon>, label: &str) -> Response {
    let theme = Theme::current(ui.ctx());
    let (text, fill, stroke, hover_fill) = match style {
        ButtonStyle::Ghost => (theme.text, Color32::TRANSPARENT, Stroke::NONE, theme.ghost_hover),
        ButtonStyle::Filled => (
            theme.text,
            theme.element,
            Stroke::new(1.0_f32, theme.border_variant),
            theme.element_hover,
        ),
        ButtonStyle::Accent => (
            theme.text_accent,
            theme.accent_background,
            Stroke::new(1.0_f32, theme.accent_border),
            theme.accent_background_hover,
        ),
        ButtonStyle::Danger => (
            theme.error,
            Color32::TRANSPARENT,
            Stroke::NONE,
            theme.error_background,
        ),
    };
    let icon_color = if style == ButtonStyle::Ghost {
        theme.icon_muted
    } else {
        text
    };

    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(13.5), text);
    let icon_width = match (icon, label.is_empty()) {
        (None, _) => 0.0,
        (Some(_), true) => ICON,
        (Some(_), false) => ICON + 6.0,
    };
    let padding = 8.0;
    let size = vec2(padding * 2.0 + icon_width + galley.size().x, HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    if ui.is_rect_visible(rect) {
        let enabled = ui.is_enabled();
        let fade = |c: Color32| if enabled { c } else { c.gamma_multiply(0.45) };
        let fill = if enabled && response.hovered() {
            hover_fill
        } else {
            fill
        };
        ui.painter().rect(rect, 4.0, fill, stroke);
        let mut x = rect.left() + padding;
        if let Some(icon) = icon {
            let icon_rect = Rect::from_min_size(pos2(x, rect.center().y - ICON / 2.0), vec2(ICON, ICON));
            icon.paint(ui, icon_rect, ICON, fade(icon_color));
            x += icon_width;
        }
        let y = rect.center().y - galley.size().y / 2.0;
        ui.painter().galley(pos2(x, y), galley, fade(text));
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// Square ghost button with just an icon, as in the tab and status bars.
pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    icon_toggle(ui, icon, false, tooltip)
}

/// Icon button that stays highlighted while `selected`.
pub fn icon_toggle(ui: &mut Ui, icon: Icon, selected: bool, tooltip: &str) -> Response {
    let theme = Theme::current(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(vec2(HEIGHT, HEIGHT), Sense::click());
    if ui.is_rect_visible(rect) {
        let fill = if response.is_pointer_button_down_on() {
            theme.element_active
        } else if response.hovered() {
            theme.ghost_hover
        } else if selected {
            theme.ghost_selected
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, 4.0, fill);
        let tint = if selected || response.hovered() {
            theme.icon
        } else {
            theme.icon_muted
        };
        icon.paint(ui, rect, ICON, tint);
    }
    response
        .on_hover_text(tooltip)
        .on_hover_cursor(CursorIcon::PointingHand)
}
