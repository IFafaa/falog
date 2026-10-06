use crate::{Error, text::fold};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Where a task is in its lifecycle. Each status is a column on the board.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Todo,
    InProgress,
    /// Blocked on someone else: code review, an answer, a deploy window...
    Waiting,
    Done,
}

impl Status {
    pub const ALL: [Self; 4] = [Self::Todo, Self::InProgress, Self::Waiting, Self::Done];

    /// Stable key used in storage and in the MCP API.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Waiting => "waiting",
            Self::Done => "done",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Todo => "To do",
            Self::InProgress => "In progress",
            Self::Waiting => "Waiting",
            Self::Done => "Done",
        }
    }

    pub fn is_open(self) -> bool {
        self != Self::Done
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

impl FromStr for Status {
    type Err = Error;

    /// Accepts the storage key and common English/Portuguese synonyms, since input
    /// often comes from dictation ("doing", "em review", "feito").
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key = fold(s).replace(['_', '-'], " ");
        let status = match key.as_str() {
            "todo" | "to do" | "open" | "backlog" | "pending" | "new" | "a fazer" | "fazer" | "pendente"
            | "aberta" | "aberto" => Self::Todo,
            "in progress" | "doing" | "started" | "wip" | "working" | "fazendo" | "em andamento"
            | "andamento" | "em progresso" => Self::InProgress,
            "waiting" | "blocked" | "on hold" | "review" | "in review" | "paused" | "aguardando"
            | "bloqueado" | "bloqueada" | "esperando" | "em review" | "revisao" | "em revisao" => {
                Self::Waiting
            }
            "done" | "complete" | "completed" | "finished" | "closed" | "concluido" | "concluida"
            | "feito" | "feita" | "finalizado" | "finalizada" | "pronto" | "pronta" | "entregue" => {
                Self::Done
            }
            _ => return Err(Error::invalid(format!("unknown status \"{s}\""))),
        };
        Ok(status)
    }
}

impl ToSql for Status {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.as_str().into())
    }
}

impl FromSql for Status {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let key = value.as_str()?;
        Self::ALL
            .into_iter()
            .find(|s| s.as_str() == key)
            .ok_or_else(|| FromSqlError::Other(format!("unknown status \"{key}\"").into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keys_and_synonyms() {
        assert_eq!("in_progress".parse::<Status>().unwrap(), Status::InProgress);
        assert_eq!("Em Review".parse::<Status>().unwrap(), Status::Waiting);
        assert_eq!("concluído".parse::<Status>().unwrap(), Status::Done);
        assert!("someday".parse::<Status>().is_err());
    }

    #[test]
    fn keys_round_trip() {
        for status in Status::ALL {
            assert_eq!(status.as_str().parse::<Status>().unwrap(), status);
        }
    }
}
