//! Calendars published as iCalendar feeds (RFC 5545), such as Google Calendar's "secret address in
//! iCal format": [`fetch`] downloads one, [`parse`] reads it, and [`Feed::events`] expands it into
//! [`Event`]s for a range of days, recurring events included.
//!
//! The address of a private feed is a credential (whoever has it reads the calendar), so nothing
//! here puts it in an error message.

use crate::model::{Event, EventTime, is_web_link};
use crate::url::{decode, encode};
use crate::{Error, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use chrono::{DateTime, Days, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);
/// Google feeds of calendars used for years are several megabytes; this only stops runaway answers.
const MAX_BYTES: u64 = 64 * 1024 * 1024;
/// Occurrences of one recurring event within the requested range.
const MAX_OCCURRENCES: u16 = 2000;

/// A downloaded calendar.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Feed {
    /// `X-WR-CALNAME`: Google puts the calendar's name there.
    pub name: Option<String>,
    /// `X-APPLE-CALENDAR-COLOR` as `#rrggbb`; Google feeds have none.
    pub color: Option<String>,
    /// The calendar id when the address is Google's (`…/calendar/ical/<id>/…`), which is the
    /// account's email for a primary calendar. Used to drop invitations the owner declined and to
    /// link events to Google Calendar's web UI.
    pub owner: Option<String>,
    entries: Vec<Entry>,
}

/// Downloads and parses the feed at `url` (`https://`, `http://` or `webcal://`).
pub fn fetch(url: &str) -> Result<Feed> {
    let address = web_address(url)?;
    let response = ureq::get(&address).timeout(TIMEOUT).call().map_err(|err| match err {
        ureq::Error::Status(404 | 410, _) => Error::Link(
            "the calendar was not found at this link; copy its secret address again (it changes when it is reset)"
                .into(),
        ),
        ureq::Error::Status(401 | 403, _) => {
            Error::Link("the calendar refused this link; it may have been reset or made private".into())
        }
        ureq::Error::Status(status, _) => Error::Link(format!("the calendar link answered {status}")),
        // Transport errors print the address, so only the kind of failure is kept.
        ureq::Error::Transport(transport) => {
            Error::Link(format!("could not reach the calendar: {}", transport.kind()))
        },
    })?;
    let mut text = String::new();
    response
        .into_reader()
        .take(MAX_BYTES)
        .read_to_string(&mut text)
        .map_err(|err| Error::Link(format!("could not read the calendar: {}", err.kind())))?;
    if !text.trim_start().starts_with("BEGIN:VCALENDAR") {
        return Err(Error::Link(
            "this link is not an iCal calendar; copy the address that ends in .ics".into(),
        ));
    }
    let mut feed = parse(&text);
    feed.owner = google_calendar_id(&address);
    Ok(feed)
}

/// `webcal://` is `https://`; anything else that is not a web address is refused.
fn web_address(url: &str) -> Result<String> {
    let url = url.trim();
    if let Some(rest) = url.strip_prefix("webcal://") {
        return Ok(format!("https://{rest}"));
    }
    if url.starts_with("https://") || url.starts_with("http://") {
        // Google's "Integrate calendar" section lists three addresses; only the secret one works for
        // a calendar that is not public, and the first one is a web page, not a feed.
        if url.contains("calendar.google.com") && url.contains("/embed") {
            return Err(Error::Link(
                "this is the link to embed the calendar in a web page; copy the \"Secret address in iCal format\" instead (it ends in basic.ics)".into(),
            ));
        }
        if url.contains("calendar.google.com") && url.contains("/public/basic.ics") {
            return Err(Error::Link(
                "this is the public address, which works only for public calendars; copy the \"Secret address in iCal format\" instead (it has private- in it)".into(),
            ));
        }
        return Ok(url.to_owned());
    }
    Err(Error::Link(
        "paste a web address that starts with https://".into(),
    ))
}

/// The calendar id in a Google feed address: `https://calendar.google.com/calendar/ical/<id>/…`.
fn google_calendar_id(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let (host, path) = rest.split_once('/')?;
    if !host.ends_with("google.com") {
        return None;
    }
    let mut segments = path.split('/');
    segments.find(|segment| *segment == "ical")?;
    Some(decode(segments.next()?)).filter(|id| !id.is_empty())
}

