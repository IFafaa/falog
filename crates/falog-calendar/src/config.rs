//! What Falog remembers about Google Calendar, under `<data dir>/calendar/`:
//! `google.json` (calendar links, the OAuth client, accounts with their refresh tokens and calendar
//! choices) and `events.json` (the last events fetched, so the view opens instantly and offline).

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
    /// Connected with permission to create and change events. Accounts connected before Falog asked
    /// for it read only, until they reconnect.
    #[serde(default)]
    pub can_write: bool,
    /// The Falog area (id from `falog-core`) this account belongs to. One area per email; an area may
    /// have several emails.
    #[serde(default)]
    pub area: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CalendarConfig {
    pub client: Client,
    pub accounts: Vec<Account>,
    /// Calendars read through their secret iCal address. Missing in files written before links.
    pub links: Vec<Link>,
}

/// A calendar read through its secret iCal address instead of a signed-in account. Its events have
/// an empty `account` and the link's id as `calendar_id`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// The id is Falog's own (`link-…`); name, color and visibility are the user's.
    #[serde(flatten)]
    pub calendar: Calendar,
    /// The secret address. Whoever has it reads the calendar: never log it or show it whole (see
    /// [`crate::ics::masked`]).
    pub url: String,
    /// The Falog area this calendar belongs to, like an account's.
    #[serde(default)]
    pub area: Option<i64>,
}

impl std::fmt::Debug for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Link")
            .field("calendar", &self.calendar)
            .field("url", &crate::ics::masked(&self.url))
            .finish()
    }
}

/// Google Calendar's calendar colors, offered for links (feeds carry no color).
pub const LINK_COLORS: [&str; 12] = [
    "#039be5", "#33b679", "#8e24aa", "#e67c73", "#f6bf26", "#f4511e", "#7986cb", "#0b8043", "#3f51b5",
    "#ad1457", "#616161", "#d50000",
];

impl CalendarConfig {
    /// Adds an account, or replaces the one with the same email keeping its calendar choices.
    pub fn upsert(&mut self, mut account: Account) {
        if let Some(existing) = self.accounts.iter_mut().find(|a| a.email == account.email) {
            for calendar in &mut account.calendars {
                if let Some(old) = existing.calendars.iter().find(|c| c.id == calendar.id) {
                    calendar.visible = old.visible;
                }
            }
            // The area is the user's choice in Falog, not something Google sends.
            account.area = account.area.or(existing.area);
            *existing = account;
        } else {
            self.accounts.push(account);
        }
    }

    pub fn remove(&mut self, email: &str) -> Option<Account> {
        let index = self.accounts.iter().position(|a| a.email == email)?;
        Some(self.accounts.remove(index))
    }

    /// Adds a calendar link with a new id and the first color no calendar uses yet.
    pub fn add_link(&mut self, name: &str, url: &str, color: Option<String>) -> Result<&Link> {
        let mut random = [0u8; 6];
        getrandom::fill(&mut random).map_err(|err| Error::Io(std::io::Error::other(err.to_string())))?;
        let id: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let color = color.unwrap_or_else(|| self.unused_color().to_owned());
        self.links.push(Link {
            calendar: Calendar {
                id: format!("link-{id}"),
                name: name.trim().to_owned(),
                color,
                primary: false,
                visible: true,
            },
            url: url.trim().to_owned(),
            area: None,
        });
        Ok(&self.links[self.links.len() - 1])
    }

    pub fn has_link(&self, url: &str) -> bool {
        self.links.iter().any(|link| link.url == url.trim())
    }

    pub fn link_mut(&mut self, id: &str) -> Option<&mut Link> {
        self.links.iter_mut().find(|link| link.calendar.id == id)
    }

    pub fn remove_link(&mut self, id: &str) -> Option<Link> {
        let index = self.links.iter().position(|link| link.calendar.id == id)?;
        Some(self.links.remove(index))
    }

