use super::{Area, AreaId, Priority, Status, TaskId};
use crate::text::fuzzy_contains;
use chrono::{NaiveDate, NaiveDateTime};

/// Label shown for tasks that do not belong to any area.
pub const UNASSIGNED: &str = "No area";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub description: String,
    pub area: Option<Area>,
    pub status: Status,
    pub priority: Priority,
    pub due: Option<NaiveDate>,
    /// Who asked for it.
    pub requester: String,
    pub note_count: u32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
    /// When a done task was put away from the board; `None` while it is on it.
    pub archived_at: Option<NaiveDateTime>,
}

impl Task {
    pub fn is_open(&self) -> bool {
        self.status.is_open()
    }

    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    pub fn area_id(&self) -> Option<AreaId> {
        self.area.as_ref().map(|a| a.id)
    }

    pub fn area_name(&self) -> &str {
        self.area.as_ref().map_or(UNASSIGNED, |a| a.name.as_str())
    }

    pub fn completed_on(&self) -> Option<NaiveDate> {
        self.completed_at.map(|at| at.date())
    }

    /// Free-text search over id, title, description, requester and area.
    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim();
        query.is_empty()
            || query.trim_start_matches('#') == self.id.to_string()
            || [&self.title, &self.description, &self.requester]
                .into_iter()
                .any(|field| fuzzy_contains(field, query))
            || fuzzy_contains(self.area_name(), query)
    }
}

/// Fields for creating a task.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewTask {
    pub title: String,
    pub description: String,
    pub area_id: Option<AreaId>,
    pub status: Status,
    pub priority: Priority,
    pub due: Option<NaiveDate>,
    pub requester: String,
}

impl NewTask {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }
}

/// A partial update; `None` keeps the current value.
/// Nullable fields use `Some(None)` to clear the value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub area_id: Option<Option<AreaId>>,
    pub status: Option<Status>,
    pub priority: Option<Priority>,
    pub due: Option<Option<NaiveDate>>,
    pub requester: Option<String>,
}

impl TaskPatch {
    pub fn status(status: Status) -> Self {
        Self {
            status: Some(status),
            ..Self::default()
        }
    }
}

/// A patch that overwrites every field, e.g. when saving a full edit form.
impl From<NewTask> for TaskPatch {
    fn from(task: NewTask) -> Self {
        Self {
            title: Some(task.title),
            description: Some(task.description),
            area_id: Some(task.area_id),
            status: Some(task.status),
            priority: Some(task.priority),
            due: Some(task.due),
            requester: Some(task.requester),
        }
    }
}
