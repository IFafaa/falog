//! Where Falog keeps its files, and the one-time move from older locations.
//!
//! On Windows the data lives in `%USERPROFILE%\.falog`, not in AppData: packaged (MSIX) apps such as
//! Claude desktop redirect AppData, copy-on-write, for every process they start, so an MCP server
//! started from one would write to a private copy of the database the Falog app never sees. The user
//! profile itself is not redirected. macOS and Linux use their standard data folders.

use crate::{Error, Result};
use rusqlite::Connection;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

/// Database file name inside [`data_home`].
pub const DATABASE_FILE: &str = "falog.db";

/// What an old database is renamed to once copied, so it is kept but never picked up again.
pub const MOVED_DATABASE_FILE: &str = "falog.moved.db";

/// Folders that live next to the database and move with it.
const FOLDERS: [&str; 3] = ["assistant", "models", "calendar"];

/// Window and UI preferences saved by eframe.
pub const PREFERENCES_FILE: &str = "app.ron";

const LOCK_FILE: &str = "migration.lock";

/// A lock older than this belongs to a process that died while migrating.
const STALE_LOCK: Duration = Duration::from_secs(120);

/// How long a process waits for another one that is migrating.
const LOCK_WAIT: Duration = Duration::from_secs(150);

/// Folder for the database and everything next to it: `%USERPROFILE%\.falog` on Windows,
/// `~/Library/Application Support/Falog` on macOS, `$XDG_DATA_HOME/Falog` (`~/.local/share/Falog`)
/// on Linux. Falls back to the working directory when the OS reports no home folder.
pub fn data_home() -> PathBuf {
    let home = if cfg!(windows) {
        dirs::home_dir().map(|home| home.join(".falog"))
    } else {
        dirs::data_dir().map(|data| data.join("Falog"))
    };
    home.unwrap_or_else(|| PathBuf::from("."))
}

/// Where older versions kept their files on this platform.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Legacy {
    /// Folder with `falog.db` and the folders next to it (`%APPDATA%\Falog` on Windows).
    pub home: Option<PathBuf>,
    /// eframe's own preferences file, before Falog chose its location.
    pub preferences: Option<PathBuf>,
}

impl Legacy {
    pub fn for_this_platform() -> Self {
        let Some(data) = dirs::data_dir() else {
            return Self::default();
        };
        if cfg!(windows) {
            Self {
                home: Some(data.join("Falog")),
                preferences: Some(data.join("falog").join("data").join(PREFERENCES_FILE)),
            }
        } else {
            // macOS file systems usually ignore case, so this is often the new file already.
            Self {
                home: None,
                preferences: Some(data.join("falog").join(PREFERENCES_FILE)),
            }
        }
    }

    /// `(from, to)` pairs still to move into `home`, database first.
    fn pending(&self, home: &Path) -> Vec<(PathBuf, PathBuf)> {
        let mut moves = Vec::new();
        if let Some(old) = &self.home {
            moves.push((old.join(DATABASE_FILE), home.join(DATABASE_FILE)));
            moves.extend(FOLDERS.iter().map(|name| (old.join(name), home.join(name))));
        }
        if let Some(old) = &self.preferences {
            moves.push((old.clone(), home.join(PREFERENCES_FILE)));
        }
        moves.retain(|(from, to)| from.exists() && !to.exists());
        moves
    }
}

/// Moves the files of older versions into [`data_home`], once. See [`migrate`].
pub fn migrate_legacy_data() -> Result<()> {
    migrate(&data_home(), &Legacy::for_this_platform(), LOCK_WAIT)
}

