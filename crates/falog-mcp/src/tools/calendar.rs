//! Calendar tools: the user's Google calendars, read through the same files as the desktop app
//! (`<data dir>/calendar`), with events created in the account linked to an area.

use anyhow::{Context, Result, anyhow, bail};
use chrono::{Days, Duration, NaiveDate, NaiveDateTime, NaiveTime};
use falog_calendar::config::Files;
use falog_calendar::google::{EventPatch, NewEvent};
use falog_calendar::service::{self, Tokens};
use falog_calendar::{Account, CalendarConfig, Event, EventTime, ics};
use falog_core::text::fold;
use falog_core::{Store, date};
use serde::Deserialize;
use std::fmt::Write;

/// Where the calendar files are, and the access tokens fetched so far.
#[derive(Debug, Default)]
pub struct CalendarAccess {
    files: Option<Files>,
    tokens: Tokens,
}

impl CalendarAccess {
    /// The `calendar` folder next to the store's database, like the desktop app's.
    pub fn for_store(store: &Store) -> Self {
        let files = store
            .path()
            .and_then(std::path::Path::parent)
            .map(|dir| Files::new(dir.join("calendar")));
        Self {
            files,
            tokens: Tokens::default(),
        }
    }

    fn config(&self) -> Result<CalendarConfig> {
        let config = self.files.as_ref().map(Files::load_config).unwrap_or_default();
        if config.accounts.is_empty() && config.links.is_empty() {
            bail!("no calendars are connected; the user connects them in Falog, Settings › Calendar");
        }
        Ok(config)
    }

