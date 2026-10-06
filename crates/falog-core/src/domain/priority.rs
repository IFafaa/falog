use crate::{Error, text::fold};
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Ordered from least to most pressing, so `Ord` can be used for sorting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    #[default]
    Medium,
    High,
    Urgent,
}

impl Priority {
    /// Most pressing first, as shown in pickers.
    pub const ALL: [Self; 4] = [Self::Urgent, Self::High, Self::Medium, Self::Low];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Urgent => "Urgent",
        }
    }

    /// Numeric rank stored in the database (1 = low ... 4 = urgent).
    pub const fn rank(self) -> i64 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Urgent => 4,
        }
    }

    pub const fn from_rank(rank: i64) -> Self {
        match rank {
            i64::MIN..=1 => Self::Low,
            2 => Self::Medium,
            3 => Self::High,
            _ => Self::Urgent,
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for Priority {
    type Err = Error;

    /// Accepts keys, ranks (`1`-`4`) and common English/Portuguese synonyms.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let priority = match fold(s).as_str() {
            "low" | "1" | "minor" | "whenever" | "baixa" | "baixo" => Self::Low,
            "medium" | "2" | "normal" | "media" | "medio" => Self::Medium,
            "high" | "3" | "important" | "alta" | "alto" | "importante" => Self::High,
            "urgent" | "4" | "critical" | "asap" | "urgente" | "critica" | "critico" => Self::Urgent,
            _ => return Err(Error::invalid(format!("unknown priority \"{s}\""))),
        };
        Ok(priority)
    }
}

impl ToSql for Priority {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.rank().into())
    }
}

impl FromSql for Priority {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        i64::column_result(value).map(Self::from_rank)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_by_urgency() {
        assert!(Priority::Urgent > Priority::High);
        assert!(Priority::Medium > Priority::Low);
    }

    #[test]
    fn parses_synonyms_and_ranks() {
        assert_eq!("Urgente".parse::<Priority>().unwrap(), Priority::Urgent);
        assert_eq!("3".parse::<Priority>().unwrap(), Priority::High);
        assert_eq!("média".parse::<Priority>().unwrap(), Priority::Medium);
        assert!("whatever".parse::<Priority>().is_err());
    }

    #[test]
    fn ranks_round_trip() {
        for priority in Priority::ALL {
            assert_eq!(Priority::from_rank(priority.rank()), priority);
        }
    }
}
