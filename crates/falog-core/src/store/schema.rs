use crate::Result;
use rusqlite::Connection;

/// Append-only list of migrations; `PRAGMA user_version` records how many have run.
const MIGRATIONS: &[&str] = &[
    r#"
    CREATE TABLE areas (
        id          INTEGER PRIMARY KEY,
        name        TEXT NOT NULL UNIQUE COLLATE NOCASE,
        color       TEXT NOT NULL,
        created_at  TEXT NOT NULL
    );

    CREATE TABLE tasks (
        id           INTEGER PRIMARY KEY,
        title        TEXT NOT NULL,
        description  TEXT NOT NULL DEFAULT '',
        area_id      INTEGER REFERENCES areas(id) ON DELETE SET NULL,
        status       TEXT NOT NULL DEFAULT 'todo'
                     CHECK (status IN ('todo', 'in_progress', 'waiting', 'done')),
        priority     INTEGER NOT NULL DEFAULT 2 CHECK (priority BETWEEN 1 AND 4),
        due_date     TEXT,
        requester    TEXT NOT NULL DEFAULT '',
        created_at   TEXT NOT NULL,
        updated_at   TEXT NOT NULL,
        completed_at TEXT
    );
    CREATE INDEX tasks_status ON tasks(status);
    CREATE INDEX tasks_area ON tasks(area_id);

    CREATE TABLE notes (
        id          INTEGER PRIMARY KEY,
        task_id     INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
        body        TEXT NOT NULL,
        created_at  TEXT NOT NULL
    );
    CREATE INDEX notes_task ON notes(task_id);
"#,
    // Intentionally empty: databases created by earlier builds count it, so later migrations must keep
    // their numbers.
    "SELECT 1;",
    // Done tasks put away from the board; NULL while the task is on it.
    "ALTER TABLE tasks ADD COLUMN archived_at TEXT;",
    // How a task repeats (`daily`, `weekly:mon,fri`, `monthly:5`); NULL when it does not.
    "ALTER TABLE tasks ADD COLUMN recurrence TEXT;",
];

pub(super) fn migrate(conn: &Connection) -> Result<()> {
    if applied(conn)? >= MIGRATIONS.len() {
        return Ok(());
    }
    // Both processes may start at the same time on a fresh install: take the write lock
    // first and re-check inside it.
    conn.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| {
        for (index, sql) in MIGRATIONS.iter().enumerate().skip(applied(conn)?) {
            conn.execute_batch(sql)?;
            conn.pragma_update(None, "user_version", (index + 1) as i64)?;
        }
        Ok(())
    })();
    conn.execute_batch(if result.is_ok() { "COMMIT" } else { "ROLLBACK" })?;
    result
}

fn applied(conn: &Connection) -> Result<usize> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    Ok(usize::try_from(version).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::types::Value;

    #[test]
    fn migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();
        assert_eq!(applied(&conn).unwrap(), MIGRATIONS.len());
    }

    #[test]
    fn tasks_belong_to_areas() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO areas (id, name, color, created_at) VALUES (7, 'Work', '#61afef', '');
             INSERT INTO tasks (title, area_id, created_at, updated_at) VALUES ('Fix login', 7, '', '');",
        )
        .unwrap();
        let area: String = conn
            .query_row(
                "SELECT a.name FROM tasks t JOIN areas a ON a.id = t.area_id",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(area, "Work");
    }

    /// A database left by an older build: its first `version` migrations and some data.
    fn old_database(version: usize) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        for sql in &MIGRATIONS[..version] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", version as i64).unwrap();
        conn.execute_batch(
            "INSERT INTO areas (id, name, color, created_at)
                 VALUES (1, 'Work', '#74ade8', '2026-10-01 09:00:00');
             INSERT INTO tasks (id, title, description, area_id, status, priority, due_date, requester,
                                created_at, updated_at, completed_at)
                 VALUES (1, 'Fix login', 'Android only', 1, 'done', 3, '2026-10-09', 'Ana',
                         '2026-10-01 09:00:00', '2026-10-02 10:00:00', '2026-10-02 10:00:00'),
                        (2, 'Call the bank', '', NULL, 'todo', 2, NULL, '',
                         '2026-10-01 09:30:00', '2026-10-01 09:30:00', NULL);
             INSERT INTO notes (id, task_id, body, created_at)
                 VALUES (1, 1, 'reproduced on the emulator', '2026-10-02 09:00:00');",
        )
        .unwrap();
        conn
    }

    fn rows(conn: &Connection, sql: &str) -> Vec<Vec<Value>> {
        let mut stmt = conn.prepare(sql).unwrap();
        let columns = stmt.column_count();
        stmt.query_map([], |row| (0..columns).map(|i| row.get(i)).collect())
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    /// Every row of the tables as the first migration defined them.
    fn snapshot(conn: &Connection) -> Vec<Vec<Vec<Value>>> {
        [
            "SELECT * FROM areas ORDER BY id",
            "SELECT id, title, description, area_id, status, priority, due_date, requester, created_at,
                    updated_at, completed_at FROM tasks ORDER BY id",
            "SELECT * FROM notes ORDER BY id",
        ]
        .into_iter()
        .map(|sql| rows(conn, sql))
        .collect()
    }

    #[test]
    fn upgrading_an_old_database_keeps_every_row() {
        for version in 1..MIGRATIONS.len() {
            let conn = old_database(version);
            let archived_at = "2026-10-03 11:00:00";
            if version >= 3 {
                conn.execute("UPDATE tasks SET archived_at = ?1 WHERE id = 1", [archived_at])
                    .unwrap();
            }
            let before = snapshot(&conn);

            migrate(&conn).unwrap();

            assert_eq!(applied(&conn).unwrap(), MIGRATIONS.len(), "from {version}");
            assert_eq!(snapshot(&conn), before, "from {version}");
            let added = rows(&conn, "SELECT archived_at, recurrence FROM tasks ORDER BY id");
            let kept_archive = if version >= 3 {
                Value::Text(archived_at.into())
            } else {
                Value::Null
            };
            assert_eq!(
                added,
                [vec![kept_archive, Value::Null], vec![Value::Null, Value::Null]],
                "from {version}"
            );
        }
    }

    #[test]
    fn an_upgraded_database_reads_back_as_tasks() {
        let conn = old_database(3);
        migrate(&conn).unwrap();
        let store = crate::Store { conn, path: None };

        let tasks = store.tasks().unwrap();
        assert_eq!(tasks.len(), 2);
        let task = &tasks[0];
        assert_eq!(task.title, "Fix login");
        assert_eq!(task.area_name(), "Work");
        assert_eq!(task.status, crate::domain::Status::Done);
        assert_eq!(task.priority, crate::domain::Priority::High);
        assert_eq!(task.due, chrono::NaiveDate::from_ymd_opt(2026, 10, 9));
        assert_eq!(task.note_count, 1);
        assert!(task.completed_at.is_some());
        assert_eq!((task.archived_at, task.recurrence), (None, None));
        assert_eq!(
            store.notes(task.id).unwrap()[0].body,
            "reproduced on the emulator"
        );
    }
}
