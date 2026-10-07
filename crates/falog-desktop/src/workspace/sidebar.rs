//! Left dock listing areas, styled like Zed's project panel.

use super::BAR_HEIGHT;
use crate::action::{Action, Actions};
use crate::components::{ButtonStyle, button, icon_button, single_line};
use crate::fonts;
use crate::icons::Icon;
use crate::theme::{self, Theme};
use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Frame, Layout, Margin, Rect, Response, RichText,
    ScrollArea, Sense, SidePanel, Stroke, Ui, pos2, vec2,
};
use falog_core::domain::{Area, AreaId, Task};

const ROW_HEIGHT: f32 = 26.0;

pub fn show(
    ctx: &egui::Context,
    theme: &Theme,
    tasks: &[Task],
    areas: &[Area],
    selected: Option<AreaId>,
    actions: &mut Actions,
) {
    SidePanel::left("sidebar")
        .resizable(true)
        .default_width(232.0)
        .width_range(180.0..=360.0)
        .frame(Frame::none().fill(theme.panel))
        .show(ctx, |ui| {
            header(ui, theme, actions);
            let open = |area: Option<AreaId>| {
                tasks
                    .iter()
                    .filter(|t| t.is_open() && (area.is_none() || t.area_id() == area))
                    .count()
            };
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.add_space(4.0);
                ui.spacing_mut().item_spacing.y = 0.0;
                let all = row(
                    ui,
                    theme,
                    Icon::FolderOpen,
                    theme.icon_muted,
                    "All areas",
                    open(None),
                    selected.is_none(),
                );
                if all.clicked() {
                    actions.push(Action::FilterArea(None));
                }
                for area in areas {
                    let is_selected = selected == Some(area.id);
                    let color = theme::color(area.color);
                    let response = row(
                        ui,
                        theme,
                        Icon::Folder,
                        color,
                        &area.name,
                        open(Some(area.id)),
                        is_selected,
                    );
                    if response.clicked() {
                        actions.push(Action::FilterArea(Some(area.id)));
                    }
                    response.context_menu(|ui| {
                        if ui.button("Manage areas…").clicked() {
                            actions.push(Action::ManageAreas);
                            ui.close_menu();
                        }
                    });
                }
                if areas.is_empty() {
                    empty_state(ui, theme, actions);
                }
            });
        });
}

fn header(ui: &mut Ui, theme: &Theme, actions: &mut Actions) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), BAR_HEIGHT), Sense::hover());
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0_f32, theme.border_variant),
    );
    ui.painter().text(
        pos2(rect.left() + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        "Areas",
        fonts::semibold(13.0),
        theme.text_muted,
    );
    let button_rect = Rect::from_min_max(pos2(rect.right() - 32.0, rect.top()), rect.max);
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(button_rect)
            .layout(Layout::right_to_left(Align::Center)),
        |ui| {
            ui.add_space(6.0);
            if icon_button(ui, Icon::Settings, "Manage areas").clicked() {
                actions.push(Action::ManageAreas);
            }
        },
    );
}

/// A project-panel row: icon, name, and the number of open tasks.
fn row(
    ui: &mut Ui,
    theme: &Theme,
    icon: Icon,
    icon_color: Color32,
    label: &str,
    count: usize,
    selected: bool,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if selected {
        ui.painter().rect(
            rect.shrink2(vec2(0.0, 0.5)),
            0.0,
            theme.ghost_selected,
            Stroke::new(1.0_f32, theme.border_focused),
        );
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme.ghost_hover);
    }
    let y = rect.center().y;
    let icon_rect = Rect::from_center_size(pos2(rect.left() + 20.0, y), vec2(14.0, 14.0));
    icon.paint(ui, icon_rect, 14.0, icon_color);

    let count_galley = ui.painter().layout_no_wrap(
        count.to_string(),
        FontId::proportional(12.0),
        theme.text_placeholder,
    );
    let count_x = rect.right() - 12.0 - count_galley.size().x;
    let text_color = if selected { theme.text } else { theme.text_muted };
    let name = single_line(
        ui,
        label,
        FontId::proportional(13.5),
        text_color,
        count_x - icon_rect.right() - 14.0,
        false,
    );
    ui.painter().galley(
        pos2(icon_rect.right() + 7.0, y - name.size().y / 2.0),
        name,
        text_color,
    );
    ui.painter().galley(
        pos2(count_x, y - count_galley.size().y / 2.0),
        count_galley,
        theme.text_placeholder,
    );
    response.on_hover_cursor(CursorIcon::PointingHand)
}

fn empty_state(ui: &mut Ui, theme: &Theme, actions: &mut Actions) {
    Frame::none()
        .inner_margin(Margin::symmetric(12.0, 12.0))
        .show(ui, |ui| {
            ui.label(RichText::new("No areas yet.").size(13.0).color(theme.text_muted));
            ui.add_space(6.0);
            if button(ui, ButtonStyle::Filled, Some(Icon::Plus), "Add area").clicked() {
                actions.push(Action::ManageAreas);
            }
        });
}
