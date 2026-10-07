//! The two Google Calendar API calls Falog needs: the calendar list and events in a time range.

use crate::Result;
use crate::model::{Calendar, Event, EventTime};
use crate::url::encode;
use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone};
use serde_json::Value;
use std::time::Duration;

const API: &str = "https://www.googleapis.com/calendar/v3";
const TIMEOUT: Duration = Duration::from_secs(30);

/// The account's calendars; the primary one comes first, the rest alphabetically.
pub fn calendars(token: &str) -> Result<Vec<Calendar>> {
    let mut calendars = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let mut request = get(&format!("{API}/users/me/calendarList"), token).query("maxResults", "250");
        if let Some(page) = &page {
            request = request.query("pageToken", page);
        }
        let json: Value = serde_json::from_str(&request.call()?.into_string()?)?;
        calendars.extend(parse_calendars(&json));
        match json.get("nextPageToken").and_then(Value::as_str) {
            Some(next) => page = Some(next.to_owned()),
            None => break,
        }
    }
    calendars.sort_by_key(|c| (!c.primary, c.name.to_lowercase()));
    Ok(calendars)
}

/// Email of the account, which is the id of its primary calendar.
pub fn account_email(calendars: &[Calendar]) -> Option<&str> {
    calendars.iter().find(|c| c.primary).map(|c| c.id.as_str())
}

/// Events of one calendar touching the days `from..to` (end exclusive), recurring events expanded.
pub fn events(
    token: &str,
    account: &str,
    calendar_id: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<Event>> {
    let url = format!("{API}/calendars/{}/events", encode(calendar_id));
    let mut events = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let mut request = get(&url, token)
            .query("timeMin", &rfc3339(from))
            .query("timeMax", &rfc3339(to))
            .query("singleEvents", "true")
            .query("orderBy", "startTime")
            .query("maxResults", "2500");
        if let Some(page) = &page {
            request = request.query("pageToken", page);
        }
        let json: Value = serde_json::from_str(&request.call()?.into_string()?)?;
        events.extend(parse_events(&json, account, calendar_id));
        match json.get("nextPageToken").and_then(Value::as_str) {
            Some(next) => page = Some(next.to_owned()),
            None => break,
        }
    }
    Ok(events)
}

fn get(url: &str, token: &str) -> ureq::Request {
    ureq::get(url)
        .timeout(TIMEOUT)
        .set("Authorization", &format!("Bearer {token}"))
}

/// Local midnight of `day` in RFC 3339, as the API wants it.
fn rfc3339(day: NaiveDate) -> String {
    let midnight = day.and_time(NaiveTime::MIN);
    Local
        .from_local_datetime(&midnight)
        .earliest()
        .map_or_else(|| midnight.and_utc().to_rfc3339(), |at| at.to_rfc3339())
}

fn text(json: &Value, key: &str) -> String {
    json.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn parse_calendars(json: &Value) -> Vec<Calendar> {
    let Some(items) = json.get("items").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter(|item| !item.get("deleted").and_then(Value::as_bool).unwrap_or(false))
        .map(|item| {
            let name = Some(text(item, "summaryOverride"))
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| text(item, "summary"));
            Calendar {
                id: text(item, "id"),
                name,
                color: text(item, "backgroundColor"),
                primary: item.get("primary").and_then(Value::as_bool).unwrap_or(false),
                visible: item.get("selected").and_then(Value::as_bool).unwrap_or(false),
            }
        })
        .collect()
}

fn parse_events(json: &Value, account: &str, calendar_id: &str) -> Vec<Event> {
    let Some(items) = json.get("items").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter(|item| text(item, "status") != "cancelled")
        // Working-location entries ("Home", "Office") are status, not meetings.
        .filter(|item| text(item, "eventType") != "workingLocation")
        .filter(|item| !declined(item))
        .filter_map(|item| {
            Some(Event {
                account: account.to_owned(),
                calendar_id: calendar_id.to_owned(),
                id: text(item, "id"),
                ical_uid: text(item, "iCalUID"),
                title: Some(text(item, "summary"))
                    .filter(|title| !title.trim().is_empty())
                    .unwrap_or_else(|| "(No title)".to_owned()),
                start: event_time(item.get("start")?)?,
                end: event_time(item.get("end")?)?,
                location: text(item, "location"),
                description: text(item, "description"),
                join_link: join_link(item),
                html_link: Some(text(item, "htmlLink")).filter(|link| !link.is_empty()),
            })
        })
        .collect()
}

