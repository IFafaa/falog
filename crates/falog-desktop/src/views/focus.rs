//! What needs attention now, grouped by urgency: the Monday-morning view.

use super::ViewCx;
use crate::action::{Action, Actions};
use crate::components::{badge, single_line};
use crate::fonts;
use crate::icons::Icon;
use crate::theme::{self, Theme};
use eframe::egui::{
    Align2, Color32, CursorIcon, FontId, Frame, Margin, Rect, RichText, ScrollArea, Sense, Ui, pos2, vec2,
};
use falog_core::date;
use falog_core::domain::{Priority, Task};
use falog_core::focus::{self, Bucket};
use std::collections::HashSet;

const MAX_WIDTH: f32 = 880.0;
const ROW_HEIGHT: f32 = 30.0;
const COMPLETED: &str = "Completed in the last 7 days";

#[derive(Debug, Default)]
pub struct FocusState {
    collapsed: HashSet<&'static str>,
}

pub fn show(ui: &mut Ui, cx: &ViewCx<'_>, state: &mut FocusState, actions: &mut Actions) {
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let width = ui.available_width().min(MAX_WIDTH);
        let margin = (ui.available_width() - width) / 2.0;
        ui.horizontal(|ui| {
            ui.add_space(margin);
            ui.vertical(|ui| {
                ui.set_width(width);
                content(ui, cx, state, actions);
            });
        });
    });
}

fn content(ui: &mut Ui, cx: &ViewCx<'_>, state: &mut FocusState, actions: &mut Actions) {
    let theme = cx.theme;
    let summary = focus::summary(cx.tasks, cx.today);

    ui.add_space(20.0);
    ui.label(
        RichText::new(date::format_long(cx.today))
            .font(fonts::semibold(22.0))
            .color(theme.text),
    );
    ui.label(
        RichText::new(format!(
            "{} open · {} overdue · {} due today · {} more this week · {} completed in the last 7 days",
            summary.open,
            summary.overdue,
            summary.due_today,
            summary.due_this_week,
            summary.completed_last_7_days
        ))
        .size(13.0)
        .color(theme.text_muted),
    );
    ui.add_space(10.0);
    area_chips(ui, cx, actions);

    for section in focus::sections(cx.tasks, cx.today)
        .iter()
        .filter(|s| !s.tasks.is_empty())
    {
        let title = section.bucket.title();
        if section_header(
            ui,
            theme,
            title,
            section.tasks.len(),
            bucket_color(theme, section.bucket),
            state,
        ) {
            for task in &section.tasks {
                task_row(ui, cx, task, actions);
            }
        }
    }

    let completed = focus::recently_completed(cx.tasks, cx.today, 7);
    if !completed.is_empty() && section_header(ui, theme, COMPLETED, completed.len(), theme.success, state) {
        for task in completed {
            task_row(ui, cx, task, actions);
        }
    }

    if summary.open == 0 {
        ui.add_space(32.0);
        ui.label(
            RichText::new("All clear. Nothing open right now.")
                .size(15.0)
                .color(theme.text_muted),
        );
    }
    ui.add_space(24.0);
}

fn bucket_color(theme: &Theme, bucket: Bucket) -> Color32 {
    match bucket {
        Bucket::Overdue => theme.error,
        Bucket::DueToday => theme.warning,
        Bucket::DueThisWeek | Bucket::InProgress => theme.text_accent,
        Bucket::Waiting => theme.hint,
        Bucket::Upcoming | Bucket::Backlog => theme.text_muted,
    }
}

