//! SQLite persistence.
//!
//! The desktop app and the MCP server open the same database file concurrently, so the
//! connection runs in WAL mode with a busy timeout, and the app polls [`Store::data_version`]
//! to notice writes made by the other process.

mod schema;

use crate::{Error, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Environment variable that overrides the database location.
pub const DB_PATH_ENV: &str = "FALOG_DB";

/// `$FALOG_DB`, or `<data dir>/Falog/falog.db` (`%APPDATA%\Falog\falog.db` on Windows).
pub fn default_path() -> PathBuf {
    if let Some(path) = std::env::var_os(DB_PATH_ENV).filter(|p| !p.is_empty()) {
        return PathBuf::from(path);
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Falog")
        .join("falog.db")
}

#[derive(Debug)]
pub struct Store {
    conn: Connection,
    path: Option<PathBuf>,
}

impl Store {
    pub fn open_default() -> Result<Self> {
        Self::open(default_path())
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(|source| Error::DataDir {
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
