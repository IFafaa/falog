//! Done tasks put away from the board, grouped by the month they were completed.

use super::ViewCx;
use super::board::task_menu;
use crate::action::{Action, Actions};
use crate::components::{badge, single_line};
use crate::fonts;
use crate::icons::Icon;
use crate::theme;
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use eframe::egui::{Align2, CursorIcon, FontId, Id, Rect, RichText, ScrollArea, Sense, Ui, pos2, vec2};
use falog_core::domain::{Status, Task};
use std::collections::HashSet;

const MAX_WIDTH: f32 = 880.0;
const ROW_HEIGHT: f32 = 30.0;

#[derive(Debug, Default)]
pub struct ArchiveState {
    /// Collapsed months, as (year, month).
    collapsed: HashSet<(i32, u32)>,
}

/// Archived tasks of one month, newest first.
#[derive(Debug, PartialEq)]
struct Month<'a> {
    year: i32,
    month: u32,
    tasks: Vec<&'a Task>,
}

impl Month<'_> {
    fn title(&self) -> String {
        NaiveDate::from_ymd_opt(self.year, self.month, 1)
            .map_or_else(String::new, |first| first.format("%B %Y").to_string())
    }
}

/// When a task was finished: its completion time, else when it was archived.
fn finished_at(task: &Task) -> Option<NaiveDateTime> {
    task.completed_at.or(task.archived_at)
}

/// Groups tasks by the month they were finished, newest month and task first.
fn months(tasks: &[Task]) -> Vec<Month<'_>> {
    let mut sorted: Vec<&Task> = tasks.iter().collect();
    sorted.sort_by_key(|t| std::cmp::Reverse((finished_at(t), t.id)));
    let mut months: Vec<Month<'_>> = Vec::new();
    for task in sorted {
        let (year, month) = finished_at(task).map_or((0, 0), |at| (at.year(), at.month()));
        match months.last_mut() {
            Some(last) if (last.year, last.month) == (year, month) => last.tasks.push(task),
            _ => months.push(Month {
                year,
                month,
                tasks: vec![task],
            }),
        }
    }
    months
}