/// The address with its secret hidden, for showing in the UI: `calendar.google.com/…/basic.ics`.
pub fn masked(url: &str) -> String {
    let url = url.trim();
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    match rest.split_once('/') {
        Some((host, path)) => {
            let last = path.rsplit('/').next().unwrap_or_default();
            if last.is_empty() || last.len() > 24 {
                format!("{host}/…")
            } else {
                format!("{host}/…/{last}")
            }
        }
        None => rest.to_owned(),
    }
}

// ---- parsing -------------------------------------------------------------------------------------

/// One VEVENT, as written in the feed.
#[derive(Clone, Debug, Default, PartialEq)]
struct Entry {
    uid: String,
    summary: String,
    description: String,
    location: String,
    url: Option<String>,
    conference: Option<String>,
    cancelled: bool,
    start: Option<Moment>,
    end: Option<Moment>,
    duration: Option<chrono::Duration>,
    rrule: Option<String>,
    rdates: Vec<Moment>,
    exdates: Vec<Moment>,
    /// Set on an override: the start of the occurrence it replaces.
    recurrence_id: Option<Moment>,
    /// Lowercase email and `PARTSTAT` of each attendee.
    attendees: Vec<(String, String)>,
}

/// A DATE or DATE-TIME value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Moment {
    Date(NaiveDate),
    Utc(NaiveDateTime),
    /// No time zone: the same wall-clock time wherever the reader is.
    Floating(NaiveDateTime),
    Zoned(NaiveDateTime, chrono_tz::Tz),
}

/// Reads a VCALENDAR. Unknown properties and components are skipped; a VEVENT without a start is
/// dropped.
pub fn parse(text: &str) -> Feed {
    let mut feed = Feed::default();
    // Nested components (VALARM inside VEVENT, VTIMEZONE) by name; properties belong to the innermost.
    let mut stack: Vec<String> = Vec::new();
    let mut entry = Entry::default();
    for line in unfold(text) {
        let Some(property) = Property::parse(&line) else {
            continue;
        };
        match property.name.as_str() {
            "BEGIN" => {
                let component = property.value.to_ascii_uppercase();
                if component == "VEVENT" {
                    entry = Entry::default();
                }
                stack.push(component);
                continue;
            }
            "END" => {
                if stack.pop().as_deref() == Some("VEVENT") && entry.start.is_some() {
                    feed.entries.push(std::mem::take(&mut entry));
                }
                continue;
            }
            _ => {}
        }
        match stack.last().map(String::as_str) {
            Some("VCALENDAR") => match property.name.as_str() {
                "X-WR-CALNAME" => {
                    feed.name = Some(unescape(&property.value)).filter(|n| !n.trim().is_empty())
                }
                "X-APPLE-CALENDAR-COLOR" => feed.color = hex_color(&property.value),
                _ => {}
            },
            Some("VEVENT") => entry.read(&property),
            _ => {}
        }
    }
    feed
}

impl Entry {
    fn read(&mut self, property: &Property) {
        let value = property.value.as_str();
        match property.name.as_str() {
            "UID" => self.uid = value.trim().to_owned(),
            "SUMMARY" => self.summary = unescape(value),
            "DESCRIPTION" => self.description = unescape(value),
            "LOCATION" => self.location = unescape(value),
            "URL" => self.url = Some(value.trim().to_owned()).filter(|url| !url.is_empty()),
            "X-GOOGLE-CONFERENCE" => {
                self.conference = Some(value.trim().to_owned()).filter(|u| !u.is_empty())
            }
            "STATUS" => self.cancelled = value.trim().eq_ignore_ascii_case("CANCELLED"),
            "DTSTART" => self.start = property.moments().into_iter().next(),
            "DTEND" => self.end = property.moments().into_iter().next(),
            "DURATION" => self.duration = parse_duration(value),
            "RRULE" => self.rrule = Some(value.trim().to_owned()),
            "RDATE" => self.rdates.extend(property.moments()),
            "EXDATE" => self.exdates.extend(property.moments()),
            "RECURRENCE-ID" => self.recurrence_id = property.moments().into_iter().next(),
            "ATTENDEE" => {
                let email = value.trim();
                let email = email
                    .get(..7)
                    .filter(|scheme| scheme.eq_ignore_ascii_case("mailto:"))
                    .map_or(email, |_| &email[7..]);
                let status = property.param("PARTSTAT").unwrap_or("NEEDS-ACTION");
                self.attendees
                    .push((email.to_ascii_lowercase(), status.to_ascii_uppercase()));
            }
            _ => {}
        }
    }
}

