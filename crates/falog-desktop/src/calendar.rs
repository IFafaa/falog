//! Google Calendar in the app: the calendar links and signed-in accounts, the events on screen, and
//! background threads that check links, sign in and fetch without blocking the UI. Results come back
//! over a channel that [`CalendarState::poll`] drains each frame, like the assistant does with
//! Claude Code.

use chrono::{Datelike, Days, Local, NaiveDate};
use eframe::egui;
use falog_calendar::config::Files;
use falog_calendar::ics::{self, Feed};
use falog_calendar::oauth;
use falog_calendar::service::{self, Tokens};
use falog_calendar::{
    Account, Calendar, CalendarConfig, Client, Error, Event, EventCache, Link, google, model,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// Events are fetched again after this long, while the calendar is on screen.
const REFRESH_EVERY: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalendarMode {
    #[default]
    Week,
    Month,
}

impl CalendarMode {
    pub const ALL: [Self; 2] = [Self::Week, Self::Month];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Week => "Week",
            Self::Month => "Month",
        }
    }
}

/// An event picked in the view, shown in a popover.
#[derive(Clone, Debug)]
pub struct Selected {
    pub event: Event,
    pub anchor: egui::Pos2,
}

enum Update {
    OpenUrl(String),
    Connected(Account),
    ConnectFailed(Error),
    Fetched {
        from: NaiveDate,
        to: NaiveDate,
        accounts: Vec<(String, AccountResult)>,
        /// By link id.
        links: Vec<(String, Result<Vec<Event>, Error>)>,
    },
    /// A pasted link was read once, or could not be.
    LinkChecked {
        name: String,
        url: String,
        result: Result<Feed, Error>,
    },
}

/// What one account returned: its calendars and the events of the visible ones.
type AccountResult = Result<(Vec<Calendar>, Vec<Event>), Error>;

pub struct CalendarState {
    files: Option<Files>,
    pub config: CalendarConfig,
    cache: EventCache,
    tokens: Arc<Tokens>,
    sender: Sender<Update>,
    receiver: Receiver<Update>,
    refreshing: bool,
    /// Set while the browser sign-in is open; storing `true` cancels it.
    connecting: Option<Arc<AtomicBool>>,
    /// Set while a pasted link is being checked.
    adding_link: bool,
    /// Why the last pasted link was refused.
    pub link_error: Option<String>,
    last_fetch: Option<Instant>,
    /// The MCP server's last change seen, and when the marker was last checked.
    seen_change: Option<std::time::SystemTime>,
    last_change_check: Instant,
    /// Time of the last successful fetch, shown as "Updated 10:32".
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub error: Option<String>,
    /// Day the view is centered on.
    pub anchor: NaiveDate,
    pub selected: Option<Selected>,
    /// The week view scrolls to the working day once, then keeps the user's position.
    pub scrolled: bool,
}

impl std::fmt::Debug for CalendarState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalendarState")
            .field("links", &self.config.links.len())
            .field("accounts", &self.config.accounts.len())
            .field("events", &self.cache.events.len())
            .field("refreshing", &self.refreshing)
            .finish_non_exhaustive()
    }
}

impl CalendarState {
    /// Loads links, accounts and cached events from `<data dir>/calendar`, next to the database.
    pub fn new(data_dir: Option<&Path>) -> Self {
        let files = data_dir.map(|dir| Files::new(dir.join("calendar")));
        let (config, cache) = files
            .as_ref()
            .map(|f| (f.load_config(), f.load_cache()))
            .unwrap_or_default();
        let seen_change = files.as_ref().and_then(Files::changed_at);
        let (sender, receiver) = channel();
        Self {
            files,
            config,
            cache,
            tokens: Arc::default(),
            sender,
            receiver,
            refreshing: false,
            connecting: None,
            adding_link: false,
            link_error: None,
            last_fetch: None,
            seen_change,
            last_change_check: Instant::now(),
            updated_at: None,
            error: None,
            anchor: Local::now().date_naive(),
            selected: None,
            scrolled: false,
        }
    }

