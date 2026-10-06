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

    #[error("company \"{query}\" not found{}", known_companies(.known))]
    CompanyNotFound { query: String, known: Vec<String> },

    #[error("\"{query}\" matches more than one company: {}", .candidates.join(", "))]
    AmbiguousCompany { query: String, candidates: Vec<String> },

    #[error("company \"{0}\" already exists")]
    DuplicateCompany(String),

    #[error("could not create the data directory {path}: {source}")]
    DataDir {
        path: PathBuf,
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

fn known_companies(known: &[String]) -> String {
    if known.is_empty() {
        " (no companies registered yet)".to_string()
    } else {
        format!("; registered companies: {}", known.join(", "))
    }
}
