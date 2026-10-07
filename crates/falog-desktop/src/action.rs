//! User intents emitted by widgets and applied by the app after the frame is drawn.
//! Keeping UI code side-effect free avoids borrowing the app mutably while rendering it.

use crate::theme::ThemeMode;
use crate::views::View;
use falog_core::domain::{AreaId, Status, TaskId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    OpenTask(TaskId),
    NewTask(Status),
    MoveTask(TaskId, Status),
    DeleteTask(TaskId),
    SetView(View),
    FilterArea(Option<AreaId>),
    ToggleSidebar,
    FocusSearch,
    OpenCommandPalette,
    OpenTaskFinder,
    ManageAreas,
    OpenSettings,
    SetTheme(ThemeMode),
    Reload,
    ToggleAssistant,
    /// Zoom: the assistant fills the window, or goes back to its dock.
    ToggleAssistantZoom,
    /// Push-to-talk: start or finish dictating to the assistant.
    ToggleDictation,
    NewAssistantThread,
    ShowAssistantHistory,
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
