# 0014: Google Calendar through secret iCal links

**Status:** doing
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

Trade-offs the user accepted: Google refreshes these feeds every few hours, not instantly; the feed has
no calendar color, so Falog picks one the user can change; the address is a credential (whoever has it
reads the calendar), so it is stored like the refresh tokens and never shown in full or logged.

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

- [ ] Pasting a secret iCal address in Settings and clicking Add shows the calendar in the sidebar and
      its events in the view, without restarting
- [ ] A wrong or reset address is refused with a message that does not contain the address
- [ ] Weekly recurring events show every week of the visible range, without the deleted occurrence and
      with the moved one at its new time
- [ ] All-day, multi-day and time-zone events land on the right days and hours
- [ ] Links refresh with the accounts (every 5 minutes and on demand) and survive a restart offline
- [ ] Links can be renamed, recolored, hidden and removed; the address is masked everywhere
- [ ] Existing `google.json` files (OAuth only) still load
- [ ] Specs and README updated; tests with Google-shaped `.ics` samples; `fmt`, `clippy`, `test` green

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

In progress.
