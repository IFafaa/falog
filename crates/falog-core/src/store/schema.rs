use crate::Result;
use rusqlite::Connection;

/// Append-only list of migrations; `PRAGMA user_version` records how many have run.
const MIGRATIONS: &[&str] = &[
    r#"
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
"#,
    // Companies became areas: they also cover personal life, not only employers.
    r#"
    ALTER TABLE companies RENAME TO areas;
    ALTER TABLE tasks RENAME COLUMN company_id TO area_id;
    DROP INDEX tasks_company;
    CREATE INDEX tasks_area ON tasks(area_id);
"#,
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
    fn companies_become_areas_with_their_tasks() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(MIGRATIONS[0]).unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute_batch(
            "INSERT INTO companies (id, name, color, created_at) VALUES (7, 'Acme', '#61afef', '');
             INSERT INTO tasks (title, company_id, created_at, updated_at) VALUES ('Fix login', 7, '', '');",
        )
        .unwrap();

        migrate(&conn).unwrap();
        let area: String = conn
            .query_row(
                "SELECT a.name FROM tasks t JOIN areas a ON a.id = t.area_id",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(area, "Acme");
    }
}
