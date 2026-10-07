use crate::assistant::AssistantOptions;
use crate::assistant::voice::VoiceLanguage;
use crate::calendar::CalendarMode;
use crate::theme::ThemeMode;
use crate::views::{SortOrder, View};
use falog_core::domain::AreaId;
use serde::{Deserialize, Serialize};

/// UI preferences persisted by eframe between runs.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub view: View,
    /// Sidebar filter; saved as `company` before areas existed.
    #[serde(alias = "company")]
    pub area: Option<AreaId>,
    pub sort: SortOrder,
    pub calendar_mode: CalendarMode,
    pub theme: ThemeMode,
    pub sidebar_open: bool,
    /// List view: include completed tasks.
    pub show_completed: bool,
    /// Board view: keep showing tasks completed more than a week ago.
    pub show_old_completed: bool,
    pub assistant_open: bool,
    /// The assistant fills the window (Zed's zoom).
    pub assistant_zoomed: bool,
    pub voice_language: VoiceLanguage,
    /// Send dictated text right away instead of leaving it in the composer.
    pub send_after_dictation: bool,
    /// Run speech recognition on the GPU when the build supports it.
    pub voice_gpu: bool,
}

impl Prefs {
    pub fn assistant_options(&self) -> AssistantOptions {
        AssistantOptions {
            language: self.voice_language,
            send_after_dictation: self.send_after_dictation,
            zoomed: self.assistant_zoomed && self.assistant_open,
            voice_gpu: self.voice_gpu,
        }
    }
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            view: View::default(),
            area: None,
            sort: SortOrder::default(),
            calendar_mode: CalendarMode::default(),
            theme: ThemeMode::default(),
            sidebar_open: true,
            show_completed: false,
            show_old_completed: false,
            assistant_open: false,
            assistant_zoomed: false,
            voice_language: VoiceLanguage::default(),
            send_after_dictation: true,
            voice_gpu: true,
        }
    }
}
