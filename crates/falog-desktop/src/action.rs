//! User intents emitted by widgets and applied by the app after the frame is drawn.
//! Keeping UI code side-effect free avoids borrowing the app mutably while rendering it.

use crate::calendar::CalendarMode;
use crate::overlays::settings::SettingsTab;
use crate::theme::ThemeMode;
use crate::views::View;
use falog_core::domain::{AreaId, Status, TaskId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    OpenTask(TaskId),
    NewTask(Status),
    MoveTask(TaskId, Status),
    DeleteTask(TaskId),
    ArchiveTask(TaskId),
    RestoreTask(TaskId),
    /// Archive every done task the board shows (after the area filter and search).
    ArchiveDone,
    SetView(View),
    FilterArea(Option<AreaId>),
    ToggleSidebar,
    FocusSearch,
    OpenCommandPalette,
    OpenTaskFinder,
    ManageAreas,
    OpenSettings,
    OpenSettingsTab(SettingsTab),
    SetTheme(ThemeMode),
    Reload,
    ToggleAssistant,
    /// Zoom: the assistant fills the window, or goes back to its dock.
    ToggleAssistantZoom,
    /// Push-to-talk: start or finish dictating to the assistant.
    ToggleDictation,
    NewAssistantThread,
    ShowAssistantHistory,
    SetCalendarMode(CalendarMode),
    /// Sign in to another Google account in the browser.
    ConnectGoogle,
    RefreshCalendar,
    /// Show or hide a calendar, by its position in the saved accounts.
    ToggleCalendar {
        account: usize,
        calendar: usize,
    },
    /// Show or hide a calendar link, by its position in the saved links.
    ToggleCalendarLink(usize),
}

#[derive(Debug, Default)]
pub struct Actions(Vec<Action>);

impl Actions {
    pub fn push(&mut self, action: Action) {
        self.0.push(action);
    }

    pub fn take(&mut self) -> Vec<Action> {
        std::mem::take(&mut self.0)
    }
}
