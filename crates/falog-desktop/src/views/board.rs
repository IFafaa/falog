//! Kanban board: one column per status, drag cards between columns.

use super::ViewCx;
use crate::action::{Action, Actions};
use crate::components::{badge, icon_button, meta};
use crate::fonts;
use crate::icons::Icon;
use crate::theme::{self, Theme};
use eframe::egui::{
    self, Align, CursorIcon, FontId, Frame, Id, Layout, Margin, Pos2, Rect, RichText, ScrollArea, Sense,
    Stroke, Ui, vec2,
};
use falog_core::date;
use falog_core::domain::{Priority, Status, Task, TaskId};

const COLUMN_GAP: f32 = 12.0;
const MIN_COLUMN_WIDTH: f32 = 220.0;
/// Completed tasks older than this are hidden unless the user asks for them.
const RECENT_DAYS: i64 = 7;

#[derive(Debug, Default)]
pub struct BoardState {
    dragging: Option<TaskId>,
}

pub fn show(ui: &mut Ui, cx: &ViewCx<'_>, state: &mut BoardState, actions: &mut Actions) {
    let pointer = ui.ctx().pointer_interact_pos();
    let columns = Status::ALL.len() as f32;
    let width = ((ui.available_width() - COLUMN_GAP * (columns - 1.0)) / columns).max(MIN_COLUMN_WIDTH);
    let height = ui.available_height();
    let mut drop_target = None;

    ScrollArea::horizontal()
        .id_salt("board")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = COLUMN_GAP;
                for status in Status::ALL {
                    let rect = column(ui, cx, state, status, vec2(width, height), actions);
                    if state.dragging.is_some() && pointer.is_some_and(|p| rect.contains(p)) {
                        drop_target = Some(status);
                        ui.painter().rect(
                            rect,
                            6.0,
                            cx.theme.ghost_hover.gamma_multiply(0.35),
                            Stroke::new(1.0_f32, cx.theme.border_focused),
                        );
                    }
                }
            });
        });

    if let Some(id) = state.dragging {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        if let (Some(pointer), Some(task)) = (pointer, cx.tasks.iter().find(|t| t.id == id)) {
            paint_drag_preview(ui.ctx(), cx.theme, pointer, task);
        }
        if !ui.input(|i| i.pointer.any_down()) {
            if let Some(status) = drop_target {
                actions.push(Action::MoveTask(id, status));
            }
            state.dragging = None;
        }
    }
}

fn column_tasks<'a>(cx: &ViewCx<'a>, status: Status) -> (Vec<&'a Task>, usize) {
    let mut tasks: Vec<&Task> = cx.tasks.iter().filter(|t| t.status == status).collect();
    if status != Status::Done {
        cx.prefs.sort.sort(&mut tasks);
        return (tasks, 0);
    }
    let total = tasks.len();
    if !cx.prefs.show_old_completed {
        tasks.retain(|t| {
            t.completed_on()
                .is_none_or(|d| (cx.today - d).num_days() <= RECENT_DAYS)
        });
    }
    tasks.sort_by_key(|t| std::cmp::Reverse(t.completed_at));
    let hidden = total - tasks.len();
    (tasks, hidden)
}

fn column(
    ui: &mut Ui,
    cx: &ViewCx<'_>,
    state: &mut BoardState,
    status: Status,
    size: egui::Vec2,
    actions: &mut Actions,
) -> Rect {
    let (tasks, hidden) = column_tasks(cx, status);
    ui.allocate_ui_with_layout(size, Layout::top_down(Align::Min), |ui| {
        ui.set_min_size(size);
        column_header(ui, cx.theme, status, tasks.len(), tasks.len() + hidden, actions);
        ui.add_space(4.0);
        ScrollArea::vertical()
            .id_salt(("board-column", status.as_str()))
            .auto_shrink([false, false])
            .drag_to_scroll(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for task in &tasks {
                    card(ui, cx, state, task, actions);
                }
                if tasks.is_empty() {
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("No tasks")
                                .size(12.5)
                                .color(cx.theme.text_placeholder),
                        );
                    });
                }
                if hidden > 0 {
                    ui.label(
                        RichText::new(format!("{hidden} completed more than a week ago"))
                            .size(12.0)
                            .color(cx.theme.text_placeholder),
                    );
                }
            });
    })
    .response
    .rect
}

/// `done` counts every done task the column holds, including the ones hidden for being old.
fn column_header(
    ui: &mut Ui,
    theme: &Theme,
    status: Status,
    count: usize,
    done: usize,
    actions: &mut Actions,
) {
    ui.horizontal(|ui| {
        ui.set_height(28.0);
        ui.add_space(4.0);
        ui.add(Icon::for_status(status).image(14.0, theme.status_color(status)));
        ui.label(
            RichText::new(status.label())
                .font(fonts::semibold(13.5))
                .color(theme.text),
        );
        badge(ui, count.to_string());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if status != Status::Done {
                if icon_button(ui, Icon::Plus, &format!("New task in {}", status.label())).clicked() {
                    actions.push(Action::NewTask(status));
                }
            } else if done > 0 {
                let tooltip = match done {
                    1 => "Archive the done task".to_owned(),
                    n => format!("Archive the {n} done tasks"),
                };
                if icon_button(ui, Icon::Archive, &tooltip).clicked() {
                    actions.push(Action::ArchiveDone);
                }
            }
        });
    });
}

