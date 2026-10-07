//! Color tokens and egui styling of Falog's design system (see `.spec/design-system.md`).
//!
//! The values are the "One Dark" and "One Light" palettes (MIT, see `assets/THIRD-PARTY-NOTICES.md`).

use crate::fonts;
use chrono::NaiveDate;
use eframe::egui::{
    self, Color32, FontFamily, FontId, Rounding, Shadow, Stroke, TextStyle, Visuals, style::WidgetVisuals,
    vec2,
};
use falog_core::date::{self, Urgency};
use falog_core::domain::{Priority, Rgb, Status};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    #[default]
    System,
    Dark,
    Light,
}

impl ThemeMode {
    pub const ALL: [Self; 3] = [Self::System, Self::Dark, Self::Light];

    pub const fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }

    fn preference(self) -> egui::ThemePreference {
        match self {
            Self::System => egui::ThemePreference::System,
            Self::Dark => egui::ThemePreference::Dark,
            Self::Light => egui::ThemePreference::Light,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub dark: bool,

    /// Native title bar color; only Windows lets the app paint it (see `platform::title_bar`).
    #[cfg_attr(
        not(windows),
        expect(dead_code, reason = "macOS and Linux keep the native title bar")
    )]
    pub title_bar: Color32,
    pub status_bar: Color32,
    pub tab_bar: Color32,
    pub tab_active: Color32,
    pub panel: Color32,
    pub editor: Color32,
    pub elevated_surface: Color32,

    pub border: Color32,
    pub border_variant: Color32,
    pub border_focused: Color32,

    pub element: Color32,
    pub element_hover: Color32,
    pub element_active: Color32,
    pub element_selected: Color32,
    pub ghost_hover: Color32,
    pub ghost_selected: Color32,

    pub text: Color32,
    pub text_muted: Color32,
    pub text_placeholder: Color32,
    pub text_accent: Color32,
    pub icon: Color32,
    pub icon_muted: Color32,

    pub accent_background: Color32,
    pub accent_background_hover: Color32,
    pub accent_border: Color32,
    pub selection: Color32,

    pub error: Color32,
    pub error_background: Color32,
    pub warning: Color32,
    pub success: Color32,
    pub hint: Color32,
}

/// Premultiplies a translucent color at compile time.
const fn rgba(hex: u32, alpha: u8) -> Color32 {
    const fn premultiply(channel: u32, alpha: u8) -> u8 {
        ((channel & 0xff) * alpha as u32 / 255) as u8
    }
    Color32::from_rgba_premultiplied(
        premultiply(hex >> 16, alpha),
        premultiply(hex >> 8, alpha),
        premultiply(hex, alpha),
        alpha,
    )
}

