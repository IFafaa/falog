//! Dense table of tasks, in the style of Zed's tables.

use super::ViewCx;
use super::board::task_menu;
use crate::action::{Action, Actions};
use crate::components::single_line;
use crate::icons::Icon;
use crate::theme;
use eframe::egui::{
    Align2, CursorIcon, FontId, Id, Rect, RichText, ScrollArea, Sense, Stroke, Ui, pos2, vec2,
};
use falog_core::date;
use falog_core::domain::{Status, Task};

const ROW_HEIGHT: f32 = 30.0;
const HEADER_HEIGHT: f32 = 28.0;
const MIN_TITLE_WIDTH: f32 = 180.0;

/// Column widths; the title column takes the remaining space.
struct Columns {
    status: f32,
    id: f32,
    title: f32,
    company: f32,
    priority: f32,
    due: f32,
    requester: f32,
}

impl Columns {
    fn fit(width: f32) -> Self {
        let mut columns = Self {
            status: 32.0,
            id: 52.0,
            title: 0.0,
            company: 170.0,
            priority: 96.0,
            due: 120.0,
            requester: 130.0,
        };
        let fixed = columns.status
            + columns.id
            + columns.company
            + columns.priority
            + columns.due
            + columns.requester;
        columns.title = (width - fixed).max(MIN_TITLE_WIDTH);
        columns
    }

    /// Left edges of each column.
    fn edges(&self, left: f32) -> [f32; 7] {
        let widths = [
            self.status,
            self.id,
            self.title,
            self.company,
            self.priority,
            self.due,
            self.requester,
        ];
        let mut edges = [left; 7];
        for i in 1..7 {
            edges[i] = edges[i - 1] + widths[i - 1];
        }
        edges
    }
}

pub fn show(ui: &mut Ui, cx: &ViewCx<'_>, actions: &mut Actions) {
    let mut rows: Vec<&Task> = cx
        .tasks
        .iter()
        .filter(|t| cx.prefs.show_completed || t.is_open())
        .collect();
    cx.prefs.sort.sort(&mut rows);
    rows.sort_by_key(|t| !t.is_open()); // stable: completed tasks go last

    if rows.is_empty() {
        empty_state(ui, cx);
        return;
    }

    let columns = Columns::fit(ui.available_width());
    header(ui, cx, &columns);
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, ROW_HEIGHT, rows.len(), |ui, range| {
            for task in &rows[range] {
                row(ui, cx, &columns, task, actions);
            }
        });
}

fn header(ui: &mut Ui, cx: &ViewCx<'_>, columns: &Columns) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEADER_HEIGHT), Sense::hover());
    let edges = columns.edges(rect.left());
    let labels = ["", "#", "Task", "Company", "Priority", "Due", "Requested by"];
    for (edge, label) in edges.iter().zip(labels) {
        ui.painter().text(
            pos2(edge + 6.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(12.0),
            cx.theme.text_muted,
        );
    }
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0_f32, cx.theme.border_variant),
    );
}

fn row(ui: &mut Ui, cx: &ViewCx<'_>, columns: &Columns, task: &Task, actions: &mut Actions) {
    let theme = cx.theme;
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let selected = cx.selected == Some(task.id);
    if selected {
        ui.painter().rect_filled(rect, 0.0, theme.ghost_selected);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme.ghost_hover);
    }
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0_f32, theme.border_variant),
    );

    let edges = columns.edges(rect.left());
    let y = rect.center().y;
    let cell = |index: usize, width: f32| {
        Rect::from_min_size(pos2(edges[index], rect.top()), vec2(width, ROW_HEIGHT))
    };
    let text_at = |index: usize, width: f32, text: &str, color, strike: bool| {
        let galley = single_line(ui, text, FontId::proportional(13.5), color, width - 12.0, strike);
        ui.painter()
            .galley(pos2(edges[index] + 6.0, y - galley.size().y / 2.0), galley, color);
    };

    // Status: click the icon to complete or reopen.
    let status_rect = cell(0, columns.status);
    let status_response = ui
        .interact(status_rect, Id::new(("list-status", task.id)), Sense::click())
        .on_hover_text(if task.is_open() { "Mark as done" } else { "Reopen" })
        .on_hover_cursor(CursorIcon::PointingHand);
    Icon::for_status(task.status).paint(ui, status_rect, 15.0, theme.status_color(task.status));
    if status_response.clicked() {
        let status = if task.is_open() {
            Status::Done
        } else {
            Status::Todo
        };
        actions.push(Action::MoveTask(task.id, status));
    }

    let id_galley = single_line(
        ui,
        &format!("#{}", task.id),
        FontId::monospace(12.0),
        theme.text_placeholder,
        columns.id,
        false,
    );
    ui.painter().galley(
        pos2(edges[1] + 6.0, y - id_galley.size().y / 2.0),
        id_galley,
        theme.text_placeholder,
    );

    let done = !task.is_open();
    text_at(
        2,
        columns.title,
        &task.title,
        if done { theme.text_placeholder } else { theme.text },
        done,
    );

    let company_color = task
        .company
        .as_ref()
        .map_or(theme.text_placeholder, |c| theme::color(c.color));
    ui.painter()
        .circle_filled(pos2(edges[3] + 10.0, y), 4.0, company_color);
    let galley = single_line(
        ui,
        task.company_name(),
        FontId::proportional(13.0),
        theme.text_muted,
        columns.company - 28.0,
        false,
    );
    ui.painter().galley(
        pos2(edges[3] + 20.0, y - galley.size().y / 2.0),
        galley,
        theme.text_muted,
    );

    let priority_color = theme.priority_color(task.priority);
    let priority_icon = Rect::from_min_size(pos2(edges[4] + 6.0, y - 6.0), vec2(12.0, 12.0));
    Icon::for_priority(task.priority).paint(ui, priority_icon, 12.0, priority_color);
    let galley = single_line(
        ui,
        task.priority.label(),
        FontId::proportional(13.0),
        priority_color,
        columns.priority - 30.0,
        false,
    );
    ui.painter().galley(
        pos2(edges[4] + 24.0, y - galley.size().y / 2.0),
        galley,
        priority_color,
    );

    match task.due {
        Some(due) => {
            let color = if task.is_open() {
                theme.due_color(due, cx.today)
            } else {
                theme.text_placeholder
            };
            text_at(5, columns.due, &date::describe_due(due, cx.today), color, false);
        }
        None => text_at(5, columns.due, "—", theme.text_placeholder, false),
    }
    text_at(6, columns.requester, &task.requester, theme.text_muted, false);

    if response.clicked() {
        actions.push(Action::OpenTask(task.id));
    }
    response
        .on_hover_cursor(CursorIcon::PointingHand)
        .context_menu(|ui| task_menu(ui, task, actions));
}

fn empty_state(ui: &mut Ui, cx: &ViewCx<'_>) {
    ui.add_space(48.0);
    ui.vertical_centered(|ui| {
        let (title, hint) = if cx.has_any_task {
            (
                "No matching tasks",
                "Clear the search or pick another company in the sidebar.",
            )
        } else {
            (
                "No tasks yet",
                "Tell your assistant about a request, or press Ctrl+N.",
            )
        };
        ui.label(RichText::new(title).size(15.0).color(cx.theme.text_muted));
        ui.label(RichText::new(hint).size(13.0).color(cx.theme.text_placeholder));
    });
}
