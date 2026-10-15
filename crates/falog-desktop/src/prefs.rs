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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// The preferences file in memory: eframe keeps `app.ron` as one RON string per key.
    #[derive(Debug, Default)]
    struct Storage(HashMap<String, String>);

    impl eframe::Storage for Storage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.to_owned(), value);
        }
        fn flush(&mut self) {}
    }

    /// What the app reads at startup from a file holding `ron`.
    fn load(ron: &str) -> Option<Prefs> {
        let mut storage = Storage::default();
        storage.0.insert(eframe::APP_KEY.to_owned(), ron.to_owned());
        eframe::get_value(&storage, eframe::APP_KEY)
    }

    #[test]
    fn saved_preferences_load_back() {
        let saved = Prefs {
            view: View::Archive,
            area: Some(AreaId(3)),
            sort: SortOrder::Area,
            theme: ThemeMode::Light,
            show_old_completed: true,
            language: Some(Language::Portuguese),
            ..Prefs::default()
        };
        let mut storage = Storage::default();
        eframe::set_value(&mut storage, eframe::APP_KEY, &saved);
        let loaded: Prefs = eframe::get_value(&storage, eframe::APP_KEY).unwrap();
        assert_eq!(loaded.view, View::Archive);
        assert_eq!(loaded.area, Some(AreaId(3)));
        assert_eq!(loaded.sort, SortOrder::Area);
        assert_eq!(loaded.theme, ThemeMode::Light);
        assert!(loaded.show_old_completed);
        assert_eq!(loaded.language, Some(Language::Portuguese));
    }

    #[test]
    fn preferences_of_an_older_version_keep_what_they_had() {
        // Written before the language, the archive and the updater existed, with a setting since
        // removed. Losing the whole file to one unknown or missing field would reset everything.
        let prefs =
            load("(view:List,sort:DueDate,theme:Dark,sidebar_open:false,show_completed:true,zoom:1.25)")
                .expect("an older file still loads");
        assert_eq!(prefs.view, View::List);
        assert_eq!(prefs.sort, SortOrder::DueDate);
        assert_eq!(prefs.theme, ThemeMode::Dark);
        assert!(!prefs.sidebar_open);
        assert!(prefs.show_completed);
        // What the file did not have takes its default.
        assert_eq!(prefs.language, None, "picked from the computer at startup");
        assert_eq!(prefs.calendar_mode, CalendarMode::Week);
        assert!(prefs.check_updates);
        assert!(!prefs.update_preview);
        assert!(prefs.send_after_dictation);
    }

    #[test]
    fn the_first_start_picks_the_suggested_language_and_later_ones_keep_the_saved_one() {
        let mut prefs = Prefs::default();
        prefs.pick_language(|| Language::Portuguese);
        assert_eq!(prefs.language, Some(Language::Portuguese));

        let mut prefs = Prefs {
            language: Some(Language::English),
            ..Prefs::default()
        };
        prefs.pick_language(|| Language::Portuguese);
        assert_eq!(prefs.language, Some(Language::English), "the user's choice stays");
    }

    #[test]
    fn the_assistant_is_zoomed_only_while_open() {
        let prefs = Prefs {
            assistant_zoomed: true,
            ..Prefs::default()
        };
        assert!(!prefs.assistant_options().zoomed);
        let prefs = Prefs {
            assistant_open: true,
            ..prefs
        };
        assert!(prefs.assistant_options().zoomed);
    }
}
