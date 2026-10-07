//! Google Calendar in the app: the saved accounts, the events on screen, and background threads
//! that sign in and fetch without blocking the UI. Results come back over a channel that
//! [`CalendarState::poll`] drains each frame, like the assistant does with Claude Code.

use chrono::{Datelike, Days, Local, NaiveDate};
use eframe::egui;
use falog_calendar::config::Files;
use falog_calendar::oauth::{self, AccessToken};
use falog_calendar::{Account, Calendar, CalendarConfig, Client, Error, Event, EventCache, google, model};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
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
    },
}

type Tokens = Arc<Mutex<HashMap<String, AccessToken>>>;

/// What one account returned: its calendars and the events of the visible ones.
type AccountResult = Result<(Vec<Calendar>, Vec<Event>), Error>;

pub struct CalendarState {
    files: Option<Files>,
    pub config: CalendarConfig,
    cache: EventCache,
    tokens: Tokens,
    sender: Sender<Update>,
    receiver: Receiver<Update>,
    refreshing: bool,
    /// Set while the browser sign-in is open; storing `true` cancels it.
    connecting: Option<Arc<AtomicBool>>,
    last_fetch: Option<Instant>,
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
            .field("accounts", &self.config.accounts.len())
            .field("events", &self.cache.events.len())
            .field("refreshing", &self.refreshing)
            .finish_non_exhaustive()
    }
}

impl CalendarState {
    /// Loads accounts and cached events from `<data dir>/calendar`, next to the database.
    pub fn new(data_dir: Option<&Path>) -> Self {
        let files = data_dir.map(|dir| Files::new(dir.join("calendar")));
        let (config, cache) = files
            .as_ref()
            .map(|f| (f.load_config(), f.load_cache()))
            .unwrap_or_default();
        let (sender, receiver) = channel();
        Self {
            files,
            config,
            cache,
            tokens: Tokens::default(),
            sender,
            receiver,
            refreshing: false,
            connecting: None,
            last_fetch: None,
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

    pub fn is_connecting(&self) -> bool {
        self.connecting.is_some()
    }

    pub fn is_busy(&self) -> bool {
        self.refreshing || self.connecting.is_some()
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

    pub fn remove_account(&mut self, email: &str) {
        if let Some(account) = self.config.remove(email) {
            std::thread::spawn(move || oauth::revoke(&account.refresh_token));
        }
        self.tokens.lock().map(|mut t| t.remove(email)).ok();
        self.cache.events.retain(|e| e.account != email);
        self.save_config();
        self.save_cache();
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
        if self.refreshing || self.config.accounts.is_empty() || !self.has_client() {
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
        let accounts: Vec<Account> = self
            .config
            .accounts
            .iter()
            .filter(|a| !a.needs_sign_in)
            .cloned()
            .collect();
        let sender = self.sender.clone();
        let tokens = self.tokens.clone();
        std::thread::spawn(move || {
            let accounts = accounts
                .into_iter()
                .map(|account| {
                    let result = fetch_account(&client, &account, &tokens, from, to);
                    (account.email, result)
                })
                .collect();
            let _ = sender.send(Update::Fetched { from, to, accounts });
        });
    }

    /// Applies finished background work. Returns true when something changed on screen.
    pub fn poll(&mut self, ctx: &egui::Context) -> bool {
        let mut changed = false;
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
                Update::Fetched { from, to, accounts } => self.fetched(from, to, accounts),
            }
        }
        changed
    }

    fn fetched(&mut self, from: NaiveDate, to: NaiveDate, accounts: Vec<(String, AccountResult)>) {
        self.refreshing = false;
        let mut events = Vec::new();
        let mut errors = Vec::new();
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
    let refresh_token = pending.finish(client, cancel)?;
    let access = oauth::refresh(client, &refresh_token, "the new account")?;
    let calendars = google::calendars(&access.token)?;
    let email = google::account_email(&calendars)
        .ok_or_else(|| Error::Parse("the account has no primary calendar".into()))?
        .to_owned();
    if let Ok(mut tokens) = tokens.lock() {
        tokens.insert(email.clone(), access);
    }
    Ok(Account {
        email,
        refresh_token,
        calendars,
        needs_sign_in: false,
    })
}

fn access_token(client: &Client, account: &Account, tokens: &Tokens) -> Result<String, Error> {
    if let Some(token) = tokens
        .lock()
        .ok()
        .and_then(|t| t.get(&account.email).filter(|t| t.is_fresh()).cloned())
    {
        return Ok(token.token);
    }
    let fresh = oauth::refresh(client, &account.refresh_token, &account.email)?;
    let token = fresh.token.clone();
    if let Ok(mut tokens) = tokens.lock() {
        tokens.insert(account.email.clone(), fresh);
    }
    Ok(token)
}

/// The account's current calendar list (keeping the user's show/hide choices) and the events of
/// its visible calendars.
fn fetch_account(
    client: &Client,
    account: &Account,
    tokens: &Tokens,
    from: NaiveDate,
    to: NaiveDate,
) -> AccountResult {
    let token = access_token(client, account, tokens)?;
    let mut calendars = google::calendars(&token)?;
    for calendar in &mut calendars {
        if let Some(old) = account.calendars.iter().find(|c| c.id == calendar.id) {
            calendar.visible = old.visible;
        }
    }
    let mut events = Vec::new();
    for calendar in calendars.iter().filter(|c| c.visible) {
        events.extend(google::events(&token, &account.email, &calendar.id, from, to)?);
    }
    Ok((calendars, events))
}
