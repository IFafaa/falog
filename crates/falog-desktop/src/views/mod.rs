//! The ways to look at tasks (and meetings), shown as tabs.

pub mod archive;
pub mod board;
pub mod calendar;
pub mod focus;
pub mod list;

use crate::components::badge;
use crate::fonts;
use crate::icons::Icon;
use crate::prefs::Prefs;
use crate::theme::Theme;
use chrono::NaiveDate;
use eframe::egui::{Color32, CursorIcon, RichText, ScrollArea, Sense, Ui};
use falog_core::domain::{Area, Task, TaskId};
use falog_core::focus::{by_urgency, cmp_due};
use falog_core::text::fold;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    #[default]
    Board,
    List,
    Focus,
    Calendar,
    /// Done tasks put away from the board.
    Archive,
}

impl View {
    pub const ALL: [Self; 5] = [
        Self::Board,
        Self::List,
        Self::Focus,
        Self::Calendar,
        Self::Archive,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Board => "Board",
            Self::List => "List",
            Self::Focus => "Focus",
            Self::Calendar => "Calendar",
            Self::Archive => "Archive",
        }
    }

    pub const fn icon(self) -> Icon {
        match self {
            Self::Board => Icon::Board,
            Self::List => Icon::List,
            Self::Focus => Icon::Reader,
            Self::Calendar => Icon::Calendar,
            Self::Archive => Icon::Archive,
        }
    }

    pub const fn shortcut(self) -> &'static str {
        match self {
            Self::Board => "Ctrl+1",
            Self::List => "Ctrl+2",
            Self::Focus => "Ctrl+3",
            Self::Calendar => "Ctrl+4",
            Self::Archive => "Ctrl+5",
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
            Self::Area => {
                // Both sorts are stable: by urgency, then by area folded once per task.
                tasks.sort_by(|a, b| by_urgency(a, b));
                tasks.sort_by_cached_key(|t| fold(t.area_name()));
            }
            Self::Newest => tasks.sort_by_key(|t| std::cmp::Reverse(t.id)),
        }
    }
}

/// Read-only state shared by every view for one frame.
#[derive(Debug)]
pub struct ViewCx<'a> {
    pub theme: &'static Theme,
    pub today: NaiveDate,
    /// Tasks after the area filter and search: archived ones in the Archive view, the others elsewhere.
    pub tasks: &'a [Task],
    pub areas: &'a [Area],
    pub prefs: &'a Prefs,
    /// The task open in the side panel.
    pub selected: Option<TaskId>,
    /// Whether any task exists at all, to tell "empty" from "filtered out".
    pub has_any_task: bool,
}

/// A vertically scrolling column at most `max_width` wide, centered in the view.
pub fn centered_column(ui: &mut Ui, max_width: f32, add_contents: impl FnOnce(&mut Ui)) {
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let width = ui.available_width().min(max_width);
        let margin = (ui.available_width() - width) / 2.0;
        ui.horizontal(|ui| {
            ui.add_space(margin);
            ui.vertical(|ui| {
                ui.set_width(width);
                add_contents(ui);
            });
        });
    });
}

/// A collapsible section header: chevron, title, count badge and a rule under it. `collapsed` holds
/// the keys of the folded sections; a click folds or unfolds this one. Returns whether it is expanded.
pub fn section_header<K: Eq + Hash>(
    ui: &mut Ui,
    theme: &Theme,
    title: &str,
    color: Color32,
    count: usize,
    key: K,
    collapsed: &mut HashSet<K>,
) -> bool {
    ui.add_space(16.0);
    let expanded = !collapsed.contains(&key);
    let response = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let chevron = if expanded {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            };
            ui.add(chevron.image(14.0, theme.icon_muted));
            ui.label(RichText::new(title).font(fonts::semibold(14.5)).color(color));
            badge(ui, count.to_string());
        })
        .response;
    let response = ui.interact(response.rect, response.id.with("toggle"), Sense::click());
    if response.on_hover_cursor(CursorIcon::PointingHand).clicked() && !collapsed.remove(&key) {
        collapsed.insert(key);
    }
    ui.add_space(2.0);
    ui.painter().hline(
        ui.min_rect().x_range(),
        ui.cursor().top(),
        (1.0, theme.border_variant),
    );
    ui.add_space(2.0);
    expanded
}