    fn unused_color(&self) -> &'static str {
        let used: Vec<&str> = self
            .accounts
            .iter()
            .flat_map(|a| &a.calendars)
            .chain(self.links.iter().map(|l| &l.calendar))
            .map(|c| c.color.as_str())
            .collect();
        LINK_COLORS
            .iter()
            .find(|color| !used.iter().any(|used| used.eq_ignore_ascii_case(color)))
            .unwrap_or(&LINK_COLORS[self.links.len() % LINK_COLORS.len()])
    }

    /// The calendar of an account, or the link `id` when `account` is empty.
    pub fn calendar(&self, account: &str, id: &str) -> Option<&Calendar> {
        if account.is_empty() {
            return self.links.iter().map(|link| &link.calendar).find(|c| c.id == id);
        }
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

    /// Mutable access to a calendar of an account, or to the link `id` when `account` is empty.
    pub fn calendar_mut(&mut self, account: &str, id: &str) -> Option<&mut Calendar> {
        if account.is_empty() {
            return self
                .links
                .iter_mut()
                .map(|link| &mut link.calendar)
                .find(|c| c.id == id);
        }
        self.accounts
            .iter_mut()
            .find(|a| a.email == account)?
            .calendars
            .iter_mut()
            .find(|c| c.id == id)
    }

    /// Puts the account with this email, or the link with this id, in `area` (or in none).
    pub fn set_area(&mut self, source: &str, area: Option<i64>) {
        if let Some(account) = self.accounts.iter_mut().find(|a| a.email == source) {
            account.area = area;
        } else if let Some(link) = self.link_mut(source) {
            link.area = area;
        }
    }

    /// Takes a deleted area off every account and link.
    pub fn forget_area(&mut self, area: i64) {
        for account in self.accounts.iter_mut().filter(|a| a.area == Some(area)) {
            account.area = None;
        }
        for link in self.links.iter_mut().filter(|l| l.area == Some(area)) {
            link.area = None;
        }
    }

    /// The area of the account or link an event comes from.
    pub fn area_of(&self, event: &Event) -> Option<i64> {
        if event.account.is_empty() {
            return self
                .links
                .iter()
                .find(|l| l.calendar.id == event.calendar_id)?
                .area;
        }
        self.accounts.iter().find(|a| a.email == event.account)?.area
    }

    /// The signed-in accounts in `area`; those are the ones events can be created in.
    pub fn accounts_of_area(&self, area: i64) -> Vec<&Account> {
        self.accounts.iter().filter(|a| a.area == Some(area)).collect()
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

    /// Tells the desktop app that events changed elsewhere (the MCP server created one), so it
    /// fetches again instead of waiting for its next refresh.
    pub fn mark_changed(&self) -> Result<()> {
        save(&self.dir, "changed", &chrono::Local::now().to_rfc3339())
    }

    /// When [`Files::mark_changed`] last ran, if ever.
    pub fn changed_at(&self) -> Option<std::time::SystemTime> {
        std::fs::metadata(self.dir.join("changed")).ok()?.modified().ok()
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
    write_private(&temp, &serde_json::to_vec_pretty(value)?).map_err(file_error(&temp))?;
    std::fs::rename(&temp, &path).map_err(file_error(&path))
}

/// Writes a file only the user can read on Unix (0600): it holds refresh tokens, the OAuth client
/// secret and secret calendar addresses. Windows already keeps the user's profile private.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        // `mode` applies only to new files; a temp file left by a crash keeps its old one.
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        file.write_all(bytes)
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, bytes)
    }
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
            can_write: false,
            area: None,
        }
    }

    #[test]
    fn each_email_has_one_area_and_an_area_many_emails() {
        let mut config = CalendarConfig::default();
        config.upsert(account("me@work.example", vec![calendar("work", true)]));
        config.upsert(account("me@gmail.example", vec![calendar("home", true)]));
        config.upsert(account("me@home.example", vec![calendar("home", true)]));
        config
            .add_link("Team", "https://example.com/team.ics", None)
            .unwrap();
        let link = config.links[0].calendar.id.clone();

        config.set_area("me@work.example", Some(1));
        config.set_area("me@home.example", Some(1));
        config.set_area(&link, Some(2));
        config.set_area("me@work.example", Some(3));
        let emails: Vec<&str> = config
            .accounts_of_area(1)
            .iter()
            .map(|a| a.email.as_str())
            .collect();
        assert_eq!(emails, vec!["me@home.example"]);

        let event = |account: &str, calendar: &str| Event {
            account: account.into(),
            calendar_id: calendar.into(),
            id: "e".into(),
            ical_uid: String::new(),
            title: "Meeting".into(),
            start: crate::EventTime::Date(chrono::NaiveDate::from_ymd_opt(2026, 10, 7).unwrap()),
            end: crate::EventTime::Date(chrono::NaiveDate::from_ymd_opt(2026, 10, 8).unwrap()),
            location: String::new(),
            description: String::new(),
            join_link: None,
            html_link: None,
        };
        assert_eq!(config.area_of(&event("me@work.example", "work")), Some(3));
        assert_eq!(config.area_of(&event("", &link)), Some(2));
        assert_eq!(config.area_of(&event("me@gmail.example", "home")), None);

        // Reconnecting brings a fresh account from Google; the area stays.
        config.upsert(account("me@work.example", vec![calendar("work", true)]));
        assert_eq!(config.area_of(&event("me@work.example", "work")), Some(3));

        config.forget_area(3);
        assert_eq!(config.area_of(&event("me@work.example", "work")), None);
        assert_eq!(config.accounts_of_area(1).len(), 1);
    }

    #[test]
    fn reconnecting_keeps_calendar_choices() {
        let mut config = CalendarConfig::default();
        config.upsert(account(
            "me@work.com",
            vec![calendar("work", true), calendar("team", true)],
        ));
        config.set_visible("me@work.com", "team", false);
        config.upsert(account(
            "me@work.com",
            vec![calendar("work", true), calendar("team", true)],
        ));
        assert_eq!(config.accounts.len(), 1);
        assert!(!config.calendar("me@work.com", "team").unwrap().visible);
        assert!(config.remove("me@work.com").is_some());
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
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join("google.json"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "tokens must not be readable by other users");
        }

        std::fs::write(dir.join("google.json"), "{ not json").unwrap();
        assert_eq!(files.load_config(), CalendarConfig::default());
        assert!(dir.join("google.json.bak").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn files_without_links_still_load() {
        let old = r##"{ "client": { "id": "id", "secret": "s" }, "accounts": [ { "email": "me@work.com",
            "refresh_token": "t", "calendars": [ { "id": "me@work.com", "name": "Work",
            "color": "#9fe1e7", "primary": true, "visible": true } ] } ] }"##;
        let config: CalendarConfig = serde_json::from_str(old).unwrap();
        assert_eq!(config.accounts.len(), 1);
        assert!(config.links.is_empty());
    }

    #[test]
    fn links_get_ids_and_unused_colors_and_round_trip() {
        let mut config = CalendarConfig::default();
        config.upsert(account("me@work.com", vec![calendar("work", true)]));
        let url = "https://calendar.google.com/calendar/ical/me%40gmail.com/private-s3cr3t/basic.ics";
        let link = config.add_link(" Personal ", url, None).unwrap().clone();
        assert!(link.calendar.id.starts_with("link-"));
        assert_eq!(link.calendar.name, "Personal");
        // #4285f4 is taken by the account's calendar in these tests; the palette's first is free.
        assert_eq!(link.calendar.color, LINK_COLORS[0]);
        assert!(config.has_link(url));
        let second = config
            .add_link("Work", "https://example.com/b.ics", None)
            .unwrap();
        assert_eq!(second.calendar.color, LINK_COLORS[1]);
        assert_ne!(second.calendar.id, link.calendar.id);

        assert_eq!(config.calendar("", &link.calendar.id), Some(&link.calendar));
        assert_eq!(config.calendar("me@work.com", &link.calendar.id), None);
        assert!(!format!("{link:?}").contains("s3cr3t"));

        let json = serde_json::to_string(&config).unwrap();
        let back: CalendarConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, config);

        config.link_mut(&link.calendar.id).unwrap().calendar.visible = false;
        let day = NaiveDate::from_ymd_opt(2026, 10, 7).unwrap();
        let mut event_of_link = Event {
            account: String::new(),
            calendar_id: link.calendar.id.clone(),
            id: "e".into(),
            ical_uid: String::new(),
            title: "Dentist".into(),
            start: crate::EventTime::Date(day),
            end: crate::EventTime::Date(day),
            location: String::new(),
            description: String::new(),
            join_link: None,
            html_link: None,
        };
        assert!(!config.is_visible(&event_of_link));
        event_of_link.calendar_id = "gone".into();
        assert!(!config.is_visible(&event_of_link));
        assert_eq!(
            config.remove_link(&link.calendar.id).map(|l| l.url),
            Some(url.to_owned())
        );
        assert_eq!(config.links.len(), 1);
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