fn area_chips(ui: &mut Ui, cx: &ViewCx<'_>, actions: &mut Actions) {
    let theme = cx.theme;
    ui.horizontal_wrapped(|ui| {
        for area in cx.areas {
            let open = cx
                .tasks
                .iter()
                .filter(|t| t.is_open() && t.area_id() == Some(area.id))
                .count();
            if open == 0 {
                continue;
            }
            let color = theme::color(area.color);
            let chip = Frame::none()
                .fill(theme.element)
                .stroke((1.0, theme.border_variant))
                .rounding(12.0)
                .inner_margin(Margin::symmetric(10.0, 3.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.add(Icon::Folder.image(12.0, color));
                        ui.label(RichText::new(&area.name).size(13.0).color(theme.text));
                        ui.label(RichText::new(open.to_string()).size(12.0).color(theme.text_muted));
                    });
                })
                .response;
            let chip = ui.interact(chip.rect, chip.id.with("click"), Sense::click());
            if chip
                .on_hover_text("Show only this area")
                .on_hover_cursor(CursorIcon::PointingHand)
                .clicked()
            {
                actions.push(Action::FilterArea(Some(area.id)));
            }
        }
    });
}

/// Draws a collapsible section header; returns whether the section is expanded.
fn section_header(
    ui: &mut Ui,
    theme: &Theme,
    title: &'static str,
    count: usize,
    color: Color32,
    state: &mut FocusState,
) -> bool {
    ui.add_space(16.0);
    let expanded = !state.collapsed.contains(title);
    let response = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let chevron = if expanded {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            };
            ui.add(chevron.image(14.0, theme.icon_muted));
            ui.label(RichText::new(title).font(fonts::semibold(14.5)).color(color));
            badge(ui, count.to_string());
        })
        .response;
    let response = ui.interact(response.rect, response.id.with("toggle"), Sense::click());
    if response.on_hover_cursor(CursorIcon::PointingHand).clicked() && !state.collapsed.remove(title) {
        state.collapsed.insert(title);
    }
    ui.add_space(2.0);
    ui.painter().hline(
        ui.min_rect().x_range(),
        ui.cursor().top(),
        (1.0, theme.border_variant),
    );
    ui.add_space(2.0);
    expanded
}

fn task_row(ui: &mut Ui, cx: &ViewCx<'_>, task: &Task, actions: &mut Actions) {
    let theme = cx.theme;
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if cx.selected == Some(task.id) {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_selected);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_hover);
    }
    let y = rect.center().y;
    let status_icon = Rect::from_center_size(pos2(rect.left() + 14.0, y), vec2(15.0, 15.0));
    Icon::for_status(task.status).paint(ui, status_icon, 15.0, theme.status_color(task.status));

    // Right-aligned details, laid out from the right edge inwards.
    let mut right = rect.right() - 8.0;
    let mut detail = |text: &str, color: Color32| {
        let galley = ui
            .painter()
            .layout_no_wrap(text.to_owned(), FontId::proportional(12.5), color);
        right -= galley.size().x;
        ui.painter()
            .galley(pos2(right, y - galley.size().y / 2.0), galley, color);
        right -= 14.0;
    };
    if let (Some(due), true) = (task.due, task.is_open()) {
        detail(&date::describe_due(due, cx.today), theme.due_color(due, cx.today));
    }
    if task.priority != Priority::Medium {
        detail(task.priority.label(), theme.priority_color(task.priority));
    }
    detail(task.area_name(), theme.text_muted);
    let area_color = task
        .area
        .as_ref()
        .map_or(theme.text_placeholder, |a| theme::color(a.color));
    ui.painter().circle_filled(pos2(right + 6.0, y), 3.5, area_color);

    let left = rect.left() + 32.0;
    ui.painter().text(
        pos2(left, y),
        Align2::LEFT_CENTER,
        format!("#{}", task.id),
        FontId::monospace(12.0),
        theme.text_placeholder,
    );
    let title_left = left + 44.0;
    let done = !task.is_open();
    let color = if done { theme.text_placeholder } else { theme.text };
    let galley = single_line(
        ui,
        &task.title,
        FontId::proportional(14.0),
        color,
        right - title_left - 8.0,
        done,
    );
    ui.painter()
        .galley(pos2(title_left, y - galley.size().y / 2.0), galley, color);

    if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
        actions.push(Action::OpenTask(task.id));
    }
}