    pub fn has_client(&self) -> bool {
        self.config.client.is_set()
    }

    /// At least one calendar is linked or one account connected.
    pub fn has_calendars(&self) -> bool {
        !self.config.links.is_empty() || !self.config.accounts.is_empty()
    }

    pub fn is_connecting(&self) -> bool {
        self.connecting.is_some()
    }

    pub fn is_adding_link(&self) -> bool {
        self.adding_link
    }

    pub fn is_busy(&self) -> bool {
        self.refreshing || self.connecting.is_some() || self.adding_link
    }

    /// Visible events touching `from..to`, deduplicated and sorted.
    pub fn events(&self, from: NaiveDate, to: NaiveDate) -> Vec<Event> {
        let mut events: Vec<Event> = self
            .cache
            .events
            .iter()
            .filter(|e| self.config.is_visible(e))
            .filter(|e| {
                let (first, last) = e.first_and_last_day();
                first < to && last >= from
            })
            .cloned()
            .collect();
        model::sort_and_dedupe(&mut events);
        events
    }

    pub fn calendar_of(&self, event: &Event) -> Option<&Calendar> {
        self.config.calendar(&event.account, &event.calendar_id)
    }

    // ---- settings ------------------------------------------------------------------------------

    pub fn set_client(&mut self, client: Client) {
        self.config.client = client;
        self.save_config();
    }

    pub fn toggle_calendar(&mut self, account: usize, calendar: usize) {
        let Some(cal) = self
            .config
            .accounts
            .get_mut(account)
            .and_then(|a| a.calendars.get_mut(calendar))
        else {
            return;
        };
        cal.visible = !cal.visible;
        let shown = cal.visible;
        self.save_config();
        if shown {
            // Hidden calendars are not fetched, so showing one needs its events.
            self.last_fetch = None;
        }
    }

    pub fn toggle_link(&mut self, index: usize) {
        let Some(link) = self.config.links.get_mut(index) else {
            return;
        };
        link.calendar.visible = !link.calendar.visible;
        let shown = link.calendar.visible;
        self.save_config();
        if shown {
            self.last_fetch = None;
        }
    }

    pub fn remove_account(&mut self, email: &str) {
        if let Some(account) = self.config.remove(email) {
            std::thread::spawn(move || oauth::revoke(&account.refresh_token));
        }
        self.tokens.forget(email);
        self.cache.events.retain(|e| e.account != email);
        self.save_config();
        self.save_cache();
    }

    pub fn rename_link(&mut self, id: &str, name: &str) {
        if let Some(link) = self.config.link_mut(id)
            && !name.trim().is_empty()
        {
            link.calendar.name = name.trim().to_owned();
            self.save_config();
        }
    }

    pub fn set_link_color(&mut self, id: &str, color: &str) {
        if let Some(link) = self.config.link_mut(id) {
            link.calendar.color = color.to_owned();
            self.save_config();
        }
    }

    /// Forgets the link and its events. The address stays valid in Google until the user resets it.
    pub fn remove_link(&mut self, id: &str) {
        self.config.remove_link(id);
        self.cache
            .events
            .retain(|e| !(e.account.is_empty() && e.calendar_id == id));
        self.save_config();
        self.save_cache();
    }

    /// Puts an account (by email) or a calendar link (by id) in an area, or in none.
    pub fn set_area(&mut self, source: &str, area: Option<falog_core::domain::AreaId>) {
        self.config.set_area(source, area.map(|a| a.0));
        self.save_config();
    }

    /// A deleted area leaves every calendar.
    pub fn forget_area(&mut self, area: falog_core::domain::AreaId) {
        self.config.forget_area(area.0);
        self.save_config();
    }

    fn save_config(&mut self) {
        if let Some(files) = &self.files
            && let Err(err) = files.save_config(&self.config)
        {
            self.error = Some(err.to_string());
        }
    }

    fn save_cache(&mut self) {
        if let Some(files) = &self.files {
            let _ = files.save_cache(&self.cache);
        }
    }

    // ---- adding a link ------------------------------------------------------------------------

