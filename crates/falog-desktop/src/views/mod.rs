//! The ways to look at tasks (and meetings), shown as tabs.

pub mod board;
pub mod calendar;
pub mod focus;
pub mod list;

use crate::icons::Icon;
use crate::prefs::Prefs;
use crate::theme::Theme;
use chrono::NaiveDate;
use falog_core::agenda::{by_urgency, cmp_due};
use falog_core::domain::{Area, Task, TaskId};
use falog_core::text::fold;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    #[default]
    Board,
    List,
    /// Saved as `Agenda` before the rename.
    #[serde(alias = "Agenda")]
    Focus,
    Calendar,
}

impl View {
    pub const ALL: [Self; 4] = [Self::Board, Self::List, Self::Focus, Self::Calendar];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Board => "Board",
            Self::List => "List",
            Self::Focus => "Focus",
            Self::Calendar => "Calendar",
        }
    }

    pub const fn icon(self) -> Icon {
        match self {
            Self::Board => Icon::Board,
            Self::List => Icon::List,
            Self::Focus => Icon::Reader,
            Self::Calendar => Icon::Calendar,
        }
    }

    pub const fn shortcut(self) -> &'static str {
        match self {
            Self::Board => "Ctrl+1",
            Self::List => "Ctrl+2",
            Self::Focus => "Ctrl+3",
            Self::Calendar => "Ctrl+4",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    #[default]
    Urgency,
    DueDate,
    Area,
    Newest,
}

impl SortOrder {
    pub const ALL: [Self; 4] = [Self::Urgency, Self::DueDate, Self::Area, Self::Newest];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Urgency => "Urgency",
            Self::DueDate => "Due date",
            Self::Area => "Area",
            Self::Newest => "Newest",
        }
    }

    pub fn sort(self, tasks: &mut [&Task]) {
        match self {
            Self::Urgency => tasks.sort_by(|a, b| by_urgency(a, b)),
            Self::DueDate => tasks.sort_by(|a, b| cmp_due(a.due, b.due).then_with(|| by_urgency(a, b))),
            Self::Area => tasks.sort_by(|a, b| {
                fold(a.area_name())
                    .cmp(&fold(b.area_name()))
                    .then_with(|| by_urgency(a, b))
            }),
            Self::Newest => tasks.sort_by_key(|t| std::cmp::Reverse(t.id)),
        }
    }
}

/// Read-only state shared by every view for one frame.
#[derive(Debug)]
pub struct ViewCx<'a> {
    pub theme: &'static Theme,
    pub today: NaiveDate,
    /// Tasks after the area filter and search.
    pub tasks: &'a [Task],
    pub areas: &'a [Area],
    pub prefs: &'a Prefs,
    /// The task open in the side panel.
    pub selected: Option<TaskId>,
    /// Whether any task exists at all, to tell "empty" from "filtered out".
    pub has_any_task: bool,
}
