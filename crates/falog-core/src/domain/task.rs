use super::{Company, CompanyId, Priority, Status, TaskId};
use crate::text::fuzzy_contains;
use chrono::{NaiveDate, NaiveDateTime};

/// Label shown for tasks that do not belong to any company.
pub const UNASSIGNED: &str = "No company";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub description: String,
    pub company: Option<Company>,
    pub status: Status,
    pub priority: Priority,
    pub due: Option<NaiveDate>,
    /// Who asked for it.
    pub requester: String,
    pub note_count: u32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
}

impl Task {
    pub fn is_open(&self) -> bool {
        self.status.is_open()
    }

    pub fn company_id(&self) -> Option<CompanyId> {
        self.company.as_ref().map(|c| c.id)
    }

    pub fn company_name(&self) -> &str {
        self.company.as_ref().map_or(UNASSIGNED, |c| c.name.as_str())
    }

    pub fn completed_on(&self) -> Option<NaiveDate> {
        self.completed_at.map(|at| at.date())
    }

    /// Free-text search over id, title, description, requester and company.
    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim();
        query.is_empty()
            || query.trim_start_matches('#') == self.id.to_string()
            || [&self.title, &self.description, &self.requester]
                .into_iter()
                .any(|field| fuzzy_contains(field, query))
            || fuzzy_contains(self.company_name(), query)
    }
}

/// Fields for creating a task.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewTask {
    pub title: String,
    pub description: String,
    pub company_id: Option<CompanyId>,
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
    pub company_id: Option<Option<CompanyId>>,
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
            company_id: Some(task.company_id),
            status: Some(task.status),
            priority: Some(task.priority),
            due: Some(task.due),
            requester: Some(task.requester),
        }
    }
}
