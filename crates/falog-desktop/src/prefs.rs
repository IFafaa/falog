use crate::assistant::AssistantOptions;
use crate::assistant::voice::VoiceLanguage;
use crate::calendar::CalendarMode;
use crate::i18n::Language;
use crate::theme::ThemeMode;
use crate::views::{SortOrder, View};
use falog_core::domain::AreaId;
use serde::{Deserialize, Serialize};

/// UI preferences persisted by eframe between runs.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub view: View,
    /// Sidebar filter.
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
    /// The assistant fills the window (zoom).
    pub assistant_zoomed: bool,
    pub voice_language: VoiceLanguage,
    /// Send dictated text right away instead of leaving it in the composer.
    pub send_after_dictation: bool,
    /// Run speech recognition on the GPU when the build supports it.
    pub voice_gpu: bool,
    /// Look for a newer Falog now and then.
    pub check_updates: bool,
    /// Offer preview releases too, not only stable ones.
    pub update_preview: bool,
    /// The interface language; `None` until the first start sets it from the computer's language.
    pub language: Option<Language>,
}

impl Prefs {
    /// The first start speaks the language the computer suggests; afterwards the language saved
    /// (from Settings › General) stays.
    pub fn pick_language(&mut self, suggested: impl FnOnce() -> Language) {
        self.language.get_or_insert_with(suggested);
    }

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
            check_updates: true,
            update_preview: false,
            language: None,
        }
    }
}