    /// Reads the pasted link once in the background; it is saved only if that works.
    pub fn add_link(&mut self, name: &str, url: &str) {
        if self.adding_link {
            return;
        }
        let url = url.trim().to_owned();
        if self.config.has_link(&url) {
            self.link_error = Some("This calendar is already linked".into());
            return;
        }
        self.adding_link = true;
        self.link_error = None;
        let name = name.trim().to_owned();
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = ics::fetch(&url);
            let _ = sender.send(Update::LinkChecked { name, url, result });
        });
    }

    fn link_checked(&mut self, name: String, url: String, result: Result<Feed, Error>) {
        self.adding_link = false;
        let feed = match result {
            Ok(feed) => feed,
            Err(err) => {
                self.link_error = Some(sentence(&err.to_string()));
                return;
            }
        };
        // The user's name, else the calendar's own (Google uses the account's email for the main
        // calendar), else the owner from the address.
        let name = Some(name)
            .filter(|n| !n.is_empty())
            .or_else(|| feed.name.clone())
            .or_else(|| feed.owner.clone())
            .unwrap_or_else(|| "Calendar".to_owned());
        let id = match self.config.add_link(&name, &url, feed.color.clone()) {
            Ok(link) => link.calendar.id.clone(),
            Err(err) => {
                self.link_error = Some(sentence(&err.to_string()));
                return;
            }
        };
        self.save_config();
        // Show its events right away for the range already on screen; the next refresh widens it.
        if let (Some(from), Some(to)) = (self.cache.from, self.cache.to) {
            self.cache.events.extend(feed.events(&id, from, to));
            self.save_cache();
        } else {
            self.last_fetch = None;
        }
    }

    // ---- sign-in ------------------------------------------------------------------------------

    /// Opens Google's consent page in the browser and waits for it in the background.
    pub fn connect(&mut self) {
        if self.connecting.is_some() {
            return;
        }
        let client = self.config.client.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        self.connecting = Some(cancel.clone());
        self.error = None;
        let sender = self.sender.clone();
        let tokens = self.tokens.clone();
        std::thread::spawn(move || {
            let result = sign_in(&client, &cancel, &sender, &tokens);
            let _ = sender.send(match result {
                Ok(account) => Update::Connected(account),
                Err(err) => Update::ConnectFailed(err),
            });
        });
    }

    pub fn cancel_connect(&mut self) {
        if let Some(cancel) = self.connecting.take() {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    // ---- fetching -----------------------------------------------------------------------------

    /// Makes sure the days `from..to` are loaded and recent; fetches in the background otherwise.
    pub fn ensure(&mut self, from: NaiveDate, to: NaiveDate) {
        let stale = self.last_fetch.is_none_or(|at| at.elapsed() >= REFRESH_EVERY);
        // Accounts are read with the OAuth client they signed in with; links need nothing.
        let fetchable =
            !self.config.links.is_empty() || (self.has_client() && !self.config.accounts.is_empty());
        if self.refreshing || !fetchable {
            return;
        }
        if stale || !self.cache.covers(from, to) {
            self.fetch(from, to);
        }
    }

    /// Fetches the visible range now (the refresh button).
    pub fn refresh(&mut self) {
        self.last_fetch = None;
    }

    fn fetch(&mut self, from: NaiveDate, to: NaiveDate) {
        // A wide window around the view so paging back and forth stays instant.
        let from = month_start(from) - Days::new(7);
        let to = month_start(to)
            .checked_add_months(chrono::Months::new(1))
            .unwrap_or(to)
            + Days::new(7);
        self.refreshing = true;
        self.last_fetch = Some(Instant::now());
        let client = self.config.client.clone();
        let accounts: Vec<Account> = if self.has_client() {
            self.config
                .accounts
                .iter()
                .filter(|a| !a.needs_sign_in)
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        // Hidden links are not fetched, like hidden calendars of an account.
        let links: Vec<Link> = self
            .config
            .links
            .iter()
            .filter(|l| l.calendar.visible)
            .cloned()
            .collect();
        let sender = self.sender.clone();
        let tokens = self.tokens.clone();
        std::thread::spawn(move || {
            let accounts = accounts
                .into_iter()
                .map(|account| {
                    let result = service::fetch_account(&client, &account, &tokens, from, to);
                    (account.email, result)
                })
                .collect();
            let links = links
                .into_iter()
                .map(|link| {
                    let id = link.calendar.id;
                    let result = ics::fetch(&link.url).map(|feed| feed.events(&id, from, to));
                    (id, result)
                })
                .collect();
            let _ = sender.send(Update::Fetched {
                from,
                to,
                accounts,
                links,
            });
        });
    }

    /// Applies finished background work. Returns true when something changed on screen.
    pub fn poll(&mut self, ctx: &egui::Context) -> bool {
        let mut changed = false;
        // The assistant created or changed an event through the MCP server: fetch again now.
        if self.last_change_check.elapsed() >= Duration::from_secs(1) {
            self.last_change_check = Instant::now();
            let marker = self.files.as_ref().and_then(Files::changed_at);
            if marker.is_some() && marker != self.seen_change {
                self.seen_change = marker;
                self.refresh();
                changed = true;
            }
        }
        while let Ok(update) = self.receiver.try_recv() {
            changed = true;
            match update {
                Update::OpenUrl(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
                Update::Connected(account) => {
                    self.connecting = None;
                    self.config.upsert(account);
                    self.save_config();
                    self.last_fetch = None;
                }
                Update::ConnectFailed(err) => {
                    self.connecting = None;
                    if !matches!(err, Error::Cancelled) {
                        self.error = Some(err.to_string());
                    }
                }
                Update::Fetched {
                    from,
                    to,
                    accounts,
                    links,
                } => self.fetched(from, to, accounts, links),
                Update::LinkChecked { name, url, result } => self.link_checked(name, url, result),
            }
        }
        changed
    }

    fn fetched(
        &mut self,
        from: NaiveDate,
        to: NaiveDate,
        accounts: Vec<(String, AccountResult)>,
        links: Vec<(String, Result<Vec<Event>, Error>)>,
    ) {
        self.refreshing = false;
        let mut events = Vec::new();
        let mut errors = Vec::new();
        // A link added while this fetch ran keeps the events it was added with, and gets fetched
        // properly next frame.
        for link in self.config.links.iter().filter(|l| l.calendar.visible) {
            if !links.iter().any(|(id, _)| *id == link.calendar.id) {
                let id = &link.calendar.id;
                events.extend(
                    self.cache
                        .events
                        .iter()
                        .filter(|e| e.account.is_empty() && e.calendar_id == *id)
                        .cloned(),
                );
                self.last_fetch = None;
            }
        }
        // Accounts this fetch skipped (no OAuth client, waiting to sign in again, connected while it
        // ran) keep their events too.
        let skipped: Vec<String> = self
            .config
            .accounts
            .iter()
            .map(|a| a.email.clone())
            .filter(|email| !accounts.iter().any(|(fetched, _)| fetched == email))
            .collect();
        // Links first: when the same meeting comes through a link and an account, the link's copy
        // is the one kept.
        for (id, result) in links {
            match result {
                Ok(link_events) => events.extend(link_events),
                Err(err) => {
                    // Keep what we had for this link rather than blanking it.
                    events.extend(
                        self.cache
                            .events
                            .iter()
                            .filter(|e| e.account.is_empty() && e.calendar_id == id)
                            .cloned(),
                    );
                    let name = self.config.calendar("", &id).map(|c| c.name.clone());
                    errors.push(match name {
                        Some(name) => format!("{name}: {err}"),
                        None => err.to_string(),
                    });
                }
            }
        }
        for (email, result) in accounts {
            match result {
                Ok((calendars, account_events)) => {
                    if let Some(account) = self.config.accounts.iter().find(|a| a.email == email) {
                        let mut updated = account.clone();
                        updated.calendars = calendars;
                        self.config.upsert(updated);
                    }
                    events.extend(account_events);
                }
                Err(err) => {
                    if let Error::Reauthorize(_) = err
                        && let Some(account) = self.config.accounts.iter_mut().find(|a| a.email == email)
                    {
                        account.needs_sign_in = true;
                    }
                    // Keep what we had for this account rather than blanking it.
                    events.extend(self.cache.events.iter().filter(|e| e.account == email).cloned());
                    errors.push(err.to_string());
                }
            }
        }
        for email in skipped {
            events.extend(self.cache.events.iter().filter(|e| e.account == email).cloned());
        }
        self.save_config();
        self.cache = EventCache {
            from: Some(from),
            to: Some(to),
            events,
        };
        self.save_cache();
        self.error = errors.into_iter().next();
        if self.error.is_none() {
            self.updated_at = Some(Local::now().naive_local());
        }
    }
}

/// Error texts are lowercase clauses; the Settings page shows them as sentences.
fn sentence(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn month_start(day: NaiveDate) -> NaiveDate {
    day.with_day(1).unwrap_or(day)
}

fn sign_in(
    client: &Client,
    cancel: &AtomicBool,
    sender: &Sender<Update>,
    tokens: &Tokens,
) -> Result<Account, Error> {
    let pending = oauth::begin(client)?;
    let _ = sender.send(Update::OpenUrl(pending.url().to_owned()));
    let grant = pending.finish(client, cancel)?;
    let access = oauth::refresh(client, &grant.refresh_token, "the new account")?;
    let calendars = google::calendars(&access.token)?;
    let email = google::account_email(&calendars)
        .ok_or_else(|| Error::Parse("the account has no primary calendar".into()))?
        .to_owned();
    tokens.insert(&email, access);
    Ok(Account {
        email,
        can_write: grant.can_write(),
        refresh_token: grant.refresh_token,
        calendars,
        needs_sign_in: false,
        area: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, d).unwrap()
    }

    fn event(account: &str, calendar_id: &str, title: &str) -> Event {
        Event {
            account: account.into(),
            calendar_id: calendar_id.into(),
            id: title.into(),
            ical_uid: String::new(),
            title: title.into(),
            start: falog_calendar::EventTime::Date(day(7)),
            end: falog_calendar::EventTime::Date(day(8)),
            location: String::new(),
            description: String::new(),
            join_link: None,
            html_link: None,
        }
    }

    fn titles(state: &CalendarState) -> Vec<&str> {
        let mut titles: Vec<&str> = state.cache.events.iter().map(|e| e.title.as_str()).collect();
        titles.sort_unstable();
        titles
    }

    #[test]
    fn a_refresh_keeps_what_it_did_not_or_could_not_fetch() {
        let mut state = CalendarState::new(None);
        state.config.upsert(Account {
            email: "me@acme.com".into(),
            refresh_token: "t".into(),
            calendars: Vec::new(),
            needs_sign_in: false,
            can_write: false,
            area: None,
        });
        let work = state
            .config
            .add_link("Work", "https://example.com/w.ics", None)
            .unwrap()
            .calendar
            .id
            .clone();
        let home = state
            .config
            .add_link("Home", "https://example.com/h.ics", None)
            .unwrap()
            .calendar
            .id
            .clone();
        state.cache.events = vec![
            event("me@acme.com", "me@acme.com", "Account meeting"),
            event("", &work, "Old work"),
            event("", &home, "Old home"),
        ];
        // No OAuth client, so the account was skipped; the work link answered, the home one failed.
        state.fetched(
            day(1),
            day(31),
            Vec::new(),
            vec![
                (work.clone(), Ok(vec![event("", &work, "New work")])),
                (
                    home.clone(),
                    Err(Error::Link("the calendar link answered 500".into())),
                ),
            ],
        );
        assert_eq!(titles(&state), vec!["Account meeting", "New work", "Old home"]);
        assert_eq!(
            state.error.as_deref(),
            Some("Home: the calendar link answered 500")
        );
    }

    #[test]
    fn errors_read_as_sentences() {
        assert_eq!(
            sentence("the calendar link answered 500"),
            "The calendar link answered 500"
        );
        assert_eq!(sentence(""), "");
    }
}
