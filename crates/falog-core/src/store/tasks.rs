use super::Store;
use crate::date::now;
use crate::domain::{Area, NewTask, Status, Task, TaskId, TaskPatch};
use crate::{Error, Result};
use rusqlite::{OptionalExtension, Row, params};

const SELECT: &str = "
    SELECT t.id, t.title, t.description, t.status, t.priority, t.due_date, t.requester,
           t.created_at, t.updated_at, t.completed_at,
           a.id, a.name, a.color,
           (SELECT COUNT(*) FROM notes n WHERE n.task_id = t.id),
           t.archived_at
    FROM tasks t
    LEFT JOIN areas a ON a.id = t.area_id";

fn map_row(row: &Row<'_>) -> rusqlite::Result<Task> {
    let area = match row.get(10)? {
        Some(id) => Some(Area {
            id,
            name: row.get(11)?,
            color: row.get(12)?,
        }),
        None => None,
    };
    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        status: row.get(3)?,
        priority: row.get(4)?,
        due: row.get(5)?,
        requester: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        completed_at: row.get(9)?,
        area,
        note_count: row.get(13)?,
        archived_at: row.get(14)?,
    })
}

fn validate_title(title: &str) -> Result<&str> {
    let title = title.trim();
    if title.is_empty() {
        return Err(Error::invalid("task title cannot be empty"));
    }
    Ok(title)
}

impl Store {
    /// Every task, oldest first. Filtering and sorting happen in the callers: a personal
    /// task list stays small enough to keep in memory.
    pub fn tasks(&self) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare(&format!("{SELECT} ORDER BY t.id"))?;
        let rows = stmt.query_map([], map_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn task(&self, id: TaskId) -> Result<Option<Task>> {
        Ok(self
            .conn
            .query_row(&format!("{SELECT} WHERE t.id = ?1"), [id], map_row)
            .optional()?)
    }

    pub fn require_task(&self, id: TaskId) -> Result<Task> {
        self.task(id)?.ok_or(Error::TaskNotFound(id))
    }

    pub fn create_task(&self, new: &NewTask) -> Result<Task> {
        let title = validate_title(&new.title)?;
        let now = now();
        self.conn.execute(
            "INSERT INTO tasks (title, description, area_id, status, priority, due_date,
                                requester, created_at, updated_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?9)",
            params![
                title,
                new.description.trim(),
                new.area_id,
                new.status,
                new.priority,
                new.due,
                new.requester.trim(),
                now,
                (new.status == Status::Done).then_some(now),
            ],
        )?;
        self.require_task(TaskId(self.conn.last_insert_rowid()))
    }

    /// Applies a partial update. Moving into `Done` stamps `completed_at`; moving out clears it and
    /// takes the task out of the archive.
    pub fn update_task(&self, id: TaskId, patch: &TaskPatch) -> Result<Task> {
        let current = self.require_task(id)?;
        let title = validate_title(patch.title.as_deref().unwrap_or(&current.title))?;
        let status = patch.status.unwrap_or(current.status);
        let completed_at = match (current.status, status) {
            (Status::Done, Status::Done) => current.completed_at,
            (_, Status::Done) => Some(now()),
            _ => None,
        };
        let archived_at = if status == Status::Done {
            current.archived_at
        } else {
            None
        };
        self.conn.execute(
            "UPDATE tasks SET title = ?1, description = ?2, area_id = ?3, status = ?4,
                              priority = ?5, due_date = ?6, requester = ?7, updated_at = ?8,
                              completed_at = ?9, archived_at = ?10
             WHERE id = ?11",
            params![
                title,
                patch
                    .description
                    .as_deref()
                    .unwrap_or(&current.description)
                    .trim(),
                patch.area_id.unwrap_or(current.area_id()),
                status,
                patch.priority.unwrap_or(current.priority),
                patch.due.unwrap_or(current.due),
                patch.requester.as_deref().unwrap_or(&current.requester).trim(),
                now(),
                completed_at,
                archived_at,
                id,
            ],
        )?;
        self.require_task(id)
    }

    pub fn set_status(&self, id: TaskId, status: Status) -> Result<Task> {
        self.update_task(id, &TaskPatch::status(status))
    }

