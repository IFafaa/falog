//! Persists assistant threads as JSON in `<data dir>/assistant/threads.json`.

use super::thread::ThreadRecord;
use std::io;
use std::path::Path;

/// Older threads beyond this are dropped when saving.
pub const MAX_THREADS: usize = 100;

/// Loads saved threads. A missing file means no history; an unreadable one is set aside as
/// `threads.json.bak` instead of being overwritten.
pub fn load(path: &Path) -> Vec<ThreadRecord> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    match serde_json::from_str(&text) {
        Ok(records) => records,
        Err(_) => {
            let _ = std::fs::copy(path, path.with_extension("json.bak"));
            Vec::new()
        }
    }
}

/// Writes atomically (temporary file, then rename), newest threads first.
pub fn save(path: &Path, records: &mut Vec<ThreadRecord>) -> io::Result<()> {
    records.sort_by_key(|record| std::cmp::Reverse(record.updated_at));
    records.truncate(MAX_THREADS);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let partial = path.with_extension("json.tmp");
    std::fs::write(&partial, serde_json::to_vec(records).map_err(io::Error::other)?)?;
    std::fs::rename(&partial, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::thread::{Item, ThreadId};

    fn record(id: u64, updated_at: i64) -> ThreadRecord {
        ThreadRecord {
            id: ThreadId(id),
            created_at: updated_at,
            updated_at,
            session_id: Some(format!("session-{id}")),
            items: vec![Item::User(format!("message {id}"))],
            draft: String::new(),
            settings: Default::default(),
        }
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("falog-history-{name}-{}", std::process::id()))
            .join("threads.json")
    }

    #[test]
    fn saves_and_loads_newest_first() {
        let path = temp_path("roundtrip");
        save(&path, &mut vec![record(1, 10), record(2, 30), record(3, 20)]).unwrap();
        let ids: Vec<u64> = load(&path).iter().map(|r| r.id.0).collect();
        assert_eq!(ids, vec![2, 3, 1]);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn keeps_a_backup_of_unreadable_files() {
        let path = temp_path("corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{ not json").unwrap();
        assert!(load(&path).is_empty());
        assert!(path.with_extension("json.bak").is_file());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn missing_file_means_no_history() {
        assert!(load(&temp_path("missing")).is_empty());
    }
}