const fn rgb_hex(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub const ONE_DARK: Theme = Theme {
    dark: true,
    title_bar: rgb_hex(0x3b414d),
    status_bar: rgb_hex(0x3b414d),
    tab_bar: rgb_hex(0x2f343e),
    tab_active: rgb_hex(0x282c33),
    panel: rgb_hex(0x2f343e),
    editor: rgb_hex(0x282c33),
    elevated_surface: rgb_hex(0x2f343e),
    border: rgb_hex(0x464b57),
    border_variant: rgb_hex(0x363c46),
    border_focused: rgb_hex(0x47679e),
    element: rgb_hex(0x2e343e),
    element_hover: rgb_hex(0x363c46),
    element_active: rgb_hex(0x454a56),
    element_selected: rgb_hex(0x454a56),
    ghost_hover: rgb_hex(0x363c46),
    ghost_selected: rgb_hex(0x454a56),
    text: rgb_hex(0xdce0e5),
    text_muted: rgb_hex(0xa9afbc),
    text_placeholder: rgb_hex(0x878a98),
    text_accent: rgb_hex(0x74ade8),
    icon: rgb_hex(0xdce0e5),
    icon_muted: rgb_hex(0xa9afbc),
    accent_background: rgba(0x74ade8, 0x1a),
    accent_background_hover: rgba(0x74ade8, 0x33),
    accent_border: rgb_hex(0x293b5b),
    selection: rgba(0x74ade8, 0x3d),
    error: rgb_hex(0xd07277),
    error_background: rgba(0xd07277, 0x1a),
    warning: rgb_hex(0xdec184),
    success: rgb_hex(0xa1c181),
    hint: rgb_hex(0x788ca6),
};

pub const ONE_LIGHT: Theme = Theme {
    dark: false,
    title_bar: rgb_hex(0xdcdcdd),
    status_bar: rgb_hex(0xdcdcdd),
    tab_bar: rgb_hex(0xebebec),
    tab_active: rgb_hex(0xfafafa),
    panel: rgb_hex(0xebebec),
    editor: rgb_hex(0xfafafa),
    elevated_surface: rgb_hex(0xebebec),
    border: rgb_hex(0xc9c9ca),
    border_variant: rgb_hex(0xdfdfe0),
    border_focused: rgb_hex(0x7d82e8),
    element: rgb_hex(0xebebec),
    element_hover: rgb_hex(0xdfdfe0),
    element_active: rgb_hex(0xcacaca),
    element_selected: rgb_hex(0xcacaca),
    ghost_hover: rgb_hex(0xdfdfe0),
    ghost_selected: rgb_hex(0xcacaca),
    text: rgb_hex(0x242529),
    text_muted: rgb_hex(0x58585a),
    text_placeholder: rgb_hex(0x7e8086),
    text_accent: rgb_hex(0x5c78e2),
    icon: rgb_hex(0x242529),
    icon_muted: rgb_hex(0x58585a),
    accent_background: rgba(0x5c78e2, 0x1a),
    accent_background_hover: rgba(0x5c78e2, 0x33),
    accent_border: rgb_hex(0xcbcdf6),
    selection: rgba(0x5c78e2, 0x3d),
    error: rgb_hex(0xd36151),
    error_background: rgba(0xd36151, 0x1a),
    warning: rgb_hex(0xa48819),
    success: rgb_hex(0x669f59),
    hint: rgb_hex(0x7274a7),
};

impl Theme {
    /// The theme matching the context's active light/dark mode.
    pub fn current(ctx: &egui::Context) -> &'static Theme {
        if ctx.style().visuals.dark_mode {
            &ONE_DARK
        } else {
            &ONE_LIGHT
        }
    }

    pub fn status_color(&self, status: Status) -> Color32 {
        match status {
            Status::Todo => self.icon_muted,
            Status::InProgress => self.text_accent,
            Status::Waiting => self.warning,
            Status::Done => self.success,
        }
    }

    pub fn priority_color(&self, priority: Priority) -> Color32 {
        match priority {
            Priority::Urgent => self.error,
            Priority::High => self.warning,
            Priority::Medium => self.text_muted,
            Priority::Low => self.hint,
        }
    }

    pub fn due_color(&self, due: NaiveDate, today: NaiveDate) -> Color32 {
        match date::urgency(due, today) {
            Urgency::Overdue { .. } => self.error,
            Urgency::Today | Urgency::Tomorrow => self.warning,
            Urgency::ThisWeek | Urgency::Later => self.text_muted,
        }
    }

    pub fn popover_shadow(&self) -> Shadow {
        Shadow {
            offset: vec2(0.0, 4.0),
            blur: 16.0,
            spread: 0.0,
            color: Color32::from_black_alpha(if self.dark { 110 } else { 40 }),
        }
    }

    fn visuals(&self) -> Visuals {
        let mut visuals = if self.dark {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        let rounding = Rounding::same(4.0);
        let widget = |fill: Color32, stroke: Color32, text: Color32| WidgetVisuals {
            bg_fill: fill,
            weak_bg_fill: fill,
            bg_stroke: Stroke::new(1.0_f32, stroke),
            rounding,
            fg_stroke: Stroke::new(1.0_f32, text),
            expansion: 0.0,
        };
        visuals.widgets.noninteractive = widget(self.editor, self.border_variant, self.text);
        visuals.widgets.inactive = widget(self.element, self.border_variant, self.text);
        visuals.widgets.hovered = widget(self.element_hover, self.border, self.text);
        visuals.widgets.active = widget(self.element_active, self.border_focused, self.text);
        visuals.widgets.open = widget(self.element_selected, self.border, self.text);

        visuals.selection.bg_fill = self.selection;
        visuals.selection.stroke = Stroke::new(1.0_f32, self.text_accent);
        visuals.hyperlink_color = self.text_accent;
        visuals.faint_bg_color = self.panel;
        visuals.extreme_bg_color = self.editor;
        visuals.code_bg_color = self.element;
        visuals.warn_fg_color = self.warning;
        visuals.error_fg_color = self.error;
        visuals.panel_fill = self.editor;
        visuals.window_fill = self.elevated_surface;
        visuals.window_stroke = Stroke::new(1.0_f32, self.border);
        visuals.window_rounding = Rounding::same(8.0);
        visuals.window_shadow = self.popover_shadow();
        visuals.popup_shadow = self.popover_shadow();
        visuals.menu_rounding = Rounding::same(6.0);
        visuals.text_cursor.stroke = Stroke::new(2.0_f32, self.text_accent);
        visuals.striped = false;
        visuals
    }
}

pub fn color(rgb: Rgb) -> Color32 {
    let [r, g, b] = rgb.0;
    Color32::from_rgb(r, g, b)
}

/// Installs both themes, the spacing and type scale, and the user's light/dark preference.
pub fn install(ctx: &egui::Context, mode: ThemeMode) {
    ctx.set_visuals_of(egui::Theme::Dark, ONE_DARK.visuals());
    ctx.set_visuals_of(egui::Theme::Light, ONE_LIGHT.visuals());
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = vec2(8.0, 6.0);
        style.spacing.button_padding = vec2(8.0, 3.0);
        style.spacing.interact_size.y = 24.0;
        style.spacing.menu_margin = egui::Margin::same(4.0);
        style.spacing.window_margin = egui::Margin::same(12.0);
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(12.0)),
            (TextStyle::Body, FontId::proportional(14.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
            (
                TextStyle::Heading,
                FontId::new(18.0, FontFamily::Name(fonts::SEMIBOLD.into())),
            ),
        ]
        .into();
    });
    set_mode(ctx, mode);
}

pub fn set_mode(ctx: &egui::Context, mode: ThemeMode) {
    ctx.set_theme(mode.preference());
}