/// Joins folded lines (a line break followed by a space or tab continues the previous line).
fn unfold(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        match raw.strip_prefix([' ', '\t']) {
            Some(continuation) if !lines.is_empty() => {
                if let Some(last) = lines.last_mut() {
                    last.push_str(continuation);
                }
            }
            _ if raw.is_empty() => {}
            _ => lines.push(raw.to_owned()),
        }
    }
    lines
}

/// `NAME;PARAM=value;PARAM="quoted":value`
#[derive(Debug)]
struct Property {
    name: String,
    params: Vec<(String, String)>,
    value: String,
}

impl Property {
    fn parse(line: &str) -> Option<Self> {
        // The value starts at the first ':' outside double quotes.
        let mut quoted = false;
        let colon = line.char_indices().find_map(|(i, c)| {
            match c {
                '"' => quoted = !quoted,
                ':' if !quoted => return Some(i),
                _ => {}
            }
            None
        })?;
        let (head, value) = (&line[..colon], &line[colon + 1..]);
        let mut parts = split_unquoted(head, ';').into_iter();
        let name = parts.next()?.trim().to_ascii_uppercase();
        let params = parts
            .filter_map(|param| {
                let (key, value) = param.split_once('=')?;
                Some((
                    key.trim().to_ascii_uppercase(),
                    value.trim().trim_matches('"').to_owned(),
                ))
            })
            .collect();
        Some(Self {
            name,
            params,
            value: value.to_owned(),
        })
    }

    fn param(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, value)| value.as_str())
    }

    /// The DATE or DATE-TIME values of this property (EXDATE and RDATE may list several).
    fn moments(&self) -> Vec<Moment> {
        let is_date = self
            .param("VALUE")
            .is_some_and(|value| value.eq_ignore_ascii_case("DATE"));
        let zone = self.param("TZID").and_then(time_zone);
        self.value
            .split(',')
            .filter_map(|value| parse_moment(value.trim(), is_date, zone))
            .collect()
    }
}

fn split_unquoted(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut quoted = false;
    let mut start = 0;
    for (i, c) in text.char_indices() {
        if c == '"' {
            quoted = !quoted;
        } else if c == separator && !quoted {
            parts.push(&text[start..i]);
            start = i + 1;
        }
    }
    parts.push(&text[start..]);
    parts
}

/// An IANA name like `America/Sao_Paulo`. Some producers prefix it (`/mozilla.org/.../Europe/Berlin`);
/// Windows names (`E. South America Standard Time`) are not known and fall back to floating time.
fn time_zone(name: &str) -> Option<chrono_tz::Tz> {
    let name = name.trim();
    if let Ok(zone) = name.parse() {
        return Some(zone);
    }
    // Try the last two or three path segments.
    let segments: Vec<&str> = name.split('/').filter(|s| !s.is_empty()).collect();
    (2..=3)
        .rev()
        .filter(|n| segments.len() >= *n)
        .find_map(|n| segments[segments.len() - n..].join("/").parse().ok())
}

fn parse_moment(value: &str, is_date: bool, zone: Option<chrono_tz::Tz>) -> Option<Moment> {
    if is_date || value.len() == 8 {
        return NaiveDate::parse_from_str(value.get(..8)?, "%Y%m%d")
            .ok()
            .map(Moment::Date);
    }
    let (value, utc) = match value.strip_suffix(['Z', 'z']) {
        Some(value) => (value, true),
        None => (value, false),
    };
    let at = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").ok()?;
    Some(match (utc, zone) {
        (true, _) => Moment::Utc(at),
        (false, Some(zone)) => Moment::Zoned(at, zone),
        (false, None) => Moment::Floating(at),
    })
}

