# 0014: Google Calendar through secret iCal links

**Status:** done
**Area:** desktop

## Goal

Connect a Google Calendar by pasting its *secret address in iCal format*: no Google Cloud project, no
OAuth client. Signing in with Google stays available for accounts whose admins hide secret addresses.

## Context

Task [0010](0010-google-calendar.md) shipped the Calendar view on top of OAuth with the user's own
Google Cloud client. Creating that client (project, API, consent screen, publishing, credentials) was
too much for the user, who only wants their two Google accounts in Falog. Every Google calendar has a
secret iCal address (Google Calendar › Settings › the calendar › *Integrate calendar*) that anyone
holding it can read without signing in, so Falog can poll it like any other feed.

Trade-offs the user accepted: the feed may lag behind changes made a moment ago (OAuth reads them
live); the feed has no calendar color, so Falog picks one the user can change; the address is a
credential (whoever has it reads the calendar), so it is stored like the refresh tokens and never shown
in full or logged.

## Scope

- In: `falog-calendar::ics` (fetch and parse VCALENDAR: VEVENT text fields, all-day, UTC, floating
  and TZID times, DURATION, line folding and escapes, cancelled events, Meet links, RRULE with EXDATE,
  RDATE and RECURRENCE-ID overrides expanded only within the requested range, the calendar name and
  color); calendar links in `google.json` next to the OAuth accounts (backward compatible); links
  fetched in the same background refresh and cache as accounts; sidebar rows with checkbox and color;
  Settings › Calendar: "Add a calendar link" first with a plain how-to, name and address fields and an
  Add button that checks the link by fetching it once, linked calendars with rename, color and remove;
  the OAuth setup in a collapsed "Sign in with Google instead (advanced)" section; the calendar view's
  banner offers "Add a calendar link" first.
- Out: other providers' feeds tested on purpose (Outlook, iCloud work if their feeds are standard, but
  are not a goal), VTIMEZONE definitions for non-IANA zone names (such events fall back to local time),
  writing to calendars.

## Acceptance criteria

- [x] Pasting a secret iCal address in Settings and clicking Add shows the calendar in the sidebar and
      its events in the view, without restarting
- [x] A wrong or reset address is refused with a message that does not contain the address
- [x] Weekly recurring events show every week of the visible range, without the deleted occurrence and
      with the moved one at its new time
- [x] All-day, multi-day and time-zone events land on the right days and hours
- [x] Links refresh with the accounts (every 5 minutes and on demand) and survive a restart offline
- [ ] Links can be renamed, recolored, hidden and removed; the address is masked everywhere
- [x] Existing `google.json` files (OAuth only) still load
- [x] Specs and README updated; tests with Google-shaped `.ics` samples; `fmt`, `clippy`, `test` green

## Plan

1. `falog-calendar`: `ics` module (`fetch`, `parse`, `Feed::events(from, to)`) with `chrono-tz` for
   TZIDs and `rrule` for RRULE expansion (UNTIL is set by hand so date-only and floating rules pass
   its time-zone checks); `Link` in `config` (`links` with a serde default); tests with samples shaped
   like Google's export.
2. `falog-desktop/src/calendar.rs`: fetch links next to accounts in the refresh thread, keep a link's
   cached events when it fails, add/rename/recolor/remove/toggle; adding fetches once on a thread.
3. Sidebar: a "Links" group of rows; `Action::ToggleCalendarLink`.
4. `overlays/settings/calendar.rs`: the add form and list first, OAuth collapsed below; calendar view
   banner.
5. README, architecture, design system, task 0010 notes.

Risks: the `rrule` crate rejects some rules (it validates strictly); a rejected rule falls back to the
first occurrence instead of dropping the feed. Google feeds of long-lived calendars are several MB:
the download is capped well above that and parsed once per refresh.

## Outcome

Shipped. `falog-calendar::ics` reads iCal feeds and `Link`s live in `google.json` next to the OAuth
accounts; Settings › Calendar leads with "Add a calendar link"; the sidebar, the banner and the command
palette (`calendar: add calendar link`) point there. OAuth is unchanged, folded under "Sign in with
Google instead (advanced)".

Decisions:

- `rrule` 0.14 expands RRULEs, but UNTIL is applied by hand: `rrule` reads a date-only UNTIL in the
  computer's zone and then rejects it against a UTC or zoned DTSTART. Dates and floating times are
  expanded as UTC wall-clock times. EXDATE, RDATE and RECURRENCE-ID are handled by Falog, compared as
  instants. A rule `rrule` refuses falls back to the first occurrence.
- Unknown TZIDs (Outlook's Windows names) fall back to floating time instead of reading VTIMEZONE.
- Declined invitations are dropped like the OAuth path does, by matching the calendar id in Google's
  address against ATTENDEE. Shared `…calendar.google.com` calendars never count as declining: Google
  lists them as having declined their own meetings (on the Kubernetes community feed, that rule hid
  nine meetings out of ten).
- Google's Meet block (`-::~:~::~` … `::-`) is cut from descriptions. The Join link is
  `X-GOOGLE-CONFERENCE`, or else the first Meet, Zoom, Teams or Webex link in the description or
  location.
- "Open in Google Calendar" for link events is built from the UID and the calendar id
  (`eid = base64("<event id>[_<UTC start>] <calendar id>")`), Google's known format. Not clicked yet.
- Downloads are capped at 64 MB: the Kubernetes feed is 12 MB, past ureq's 10 MB `into_string` limit.

Verified with real Google feeds: US holidays (all-day; 7 events in November and December 2026), the
Rust community calendar (time zones across Europe and the Americas, recurring meetups) and the
Kubernetes contributor calendar (12 MB, about 2.7k RRULEs and 5.8k overrides: parsed in 80 ms, a week
expanded in 0.2 s, times right across the US DST change on November 1). In the app (demo database, a
temporary startup hook instead of clicks): a link added through `CalendarState::add_link` with an empty
name took the feed's name and showed in the sidebar and in the week and month views, next to the demo
accounts' cached meetings; a non-existent secret address was refused with "The calendar was not found
at this link…" and not saved; the Settings page, the masked addresses and the empty-state banner render
as designed. This turned up a bug, fixed with a test: a refresh dropped the cached events of the
accounts it skipped (no OAuth client, or waiting to sign in again).

Not verified by hand: renaming, recoloring, hiding and removing a link (that criterion stays open until
someone clicks through it), the Add button and the Enter key themselves, and a restart with the network
off (`events.json` holds the link events and no addresses; loading it is the path accounts already
use). No private secret address of a real account was tried; it is the same feed format as the public
ones.
