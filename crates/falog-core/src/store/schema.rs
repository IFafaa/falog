use crate::Result;
use rusqlite::Connection;

/// Append-only list of migrations; `PRAGMA user_version` records how many have run.
const MIGRATIONS: &[&str] = &[r#"
    CREATE TABLE companies (
        id          INTEGER PRIMARY KEY,
        name        TEXT NOT NULL UNIQUE COLLATE NOCASE,
        color       TEXT NOT NULL,
        created_at  TEXT NOT NULL
    );

    CREATE TABLE tasks (
        id           INTEGER PRIMARY KEY,
        title        TEXT NOT NULL,
        description  TEXT NOT NULL DEFAULT '',
        company_id   INTEGER REFERENCES companies(id) ON DELETE SET NULL,
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
    CREATE INDEX tasks_company ON tasks(company_id);

    CREATE TABLE notes (
        id          INTEGER PRIMARY KEY,
        task_id     INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
        body        TEXT NOT NULL,
        created_at  TEXT NOT NULL
    );
    CREATE INDEX notes_task ON notes(task_id);
"#];

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
}
