use super::Store;
use crate::date::now;
use crate::domain::{Note, NoteId, TaskId};
use crate::{Error, Result};
use rusqlite::params;

impl Store {
    /// Appends a note to a task's activity log and bumps the task's `updated_at`.
    pub fn add_note(&self, task_id: TaskId, body: &str) -> Result<Note> {
        let body = body.trim();
        if body.is_empty() {
            return Err(Error::invalid("note cannot be empty"));
        }
        self.require_task(task_id)?;
        let created_at = now();
        self.conn.execute(
            "INSERT INTO notes (task_id, body, created_at) VALUES (?1, ?2, ?3)",
            params![task_id, body, created_at],
        )?;
        let id = NoteId(self.conn.last_insert_rowid());
        self.conn.execute(
            "UPDATE tasks SET updated_at = ?1 WHERE id = ?2",
            params![created_at, task_id],
        )?;
        Ok(Note {
            id,
            task_id,
            body: body.to_string(),
            created_at,
        })
    }

    /// A task's notes, oldest first.
    pub fn notes(&self, task_id: TaskId) -> Result<Vec<Note>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, task_id, body, created_at FROM notes WHERE task_id = ?1 ORDER BY id")?;
        let rows = stmt.query_map([task_id], |row| {
            Ok(Note {
                id: row.get(0)?,
                task_id: row.get(1)?,
                body: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn delete_note(&self, id: NoteId) -> Result<()> {
        self.conn.execute("DELETE FROM notes WHERE id = ?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Store;
    use crate::domain::NewTask;

    #[test]
    fn notes_round_trip_and_cascade() {
        let store = Store::open_in_memory().unwrap();
        let task = store.create_task(&NewTask::new("A")).unwrap();
        let note = store.add_note(task.id, " reproduced on the emulator ").unwrap();
        assert_eq!(note.body, "reproduced on the emulator");
        assert_eq!(store.notes(task.id).unwrap(), vec![note.clone()]);
        assert_eq!(store.require_task(task.id).unwrap().note_count, 1);

        store.delete_note(note.id).unwrap();
        assert!(store.notes(task.id).unwrap().is_empty());

        store.add_note(task.id, "again").unwrap();
        store.delete_task(task.id).unwrap();
        assert!(store.notes(task.id).unwrap().is_empty());
        assert!(store.add_note(task.id, "orphan").is_err());
    }
}
