use super::*;
use chrono_tz::America::Sao_Paulo;
use std::io::Write;
use std::net::TcpListener;

/// Shaped like Google Calendar's "secret address in iCal format" export: CRLF lines folded at 75
/// octets, a VTIMEZONE, TZID times, an alarm inside an event, Google's Meet block.
const GOOGLE: &str = "BEGIN:VCALENDAR
PRODID:-//Google Inc//Google Calendar 70.9054//EN
VERSION:2.0
CALSCALE:GREGORIAN
METHOD:PUBLISH
X-WR-CALNAME:me@example.com
X-WR-TIMEZONE:America/Sao_Paulo
BEGIN:VTIMEZONE
TZID:America/Sao_Paulo
X-LIC-LOCATION:America/Sao_Paulo
BEGIN:STANDARD
TZOFFSETFROM:-0300
TZOFFSETTO:-0300
TZNAME:-03
DTSTART:19700101T000000
END:STANDARD
END:VTIMEZONE
BEGIN:VEVENT
DTSTART;VALUE=DATE:20261012
DTEND;VALUE=DATE:20261013
DTSTAMP:20261007T122846Z
UID:0a1b2c3d4e5f6g@google.com
CREATED:20260901T101345Z
DESCRIPTION:
LAST-MODIFIED:20260901T101345Z
LOCATION:
SEQUENCE:0
STATUS:CONFIRMED
SUMMARY:Holiday
TRANSP:TRANSPARENT
END:VEVENT
BEGIN:VEVENT
DTSTART;VALUE=DATE:20261014
DTEND;VALUE=DATE:20261017
DTSTAMP:20261007T122846Z
UID:trip0123456789@google.com
LOCATION:Lisbon\\, Portugal
STATUS:CONFIRMED
SUMMARY:Trip
END:VEVENT
BEGIN:VEVENT
DTSTART;TZID=America/New_York:20261007T100000
DTEND;TZID=America/New_York:20261007T110000
DTSTAMP:20261007T122846Z
ORGANIZER;CN=boss@example.com:mailto:boss@example.com
UID:nyc0123456789@google.com
ATTENDEE;CUTYPE=INDIVIDUAL;ROLE=REQ-PARTICIPANT;PARTSTAT=ACCEPTED;CN=me@exam
 ple.com;X-NUM-GUESTS=0:mailto:me@example.com
X-GOOGLE-CONFERENCE:https://meet.google.com/abc-defg-hij
DESCRIPTION:Agenda: roadmap\\; budget\\n\\n-::~:~::~:~:~:~:~:~:~:~:~:~:~:~:~:~:~
 :~:~:~:~:~:~:~:~:~:~:~:~:~:~:~:~::~:~::-\\nJoin with Google Meet: https://meet
 .google.com/abc-defg-hij\\n\\nLearn more about Meet at: https://support.google.
 com/a/users/answer/9282720\\n\\nPlease do not edit this section.\\n-::~:~::~:~:~
 :~:~:~:~:~:~:~::~:~::-
STATUS:CONFIRMED
SUMMARY:Client sync
BEGIN:VALARM
ACTION:DISPLAY
DESCRIPTION:This is an event reminder
TRIGGER:-P0DT0H10M0S
END:VALARM
END:VEVENT
BEGIN:VEVENT
DTSTART:20261008T120000Z
DURATION:PT45M
UID:lunch0123456789@google.com
SUMMARY:Lunch
DESCRIPTION:Zoom: https://us02web.zoom.us/j/8123456789?pwd=abc.
END:VEVENT
BEGIN:VEVENT
DTSTART;TZID=America/Sao_Paulo:20260907T093000
DTEND;TZID=America/Sao_Paulo:20260907T094500
RRULE:FREQ=WEEKLY;WKST=SU;UNTIL=20261231T025959Z;BYDAY=MO,WE
EXDATE;TZID=America/Sao_Paulo:20261007T093000
DTSTAMP:20261007T122846Z
UID:standup0123456789@google.com
SUMMARY:Standup
END:VEVENT
BEGIN:VEVENT
DTSTART;TZID=America/Sao_Paulo:20261013T140000
DTEND;TZID=America/Sao_Paulo:20261013T141500
RECURRENCE-ID;TZID=America/Sao_Paulo:20261012T093000
UID:standup0123456789@google.com
SEQUENCE:1
SUMMARY:Standup (moved)
END:VEVENT
BEGIN:VEVENT
DTSTART;VALUE=DATE:20201010
DTEND;VALUE=DATE:20201011
RRULE:FREQ=YEARLY;UNTIL=20281010
UID:birthday0123456789@google.com
SUMMARY:Ana's birthday
END:VEVENT
BEGIN:VEVENT
DTSTART:20261009T080000
DTEND:20261009T090000
UID:floating@example.com
SUMMARY:Run
END:VEVENT
BEGIN:VEVENT
DTSTART:20261009T150000Z
DTEND:20261009T160000Z
UID:cancelled0123456789@google.com
STATUS:CANCELLED
SUMMARY:Cancelled
END:VEVENT
BEGIN:VEVENT
DTSTART:20261009T170000Z
DTEND:20261009T180000Z
UID:declined0123456789@google.com
ATTENDEE;CUTYPE=INDIVIDUAL;ROLE=REQ-PARTICIPANT;PARTSTAT=DECLINED;CN=me@exam
 ple.com;X-NUM-GUESTS=0:mailto:me@example.com