/// The user said no to this invitation.
fn declined(item: &Value) -> bool {
    item.get("attendees")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|a| {
            a.get("self").and_then(Value::as_bool) == Some(true) && text(a, "responseStatus") == "declined"
        })
}

fn event_time(json: &Value) -> Option<EventTime> {
    if let Some(at) = json.get("dateTime").and_then(Value::as_str) {
        let at = DateTime::parse_from_rfc3339(at).ok()?;
        return Some(EventTime::At(at.with_timezone(&Local).naive_local()));
    }
    let date = json.get("date").and_then(Value::as_str)?;
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .ok()
        .map(EventTime::Date)
}

/// Meet link, or the first video entry point of another conference provider.
fn join_link(item: &Value) -> Option<String> {
    if let Some(link) = item.get("hangoutLink").and_then(Value::as_str) {
        return Some(link.to_owned());
    }
    item.get("conferenceData")?
        .get("entryPoints")?
        .as_array()?
        .iter()
        .find(|entry| text(entry, "entryPointType") == "video")
        .map(|entry| text(entry, "uri"))
        .filter(|uri| !uri.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_calendars() {
        let json = json!({ "items": [
            { "id": "me@acme.com", "summary": "me@acme.com", "summaryOverride": "Acme",
              "backgroundColor": "#9fe1e7", "primary": true, "selected": true },
            { "id": "holidays", "summary": "Holidays in Brazil", "backgroundColor": "#16a765" },
            { "id": "gone", "summary": "Old", "deleted": true },
        ]});
        let calendars = parse_calendars(&json);
        assert_eq!(calendars.len(), 2);
        assert_eq!(calendars[0].name, "Acme");
        assert!(calendars[0].primary && calendars[0].visible);
        assert!(!calendars[1].visible);
        assert_eq!(account_email(&calendars), Some("me@acme.com"));
    }

    #[test]
    fn parses_events() {
        let json = json!({ "items": [
            { "id": "a", "iCalUID": "a@google.com", "summary": "Standup", "status": "confirmed",
              "start": { "dateTime": "2026-10-07T10:00:00-03:00" },
              "end": { "dateTime": "2026-10-07T10:15:00-03:00" },
              "hangoutLink": "https://meet.google.com/abc-defg-hij",
              "htmlLink": "https://www.google.com/calendar/event?eid=a" },
            { "id": "b", "start": { "date": "2026-10-09" }, "end": { "date": "2026-10-10" },
              "conferenceData": { "entryPoints": [
                  { "entryPointType": "phone", "uri": "tel:+1" },
                  { "entryPointType": "video", "uri": "https://zoom.us/j/1" } ] } },
            { "id": "c", "status": "cancelled", "start": { "date": "2026-10-09" }, "end": { "date": "2026-10-10" } },
            { "id": "d", "eventType": "workingLocation", "start": { "date": "2026-10-09" }, "end": { "date": "2026-10-10" } },
            { "id": "e", "summary": "Declined", "attendees": [ { "self": true, "responseStatus": "declined" } ],
              "start": { "date": "2026-10-09" }, "end": { "date": "2026-10-10" } },
        ]});
        let events = parse_events(&json, "me@acme.com", "primary");
        assert_eq!(events.len(), 2);

        let standup = &events[0];
        assert_eq!(standup.title, "Standup");
        assert_eq!(
            standup.join_link.as_deref(),
            Some("https://meet.google.com/abc-defg-hij")
        );
        let expected = DateTime::parse_from_rfc3339("2026-10-07T10:00:00-03:00")
            .unwrap()
            .with_timezone(&Local)
            .naive_local();
        assert_eq!(standup.start, EventTime::At(expected));

        let untitled = &events[1];
        assert_eq!(untitled.title, "(No title)");
        assert!(untitled.is_all_day());
        assert_eq!(untitled.join_link.as_deref(), Some("https://zoom.us/j/1"));
        assert_eq!(untitled.html_link, None);
    }

    #[test]
    fn asks_for_local_midnight() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 7).unwrap();
        let text = rfc3339(day);
        assert!(text.starts_with("2026-10-07T00:00:00"), "{text}");
    }
}
