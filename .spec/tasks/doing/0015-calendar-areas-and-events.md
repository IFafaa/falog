# 0015: Calendars linked to areas, events through the assistant

**Status:** doing
**Area:** calendar, desktop, mcp

## Goal

Know which meetings belong to which area (Acme, Globex, Personal...), and let the assistant read
and create calendar events by voice: "what meetings do I have today?", "book a call with Ana tomorrow
at 3 pm on Acme".

## Context

The calendar (0010, 0014) shows several Google accounts and calendar links, but nothing ties a
calendar to an area, and only the desktop app reads events. The user connects one Google account per
employer plus a personal one, so an area maps naturally to one or more calendars, and one calendar
may serve more than one area (many to many).

Writing needs a wider OAuth scope: `calendar.events` (read and write events) plus
`calendar.calendarlist.readonly` (the calendar list) instead of `calendar.readonly`. Accounts connected
before must reconnect once; until then writing fails with a message saying so. Calendar links are
read-only by nature.

## Scope

- In: `areas` (area ids) on every calendar, accounts' and links' alike, kept across refreshes and
  reconnects; Settings › Calendar lets each calendar pick its areas; the sidebar area filter also
  filters the calendar view (calendars with no area show only under "All areas"); the event popover
  names the areas. MCP tools `list_events` (range, area) and `create_event` (title, start, end or
  duration, area or calendar, description, location, attendees, Meet link) plus `update_event` and
  `delete_event`; the MCP server reads the same `calendar/` files and refreshes tokens itself; the
  desktop refreshes when the MCP server changed something; assistant prompt and allowed tools.
- Out: editing events in the desktop UI, recurring-event creation, invitations' RSVP.

## Acceptance criteria

- [ ] A calendar can be linked to several areas and an area to several calendars, from Settings
- [ ] Filtering by an area in the sidebar shows only that area's meetings in the calendar view
- [ ] `list_events` returns the meetings of a day or range, optionally for one area
- [ ] `create_event` puts the event in the area's calendar and the desktop shows it within seconds
- [ ] An account connected with the old read-only scope gets a clear "reconnect" message on write
- [ ] Specs and README updated; tests cover the area links, the tools' arguments and the API payloads

## Plan

1. `falog-calendar`: `Calendar::areas`, kept by `upsert` and refreshes; area queries on the config; a
   `google::insert_event` / `patch_event` / `delete_event` client; scopes; a `changed` marker file.
2. Desktop: area filter on events; Settings › Calendar area pickers (the page gets the areas); event
   popover; refresh on the marker.
3. MCP: depend on `falog-calendar`; tools and their text output; tests through `Server::handle_line`
   with the HTTP layer behind a trait or a local test server.
4. Assistant prompt, `assistant/CLAUDE.md`, allowed tools; docs.

## Outcome

Filled when done.