SUMMARY:Declined
END:VEVENT
END:VCALENDAR
";

fn google_feed() -> Feed {
    let mut feed = parse(&GOOGLE.replace('\n', "\r\n"));
    feed.owner = Some("me@example.com".into());
    feed
}

fn day(m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, m, d).unwrap()
}

fn at(m: u32, d: u32, h: u32, min: u32) -> EventTime {
    EventTime::At(day(m, d).and_hms_opt(h, min, 0).unwrap())
}

/// Two weeks, Monday Oct 5 to Sunday Oct 18, read from São Paulo (UTC-3, no DST).
fn two_weeks() -> Vec<Event> {
    google_feed().events_in(&Sao_Paulo, "link-1", day(10, 5), day(10, 19))
}

fn find<'a>(events: &'a [Event], title: &str) -> Vec<&'a Event> {
    events.iter().filter(|e| e.title == title).collect()
}

#[test]
fn reads_the_calendar_name_and_events() {
    let feed = google_feed();
    assert_eq!(feed.name.as_deref(), Some("me@example.com"));
    assert_eq!(feed.color, None);
    assert_eq!(feed.entries.len(), 10);
}

#[test]
fn all_day_and_multi_day_events() {
    let events = two_weeks();
    let holiday = find(&events, "Holiday")[0];
    assert_eq!(holiday.start, EventTime::Date(day(10, 12)));
    assert_eq!(holiday.end, EventTime::Date(day(10, 13)));
    assert_eq!(holiday.account, "");
    assert_eq!(holiday.calendar_id, "link-1");
    assert_eq!(holiday.ical_uid, "0a1b2c3d4e5f6g@google.com");

    let trip = find(&events, "Trip")[0];
    assert_eq!(trip.first_and_last_day(), (day(10, 14), day(10, 16)));
    assert_eq!(trip.location, "Lisbon, Portugal");
}

#[test]
fn converts_time_zones_to_local_time() {
    let events = two_weeks();
    // 10:00 in New York (EDT, UTC-4) is 11:00 in São Paulo.
    let sync = find(&events, "Client sync")[0];
    assert_eq!((sync.start, sync.end), (at(10, 7, 11, 0), at(10, 7, 12, 0)));
    // UTC, with DURATION instead of DTEND.
    let lunch = find(&events, "Lunch")[0];
    assert_eq!((lunch.start, lunch.end), (at(10, 8, 9, 0), at(10, 8, 9, 45)));
    // Floating times stay on the wall clock.
    let run = find(&events, "Run")[0];
    assert_eq!((run.start, run.end), (at(10, 9, 8, 0), at(10, 9, 9, 0)));
}

#[test]
fn reads_google_meet_and_cleans_the_description() {
    let events = two_weeks();
    let sync = find(&events, "Client sync")[0];
    assert_eq!(
        sync.join_link.as_deref(),
        Some("https://meet.google.com/abc-defg-hij")
    );
    // The alarm's description belongs to the alarm; Google's Meet block is dropped.
    assert_eq!(sync.description, "Agenda: roadmap; budget");

    let lunch = find(&events, "Lunch")[0];
    assert_eq!(
        lunch.join_link.as_deref(),
        Some("https://us02web.zoom.us/j/8123456789?pwd=abc")
    );
}