    /// Puts done tasks away from the board, keeping them for later. Tasks that are not done or
    /// already archived are skipped; returns how many were archived.
    pub fn archive_tasks(&self, ids: &[TaskId]) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        let now = now();
        let mut archived = 0;
        for id in ids {
            archived += tx.execute(
                "UPDATE tasks SET archived_at = ?1
                 WHERE id = ?2 AND status = 'done' AND archived_at IS NULL",
                params![now, id],
            )?;
        }
        tx.commit()?;
        Ok(archived)
    }

    /// Archives one task, which must be done.
    pub fn archive_task(&self, id: TaskId) -> Result<Task> {
        let task = self.require_task(id)?;
        if task.status != Status::Done {
            return Err(Error::invalid(format!(
                "#{id} is not done; only finished tasks can be archived"
            )));
        }
        self.archive_tasks(&[id])?;
        self.require_task(id)
    }

    /// Takes a task out of the archive, back to the board's Done column.
    pub fn restore_task(&self, id: TaskId) -> Result<Task> {
        self.require_task(id)?;
        self.conn
            .execute("UPDATE tasks SET archived_at = NULL WHERE id = ?1", [id])?;
        self.require_task(id)
    }

    /// Deletes a task with its notes and returns what was deleted.
    pub fn delete_task(&self, id: TaskId) -> Result<Task> {
        let task = self.require_task(id)?;
        self.conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Priority;
    use chrono::NaiveDate;

    fn store() -> Store {
        Store::open_in_memory().unwrap()
    }

    #[test]
    fn creates_and_reads_back_all_fields() {
        let store = store();
        let area = store.create_area("Work", None).unwrap();
        let due = NaiveDate::from_ymd_opt(2026, 10, 9);
        let task = store
            .create_task(&NewTask {
                title: "  Fix login  ".into(),
                description: "Android only".into(),
                area_id: Some(area.id),
                priority: Priority::High,
                due,
                requester: "Ana".into(),
                ..NewTask::default()
            })
            .unwrap();
        assert_eq!(task.title, "Fix login");
        assert_eq!(task.area, Some(area));
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.due, due);
        assert_eq!(task.status, Status::Todo);
        assert_eq!(store.tasks().unwrap(), vec![task]);
    }

    #[test]
    fn rejects_empty_titles() {
        let store = store();
        assert!(matches!(
            store.create_task(&NewTask::new(" ")),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn tracks_completion_time() {
        let store = store();
        let task = store.create_task(&NewTask::new("Ship it")).unwrap();
        let done = store.set_status(task.id, Status::Done).unwrap();
        assert!(done.completed_at.is_some());
        let reopened = store.set_status(task.id, Status::InProgress).unwrap();
        assert_eq!(reopened.completed_at, None);
    }

    #[test]
    fn archives_only_done_tasks_and_restores_them() {
        let store = store();
        let done = store.create_task(&NewTask::new("Shipped")).unwrap();
        store.set_status(done.id, Status::Done).unwrap();
        let open = store.create_task(&NewTask::new("Still open")).unwrap();

        assert_eq!(store.archive_tasks(&[done.id, open.id]).unwrap(), 1);
        assert!(store.require_task(done.id).unwrap().is_archived());
        assert!(!store.require_task(open.id).unwrap().is_archived());
        assert_eq!(store.archive_tasks(&[done.id]).unwrap(), 0);
        assert!(matches!(store.archive_task(open.id), Err(Error::Invalid(_))));

        let restored = store.restore_task(done.id).unwrap();
        assert!(!restored.is_archived());
        assert_eq!(restored.status, Status::Done);
    }

    #[test]
    fn reopening_takes_a_task_out_of_the_archive() {
        let store = store();
        let task = store.create_task(&NewTask::new("Ship it")).unwrap();
        store.set_status(task.id, Status::Done).unwrap();
        let archived = store.archive_task(task.id).unwrap();
        assert!(archived.is_archived());

        let edited = store
            .update_task(
                task.id,
                &TaskPatch {
                    title: Some("Shipped".into()),
                    ..TaskPatch::default()
                },
            )
            .unwrap();
        assert!(edited.is_archived(), "editing a done task keeps it archived");

        let reopened = store.set_status(task.id, Status::Todo).unwrap();
        assert!(!reopened.is_archived());
    }

    #[test]
    fn patches_only_given_fields() {
        let store = store();
        let task = store
            .create_task(&NewTask {
                requester: "Ana".into(),
                ..NewTask::new("A")
            })
            .unwrap();
        let patched = store
            .update_task(
                task.id,
                &TaskPatch {
                    title: Some("B".into()),
                    due: Some(None),
                    ..TaskPatch::default()
                },
            )
            .unwrap();
        assert_eq!(patched.title, "B");
        assert_eq!(patched.requester, "Ana");
    }

    #[test]
    fn deleting_a_area_keeps_its_tasks() {
        let store = store();
        let area = store.create_area("Work", None).unwrap();
        let task = store
            .create_task(&NewTask {
                area_id: Some(area.id),
                ..NewTask::new("A")
            })
            .unwrap();
        store.delete_area(area.id).unwrap();
        assert_eq!(store.require_task(task.id).unwrap().area, None);
    }

    #[test]
    fn deletes_tasks() {
        let store = store();
        let task = store.create_task(&NewTask::new("A")).unwrap();
        assert_eq!(store.delete_task(task.id).unwrap().id, task.id);
        assert!(matches!(store.delete_task(task.id), Err(Error::TaskNotFound(_))));
    }
}