/// `P1D`, `PT1H30M`, `-PT15M`, `P2W`.
fn parse_duration(value: &str) -> Option<chrono::Duration> {
    let value = value.trim();
    let (negative, value) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let value = value.strip_prefix(['P', 'p'])?;
    let mut seconds: i64 = 0;
    let mut number = String::new();
    for c in value.chars() {
        match c.to_ascii_uppercase() {
            '0'..='9' => number.push(c),
            'T' => {}
            unit => {
                let n: i64 = number.parse().ok()?;
                number.clear();
                seconds += n * match unit {
                    'W' => 7 * 86_400,
                    'D' => 86_400,
                    'H' => 3_600,
                    'M' => 60,
                    'S' => 1,
                    _ => return None,
                };
            }
        }
    }
    let duration = chrono::Duration::seconds(seconds);
    Some(if negative { -duration } else { duration })
}

/// TEXT escapes: `\n`, `\,`, `\;`, `\\`.
fn unescape(value: &str) -> String {
    let mut text = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            text.push(c);
            continue;
        }
        match chars.next() {
            Some('n' | 'N') => text.push('\n'),
            Some(other) => text.push(other),
            None => text.push('\\'),
        }
    }
    text
}

fn hex_color(value: &str) -> Option<String> {
    let hex = value.trim().strip_prefix('#')?;
    let hex = hex.get(..6)?;
    hex.chars()
        .all(|c| c.is_ascii_hexdigit())
        .then(|| format!("#{}", hex.to_ascii_lowercase()))
}

// ---- events --------------------------------------------------------------------------------------

impl Moment {
    /// The same moment for comparisons: UTC for zoned times, the wall clock for floating ones.
    fn key(self) -> NaiveDateTime {
        match self {
            Self::Date(date) => date.and_time(NaiveTime::MIN),
            Self::Utc(at) | Self::Floating(at) => at,
            Self::Zoned(at, zone) => zoned(zone, at).naive_utc(),
        }
    }

    fn event_time<Z: TimeZone>(self, local: &Z) -> EventTime {
        match self {
            Self::Date(date) => EventTime::Date(date),
            Self::Floating(at) => EventTime::At(at),
            Self::Utc(at) => EventTime::At(at.and_utc().with_timezone(local).naive_local()),
            Self::Zoned(at, zone) => EventTime::At(zoned(zone, at).with_timezone(local).naive_local()),
        }
    }

    /// The moment `duration` later, in the same kind of time.
    fn plus(self, duration: chrono::Duration) -> Self {
        match self {
            Self::Date(date) => {
                Self::Date(date + Days::new(u64::try_from(duration.num_days()).unwrap_or(0).max(1)))
            }
            Self::Utc(at) => Self::Utc(at + duration),
            Self::Floating(at) => Self::Floating(at + duration),
            // Added to the instant, so a meeting across a DST change keeps its length.
            Self::Zoned(at, zone) => Self::Zoned((zoned(zone, at) + duration).naive_local(), zone),
        }
    }

    /// `20261007T130000Z` or `20261007`, the suffix Google adds to the id of an occurrence.
    fn google_suffix(self) -> String {
        match self {
            Self::Date(date) => date.format("%Y%m%d").to_string(),
            other => other.key().format("%Y%m%dT%H%M%SZ").to_string(),
        }
    }
}

/// A wall-clock time in `zone`; a time skipped by a DST change moves forward an hour, like Google does.
fn zoned(zone: chrono_tz::Tz, at: NaiveDateTime) -> DateTime<chrono_tz::Tz> {
    zone.from_local_datetime(&at)
        .earliest()
        .or_else(|| {
            zone.from_local_datetime(&(at + chrono::Duration::hours(1)))
                .earliest()
        })
        .unwrap_or_else(|| zone.from_utc_datetime(&at))
}

impl Entry {
    /// From the start to DTEND, or DURATION, or the default length (a day for dates, none for times).
    fn length(&self, start: Moment) -> chrono::Duration {
        let length = match (self.end, self.duration) {
            (Some(end), _) => end.key() - start.key(),
            (None, Some(duration)) => duration,
            (None, None) if matches!(start, Moment::Date(_)) => chrono::Duration::days(1),
            (None, None) => chrono::Duration::zero(),
        };
        length.max(chrono::Duration::zero())
    }

