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
}
