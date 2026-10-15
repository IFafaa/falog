//! SQLite persistence.
//!
//! The desktop app and the MCP server open the same database file concurrently, so the
//! connection runs in WAL mode with a busy timeout, and the app polls [`Store::data_version`]
//! to notice writes made by the other process.

mod areas;
mod notes;
mod schema;
mod tasks;

use crate::{Error, Result, paths};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Environment variable that overrides the database location.
pub const DB_PATH_ENV: &str = "FALOG_DB";

/// The database chosen with `$FALOG_DB`, if any.
fn override_path() -> Option<PathBuf> {
    std::env::var_os(DB_PATH_ENV)
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
}

/// `$FALOG_DB`, or `falog.db` in [`paths::data_home`] (`%USERPROFILE%\.falog\falog.db` on Windows).
pub fn default_path() -> PathBuf {
    override_path().unwrap_or_else(|| paths::data_home().join(paths::DATABASE_FILE))
}

#[derive(Debug)]
pub struct Store {
    conn: Connection,
    path: Option<PathBuf>,
}

impl Store {
    /// Opens [`default_path`]. Without `$FALOG_DB`, first moves the files of older versions into the
    /// data folder ([`paths::migrate_legacy_data`]), so every binary finds them on its first start.
    pub fn open_default() -> Result<Self> {
        if override_path().is_none() {
            paths::migrate_legacy_data()?;
            paths::protect_data_home();
        }
        Self::open(default_path())
    }

    /// Opens (creating if needed) the database at `path`. The path is made absolute so other
    /// processes started from a different directory (the assistant's MCP server) find the same file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = std::path::absolute(path.as_ref()).map_err(|source| Error::DataDir {
            path: path.as_ref().to_path_buf(),
            source,
        })?;
        let path = path.as_path();
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            paths::create_private_dir(dir).map_err(|source| Error::DataDir {
                path: dir.to_path_buf(),
                source,
            })?;
        }
        Self::init(Connection::open(path)?, Some(path.to_path_buf()))
    }

    /// A private database that lives only as long as the store; handy for tests.
    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?, None)
    }

    fn init(conn: Connection, path: Option<PathBuf>) -> Result<Self> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;")?;
        schema::migrate(&conn)?;
        Ok(Self { conn, path })
    }

    /// Location of the database file (`None` for in-memory stores).
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Changes whenever *another* connection commits to the database.
    pub fn data_version(&self) -> Result<i64> {
        Ok(self.conn.query_row("PRAGMA data_version", [], |r| r.get(0))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembers_an_absolute_path() {
        let name = format!("falog-store-test-{}.db", std::process::id());
        let dir = std::env::temp_dir();
        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        // Not `dir`: on macOS the temp dir is under /var, a symlink the working directory resolves
        // to /private/var.
        let cwd = std::env::current_dir().unwrap();
        let store = Store::open(&name);
        std::env::set_current_dir(previous).unwrap();

        let store = store.unwrap();
        assert!(store.path().unwrap().is_absolute());
        assert_eq!(store.path().unwrap(), cwd.join(&name));
        drop(store);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(dir.join(format!("{name}{suffix}")));
        }
    }

    /// A folder in the temp dir with a unique name, removed with its contents when dropped.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("falog-store-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn data_version_changes_only_when_another_connection_writes() {
        let dir = TempDir::new("data-version");
        let path = dir.0.join("falog.db");
        let app = Store::open(&path).unwrap();
        let server = Store::open(&path).unwrap();
        let before = app.data_version().unwrap();

        app.create_area("Work", None).unwrap();
        assert_eq!(app.data_version().unwrap(), before, "its own write");

        server.create_area("Home", None).unwrap();
        let after = app.data_version().unwrap();
        assert_ne!(after, before, "another connection's write");
        assert_eq!(app.areas().unwrap().len(), 2);
        assert_eq!(app.data_version().unwrap(), after, "reading changes nothing");
    }

    #[test]
    fn opening_creates_the_missing_folders() {
        let dir = TempDir::new("nested");
        let path = dir.0.join("a").join("b").join("falog.db");
        let store = Store::open(&path).unwrap();
        assert_eq!(store.path(), Some(path.as_path()));
        assert!(path.exists());
    }

    // Regression: the folders holding the database were created with the default umask, so other
    // users on the machine could read it.
    #[cfg(unix)]
    #[test]
    fn the_folders_it_creates_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new("private");
        let path = dir.0.join("a").join("falog.db");
        Store::open(&path).unwrap();
        for folder in [dir.0.join("a"), dir.0.clone()] {
            let mode = std::fs::metadata(&folder).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700, "{}", folder.display());
        }
    }

    #[test]
    fn in_memory_stores_have_no_path_and_start_empty() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.path(), None);
        assert!(store.tasks().unwrap().is_empty());
        assert!(store.areas().unwrap().is_empty());
    }
}
