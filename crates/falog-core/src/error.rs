use crate::domain::TaskId;
use std::path::PathBuf;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Input rejected by a domain rule (empty title, malformed color, unknown status...).
    #[error("{0}")]
    Invalid(String),

    #[error("task #{0} not found")]
    TaskNotFound(TaskId),

    #[error("area \"{query}\" not found{}", known_areas(.known))]
    AreaNotFound { query: String, known: Vec<String> },

    #[error("\"{query}\" matches more than one area: {}", .candidates.join(", "))]
    AmbiguousArea { query: String, candidates: Vec<String> },

    #[error("area \"{0}\" already exists")]
    DuplicateArea(String),

    #[error("could not create the data directory {path}: {source}")]
    DataDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not move {from} to {to}: {source}")]
    Migration {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    Database(#[from] rusqlite::Error),
}

impl Error {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }
}

fn known_areas(known: &[String]) -> String {
    if known.is_empty() {
        " (no areas registered yet)".to_string()
    } else {
        format!("; registered areas: {}", known.join(", "))
    }
}
