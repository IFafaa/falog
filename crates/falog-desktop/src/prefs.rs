use crate::theme::ThemeMode;
use crate::views::{SortOrder, View};
use falog_core::domain::CompanyId;
use serde::{Deserialize, Serialize};

/// UI preferences persisted by eframe between runs.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub view: View,
    pub company: Option<CompanyId>,
    pub sort: SortOrder,
    pub theme: ThemeMode,
    pub sidebar_open: bool,
    /// List view: include completed tasks.
    pub show_completed: bool,
    /// Board view: keep showing tasks completed more than a week ago.
    pub show_old_completed: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            view: View::default(),
            company: None,
            sort: SortOrder::default(),
            theme: ThemeMode::default(),
            sidebar_open: true,
            show_completed: false,
            show_old_completed: false,
        }
    }
}