#[test]
fn drops_cancelled_and_declined_events() {
    let events = two_weeks();
    assert!(find(&events, "Cancelled").is_empty());
    assert!(find(&events, "Declined").is_empty());

    // Without knowing whose calendar it is, a declined invitation stays.
    let mut feed = google_feed();
    feed.owner = None;
    let events = feed.events_in(&Sao_Paulo, "link-1", day(10, 5), day(10, 19));
    assert_eq!(find(&events, "Declined").len(), 1);

    // Neither does a shared calendar that Google lists as having declined its own meetings.
    let text = GOOGLE.replace("me@example.com", "abc123@group.calendar.google.com");
    let mut feed = parse(&text);
    feed.owner = Some("abc123@group.calendar.google.com".into());
    let events = feed.events_in(&Sao_Paulo, "link-1", day(10, 5), day(10, 19));
    assert_eq!(find(&events, "Declined").len(), 1);
}

#[test]
fn expands_weekly_recurrence_with_an_exception_and_a_moved_occurrence() {
    let events = two_weeks();
    let mut standups: Vec<EventTime> = find(&events, "Standup").iter().map(|e| e.start).collect();
    standups.sort();
    // Mondays and Wednesdays at 9:30: Oct 7 was deleted (EXDATE), Oct 12 was moved, Oct 19 is out
    // of range.
    assert_eq!(standups, vec![at(10, 5, 9, 30), at(10, 14, 9, 30)]);

    let moved = find(&events, "Standup (moved)");
    assert_eq!(moved.len(), 1);
    assert_eq!(
        (moved[0].start, moved[0].end),
        (at(10, 13, 14, 0), at(10, 13, 14, 15))
    );
    // Same UID, different starts: both survive deduplication.
    assert_eq!(moved[0].ical_uid, "standup0123456789@google.com");
    assert_eq!(moved[0].id, "standup0123456789_20261012T123000Z");
}

#[test]
fn expands_only_within_the_range() {
    let feed = google_feed();
    // A week in 2027: the weekly rule ended on Dec 30, the yearly birthday still runs.
    let from = NaiveDate::from_ymd_opt(2027, 10, 4).unwrap();
    let events = feed.events_in(&Sao_Paulo, "link-1", from, from + Days::new(7));
    let titles: Vec<&str> = events.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(titles, vec!["Ana's birthday"]);
    assert_eq!(
        events[0].start,
        EventTime::Date(NaiveDate::from_ymd_opt(2027, 10, 10).unwrap())
    );
    // The date-only UNTIL includes its day; 2029 is past it.
    let from = NaiveDate::from_ymd_opt(2028, 10, 9).unwrap();
    assert_eq!(
        feed.events_in(&Sao_Paulo, "x", from, from + Days::new(3)).len(),
        1
    );
    let from = NaiveDate::from_ymd_opt(2029, 10, 9).unwrap();
    assert!(
        feed.events_in(&Sao_Paulo, "x", from, from + Days::new(3))
            .is_empty()
    );
}

#[test]
fn links_events_to_google_calendar() {
    let events = two_weeks();
    let holiday = find(&events, "Holiday")[0];
    let eid = STANDARD_NO_PAD.encode("0a1b2c3d4e5f6g me@example.com");
    assert_eq!(
        holiday.html_link.as_deref(),
        Some(format!("https://www.google.com/calendar/event?eid={}", encode(&eid)).as_str())
    );
    let run = find(&events, "Run")[0];
    assert_eq!(run.html_link, None);
}