    /// Tells the desktop app to fetch again so the change shows up right away.
    fn changed(&self) {
        if let Some(files) = &self.files {
            let _ = files.mark_changed();
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListArgs {
    pub from: Option<String>,
    pub to: Option<String>,
    pub area: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateArgs {
    pub title: String,
    pub start: String,
    pub end: Option<String>,
    pub duration_minutes: Option<i64>,
    #[serde(default)]
    pub all_day: bool,
    pub area: Option<String>,
    pub calendar: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub attendees: Vec<String>,
    #[serde(default)]
    pub meet: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateArgs {
    pub calendar: String,
    pub id: String,
    pub title: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteArgs {
    pub calendar: String,
    pub id: String,
}

pub fn list_events(store: &Store, access: &CalendarAccess, args: ListArgs) -> Result<String> {
    let config = access.config()?;
    let today = date::today();
    let from = day(args.from.as_deref(), today)?.unwrap_or(today);
    let last = day(args.to.as_deref(), today)?.unwrap_or(from).max(from);
    let to = last + Days::new(1);
    let area = match args.area.as_deref().filter(|a| !a.trim().is_empty()) {
        Some(name) => Some(store.find_area(name)?),
        None => None,
    };

    let mut events = Vec::new();
    let mut problems = Vec::new();
    for account in &config.accounts {
        if area.as_ref().is_some_and(|a| account.area != Some(a.id.0)) {
            continue;
        }
        if account.needs_sign_in {
            problems.push(format!("{} needs to sign in again in Falog", account.email));
            continue;
        }
        match service::fetch_account(&config.client, account, &access.tokens, from, to) {
            Ok((_, found)) => events.extend(found),
            Err(err) => problems.push(format!("{}: {err}", account.email)),
        }
    }
    for link in &config.links {
        if area.as_ref().is_some_and(|a| link.area != Some(a.id.0)) || !link.calendar.visible {
            continue;
        }
        match ics::fetch(&link.url) {
            Ok(feed) => events.extend(feed.events(&link.calendar.id, from, to)),
            Err(err) => problems.push(format!("{}: {err}", link.calendar.name)),
        }
    }
    falog_calendar::model::sort_and_dedupe(&mut events);

    let areas = store.areas()?;
    let area_name = |event: &Event| {
        config
            .area_of(event)
            .and_then(|id| areas.iter().find(|a| a.id.0 == id))
            .map(|a| a.name.clone())
    };
    let range = if from == last {
        from.format("%a %b %-d").to_string()
    } else {
        format!("{} to {}", from.format("%a %b %-d"), last.format("%a %b %-d"))
    };
    let mut out = format!("{} event(s), {range}\n", events.len());
    for event in &events {
        let _ = writeln!(out, "- {}", event_line(event, area_name(event).as_deref()));
    }
    for problem in problems {
        let _ = writeln!(out, "Could not read {problem}");
    }
    Ok(out.trim_end().to_owned())
}

pub fn create_event(store: &Store, access: &CalendarAccess, args: CreateArgs) -> Result<String> {
    let config = access.config()?;
    let account = target_account(store, &config, args.area.as_deref(), args.calendar.as_deref())?;
    let (start, end) = if args.all_day {
        let first = parse_day(&args.start)?;
        let last = match args.end.as_deref() {
            Some(end) => parse_day(end)?.max(first),
            None => first,
        };
        (EventTime::Date(first), EventTime::Date(last + Days::new(1)))
    } else {
        let start = parse_moment(&args.start)?;
        let end = match (args.end.as_deref(), args.duration_minutes) {
            (Some(end), _) => parse_moment(end)?,
            (None, Some(minutes)) => start + Duration::minutes(minutes.max(1)),
            (None, None) => start + Duration::minutes(30),
        };
        if end <= start {
            bail!("the event must end after it starts");
        }
        (EventTime::At(start), EventTime::At(end))
    };
    let event = NewEvent {
        title: args.title.trim().to_owned(),
        start: Some(start),
        end: Some(end),
        description: args.description,
        location: args.location,
        attendees: args.attendees,
        meet: args.meet,
    };
    if event.title.is_empty() {
        bail!("the event needs a title");
    }
    let created = service::create_event(&config.client, account, &access.tokens, &account.email, &event)?;
    access.changed();
    Ok(format!("Created {}", event_line(&created, None)))
}

pub fn update_event(access: &CalendarAccess, args: UpdateArgs) -> Result<String> {
    let config = access.config()?;
    let account = owner(&config, &args.calendar)?;
    let patch = EventPatch {
        title: args.title.filter(|t| !t.trim().is_empty()),
        start: args
            .start
            .as_deref()
            .map(parse_moment)
            .transpose()?
            .map(EventTime::At),
        end: args
            .end
            .as_deref()
            .map(parse_moment)
            .transpose()?
            .map(EventTime::At),
        description: args.description,
        location: args.location,
    };
    let updated = service::update_event(
        &config.client,
        account,
        &access.tokens,
        &args.calendar,
        &args.id,
        &patch,
    )?;
    access.changed();
    Ok(format!("Updated {}", event_line(&updated, None)))
}

pub fn delete_event(access: &CalendarAccess, args: DeleteArgs) -> Result<String> {
    let config = access.config()?;
    let account = owner(&config, &args.calendar)?;
    service::delete_event(&config.client, account, &access.tokens, &args.calendar, &args.id)?;
    access.changed();
    Ok(format!("Deleted event {} from {}", args.id, args.calendar))
}

/// The account to create an event in: the one named by `calendar` (an email or a part of it), else
/// the only one linked to `area`, else the only account there is.
fn target_account<'a>(
    store: &Store,
    config: &'a CalendarConfig,
    area: Option<&str>,
    calendar: Option<&str>,
) -> Result<&'a Account> {
    let list = |accounts: &[&Account]| {
        accounts
            .iter()
            .map(|a| a.email.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    if let Some(query) = calendar.filter(|c| !c.trim().is_empty()) {
        let needle = fold(query);
        let matches: Vec<&Account> = config
            .accounts
            .iter()
            .filter(|a| fold(&a.email).contains(&needle))
            .collect();
        return match matches.as_slice() {
            [account] => Ok(account),
            [] => Err(anyhow!(
                "no connected Google account matches \"{query}\"; connected: {}",
                list(&config.accounts.iter().collect::<Vec<_>>())
            )),
            many => Err(anyhow!("\"{query}\" matches several accounts: {}", list(many))),
        };
    }
    if let Some(name) = area.filter(|a| !a.trim().is_empty()) {
        let area = store.find_area(name)?;
        let accounts = config.accounts_of_area(area.id.0);
        return match accounts.as_slice() {
            [account] => Ok(account),
            [] => Err(anyhow!(
                "no Google account is linked to {}; ask which account to use (pass calendar) or link one in Falog, Settings › Calendar",
                area.name
            )),
            many => Err(anyhow!(
                "{} has several accounts ({}); pass calendar with the one to use",
                area.name,
                list(many)
            )),
        };
    }
    match config.accounts.as_slice() {
        [account] => Ok(account),
        [] => bail!("no Google account is connected for writing; calendar links are read-only"),
        _ => bail!(
            "several Google accounts are connected ({}); pass the area or the calendar",
            list(&config.accounts.iter().collect::<Vec<_>>())
        ),
    }
}

/// The account a calendar belongs to: its primary calendar is the email, others are listed.
fn owner<'a>(config: &'a CalendarConfig, calendar: &str) -> Result<&'a Account> {
    config
        .accounts
        .iter()
        .find(|a| a.email == calendar || a.calendars.iter().any(|c| c.id == calendar))
        .ok_or_else(|| {
            anyhow!(
                "no connected Google account has the calendar \"{calendar}\" (calendar links are read-only)"
            )
        })
}

/// One line per event, with what `update_event` and `delete_event` need to address it.
fn event_line(event: &Event, area: Option<&str>) -> String {
    let when = match (event.start, event.end) {
        (EventTime::At(start), EventTime::At(end)) if start.date() == end.date() => format!(
            "{} {}–{}",
            start.format("%a %b %-d"),
            start.format("%H:%M"),
            end.format("%H:%M")
        ),
        (EventTime::At(start), EventTime::At(end)) => {
            format!(
                "{} – {}",
                start.format("%a %b %-d %H:%M"),
                end.format("%a %b %-d %H:%M")
            )
        }
        _ => {
            let (first, last) = event.first_and_last_day();
            if first == last {
                format!("{} all day", first.format("%a %b %-d"))
            } else {
                format!(
                    "{} – {} all day",
                    first.format("%a %b %-d"),
                    last.format("%a %b %-d")
                )
            }
        }
    };
    let mut line = format!("{when} · {}", event.title);
    if let Some(area) = area {
        let _ = write!(line, " [{area}]");
    }
    if !event.location.is_empty() {
        let _ = write!(line, " · at {}", event.location);
    }
    if let Some(link) = &event.join_link {
        let _ = write!(line, " · join {link}");
    }
    if !event.account.is_empty() {
        let _ = write!(line, " (calendar: {}, id: {})", event.calendar_id, event.id);
    }
    line
}

/// A day like the due dates (`2026-10-08`, "tomorrow", "sexta"), or `None` when not given.
fn day(text: Option<&str>, today: NaiveDate) -> Result<Option<NaiveDate>> {
    match text.map(str::trim).filter(|t| !t.is_empty()) {
        None => Ok(None),
        Some(text) => date::parse_due(text, today)
            .map(Some)
            .ok_or_else(|| anyhow!("could not read the date \"{text}\"; use YYYY-MM-DD")),
    }
}

fn parse_day(text: &str) -> Result<NaiveDate> {
    day(Some(text), date::today())?.ok_or_else(|| anyhow!("missing date"))
}

/// A local date and time: `2026-10-08T15:00`, `2026-10-08 15:00` (seconds optional).
fn parse_moment(text: &str) -> Result<NaiveDateTime> {
    let text = text.trim();
    for format in [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
    ] {
        if let Ok(at) = NaiveDateTime::parse_from_str(text, format) {
            return Ok(at);
        }
    }
    // An RFC 3339 time with an offset: keep the local wall time it names.
    if let Ok(at) = chrono::DateTime::parse_from_rfc3339(text) {
        return Ok(at.with_timezone(&chrono::Local).naive_local());
    }
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .map(|day| day.and_time(NaiveTime::MIN))
        .with_context(|| format!("could not read the time \"{text}\"; use YYYY-MM-DDTHH:MM"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use falog_calendar::Calendar;

    fn account(email: &str, area: Option<i64>) -> Account {
        Account {
            email: email.into(),
            refresh_token: "t".into(),
            calendars: vec![Calendar {
                id: email.into(),
                name: email.into(),
                color: "#039be5".into(),
                primary: true,
                visible: true,
            }],
            needs_sign_in: false,
            can_write: true,
            area,
        }
    }

    #[test]
    fn picks_the_account_of_an_area() {
        let store = Store::open_in_memory().unwrap();
        let work = store.create_area("Work", None).unwrap();
        let home = store.create_area("Home", None).unwrap();
        store.create_area("Studies", None).unwrap();
        let config = CalendarConfig {
            accounts: vec![
                account("me@work.example", Some(work.id.0)),
                account("me@home.example", Some(home.id.0)),
                account("me@home2.example", Some(home.id.0)),
                account("me@gmail.example", None),
            ],
            ..CalendarConfig::default()
        };
        let pick = |area: Option<&str>, calendar: Option<&str>| {
            target_account(&store, &config, area, calendar).map(|a| a.email.clone())
        };
        assert_eq!(pick(Some("work"), None).unwrap(), "me@work.example");
        assert!(
            pick(Some("home"), None)
                .unwrap_err()
                .to_string()
                .contains("several accounts")
        );
        assert_eq!(pick(Some("home"), Some("home2")).unwrap(), "me@home2.example");
        assert!(
            pick(Some("studies"), None)
                .unwrap_err()
                .to_string()
                .contains("no Google account is linked")
        );
        assert!(
            pick(None, None)
                .unwrap_err()
                .to_string()
                .contains("pass the area or the calendar")
        );
        assert!(
            pick(None, Some("home"))
                .unwrap_err()
                .to_string()
                .contains("several accounts")
        );
    }

    #[test]
    fn finds_the_owner_of_a_calendar() {
        let mut work = account("me@work.example", None);
        work.calendars.push(Calendar {
            id: "team@group.calendar.google.com".into(),
            name: "Team".into(),
            color: "#039be5".into(),
            primary: false,
            visible: true,
        });
        let config = CalendarConfig {
            accounts: vec![work, account("me@gmail.example", None)],
            ..CalendarConfig::default()
        };
        assert_eq!(
            owner(&config, "team@group.calendar.google.com").unwrap().email,
            "me@work.example"
        );
        assert_eq!(
            owner(&config, "me@gmail.example").unwrap().email,
            "me@gmail.example"
        );
        assert!(owner(&config, "link-123").is_err());
    }

    #[test]
    fn reads_dates_and_times() {
        let at = |text| parse_moment(text).unwrap().format("%Y-%m-%d %H:%M").to_string();
        assert_eq!(at("2026-10-08T15:00"), "2026-10-08 15:00");
        assert_eq!(at("2026-10-08 15:30:00"), "2026-10-08 15:30");
        assert_eq!(at("2026-10-08"), "2026-10-08 00:00");
        assert!(parse_moment("tomorrow at 3").is_err());
        let today = NaiveDate::from_ymd_opt(2026, 10, 7).unwrap();
        assert_eq!(day(Some("tomorrow"), today).unwrap(), today.succ_opt());
        assert_eq!(day(None, today).unwrap(), None);
    }

    #[test]
    fn writes_one_line_per_event_with_its_address() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let event = Event {
            account: "me@work.example".into(),
            calendar_id: "me@work.example".into(),
            id: "abc".into(),
            ical_uid: String::new(),
            title: "Call with Ana".into(),
            start: EventTime::At(day.and_hms_opt(15, 0, 0).unwrap()),
            end: EventTime::At(day.and_hms_opt(15, 30, 0).unwrap()),
            location: String::new(),
            description: String::new(),
            join_link: Some("https://meet.google.com/x".into()),
            html_link: None,
        };
        assert_eq!(
            event_line(&event, Some("Work")),
            "Thu Oct 8 15:00–15:30 · Call with Ana [Work] · join https://meet.google.com/x \
             (calendar: me@work.example, id: abc)"
        );
    }
}
