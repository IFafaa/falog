use super::{Area, AreaId, Priority, Recurrence, Status, TaskId};
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
    /// How the task repeats; completing it creates the next occurrence.
    pub recurrence: Option<Recurrence>,
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
    pub recurrence: Option<Recurrence>,
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
    pub recurrence: Option<Option<Recurrence>>,
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
            recurrence: Some(task.recurrence),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Rgb, Weekdays};
    use chrono::Weekday;

    fn task() -> Task {
        let at = NaiveDate::from_ymd_opt(2026, 10, 6)
            .and_then(|d| d.and_hms_opt(9, 0, 0))
            .unwrap();
        Task {
            id: TaskId(12),
            title: "Prepare the reunião deck".into(),
            description: "Slides for the quarterly review".into(),
            area: Some(Area {
                id: AreaId(1),
                name: "Saúde".into(),
                color: Rgb::new(1, 2, 3),
            }),
            status: Status::Todo,
            priority: Priority::Medium,
            due: None,
            requester: "Ana Lúcia".into(),
            note_count: 0,
            created_at: at,
            updated_at: at,
            completed_at: None,
            archived_at: None,
            recurrence: None,
        }
    }

    #[test]
    fn matches_the_id_with_or_without_hash() {
        let task = task();
        assert!(task.matches("12"));
        assert!(task.matches("#12"));
        assert!(task.matches(" #12 "));
        assert!(!task.matches("#1"));
        assert!(!task.matches("#123"));
    }

    #[test]
    fn matches_every_text_field_ignoring_case_and_accents() {
        let task = task();
        for query in ["REUNIAO", "quarterly", "ana lucia", "saude", "Saúde"] {
            assert!(task.matches(query), "{query}");
        }
        assert!(!task.matches("budget"));
    }

    #[test]
    fn an_empty_query_matches_everything() {
        assert!(task().matches(""));
        assert!(task().matches("   "));
    }

    #[test]
    fn tasks_without_an_area_match_the_unassigned_label() {
        let task = Task { area: None, ..task() };
        assert_eq!(task.area_name(), UNASSIGNED);
        assert!(task.matches("no area"));
        assert!(!task.matches("saude"));
    }

    #[test]
    fn a_patch_from_a_new_task_overwrites_every_field() {
        let new = NewTask {
            title: "Title".into(),
            description: "Description".into(),
            area_id: Some(AreaId(3)),
            status: Status::Waiting,
            priority: Priority::Urgent,
            due: NaiveDate::from_ymd_opt(2026, 10, 9),
            requester: "Ana".into(),
            recurrence: Some(Recurrence::Weekly(Weekdays::single(Weekday::Fri))),
        };
        assert_eq!(
            TaskPatch::from(new.clone()),
            TaskPatch {
                title: Some(new.title),
                description: Some(new.description),
                area_id: Some(new.area_id),
                status: Some(new.status),
                priority: Some(new.priority),
                due: Some(new.due),
                requester: Some(new.requester),
                recurrence: Some(new.recurrence),
            }
        );
        // Empty optional fields clear the task's values instead of keeping them.
        let cleared = TaskPatch::from(NewTask::new("Title"));
        assert_eq!(cleared.area_id, Some(None));
        assert_eq!(cleared.due, Some(None));
        assert_eq!(cleared.recurrence, Some(None));
    }

    #[test]
    fn new_tasks_default_to_todo_and_medium() {
        let new = NewTask::new("Call the bank");
        assert_eq!((new.status, new.priority), (Status::Todo, Priority::Medium));
        let patch = TaskPatch::status(Status::Done);
        assert_eq!(
            patch,
            TaskPatch {
                status: Some(Status::Done),
                ..TaskPatch::default()
            }
        );
    }
}