    fn declined_by(&self, owner: Option<&str>) -> bool {
        // Shared calendars (`…@group.calendar.google.com`) are often listed as an attendee that
        // "declined" every meeting they hold, which Google Calendar ignores; only people decline.
        let Some(owner) = owner.filter(|owner| !owner.ends_with(".calendar.google.com")) else {
            return false;
        };
        self.attendees
            .iter()
            .any(|(email, status)| email.eq_ignore_ascii_case(owner) && status == "DECLINED")
    }
}

impl Feed {
    /// Events touching the days `from..to` (end exclusive) in the computer's time zone, for the
    /// calendar `calendar_id`. Recurring events are expanded within the range only.
    pub fn events(&self, calendar_id: &str, from: NaiveDate, to: NaiveDate) -> Vec<Event> {
        self.events_in(&Local, calendar_id, from, to)
    }

    fn events_in<Z: TimeZone>(
        &self,
        local: &Z,
        calendar_id: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Vec<Event> {
        // Starts of the occurrences each override replaces, by UID.
        let mut overridden: HashMap<&str, HashSet<NaiveDateTime>> = HashMap::new();
        for entry in &self.entries {
            if let Some(id) = entry.recurrence_id {
                overridden.entry(&entry.uid).or_default().insert(id.key());
            }
        }
        let mut events = Vec::new();
        for entry in &self.entries {
            if entry.cancelled || entry.declined_by(self.owner.as_deref()) {
                continue;
            }
            let Some(start) = entry.start else {
                continue;
            };
            let length = entry.length(start);
            let starts = if entry.rrule.is_some() && entry.recurrence_id.is_none() {
                let skip: HashSet<NaiveDateTime> = entry
                    .exdates
                    .iter()
                    .map(|m| m.key())
                    .chain(overridden.get(entry.uid.as_str()).into_iter().flatten().copied())
                    .collect();
                let mut starts = occurrences(entry, start, length, from, to);
                starts.extend(entry.rdates.iter().copied());
                starts.sort_by_key(|m| m.key());
                starts.dedup();
                starts.retain(|m| !skip.contains(&m.key()));
                starts
            } else {
                vec![start]
            };
            for occurrence in starts {
                let event = self.event(local, calendar_id, entry, occurrence, length);
                let (first, last) = event.first_and_last_day();
                if first < to && last >= from {
                    events.push(event);
                }
            }
        }
        events
    }

    fn event<Z: TimeZone>(
        &self,
        local: &Z,
        calendar_id: &str,
        entry: &Entry,
        start: Moment,
        length: chrono::Duration,
    ) -> Event {
        // Google names an occurrence `<event id>_<original start>`; the UID is `<event id>@google.com`.
        let base = entry.uid.strip_suffix("@google.com").unwrap_or(&entry.uid);
        let id = match (entry.recurrence_id, entry.rrule.is_some()) {
            (Some(original), _) => format!("{base}_{}", original.google_suffix()),
            (None, true) => format!("{base}_{}", start.google_suffix()),
            (None, false) => base.to_owned(),
        };
        let html_link = entry.url.clone().or_else(|| {
            let owner = self.owner.as_deref()?;
            entry.uid.ends_with("@google.com").then(|| {
                let eid = STANDARD_NO_PAD.encode(format!("{id} {owner}"));
                format!("https://www.google.com/calendar/event?eid={}", encode(&eid))
            })
        });
        let description = strip_conference_block(&entry.description);
        Event {
            account: String::new(),
            calendar_id: calendar_id.to_owned(),
            id,
            ical_uid: entry.uid.clone(),
            title: Some(entry.summary.trim())
                .filter(|title| !title.is_empty())
                .unwrap_or("(No title)")
                .to_owned(),
            start: start.event_time(local),
            end: start.plus(length).event_time(local),
            location: entry.location.clone(),
            join_link: entry
                .conference
                .clone()
                .or_else(|| meeting_link(&entry.description))
                .or_else(|| meeting_link(&entry.location))
                .filter(|link| is_web_link(link)),
            description,
            html_link: html_link.filter(|link| is_web_link(link)),
        }
    }
}

/// Starts of the occurrences of a recurring entry that can touch `from..to`.
fn occurrences(
    entry: &Entry,
    start: Moment,
    length: chrono::Duration,
    from: NaiveDate,
    to: NaiveDate,
) -> Vec<Moment> {
    use rrule::{RRule, Tz, Unvalidated};

    let Some(rule) = entry.rrule.as_deref() else {
        return vec![start];
    };
    // rrule needs a zone. Dates and floating times are expanded as if they were UTC wall-clock
    // times, which keeps them on the same date and hour in every occurrence.
    let (zone, wall) = match start {
        Moment::Date(date) => (Tz::UTC, date.and_time(NaiveTime::MIN)),
        Moment::Utc(at) | Moment::Floating(at) => (Tz::UTC, at),
        Moment::Zoned(at, zone) => (Tz::Tz(zone), zoned(zone, at).naive_local()),
    };
    let Some(dt_start) = zone.from_local_datetime(&wall).earliest() else {
        return vec![start];
    };
    // UNTIL is set by hand: rrule wants it in UTC when DTSTART has a zone, and reads a date-only
    // UNTIL (Google's all-day rules) in the computer's zone, which it then rejects.
    let mut until = None;
    let parts: Vec<&str> = rule
        .split(';')
        .filter(|part| match part.split_once('=') {
            Some((key, value)) if key.trim().eq_ignore_ascii_case("UNTIL") => {
                until = parse_moment(value.trim(), false, None);
                false
            }
            _ => !part.trim().is_empty(),
        })
        .collect();
    let Ok(mut rule) = parts.join(";").parse::<RRule<Unvalidated>>() else {
        return vec![start];
    };
    if let Some(until) = until {
        let last = match (until, start) {
            // A date-only UNTIL includes that whole day.
            (Moment::Date(date), Moment::Zoned(_, zone)) => {
                zoned(zone, date.and_hms_opt(23, 59, 59).unwrap_or_default()).naive_utc()
            }
            (Moment::Date(date), _) => date.and_hms_opt(23, 59, 59).unwrap_or_default(),
            (Moment::Utc(at), Moment::Date(_)) => at.date().and_hms_opt(23, 59, 59).unwrap_or_default(),
            (Moment::Utc(at), Moment::Floating(_)) => at.and_utc().with_timezone(&Local).naive_local(),
            (Moment::Floating(at), Moment::Zoned(_, zone)) => zoned(zone, at).naive_utc(),
            (other, _) => other.key(),
        };
        rule = rule.until(Tz::UTC.from_utc_datetime(&last));
    }
    let Ok(set) = rule.build(dt_start) else {
        return vec![start];
    };
    // Two days of slack on each side covers any zone offset; the caller filters exactly.
    let after = from.and_time(NaiveTime::MIN) - length - chrono::Duration::days(2);
    let before = to.and_time(NaiveTime::MIN) + chrono::Duration::days(2);
    set.after(Tz::UTC.from_utc_datetime(&after))
        .before(Tz::UTC.from_utc_datetime(&before))
        .all(MAX_OCCURRENCES)
        .dates
        .into_iter()
        .map(|at| match start {
            Moment::Date(_) => Moment::Date(at.naive_utc().date()),
            Moment::Utc(_) => Moment::Utc(at.naive_utc()),
            Moment::Floating(_) => Moment::Floating(at.naive_utc()),
            Moment::Zoned(_, zone) => Moment::Zoned(at.with_timezone(&zone).naive_local(), zone),
        })
        .collect()
}

/// Google appends a block about the Meet call between `-::~:~::~` lines; the Join button covers it.
fn strip_conference_block(description: &str) -> String {
    const MARK: &str = "-::~:~::~";
    let Some(start) = description.find(MARK) else {
        return description.trim().to_owned();
    };
    let end = description
        .rfind("::-")
        .filter(|end| *end > start)
        .map_or(description.len(), |end| end + 3);
    let mut text = description[..start].trim_end().to_owned();
    let rest = description[end..].trim();
    if !rest.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(rest);
    }
    text.trim().to_owned()
}

/// The first video call link in free text (Meet, Zoom, Teams, Webex).
fn meeting_link(text: &str) -> Option<String> {
    const HOSTS: [&str; 5] = [
        "meet.google.com/",
        "zoom.us/j/",
        "zoom.us/my/",
        "teams.microsoft.com/l/meetup-join",
        "webex.com/",
    ];
    text.split(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '(' | ')'))
        .filter(|word| word.starts_with("https://"))
        .map(|word| word.trim_end_matches(['.', ',', ';']))
        .find(|word| HOSTS.iter().any(|host| word.contains(host)))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests;
