//! The Google Calendar API calls Falog makes: the calendar list, events in a time range, and creating,
//! changing or deleting an event.

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

/// An event to create. Times are local; all-day events use [`EventTime::Date`] with an exclusive end.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewEvent {
    pub title: String,
    pub start: Option<EventTime>,
    pub end: Option<EventTime>,
    pub description: String,
    pub location: String,
    /// Guests' emails; Google emails them the invitation.
    pub attendees: Vec<String>,
    /// Ask Google to attach a Meet link.
    pub meet: bool,
}

/// Fields to change on an event; `None` keeps the current value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventPatch {
    pub title: Option<String>,
    pub start: Option<EventTime>,
    pub end: Option<EventTime>,
    pub description: Option<String>,
    pub location: Option<String>,
}

pub fn insert_event(token: &str, account: &str, calendar_id: &str, event: &NewEvent) -> Result<Event> {
    let url = format!("{API}/calendars/{}/events", encode(calendar_id));
    let mut request = authorized(ureq::post(&url), token).query("conferenceDataVersion", "1");
    if !event.attendees.is_empty() {
        request = request.query("sendUpdates", "all");
    }
    let body = send_json(request, &insert_body(event))?.into_string()?;
    returned_event(&body, account, calendar_id)
}

pub fn patch_event(
    token: &str,
    account: &str,
    calendar_id: &str,
    event_id: &str,
    patch: &EventPatch,
) -> Result<Event> {
    let url = format!(
        "{API}/calendars/{}/events/{}",
        encode(calendar_id),
        encode(event_id)
    );
    let request = authorized(ureq::request("PATCH", &url), token).query("sendUpdates", "all");
    let body = send_json(request, &patch_body(patch))?.into_string()?;
    returned_event(&body, account, calendar_id)
}

pub fn delete_event(token: &str, calendar_id: &str, event_id: &str) -> Result<()> {
    let url = format!(
        "{API}/calendars/{}/events/{}",
        encode(calendar_id),
        encode(event_id)
    );
    authorized(ureq::delete(&url), token)
        .query("sendUpdates", "all")
        .call()?;
    Ok(())
}

fn authorized(request: ureq::Request, token: &str) -> ureq::Request {
    request
        .timeout(TIMEOUT)
        .set("Authorization", &format!("Bearer {token}"))
}

/// `ureq` 2 sends JSON only with its `json` feature; a string body does the same.
fn send_json(request: ureq::Request, body: &Value) -> Result<ureq::Response> {
    Ok(request
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())?)
}

fn returned_event(body: &str, account: &str, calendar_id: &str) -> Result<Event> {
    let json: Value = serde_json::from_str(body)?;
    parse_event(&json, account, calendar_id)
        .ok_or_else(|| crate::Error::Parse("Google returned an event without times".into()))
}

fn time_json(time: EventTime) -> Value {
    match time {
        EventTime::Date(date) => serde_json::json!({ "date": date.format("%Y-%m-%d").to_string() }),
        EventTime::At(at) => {
            let local = Local
                .from_local_datetime(&at)
                .earliest()
                .map_or_else(|| at.and_utc().to_rfc3339(), |at| at.to_rfc3339());
            serde_json::json!({ "dateTime": local })
        }
    }
}

fn insert_body(event: &NewEvent) -> Value {
    let mut body = serde_json::json!({ "summary": event.title });
    if let Some(start) = event.start {
        body["start"] = time_json(start);
    }
    if let Some(end) = event.end {
        body["end"] = time_json(end);
    }
    if !event.description.is_empty() {
        body["description"] = event.description.clone().into();
    }
    if !event.location.is_empty() {
        body["location"] = event.location.clone().into();
    }
    if !event.attendees.is_empty() {
        body["attendees"] = event
            .attendees
            .iter()
            .map(|email| serde_json::json!({ "email": email }))
            .collect();
    }
    if event.meet {
        let request_id = format!("falog-{}", Local::now().timestamp_millis());
        body["conferenceData"] = serde_json::json!({
            "createRequest": { "requestId": request_id, "conferenceSolutionKey": { "type": "hangoutsMeet" } }
        });
    }
    body
}

fn patch_body(patch: &EventPatch) -> Value {
    let mut body = serde_json::json!({});
    if let Some(title) = &patch.title {
        body["summary"] = title.clone().into();
    }
    if let Some(start) = patch.start {
        body["start"] = time_json(start);
    }
    if let Some(end) = patch.end {
        body["end"] = time_json(end);
    }
    if let Some(description) = &patch.description {
        body["description"] = description.clone().into();
    }
    if let Some(location) = &patch.location {
        body["location"] = location.clone().into();
    }
    body
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
        .filter_map(|item| parse_event(item, account, calendar_id))
        .collect()
}

/// One event resource, as listed or as returned by an insert or a patch.
fn parse_event(item: &Value, account: &str, calendar_id: &str) -> Option<Event> {
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
    fn builds_insert_and_patch_bodies() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
        let start = EventTime::At(day.and_hms_opt(15, 0, 0).unwrap());
        let event = NewEvent {
            title: "Call with Ana".into(),
            start: Some(start),
            end: Some(EventTime::At(day.and_hms_opt(15, 30, 0).unwrap())),
            attendees: vec!["ana@example.com".into()],
            meet: true,
            ..NewEvent::default()
        };
        let body = insert_body(&event);
        assert_eq!(body["summary"], "Call with Ana");
        assert!(
            body["start"]["dateTime"]
                .as_str()
                .unwrap()
                .starts_with("2026-10-08T15:00:00")
        );
        assert_eq!(body["attendees"][0]["email"], "ana@example.com");
        assert_eq!(
            body["conferenceData"]["createRequest"]["conferenceSolutionKey"]["type"],
            "hangoutsMeet"
        );
        assert!(body.get("description").is_none());

        let all_day = insert_body(&NewEvent {
            title: "Offsite".into(),
            start: Some(EventTime::Date(day)),
            end: Some(EventTime::Date(day.succ_opt().unwrap())),
            ..NewEvent::default()
        });
        assert_eq!(all_day["start"], json!({ "date": "2026-10-08" }));
        assert_eq!(all_day["end"], json!({ "date": "2026-10-09" }));

        let patch = patch_body(&EventPatch {
            title: Some("Call with Ana and Leo".into()),
            location: Some(String::new()),
            ..EventPatch::default()
        });
        assert_eq!(
            patch,
            json!({ "summary": "Call with Ana and Leo", "location": "" })
        );
    }

    #[test]
    fn reads_the_event_google_returns() {
        let body = r#"{ "id": "abc", "summary": "Call", "htmlLink": "https://calendar.google.com/x",
            "start": { "dateTime": "2026-10-08T15:00:00-03:00" }, "end": { "dateTime": "2026-10-08T15:30:00-03:00" },
            "hangoutLink": "https://meet.google.com/xyz" }"#;
        let event = returned_event(body, "me@example.com", "primary").unwrap();
        assert_eq!(event.id, "abc");
        assert_eq!(event.join_link.as_deref(), Some("https://meet.google.com/xyz"));
        assert!(returned_event("{}", "me@example.com", "primary").is_err());
    }

    #[test]
    fn asks_for_local_midnight() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 7).unwrap();
        let text = rfc3339(day);
        assert!(text.starts_with("2026-10-07T00:00:00"), "{text}");
    }
}
