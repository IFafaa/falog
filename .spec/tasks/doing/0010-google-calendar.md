# 0010: Calendar view with Google Calendar

**Status:** doing
**Area:** desktop

## Goal

See meetings from several Google accounts (work and personal) in a Calendar tab next to Board, List
and Agenda, laid out like Google Calendar: a week grid by hour and a month grid.

## Context

Tasks say what to do; meetings say when there is no time to do it. The user has calendars in more than
one Google account (work and personal) and wants them in one place, read-only. They
chose signing in with Google (OAuth) over secret iCal links: events show up right away and work
accounts that block secret links still work.

Update (task [0014](../done/0014-ical-links.md)): creating an OAuth client turned out to be too much
setup, so secret iCal links became the main way to add a calendar; what follows still describes the
OAuth path, now under "Sign in with Google instead (advanced)".

Google only lets an app sign users in with its own OAuth client, and an unverified client cannot be
shipped in an open source repo. Each user creates a **Desktop app** OAuth client in Google Cloud once
and pastes its id and secret into Settings (the "secret" of a desktop client is not confidential;
Google says so). Clients in *Testing* publishing status get refresh tokens that expire after 7 days,
so the guide tells the user to publish the app (no verification is needed for personal use with the
read-only scope; Google shows an "unverified app" warning once).

## Scope

- In: Settings section with the client id/secret, a setup guide, connected accounts (connect, remove);
  sign-in in the browser with the loopback redirect and PKCE; per-account calendar list with
  show/hide in the sidebar, colored like in Google; Calendar tab (`Ctrl+4`) with week and month views,
  today, previous/next, the current-time line; event details (time, calendar, location, description,
  join link, open in Google Calendar); tasks due on a day shown as all-day items; background refresh
  every 5 minutes and on demand; events cached on disk so the view opens instantly and offline.
- Out: creating or editing events, other providers (Outlook), reminders, meetings in the assistant's
  agenda (follow-up), day view. iCal links came later, in 0014.

## Acceptance criteria

- [ ] With a client configured, "Connect Google account" opens the browser, and after consenting the
      account and its calendars appear without restarting
- [x] Two or more accounts can be connected; each calendar can be hidden from the sidebar
- [x] The week view places timed events by hour (overlaps side by side) and all-day events on top;
      the month view lists each day's events with "+N more"
- [ ] Clicking an event shows its details with join and open links
- [x] Events survive a restart offline (cache) and refresh in the background
- [x] Refresh tokens are stored outside the repo and the prefs file, and removing an account deletes
      its token
- [x] Specs updated (`architecture.md`, `design-system.md`, README); tests cover parsing and layout
      rules; `fmt`, `clippy`, `test` green

## Plan

1. New crate `falog-calendar` (no UI): `oauth` (PKCE, loopback listener, token exchange and refresh),
   `google` (calendar list and events over the REST API with `ureq`, pagination), `model` (`Event`,
   `EventTime`, `Calendar`), `store` (`<data dir>/calendar/accounts.json` with tokens and calendar
   choices, `events.json` cache).
2. Desktop `calendar/` module: a `CalendarSync` worker thread fed by commands (connect, refresh range,
   remove account) that reports results over a channel, like the assistant does.
3. Desktop view `views/calendar.rs`: week grid and month grid painted by hand; `View::Calendar`; toolbar
   with today, previous, next and the week/month switch.
4. Sidebar "Calendars" section; Settings "Google Calendar" section; command palette commands.
5. Docs: README setup guide, specs.

Risks: Google's consent screen warns about an unverified app; work accounts whose admins block
third-party apps cannot connect (then the user has to ask their admin or use another account).

## Outcome

In progress. Built and verified with the demo data from `scripts/seed-demo.ps1` (two accounts, three
calendars, overlapping, all-day and multi-day events): week and month views render as designed
(`docs/screenshots/calendar-*.png`). Unit tests cover URL encoding, Google error and payload parsing,
the PKCE URL, the loopback redirect (including cancel and a denied consent), config and cache files,
all-day rules, deduplication and the column layout.

Not verified yet: a real sign-in and fetch against Google, which needs the user's own OAuth client.
The first criterion stays open until then. The popover was not exercised by hand (no clicks while
the user was at the machine).

Update after 0014: the Settings › Calendar page leads with calendar links; this task's OAuth setup is
folded under "Sign in with Google instead (advanced)", and the calendar banner and the sidebar's `+` open
that page instead of starting a sign-in. A real fetch from Google now works through links (verified
with public Google feeds); the OAuth sign-in itself is still unverified.
