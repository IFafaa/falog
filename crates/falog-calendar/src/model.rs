use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

/// One calendar of a connected account.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calendar {
    pub id: String,
    pub name: String,
    /// `#rrggbb`, the color the user picked in Google Calendar.
    pub color: String,
    pub primary: bool,
    /// Shown in Falog. Starts as whatever the calendar's checkbox is in Google Calendar.
    pub visible: bool,
}

impl Calendar {
    /// The calendar color as RGB, or a neutral blue when Google sent something unexpected.
    pub fn rgb(&self) -> [u8; 3] {
        parse_hex(&self.color).unwrap_or([0x61, 0xaf, 0xef])
    }
}

fn parse_hex(color: &str) -> Option<[u8; 3]> {
    let hex = color.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// When an event starts or ends: a whole day, or a local date and time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EventTime {
    /// Google's all-day dates; an all-day event's end date is exclusive.
    Date(NaiveDate),
    /// Converted to the computer's time zone when fetched.
    At(NaiveDateTime),
}

impl EventTime {
    pub fn date(self) -> NaiveDate {
        match self {
            Self::Date(date) => date,
            Self::At(at) => at.date(),
        }
    }

    fn instant(self) -> NaiveDateTime {
        match self {
            Self::Date(date) => date.and_time(NaiveTime::MIN),
            Self::At(at) => at,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Email of the account the event was read from.
    pub account: String,
    pub calendar_id: String,
    pub id: String,
    /// Same for every copy of a meeting, across calendars and accounts.
    pub ical_uid: String,
    pub title: String,
    pub start: EventTime,
    pub end: EventTime,
    pub location: String,
    pub description: String,
    /// Google Meet or another conference link.
    pub join_link: Option<String>,
    /// The event in Google Calendar's web UI.
    pub html_link: Option<String>,
}

impl Event {
    pub fn is_all_day(&self) -> bool {
        matches!(self.start, EventTime::Date(_))
    }

    /// First and last day the event touches. A timed event ending at midnight does not touch the
    /// next day.
    pub fn first_and_last_day(&self) -> (NaiveDate, NaiveDate) {
        let last = match self.end {
            EventTime::Date(end) => end.pred_opt().unwrap_or(end),
            EventTime::At(end) => (end - Duration::nanoseconds(1)).date(),
        };
        let first = self.start.date();
        (first, last.max(first))
    }

    pub fn touches(&self, day: NaiveDate) -> bool {
        let (first, last) = self.first_and_last_day();
        first <= day && day <= last
    }

    /// Google Calendar puts all-day events and timed events longer than a day on top of the week
    /// grid instead of in the hours.
    pub fn in_all_day_row(&self) -> bool {
        let (first, last) = self.first_and_last_day();
        self.is_all_day() || first != last
    }

    /// Start and end minutes within `day`, clamped to the day (for the week grid).
    pub fn minutes_on(&self, day: NaiveDate) -> (u32, u32) {
        let day_start = day.and_time(NaiveTime::MIN);
        let minutes = |at: NaiveDateTime| (at - day_start).num_minutes().clamp(0, 24 * 60) as u32;
        let start = minutes(self.start.instant());
        let end = minutes(self.end.instant()).max(start);
        (start, end)
    }

    /// Sort key: all-day first, then by start, then longest first.
    pub fn sort_key(&self) -> (bool, NaiveDateTime, std::cmp::Reverse<NaiveDateTime>) {
        (
            !self.in_all_day_row(),
            self.start.instant(),
            std::cmp::Reverse(self.end.instant()),
        )
    }
}

/// Sorts events and drops copies of the same meeting seen through several calendars, keeping the
/// first one in `events` order (callers put the calendars they prefer first).
pub fn sort_and_dedupe(events: &mut Vec<Event>) {
    let mut seen = std::collections::HashSet::new();
    events.retain(|event| event.ical_uid.is_empty() || seen.insert((event.ical_uid.clone(), event.start)));
    events.sort_by_key(Event::sort_key);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, d).unwrap()
    }

    fn at(d: u32, h: u32, m: u32) -> EventTime {
        EventTime::At(date(d).and_hms_opt(h, m, 0).unwrap())
    }

    fn event(start: EventTime, end: EventTime) -> Event {
        Event {
            account: "me@example.com".into(),
            calendar_id: "primary".into(),
            id: "1".into(),
            ical_uid: String::new(),
            title: "Standup".into(),
            start,
            end,
            location: String::new(),
            description: String::new(),
            join_link: None,
            html_link: None,
        }
    }

    #[test]
    fn all_day_end_is_exclusive() {
        let trip = event(EventTime::Date(date(7)), EventTime::Date(date(9)));
        assert_eq!(trip.first_and_last_day(), (date(7), date(8)));
        assert!(trip.touches(date(8)));
        assert!(!trip.touches(date(9)));
        assert!(trip.in_all_day_row());
    }

    #[test]
    fn timed_event_ending_at_midnight_stays_on_its_day() {
        let late = event(at(7, 22, 0), at(8, 0, 0));
        assert_eq!(late.first_and_last_day(), (date(7), date(7)));
        assert!(!late.in_all_day_row());
        assert_eq!(late.minutes_on(date(7)), (22 * 60, 24 * 60));
    }

    #[test]
    fn overnight_events_go_to_the_all_day_row_and_clamp_per_day() {
        let flight = event(at(7, 22, 0), at(8, 6, 30));
        assert!(flight.in_all_day_row());
        assert_eq!(flight.minutes_on(date(7)), (22 * 60, 24 * 60));
        assert_eq!(flight.minutes_on(date(8)), (0, 6 * 60 + 30));
    }

    #[test]
    fn dedupes_the_same_meeting_from_two_accounts() {
        let mut work = event(at(7, 10, 0), at(7, 11, 0));
        work.ical_uid = "abc@google.com".into();
        let mut personal = work.clone();
        personal.account = "me@gmail.com".into();
        let mut other = event(at(7, 9, 0), at(7, 9, 30));
        other.ical_uid = "def@google.com".into();
        let mut events = vec![work.clone(), personal, other.clone()];
        sort_and_dedupe(&mut events);
        assert_eq!(events, vec![other, work]);
    }

    #[test]
    fn parses_calendar_colors() {
        let mut calendar = Calendar {
            id: "x".into(),
            name: "Work".into(),
            color: "#9fe1e7".into(),
            primary: true,
            visible: true,
        };
        assert_eq!(calendar.rgb(), [0x9f, 0xe1, 0xe7]);
        calendar.color = "teal".into();
        assert_eq!(calendar.rgb(), [0x61, 0xaf, 0xef]);
    }
}
