use crate::Error;
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub i64);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                self.0.to_sql()
            }
        }

        impl FromSql for $name {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                i64::column_result(value).map(Self)
            }
        }
    };
}

id_type!(TaskId);
id_type!(CompanyId);
id_type!(NoteId);

impl FromStr for TaskId {
    type Err = Error;

    /// Accepts `12` or `#12`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.trim()
            .trim_start_matches('#')
            .parse()
            .map(Self)
            .map_err(|_| Error::invalid(format!("invalid task id \"{s}\"")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_task_ids_with_or_without_hash() {
        assert_eq!("12".parse::<TaskId>().unwrap(), TaskId(12));
        assert_eq!(" #7 ".parse::<TaskId>().unwrap(), TaskId(7));
        assert!("abc".parse::<TaskId>().is_err());
    }
}
