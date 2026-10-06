//! Right dock for creating and editing a task.
//!
//! Edits are kept in a [`Draft`] and written on save (Ctrl+S), like a buffer in Zed.

use super::BAR_HEIGHT;
use crate::components::{ButtonStyle, button, company_picker, icon_button, priority_picker, status_picker};
use crate::fonts;
use crate::icons::Icon;
use crate::theme::Theme;
use chrono::{Days, NaiveDate, Weekday};
use eframe::egui::{
    self, Align, Align2, FontId, Frame, Id, Key, Layout, Margin, RichText, ScrollArea, Sense, SidePanel,
    Stroke, TextEdit, Ui, pos2, vec2,
};
use falog_core::date;
use falog_core::domain::{Company, CompanyId, NewTask, Note, NoteId, Priority, Status, Task, TaskId};

const LABEL_WIDTH: f32 = 104.0;
const TITLE_ID: &str = "task-panel-title";

/// The editable fields of a task.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Draft {
    pub title: String,
    pub description: String,
    pub company_id: Option<CompanyId>,
    pub status: Status,
    pub priority: Priority,
    /// Free text; parsed with [`date::parse_due`] on save.
    pub due: String,
    pub requester: String,
}

impl Draft {
    fn from_task(task: &Task) -> Self {
        Self {
            title: task.title.clone(),
            description: task.description.clone(),
            company_id: task.company_id(),
            status: task.status,
            priority: task.priority,
            due: task
                .due
                .map(|d| d.format("%d/%m/%Y").to_string())
                .unwrap_or_default(),
            requester: task.requester.clone(),
        }
    }

    /// Validates the draft; the error is shown in the panel.
    pub fn to_new_task(&self, today: NaiveDate) -> Result<NewTask, String> {
        if self.title.trim().is_empty() {
            return Err("The title is required.".into());
        }
        let due = match self.due.trim() {
            "" => None,
            text => Some(date::parse_due(text, today).ok_or("The due date is not a date.")?),
        };
        Ok(NewTask {
            title: self.title.clone(),
            description: self.description.clone(),
            company_id: self.company_id,
            status: self.status,
            priority: self.priority,
            due,
            requester: self.requester.clone(),
        })
    }
}

#[derive(Debug)]
pub struct TaskPanel {
    /// `None` while creating a new task.
    pub id: Option<TaskId>,
    pub draft: Draft,
    saved: Draft,
    pub notes: Vec<Note>,
    pub new_note: String,
    pub error: Option<String>,
    meta: String,
    focus_title: bool,
}

impl TaskPanel {
    pub fn new(company_id: Option<CompanyId>, status: Status) -> Self {
        let draft = Draft {
            company_id,
            status,
            ..Draft::default()
        };
        Self {
            id: None,
            saved: draft.clone(),
            draft,
            notes: Vec::new(),
            new_note: String::new(),
            error: None,
            meta: String::new(),
            focus_title: true,
        }
    }

