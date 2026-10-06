use super::{NoteId, TaskId};
use chrono::NaiveDateTime;

/// A timestamped entry in a task's activity log (progress, decisions, new information).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub id: NoteId,
    pub task_id: TaskId,
    pub body: String,
    pub created_at: NaiveDateTime,
}
