//! What Falog remembers about Google Calendar, under `<data dir>/calendar/`:
//! `google.json` (OAuth client, accounts with their refresh tokens and calendar choices) and
//! `events.json` (the last events fetched, so the view opens instantly and offline).

use crate::model::{Calendar, Event};
use crate::{Error, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The user's own Google Cloud OAuth client ("Desktop app" type).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Client {
    pub id: String,
    /// Not confidential for desktop clients, per Google; still kept out of the prefs file.
    pub secret: String,
}

impl Client {
    pub fn is_set(&self) -> bool {
        !self.id.trim().is_empty() && !self.secret.trim().is_empty()
    }
}

/// A connected Google account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// The id of the account's primary calendar, which is its email address.
    pub email: String,
    pub refresh_token: String,
    pub calendars: Vec<Calendar>,
    /// Set when Google rejected the refresh token; the account must sign in again.
    #[serde(default)]
    pub needs_sign_in: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CalendarConfig {
    pub client: Client,
    pub accounts: Vec<Account>,
}

impl CalendarConfig {
    /// Adds an account, or replaces the one with the same email keeping its calendar choices.
    pub fn upsert(&mut self, mut account: Account) {
        if let Some(existing) = self.accounts.iter_mut().find(|a| a.email == account.email) {
            for calendar in &mut account.calendars {
                if let Some(old) = existing.calendars.iter().find(|c| c.id == calendar.id) {
                    calendar.visible = old.visible;
                }
            }
            *existing = account;
        } else {
            self.accounts.push(account);
        }
    }

    pub fn remove(&mut self, email: &str) -> Option<Account> {
        let index = self.accounts.iter().position(|a| a.email == email)?;
        Some(self.accounts.remove(index))
    }

    pub fn calendar(&self, account: &str, id: &str) -> Option<&Calendar> {
        self.accounts
            .iter()
            .find(|a| a.email == account)?
            .calendars
            .iter()
            .find(|c| c.id == id)
    }

    pub fn set_visible(&mut self, account: &str, id: &str, visible: bool) {
        if let Some(calendar) = self
            .accounts
            .iter_mut()
            .find(|a| a.email == account)
            .and_then(|a| a.calendars.iter_mut().find(|c| c.id == id))
        {
            calendar.visible = visible;
        }
    }

    pub fn is_visible(&self, event: &Event) -> bool {
        self.calendar(&event.account, &event.calendar_id)
            .is_some_and(|c| c.visible)
    }
}

/// Events for the days `from..to` (end exclusive), as last fetched.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventCache {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub events: Vec<Event>,
}

impl EventCache {
    pub fn covers(&self, from: NaiveDate, to: NaiveDate) -> bool {
        matches!((self.from, self.to), (Some(a), Some(b)) if a <= from && to <= b)
    }
}

/// The `calendar` folder in Falog's data directory.
#[derive(Clone, Debug)]
pub struct Files {
    dir: PathBuf,
}

impl Files {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn load_config(&self) -> CalendarConfig {
        load(&self.dir.join("google.json"))
    }

    pub fn save_config(&self, config: &CalendarConfig) -> Result<()> {
        save(&self.dir, "google.json", config)
    }

    pub fn load_cache(&self) -> EventCache {
        load(&self.dir.join("events.json"))
    }

    pub fn save_cache(&self, cache: &EventCache) -> Result<()> {
        save(&self.dir, "events.json", cache)
    }
}

/// A missing or unreadable file gives the default: losing the cache costs one refresh, and a broken
/// config is kept aside as `.bak` so the user can recover it.
fn load<T: Default + for<'de> Deserialize<'de>>(path: &Path) -> T {
    let Ok(text) = std::fs::read_to_string(path) else {
        return T::default();
    };
    serde_json::from_str(&text).unwrap_or_else(|_| {
        let _ = std::fs::rename(path, path.with_extension("json.bak"));
        T::default()
    })
}

/// Writes through a temporary file so a crash never leaves half a file behind.
fn save<T: Serialize>(dir: &Path, name: &str, value: &T) -> Result<()> {
    let file_error = |path: &Path| {
        let path = path.to_path_buf();
        move |source| Error::File { path, source }
    };
    std::fs::create_dir_all(dir).map_err(file_error(dir))?;
    let path = dir.join(name);
    let temp = dir.join(format!("{name}.tmp"));
    std::fs::write(&temp, serde_json::to_vec_pretty(value)?).map_err(file_error(&temp))?;
    std::fs::rename(&temp, &path).map_err(file_error(&path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calendar(id: &str, visible: bool) -> Calendar {
        Calendar {
            id: id.into(),
            name: id.into(),
            color: "#4285f4".into(),
            primary: false,
            visible,
        }
    }

    fn account(email: &str, calendars: Vec<Calendar>) -> Account {
        Account {
            email: email.into(),
            refresh_token: "token".into(),
            calendars,
            needs_sign_in: false,
        }
    }

    #[test]
    fn reconnecting_keeps_calendar_choices() {
        let mut config = CalendarConfig::default();
        config.upsert(account(
            "me@acme.com",
            vec![calendar("work", true), calendar("team", true)],
        ));
        config.set_visible("me@acme.com", "team", false);
        config.upsert(account(
            "me@acme.com",
            vec![calendar("work", true), calendar("team", true)],
        ));
        assert_eq!(config.accounts.len(), 1);
        assert!(!config.calendar("me@acme.com", "team").unwrap().visible);
        assert!(config.remove("me@acme.com").is_some());
        assert!(config.accounts.is_empty());
    }

    #[test]
    fn saves_and_loads_and_sets_broken_files_aside() {
        let dir = std::env::temp_dir().join(format!("falog-calendar-test-{}", std::process::id()));
        let files = Files::new(&dir);
        assert_eq!(files.load_config(), CalendarConfig::default());

        let mut config = CalendarConfig::default();
        config.client.id = "id".into();
        config.upsert(account("me@gmail.com", vec![calendar("primary", true)]));
        files.save_config(&config).unwrap();
        assert_eq!(files.load_config(), config);

        std::fs::write(dir.join("google.json"), "{ not json").unwrap();
        assert_eq!(files.load_config(), CalendarConfig::default());
        assert!(dir.join("google.json.bak").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cache_knows_its_range() {
        let day = |d| NaiveDate::from_ymd_opt(2026, 10, d).unwrap();
        let cache = EventCache {
            from: Some(day(1)),
            to: Some(day(31)),
            events: Vec::new(),
        };
        assert!(cache.covers(day(5), day(12)));
        assert!(!cache.covers(day(25), day(31).succ_opt().unwrap()));
        assert!(!EventCache::default().covers(day(1), day(2)));
    }
}
