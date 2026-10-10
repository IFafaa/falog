//! The language of Falog's own interface: English or Brazilian Portuguese, the computer's language at first
//! and then whatever Settings › General says. User data (tasks, areas, assistant replies) is never
//! translated.
//!
//! English text in the code is the key: `tr("Delete")` returns it as written in English, or its entry
//! in [`pt::text`] in Portuguese. Texts with values use `{name}` placeholders and [`trf`]. A test fails
//! when a text has no Portuguese entry, so new UI text cannot ship untranslated. Dates are written by
//! [`date`]; statuses, priorities and "No area", which `falog-core` shares with the MCP server, by
//! [`status_label`], [`priority_label`] and [`area_name`].

pub mod date;
mod pt;

use falog_core::domain::{Priority, Status, Task};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    English,
    Portuguese,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::English, Self::Portuguese];

    /// The language's own name, the same in every interface language.
    pub const fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Portuguese => "Português",
        }
    }

    /// The language the computer's region suggests, for the first start.
    pub fn suggested() -> Self {
        if crate::platform::locale::is_portuguese() {
            Self::Portuguese
        } else {
            Self::English
        }
    }
}

#[cfg(not(test))]
static CURRENT: AtomicU8 = AtomicU8::new(0);

#[cfg(test)]
thread_local! {
    /// One per test thread, so a test that switches the language does not change another's texts.
    static CURRENT: AtomicU8 = const { AtomicU8::new(0) };
}

/// Switches the interface language; the next frame draws in it.
pub fn set(language: Language) {
    #[cfg(not(test))]
    CURRENT.store(language as u8, Ordering::Relaxed);
    #[cfg(test)]
    CURRENT.with(|current| current.store(language as u8, Ordering::Relaxed));
}

pub fn current() -> Language {
    #[cfg(not(test))]
    let current = CURRENT.load(Ordering::Relaxed);
    #[cfg(test)]
    let current = CURRENT.with(|current| current.load(Ordering::Relaxed));
    match current {
        1 => Language::Portuguese,
        _ => Language::English,
    }
}

/// `english` in the current interface language.
///
/// A `|` starts a note that is not shown, for one English word that needs two translations:
/// `tr("Archive|verb")` is "Archive" in English and "Arquivar" in Portuguese, while `tr("Archive")`
/// is "Arquivo".
pub fn tr(english: &'static str) -> &'static str {
    let fallback = english.split_once('|').map_or(english, |(text, _)| text);
    match current() {
        Language::English => fallback,
        Language::Portuguese => pt::text(english).unwrap_or(fallback),
    }
}

/// [`tr`] for a text with `{name}` placeholders, filled with `values`.
pub fn trf(english: &'static str, values: &[(&str, &dyn Display)]) -> String {
    values.iter().fold(tr(english).to_owned(), |text, (name, value)| {
        text.replace(&format!("{{{name}}}"), &value.to_string())
    })
}

/// A status's name in the interface language. [`Status::label`] stays in English: the MCP server
/// shares it.
pub fn status_label(status: Status) -> &'static str {
    match status {
        Status::Todo => tr("To do"),
        Status::InProgress => tr("In progress"),
        Status::Waiting => tr("Waiting"),
        Status::Done => tr("Done"),
    }
}

/// A priority's name in the interface language. [`Priority::label`] stays in English: the MCP
/// server shares it.
pub fn priority_label(priority: Priority) -> &'static str {
    match priority {
        Priority::Low => tr("Low"),
        Priority::Medium => tr("Medium"),
        Priority::High => tr("High"),
        Priority::Urgent => tr("Urgent"),
    }
}

/// The name of a task's area, or "No area" in the interface language.
pub fn area_name(task: &Task) -> &str {
    task.area
        .as_ref()
        .map_or(tr("No area"), |area| area.name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn fills_placeholders_in_either_language() {
        set(Language::English);
        assert_eq!(trf("Archived {n} tasks", &[("n", &3)]), "Archived 3 tasks");
        set(Language::Portuguese);
        assert_eq!(trf("Archived {n} tasks", &[("n", &3)]), "3 tarefas arquivadas");
        set(Language::English);
    }

    #[test]
    fn a_note_after_a_bar_tells_translations_apart() {
        set(Language::English);
        assert_eq!(tr("Archive|verb"), "Archive");
        set(Language::Portuguese);
        assert_eq!(tr("Archive|verb"), "Arquivar");
        assert_eq!(tr("Archive"), "Arquivo");
        assert_eq!(status_label(Status::Done), "Concluída");
        set(Language::English);
    }

    /// Every text passed to `tr` and `trf` in the app's code has a Portuguese entry with the
    /// same placeholders.
    #[test]
    fn every_text_has_a_portuguese_entry() {
        let mut missing = Vec::new();
        visit(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut |source| {
                for english in texts_in(source) {
                    match pt::text(&english) {
                        None => missing.push(english),
                        Some(portuguese) if placeholders(portuguese) != placeholders(&english) => {
                            missing.push(format!("{english} (placeholders differ)"));
                        }
                        Some(_) => {}
                    }
                }
            },
        );
        missing.sort();
        missing.dedup();
        assert!(missing.is_empty(), "no Portuguese for:\n{}", missing.join("\n"));
    }

    fn visit(dir: &Path, check: &mut impl FnMut(&str)) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, check);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                check(&std::fs::read_to_string(&path).unwrap());
            }
        }
    }

    /// The literals passed to `tr(` and `trf(`, also on the next line as rustfmt puts long ones, with
    /// Rust escapes for quotes undone.
    fn texts_in(source: &str) -> Vec<String> {
        let mut texts = Vec::new();
        for call in ["tr", "trf"] {
            for (start, _) in source.match_indices(call) {
                let before = source[..start].chars().next_back();
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let Some(rest) = source[start + call.len()..]
                    .strip_prefix('(')
                    .map(str::trim_start)
                    .and_then(|rest| rest.strip_prefix('"'))
                else {
                    continue;
                };
                let mut text = String::new();
                let mut chars = rest.chars();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => text.extend(chars.next()),
                        '"' => break,
                        c => text.push(c),
                    }
                }
                texts.push(text);
            }
        }
        texts
    }

    fn placeholders(text: &str) -> Vec<&str> {
        let mut names: Vec<&str> = text
            .split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}').map(|(name, _)| name))
            .collect();
        names.sort_unstable();
        names
    }
}