/// Moves what `legacy` points at into `home`, skipping anything `home` already has.
///
/// The desktop app and the MCP server may start together, so the work happens under a lock file in
/// `home`; a process that finds it waits up to `wait` and then re-checks, which keeps it from opening
/// (and creating) an empty database while the other one is still copying. The database is copied
/// with `VACUUM INTO`, which reads a consistent snapshot even with a WAL and other connections, and
/// renamed into place only when complete; the old one becomes `falog.moved.db`. Folders are renamed,
/// or copied when they are on another volume.
pub fn migrate(home: &Path, legacy: &Legacy, wait: Duration) -> Result<()> {
    if legacy.pending(home).is_empty() {
        return Ok(());
    }
    let failed = |source: io::Error| Error::DataDir {
        path: home.to_path_buf(),
        source,
    };
    fs::create_dir_all(home).map_err(failed)?;
    let lock = home.join(LOCK_FILE);
    let deadline = SystemTime::now() + wait;
    loop {
        match fs::OpenOptions::new().write(true).create_new(true).open(&lock) {
            Ok(_) => break,
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                if is_stale(&lock) {
                    let _ = fs::remove_file(&lock);
                } else if SystemTime::now() > deadline {
                    return Err(failed(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!(
                            "another Falog process is still moving data ({} exists)",
                            lock.display()
                        ),
                    )));
                } else {
                    thread::sleep(Duration::from_millis(100));
                }
            }
            Err(err) => return Err(failed(err)),
        }
    }
    // Another process may have finished while this one waited: re-check under the lock.
    let result = legacy
        .pending(home)
        .into_iter()
        .try_for_each(|(from, to)| move_item(&from, &to));
    let _ = fs::remove_file(&lock);
    result
}

fn is_stale(lock: &Path) -> bool {
    fs::metadata(lock)
        .and_then(|meta| meta.modified())
        .is_ok_and(|modified| modified.elapsed().is_ok_and(|age| age > STALE_LOCK))
}

fn move_item(from: &Path, to: &Path) -> Result<()> {
    let moved = |source: io::Error| Error::Migration {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
        source,
    };
    if from.file_name().is_some_and(|name| name == DATABASE_FILE) {
        copy_database(from, to)?;
        retire_database(from);
        Ok(())
    } else if from.is_dir() {
        move_dir(from, to).map_err(moved)
    } else {
        fs::rename(from, to)
            .or_else(|_| fs::copy(from, to).map(drop))
            .map_err(moved)
    }
}

fn copy_database(from: &Path, to: &Path) -> Result<()> {
    let partial = partial_path(to);
    let _ = fs::remove_file(&partial);
    let conn = Connection::open(from)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute("VACUUM INTO ?1", [partial.to_string_lossy()])?;
    drop(conn);
    fs::rename(&partial, to).map_err(|source| Error::Migration {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
        source,
    })
}

/// Renames the old database and its WAL files. A process of an older version may still hold it open
/// (Windows then refuses the rename); the copy is done, so it is left as is.
fn retire_database(from: &Path) {
    let retired = from.with_file_name(MOVED_DATABASE_FILE);
    if fs::rename(from, &retired).is_ok() {
        for suffix in ["-wal", "-shm"] {
            let _ = fs::rename(sidecar(from, suffix), sidecar(&retired, suffix));
        }
    }
}