    pub fn edit(task: &Task, notes: Vec<Note>) -> Self {
        let draft = Draft::from_task(task);
        let mut meta = format!(
            "Created {} · updated {}",
            task.created_at.format("%b %-d, %H:%M"),
            task.updated_at.format("%b %-d, %H:%M")
        );
        if let Some(done) = task.completed_at {
            meta.push_str(&format!(" · completed {}", done.format("%b %-d, %H:%M")));
        }
        Self {
            id: Some(task.id),
            saved: draft.clone(),
            draft,
            notes,
            new_note: String::new(),
            error: None,
            meta,
            focus_title: false,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.draft != self.saved
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PanelEvent {
    Save,
    Close,
    Delete,
    AddNote(String),
    DeleteNote(NoteId),
}

pub fn show(
    ctx: &egui::Context,
    theme: &Theme,
    panel: &mut TaskPanel,
    companies: &[Company],
    today: NaiveDate,
) -> Vec<PanelEvent> {
    let mut events = Vec::new();
    SidePanel::right("task_panel")
        .resizable(true)
        .default_width(400.0)
        .width_range(320.0..=640.0)
        .frame(Frame::none().fill(theme.panel))
        .show(ctx, |ui| {
            header(ui, theme, panel, &mut events);
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                Frame::none()
                    .inner_margin(Margin::symmetric(16.0, 14.0))
                    .show(ui, |ui| {
                        body(ui, theme, panel, companies, today, &mut events);
                    });
            });
        });
    events
}

fn header(ui: &mut Ui, theme: &Theme, panel: &TaskPanel, events: &mut Vec<PanelEvent>) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), BAR_HEIGHT), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme.tab_bar);
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0_f32, theme.border),
    );
    let title = panel.id.map_or("New task".to_owned(), |id| format!("Task #{id}"));
    let galley = ui
        .painter()
        .layout_no_wrap(title, fonts::semibold(13.5), theme.text);
    let text_pos = pos2(rect.left() + 14.0, rect.center().y - galley.size().y / 2.0);
    let text_right = text_pos.x + galley.size().x;
    ui.painter().galley(text_pos, galley, theme.text);
    if panel.is_dirty() {
        ui.painter()
            .circle_filled(pos2(text_right + 9.0, rect.center().y), 3.0, theme.text_accent);
    }
    let close_rect = egui::Rect::from_min_max(pos2(rect.right() - 34.0, rect.top()), rect.max);
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(close_rect)
            .layout(Layout::right_to_left(Align::Center)),
        |ui| {
            ui.add_space(6.0);
            if icon_button(ui, Icon::Close, "Close (Esc)").clicked() {
                events.push(PanelEvent::Close);
            }
        },
    );
}

fn body(
    ui: &mut Ui,
    theme: &Theme,
    panel: &mut TaskPanel,
    companies: &[Company],
    today: NaiveDate,
    events: &mut Vec<PanelEvent>,
) {
    let draft = &mut panel.draft;
    let title = ui.add(
        TextEdit::multiline(&mut draft.title)
            .id(Id::new(TITLE_ID))
            .font(fonts::semibold(18.0))
            .frame(false)
            .desired_rows(1)
            .desired_width(f32::INFINITY)
            .hint_text("Task title"),
    );
    if std::mem::take(&mut panel.focus_title) {
        title.request_focus();
    }
    ui.add_space(10.0);

    let control_width = (ui.available_width() - LABEL_WIDTH - 8.0).max(160.0);
    property(ui, theme, Icon::for_status(draft.status), "Status", |ui| {
        status_picker(ui, "panel-status", &mut draft.status, control_width)
    });
    property(ui, theme, Icon::for_priority(draft.priority), "Priority", |ui| {
        priority_picker(ui, "panel-priority", &mut draft.priority, control_width)
    });
    property(ui, theme, Icon::Folder, "Company", |ui| {
        company_picker(
            ui,
            "panel-company",
            &mut draft.company_id,
            companies,
            control_width,
        )
    });
    property(ui, theme, Icon::Clock, "Due", |ui| {
        due_field(ui, theme, &mut draft.due, today)
    });
    property(ui, theme, Icon::Person, "Requested by", |ui| {
        ui.add(
            TextEdit::singleline(&mut draft.requester)
                .hint_text("Who asked for it")
                .desired_width(control_width),
        );
    });

    ui.add_space(12.0);
    section_title(ui, theme, "Description");
    ui.add(
        TextEdit::multiline(&mut draft.description)
            .desired_rows(7)
            .desired_width(f32::INFINITY)
            .hint_text("Context, links, acceptance criteria…"),
    );

    if panel.id.is_some() {
        ui.add_space(14.0);
        activity(ui, theme, panel, events);
    }

    ui.add_space(14.0);
    if let Some(error) = &panel.error {
        ui.label(RichText::new(error).size(13.0).color(theme.error));
        ui.add_space(6.0);
    }
    ui.horizontal(|ui| {
        let save = ui.add_enabled_ui(panel.is_dirty() || panel.id.is_none(), |ui| {
            button(ui, ButtonStyle::Accent, Some(Icon::Check), "Save")
        });
        if save.inner.on_hover_text("Ctrl+S").clicked() {
            events.push(PanelEvent::Save);
        }
        if button(ui, ButtonStyle::Ghost, None, "Cancel")
            .on_hover_text("Esc")
            .clicked()
        {
            events.push(PanelEvent::Close);
        }
        if panel.id.is_some() {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if button(ui, ButtonStyle::Danger, Some(Icon::Trash), "Delete").clicked() {
                    events.push(PanelEvent::Delete);
                }
            });
        }
    });
    if !panel.meta.is_empty() {
        ui.add_space(8.0);
        ui.label(
            RichText::new(&panel.meta)
                .size(11.5)
                .color(theme.text_placeholder),
        );
    }
}