#[test]
fn unfolds_and_unescapes() {
    let lines = unfold("SUMMARY:Long\r\n  title\r\n\t continued\r\nUID:1\n");
    assert_eq!(lines, vec!["SUMMARY:Long title continued", "UID:1"]);
    assert_eq!(unescape(r"a\, b\; c\\d\nnext\N"), "a, b; c\\d\nnext\n");
    let property = Property::parse(r#"ATTENDEE;CN="Doe: Jane";PARTSTAT=ACCEPTED:mailto:j@x.com"#).unwrap();
    assert_eq!(property.name, "ATTENDEE");
    assert_eq!(property.param("CN"), Some("Doe: Jane"));
    assert_eq!(property.value, "mailto:j@x.com");
}

#[test]
fn parses_values() {
    assert_eq!(parse_duration("PT1H30M"), Some(chrono::Duration::minutes(90)));
    assert_eq!(parse_duration("P1W"), Some(chrono::Duration::days(7)));
    assert_eq!(
        parse_duration("-P0DT0H10M0S"),
        Some(chrono::Duration::minutes(-10))
    );
    assert_eq!(parse_duration("1H"), None);
    assert_eq!(hex_color("#FF8800FF").as_deref(), Some("#ff8800"));
    assert_eq!(hex_color("orange"), None);
    assert_eq!(
        time_zone("/mozilla.org/20070129_1/Europe/Berlin"),
        Some(chrono_tz::Europe::Berlin)
    );
    assert_eq!(time_zone("E. South America Standard Time"), None);
    let exdates = Property::parse("EXDATE;VALUE=DATE:20261007,20261014")
        .unwrap()
        .moments();
    assert_eq!(exdates, vec![Moment::Date(day(10, 7)), Moment::Date(day(10, 14))]);
}

#[test]
fn unknown_zones_and_bad_rules_degrade_gracefully() {
    let text = "BEGIN:VCALENDAR\nX-APPLE-CALENDAR-COLOR:#1BADF8\nBEGIN:VEVENT\nUID:a\nSUMMARY:Outlook\n\
                DTSTART;TZID=E. South America Standard Time:20261007T100000\n\
                DTEND;TZID=E. South America Standard Time:20261007T110000\nEND:VEVENT\n\
                BEGIN:VEVENT\nUID:b\nSUMMARY:Odd rule\nDTSTART:20261008T100000Z\nRRULE:FREQ=SOMETIMES\n\
                END:VEVENT\nBEGIN:VEVENT\nUID:c\nSUMMARY:No start\nEND:VEVENT\nEND:VCALENDAR\n";
    let feed = parse(text);
    assert_eq!(feed.color.as_deref(), Some("#1badf8"));
    let events = feed.events_in(&Sao_Paulo, "x", day(10, 5), day(10, 12));
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].start, at(10, 7, 10, 0));
    assert_eq!(events[1].start, at(10, 8, 7, 0));
}

#[test]
fn reads_google_addresses() {
    let url = "https://calendar.google.com/calendar/ical/me%40example.com/private-0123456789abcdef/basic.ics";
    assert_eq!(google_calendar_id(url).as_deref(), Some("me@example.com"));
    assert_eq!(masked(url), "calendar.google.com/…/basic.ics");
    assert!(!masked(url).contains("0123456789abcdef"));
    assert_eq!(google_calendar_id("https://example.com/ical/x/basic.ics"), None);
    assert_eq!(
        web_address("webcal://example.com/a.ics").unwrap(),
        "https://example.com/a.ics"
    );
    assert!(web_address("calendar.google.com/x").is_err());
}

/// Serves one HTTP answer on a random local port and returns the address of a "secret" feed.
fn serve_once(status: &str, body: &'static str) -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let status = status.to_owned();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0u8; 2048];
        let _ = std::io::Read::read(&mut stream, &mut request);
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
    });
    format!("http://127.0.0.1:{port}/calendar/ical/me%40example.com/private-s3cr3t/basic.ics")
}

#[test]
fn fetches_a_feed() {
    let feed = fetch(&serve_once("200 OK", GOOGLE)).unwrap();
    assert_eq!(feed.name.as_deref(), Some("me@example.com"));
    assert_eq!(feed.entries.len(), 10);
}

#[test]
fn errors_never_contain_the_address() {
    for (status, body) in [("404 Not Found", ""), ("200 OK", "<html>Sign in</html>")] {
        let error = fetch(&serve_once(status, body)).unwrap_err().to_string();
        assert!(!error.contains("s3cr3t"), "{error}");
        assert!(matches!(fetch(&serve_once(status, body)), Err(Error::Link(_))));
    }
    // Nothing listens on port 9 (discard) on test machines.
    let error = fetch("http://127.0.0.1:9/private-s3cr3t/basic.ics")
        .unwrap_err()
        .to_string();
    assert!(!error.contains("s3cr3t"), "{error}");
}