pub fn show(ui: &mut Ui, cx: &ViewCx<'_>, state: &mut ArchiveState, actions: &mut Actions) {
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

fn content(ui: &mut Ui, cx: &ViewCx<'_>, state: &mut ArchiveState, actions: &mut Actions) {
    let theme = cx.theme;
    ui.add_space(20.0);
    ui.label(
        RichText::new("Archive")
            .font(fonts::semibold(22.0))
            .color(theme.text),
    );

    if cx.tasks.is_empty() {
        let (title, hint) = if cx.has_any_task {
            (
                "No matching archived tasks",
                "Clear the search or pick another area in the sidebar.",
            )
        } else {
            (
                "Nothing archived yet",
                "Archive finished tasks from the board's Done column, or right-click a done task.",
            )
        };
        ui.add_space(32.0);
        ui.label(RichText::new(title).size(15.0).color(theme.text_muted));
        ui.label(RichText::new(hint).size(13.0).color(theme.text_placeholder));
        return;
    }

    let count = cx.tasks.len();
    ui.label(
        RichText::new(format!(
            "{count} archived {} · restore one to put it back in Done",
            if count == 1 { "task" } else { "tasks" }
        ))
        .size(13.0)
        .color(theme.text_muted),
    );

    for month in months(cx.tasks) {
        if month_header(ui, cx, &month, state) {
            for task in &month.tasks {
                task_row(ui, cx, task, actions);
            }
        }
    }
    ui.add_space(24.0);
}

/// A collapsible month header; returns whether the month is expanded.
fn month_header(ui: &mut Ui, cx: &ViewCx<'_>, month: &Month<'_>, state: &mut ArchiveState) -> bool {
    let theme = cx.theme;
    let key = (month.year, month.month);
    ui.add_space(16.0);
    let expanded = !state.collapsed.contains(&key);
    let response = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let chevron = if expanded {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            };
            ui.add(chevron.image(14.0, theme.icon_muted));
            ui.label(
                RichText::new(month.title())
                    .font(fonts::semibold(14.5))
                    .color(theme.text),
            );
            badge(ui, month.tasks.len().to_string());
        })
        .response;
    let response = ui.interact(response.rect, response.id.with("toggle"), Sense::click());
    if response.on_hover_cursor(CursorIcon::PointingHand).clicked() && !state.collapsed.remove(&key) {
        state.collapsed.insert(key);
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
    // Long lists: rows scrolled out of view take their space but draw nothing.
    if !ui.is_rect_visible(rect) {
        return;
    }
    let hovered = ui.rect_contains_pointer(rect);
    if cx.selected == Some(task.id) {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_selected);
    } else if hovered {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_hover);
    }
    let y = rect.center().y;
    let status_icon = Rect::from_center_size(pos2(rect.left() + 14.0, y), vec2(15.0, 15.0));
    Icon::for_status(Status::Done).paint(ui, status_icon, 15.0, theme.status_color(Status::Done));

    // Right side: the restore button while hovered, else when the task was finished.
    let mut right = rect.right() - 8.0;
    if hovered {
        let button = Rect::from_center_size(pos2(right - 12.0, y), vec2(24.0, 24.0));
        let restore = ui
            .interact(button, Id::new(("archive-restore", task.id)), Sense::click())
            .on_hover_text("Restore to the board")
            .on_hover_cursor(CursorIcon::PointingHand);
        if restore.hovered() {
            ui.painter().rect_filled(button, 4.0, theme.ghost_selected);
        }
        Icon::ArchiveRestore.paint(ui, button, 14.0, theme.icon);
        if restore.clicked() {
            actions.push(Action::RestoreTask(task.id));
        }
        right = button.left() - 10.0;
    } else if let Some(at) = finished_at(task) {
        let galley = ui.painter().layout_no_wrap(
            at.format("%b %-d").to_string(),
            FontId::proportional(12.5),
            theme.text_placeholder,
        );
        right -= galley.size().x;
        ui.painter().galley(
            pos2(right, y - galley.size().y / 2.0),
            galley,
            theme.text_placeholder,
        );
        right -= 14.0;
    }
    let area = ui.painter().layout_no_wrap(
        task.area_name().to_owned(),
        FontId::proportional(12.5),
        theme.text_muted,
    );
    right -= area.size().x;
    ui.painter()
        .galley(pos2(right, y - area.size().y / 2.0), area, theme.text_muted);
    let area_color = task
        .area
        .as_ref()
        .map_or(theme.text_placeholder, |a| theme::color(a.color));
    ui.painter().circle_filled(pos2(right - 8.0, y), 3.5, area_color);
    right -= 22.0;

    let left = rect.left() + 32.0;
    ui.painter().text(
        pos2(left, y),
        Align2::LEFT_CENTER,
        format!("#{}", task.id),
        FontId::monospace(12.0),
        theme.text_placeholder,
    );
    let title_left = left + 44.0;
    let galley = single_line(
        ui,
        &task.title,
        FontId::proportional(14.0),
        theme.text_muted,
        right - title_left,
        false,
    );
    ui.painter().galley(
        pos2(title_left, y - galley.size().y / 2.0),
        galley,
        theme.text_muted,
    );

    if response.clicked() {
        actions.push(Action::OpenTask(task.id));
    }
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .context_menu(|ui| task_menu(ui, task, actions));
}

#[cfg(test)]
mod tests {
    use super::*;
    use falog_core::domain::{Priority, TaskId};

    fn task(id: i64, completed: &str) -> Task {
        let at = NaiveDateTime::parse_from_str(completed, "%Y-%m-%d %H:%M").unwrap();
        Task {
            id: TaskId(id),
            title: format!("task {id}"),
            description: String::new(),
            area: None,
            status: Status::Done,
            priority: Priority::Medium,
            due: None,
            requester: String::new(),
            note_count: 0,
            created_at: at,
            updated_at: at,
            completed_at: Some(at),
            archived_at: Some(at),
        }
    }

    #[test]
    fn groups_by_month_newest_first() {
        let tasks = vec![
            task(1, "2026-09-03 10:00"),
            task(2, "2026-10-07 09:00"),
            task(3, "2026-10-01 18:00"),
        ];
        let months = months(&tasks);
        let titles: Vec<String> = months.iter().map(Month::title).collect();
        assert_eq!(titles, ["October 2026", "September 2026"]);
        let ids: Vec<i64> = months[0].tasks.iter().map(|t| t.id.0).collect();
        assert_eq!(ids, [2, 3]);
    }
}