fn card(ui: &mut Ui, cx: &ViewCx<'_>, state: &mut BoardState, task: &Task, actions: &mut Actions) {
    let theme = cx.theme;
    let selected = cx.selected == Some(task.id);
    let area_color = task
        .area
        .as_ref()
        .map_or(theme.text_placeholder, |a| theme::color(a.color));
    let done = task.status == Status::Done;

    let frame = Frame::none()
        .fill(theme.elevated_surface)
        .stroke(Stroke::new(
            1.0_f32,
            if selected {
                theme.border_focused
            } else {
                theme.border_variant
            },
        ))
        .rounding(6.0)
        .inner_margin(Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
            ui.horizontal(|ui| {
                ui.add(Icon::Folder.image(12.0, area_color));
                ui.label(RichText::new(task.area_name()).size(12.0).color(theme.text_muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("#{}", task.id))
                            .monospace()
                            .size(11.5)
                            .color(theme.text_placeholder),
                    );
                });
            });
            let mut title = RichText::new(&task.title).size(14.0).color(theme.text);
            if done {
                title = title.strikethrough().color(theme.text_placeholder);
            }
            ui.label(title);
            card_meta(ui, cx, task);
        });

    let rect = frame.response.rect;
    let response = ui.interact(rect, Id::new(("card", task.id)), Sense::click_and_drag());
    if state.dragging == Some(task.id) {
        ui.painter()
            .rect_filled(rect, 6.0, theme.editor.gamma_multiply(0.6));
    } else if response.hovered() && !selected && state.dragging.is_none() {
        ui.painter()
            .rect_stroke(rect, 6.0, Stroke::new(1.0_f32, theme.border));
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if response.drag_started() {
        state.dragging = Some(task.id);
    }
    if response.clicked() {
        actions.push(Action::OpenTask(task.id));
    }
    response.context_menu(|ui| task_menu(ui, task, actions));
}

fn card_meta(ui: &mut Ui, cx: &ViewCx<'_>, task: &Task) {
    let theme = cx.theme;
    let show_priority = task.priority != Priority::Medium;
    if !show_priority && task.due.is_none() && task.requester.is_empty() && task.note_count == 0 {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        if show_priority {
            meta(
                ui,
                Icon::for_priority(task.priority),
                task.priority.label(),
                theme.priority_color(task.priority),
            );
        }
        if let Some(due) = task.due {
            let color = if task.is_open() {
                theme.due_color(due, cx.today)
            } else {
                theme.text_placeholder
            };
            meta(ui, Icon::Clock, date::describe_due(due, cx.today), color);
        }
        if !task.requester.is_empty() {
            meta(ui, Icon::Person, &task.requester, theme.text_muted);
        }
        if task.note_count > 0 {
            meta(ui, Icon::Notepad, task.note_count.to_string(), theme.text_muted)
                .on_hover_text(format!("{} notes in the activity log", task.note_count));
        }
    });
}

/// Right-click menu shared by board cards and list rows.
pub fn task_menu(ui: &mut Ui, task: &Task, actions: &mut Actions) {
    if ui.button("Open").clicked() {
        actions.push(Action::OpenTask(task.id));
        ui.close_menu();
    }
    ui.menu_button("Move to", |ui| {
        for status in Status::ALL.into_iter().filter(|s| *s != task.status) {
            if ui.button(status.label()).clicked() {
                actions.push(Action::MoveTask(task.id, status));
                ui.close_menu();
            }
        }
    });
    if task.is_archived() {
        if ui.button("Restore to the board").clicked() {
            actions.push(Action::RestoreTask(task.id));
            ui.close_menu();
        }
    } else if task.status == Status::Done && ui.button("Archive").clicked() {
        actions.push(Action::ArchiveTask(task.id));
        ui.close_menu();
    }
    ui.separator();
    if ui.button("Delete…").clicked() {
        actions.push(Action::DeleteTask(task.id));
        ui.close_menu();
    }
}

fn paint_drag_preview(ctx: &egui::Context, theme: &Theme, pointer: Pos2, task: &Task) {
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, Id::new("drag-preview")));
    let galley = painter.layout(task.title.clone(), FontId::proportional(14.0), theme.text, 260.0);
    let rect = Rect::from_min_size(pointer + vec2(14.0, 10.0), galley.size() + vec2(20.0, 14.0));
    painter.rect(
        rect,
        6.0,
        theme.elevated_surface,
        Stroke::new(1.0_f32, theme.border_focused),
    );
    painter.galley(rect.min + vec2(10.0, 7.0), galley, theme.text);
}
