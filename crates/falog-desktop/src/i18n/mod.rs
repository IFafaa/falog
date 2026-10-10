//! The language of Falog's own interface: English or Brazilian Portuguese, the computer's language at first
//! and then whatever Settings › General says. User data (tasks, areas, assistant replies) is never
//! translated.
//!
//! English text in the code is the key: `tr("Delete")` returns it as written in English, or its entry
//! in [`pt::text`] in Portuguese. Texts with values use `{name}` placeholders and [`trf`]. A test fails
//! when a text has no Portuguese entry, so new UI text cannot ship untranslated.

mod pt;

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

static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Switches the interface language; the next frame draws in it.
pub fn set(language: Language) {
    CURRENT.store(language as u8, Ordering::Relaxed);
}

pub fn current() -> Language {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Language::Portuguese,
        _ => Language::English,
    }
}

/// `english` in the current interface language.
pub fn tr(english: &'static str) -> &'static str {
    match current() {
        Language::English => english,
        Language::Portuguese => pt::text(english).unwrap_or(english),
    }
}

/// [`tr`] for a text with `{name}` placeholders, filled with `values`.
pub fn trf(english: &'static str, values: &[(&str, &dyn Display)]) -> String {
    values.iter().fold(tr(english).to_owned(), |text, (name, value)| {
        text.replace(&format!("{{{name}}}"), &value.to_string())
    })
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

    /// The literals passed to `tr(` and `trf(`, with Rust escapes for quotes undone.
    fn texts_in(source: &str) -> Vec<String> {
        let mut texts = Vec::new();
        for call in ["tr(\"", "trf(\""] {
            for (start, _) in source.match_indices(call) {
                let before = source[..start].chars().next_back();
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let rest = &source[start + call.len()..];
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
