//! Builders shared by the tests.

use crate::prefs::Prefs;
use crate::theme::Theme;
use crate::views::ViewCx;
use chrono::{NaiveDate, NaiveDateTime};
use falog_core::domain::{Area, AreaId, Priority, Rgb, Status, Task, TaskId};

/// A day of October 2026.
pub fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, d).unwrap()
}

/// Noon of a day of October 2026.
pub fn noon(d: u32) -> NaiveDateTime {
    day(d).and_hms_opt(12, 0, 0).unwrap()
}

/// An open task to do with only an id and a title, created on October 1, 2026.
pub fn task(id: i64, title: &str) -> Task {
    Task {
        id: TaskId(id),
        title: title.into(),
        description: String::new(),
        area: None,
        status: Status::Todo,
        priority: Priority::Medium,
        due: None,
        requester: String::new(),
        note_count: 0,
        created_at: noon(1),
        updated_at: noon(1),
        completed_at: None,
        archived_at: None,
        recurrence: None,
    }
}

/// [`task`], done at noon of `completed` (a day of October 2026).
pub fn done(id: i64, title: &str, completed: u32) -> Task {
    Task {
        status: Status::Done,
        completed_at: Some(noon(completed)),
        ..task(id, title)
    }
}

/// [`done`], then archived the same day.
pub fn archived(id: i64, title: &str, completed: u32) -> Task {
    Task {
        archived_at: Some(noon(completed)),
        ..done(id, title, completed)
    }
}

pub fn area(id: i64, name: &str) -> Area {
    Area {
        id: AreaId(id),
        name: name.into(),
        color: Rgb::new(0x61, 0xaf, 0xef),
    }
}

/// `task` filed under `area`.
pub fn in_area(task: Task, area: &Area) -> Task {
    Task {
        area: Some(area.clone()),
        ..task
    }
}

/// What a view gets for `tasks` on `today`, with `prefs`.
pub fn view_cx<'a>(tasks: &'a [Task], prefs: &'a Prefs, today: NaiveDate) -> ViewCx<'a> {
    ViewCx {
        theme: Theme::current(&eframe::egui::Context::default()),
        today,
        tasks,
        areas: &[],
        prefs,
        selected: None,
        has_any_task: !tasks.is_empty(),
    }
}

/// The ids of `tasks`, in order.
pub fn ids<'a>(tasks: impl IntoIterator<Item = &'a Task>) -> Vec<i64> {
    tasks.into_iter().map(|t| t.id.0).collect()
}