fn move_dir(from: &Path, to: &Path) -> io::Result<()> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    // Another volume: copy to a temporary name so a failed copy is not mistaken for a finished one.
    let partial = partial_path(to);
    let _ = fs::remove_dir_all(&partial);
    copy_dir(from, &partial)?;
    fs::rename(&partial, to)?;
    let _ = fs::remove_dir_all(from);
    Ok(())
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn partial_path(path: &Path) -> PathBuf {
    sidecar(path, ".partial")
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;

    /// A fresh folder in the temp dir, removed when dropped.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("falog-paths-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// An old layout with one area in the database, a thread, a model and preferences.
    fn old_layout(root: &Path) -> Legacy {
        let old = root.join("AppData").join("Falog");
        fs::create_dir_all(old.join("assistant")).unwrap();
        fs::write(old.join("assistant").join("threads.json"), "[]").unwrap();
        fs::create_dir_all(old.join("models")).unwrap();
        fs::write(old.join("models").join("model.bin"), "weights").unwrap();
        let store = Store::open(old.join(DATABASE_FILE)).unwrap();
        store.create_area("Acme", None).unwrap();
        drop(store);
        let preferences = root.join("eframe").join(PREFERENCES_FILE);
        fs::create_dir_all(preferences.parent().unwrap()).unwrap();
        fs::write(&preferences, "(theme: Dark)").unwrap();
        Legacy {
            home: Some(old),
            preferences: Some(preferences),
        }
    }

    fn area_names(database: &Path) -> Vec<String> {
        let store = Store::open(database).unwrap();
        store.areas().unwrap().into_iter().map(|area| area.name).collect()
    }

    #[test]
    fn moves_the_old_layout_into_the_new_folder() {
        let root = TempDir::new("moves");
        let legacy = old_layout(&root.0);
        let home = root.0.join(".falog");

        migrate(&home, &legacy, Duration::ZERO).unwrap();

        assert_eq!(area_names(&home.join(DATABASE_FILE)), ["Acme"]);
        let old = legacy.home.as_ref().unwrap();
        assert!(!old.join(DATABASE_FILE).exists());
        assert!(old.join(MOVED_DATABASE_FILE).exists());
        assert_eq!(
            fs::read_to_string(home.join("assistant").join("threads.json")).unwrap(),
            "[]"
        );
        assert!(home.join("models").join("model.bin").exists());
        assert!(!old.join("models").exists());
        assert_eq!(
            fs::read_to_string(home.join(PREFERENCES_FILE)).unwrap(),
            "(theme: Dark)"
        );
        assert!(!home.join(LOCK_FILE).exists());
    }

    #[test]
    fn copies_writes_still_in_the_wal_while_the_old_database_is_open() {
        let root = TempDir::new("wal");
        let legacy = old_layout(&root.0);
        let old_store = Store::open(legacy.home.as_ref().unwrap().join(DATABASE_FILE)).unwrap();
        old_store.create_area("Globex", None).unwrap();
        let home = root.0.join(".falog");

        migrate(&home, &legacy, Duration::ZERO).unwrap();

        let mut names = area_names(&home.join(DATABASE_FILE));
        names.sort();
        assert_eq!(names, ["Acme", "Globex"]);
    }

    #[test]
    fn never_overwrites_the_new_folder() {
        let root = TempDir::new("keeps");
        let legacy = old_layout(&root.0);
        let home = root.0.join(".falog");
        Store::open(home.join(DATABASE_FILE))
            .unwrap()
            .create_area("New", None)
            .unwrap();

        migrate(&home, &legacy, Duration::ZERO).unwrap();
        migrate(&home, &legacy, Duration::ZERO).unwrap();

        assert_eq!(area_names(&home.join(DATABASE_FILE)), ["New"]);
        assert!(legacy.home.as_ref().unwrap().join(DATABASE_FILE).exists());
        // Folders the new home lacks still move.
        assert!(home.join("models").join("model.bin").exists());
    }

    #[test]
    fn waits_for_a_held_lock_and_creates_nothing() {
        let root = TempDir::new("held");
        let legacy = old_layout(&root.0);
        let home = root.0.join(".falog");
        fs::create_dir_all(&home).unwrap();
        fs::write(home.join(LOCK_FILE), "").unwrap();

        let err = migrate(&home, &legacy, Duration::from_millis(250)).unwrap_err();

        assert!(err.to_string().contains("still moving data"), "{err}");
        assert!(!home.join(DATABASE_FILE).exists());
    }

    #[test]
    fn takes_over_a_stale_lock() {
        let root = TempDir::new("stale");
        let legacy = old_layout(&root.0);
        let home = root.0.join(".falog");
        fs::create_dir_all(&home).unwrap();
        let lock = fs::File::create(home.join(LOCK_FILE)).unwrap();
        lock.set_modified(SystemTime::now() - STALE_LOCK * 2).unwrap();
        drop(lock);

        migrate(&home, &legacy, Duration::ZERO).unwrap();

        assert_eq!(area_names(&home.join(DATABASE_FILE)), ["Acme"]);
    }

    #[test]
    fn two_processes_starting_together_migrate_once() {
        let root = TempDir::new("race");
        let legacy = old_layout(&root.0);
        let home = root.0.join(".falog");

        let results: Vec<Result<()>> = thread::scope(|scope| {
            let runs: Vec<_> = (0..4)
                .map(|_| scope.spawn(|| migrate(&home, &legacy, LOCK_WAIT)))
                .collect();
            runs.into_iter().map(|run| run.join().unwrap()).collect()
        });

        assert!(results.iter().all(Result::is_ok), "{results:?}");
        assert_eq!(area_names(&home.join(DATABASE_FILE)), ["Acme"]);
        assert!(home.join("models").join("model.bin").exists());
    }

    #[test]
    fn nothing_to_do_without_an_old_layout() {
        let root = TempDir::new("empty");
        let home = root.0.join(".falog");
        migrate(&home, &Legacy::default(), Duration::ZERO).unwrap();
        assert!(!home.exists());
    }
}
