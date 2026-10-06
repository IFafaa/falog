//! Typed tool arguments. Models are not always strict about JSON types, so ids and enum-like
//! fields accept numbers as well as strings.

use falog_core::domain::TaskId;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct AgendaArgs {
    pub company: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCompanyArgs {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskArgs {
    pub title: String,
    pub company: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default, deserialize_with = "lenient_string")]
    pub priority: Option<String>,
    #[serde(default, deserialize_with = "lenient_string")]
    pub due_date: Option<String>,
    #[serde(default)]
    pub requester: String,
    #[serde(default, deserialize_with = "lenient_string")]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskArgs {
    pub id: TaskRef,
    pub title: Option<String>,
    pub company: Option<String>,
    pub description: Option<String>,
    #[serde(default, deserialize_with = "lenient_string")]
    pub priority: Option<String>,
    #[serde(default, deserialize_with = "lenient_string")]
    pub due_date: Option<String>,
    pub requester: Option<String>,
    #[serde(default, deserialize_with = "lenient_string")]
    pub status: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddNoteArgs {
    pub id: TaskRef,
    pub note: String,
}

#[derive(Debug, Deserialize)]
pub struct ListTasksArgs {
    pub company: Option<String>,
    #[serde(default, deserialize_with = "lenient_string")]
    pub status: Option<String>,
    pub search: Option<String>,
    #[serde(default)]
    pub include_done: bool,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct TaskArgs {
    pub id: TaskRef,
}

/// A task id given as `12`, `"12"` or `"#12"`.
#[derive(Debug, Clone, Copy)]
pub struct TaskRef(pub TaskId);

impl<'de> Deserialize<'de> for TaskRef {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Scalar::deserialize(deserializer)? {
            Scalar::Int(id) => Ok(Self(TaskId(id))),
            other => other
                .into_string()
                .parse()
                .map(Self)
                .map_err(serde::de::Error::custom),
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Scalar {
    Int(i64),
    Float(f64),
    Bool(bool),
    Text(String),
}

impl Scalar {
    fn into_string(self) -> String {
        match self {
            Self::Int(n) => n.to_string(),
            Self::Float(n) => n.to_string(),
            Self::Bool(b) => b.to_string(),
            Self::Text(s) => s,
        }
    }
}

fn lenient_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Ok(Option::<Scalar>::deserialize(deserializer)?.map(Scalar::into_string))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn task_refs_accept_numbers_and_strings() {
        for value in [json!(12), json!("12"), json!("#12")] {
            let args: TaskArgs = serde_json::from_value(json!({ "id": value })).unwrap();
            assert_eq!(args.id.0, TaskId(12));
        }
        assert!(serde_json::from_value::<TaskArgs>(json!({ "id": "twelve" })).is_err());
    }

    #[test]
    fn enum_fields_accept_numbers() {
        let args: CreateTaskArgs = serde_json::from_value(json!({ "title": "A", "priority": 3 })).unwrap();
        assert_eq!(args.priority.as_deref(), Some("3"));
    }
}