/// A settings-style row: icon and label on the left, control on the right.
fn property<R>(ui: &mut Ui, theme: &Theme, icon: Icon, label: &str, add_control: impl FnOnce(&mut Ui) -> R) {
    ui.horizontal_top(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(LABEL_WIDTH, 24.0), Sense::hover());
        let icon_rect = egui::Rect::from_min_size(pos2(rect.left(), rect.center().y - 7.0), vec2(14.0, 14.0));
        icon.paint(ui, icon_rect, 14.0, theme.icon_muted);
        ui.painter().text(
            pos2(icon_rect.right() + 8.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(13.0),
            theme.text_muted,
        );
        ui.vertical(|ui| add_control(ui));
    });
    ui.add_space(4.0);
}

fn due_field(ui: &mut Ui, theme: &Theme, due: &mut String, today: NaiveDate) {
    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(due)
                .hint_text("friday, 15/10, next week…")
                .desired_width(170.0),
        );
        if !due.trim().is_empty() {
            let (text, color) = match date::parse_due(due, today) {
                Some(date) => (date.format("%a, %b %-d").to_string(), theme.text_muted),
                None => ("not a date".to_owned(), theme.error),
            };
            ui.label(RichText::new(text).size(12.5).color(color));
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        let shortcuts = [
            ("Today", today),
            ("Tomorrow", today + Days::new(1)),
            ("Friday", date::next_weekday(today, Weekday::Fri)),
            (
                "Next week",
                date::next_weekday(today + Days::new(1), Weekday::Mon),
            ),
        ];
        for (label, date) in shortcuts {
            if small_link(ui, theme, label).clicked() {
                *due = date.format("%d/%m/%Y").to_string();
            }
        }
        if !due.is_empty() && small_link(ui, theme, "Clear").clicked() {
            due.clear();
        }
    });
}

fn small_link(ui: &mut Ui, theme: &Theme, label: &str) -> egui::Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(12.0), theme.text_muted);
    let (rect, response) = ui.allocate_exact_size(galley.size() + vec2(12.0, 6.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_hover);
    }
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, theme.text_muted);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn section_title(ui: &mut Ui, theme: &Theme, title: &str) {
    ui.label(
        RichText::new(title)
            .font(fonts::semibold(13.0))
            .color(theme.text_muted),
    );
    ui.add_space(2.0);
}

fn activity(ui: &mut Ui, theme: &Theme, panel: &mut TaskPanel, events: &mut Vec<PanelEvent>) {
    section_title(ui, theme, &format!("Activity ({})", panel.notes.len()));
    for note in &panel.notes {
        Frame::none()
            .stroke(Stroke::new(1.0_f32, theme.border_variant))
            .rounding(4.0)
            .inner_margin(Margin {
                left: 10.0,
                right: 4.0,
                top: 2.0,
                bottom: 8.0,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let stamp = note.created_at.format("%b %-d, %H:%M").to_string();
                    ui.label(
                        RichText::new(stamp)
                            .monospace()
                            .size(11.5)
                            .color(theme.text_placeholder),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, Icon::Trash, "Delete note").clicked() {
                            events.push(PanelEvent::DeleteNote(note.id));
                        }
                    });
                });
                ui.label(RichText::new(&note.body).size(13.5).color(theme.text));
            });
        ui.add_space(4.0);
    }
    let response = ui.add(
        TextEdit::singleline(&mut panel.new_note)
            .hint_text("Add a note and press Enter")
            .desired_width(f32::INFINITY),
    );
    if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) && !panel.new_note.trim().is_empty() {
        events.push(PanelEvent::AddNote(std::mem::take(&mut panel.new_note)));
        response.request_focus();
    }
}
