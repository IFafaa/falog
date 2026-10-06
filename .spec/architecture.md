# Architecture

## Overview

```
 assistant ──stdio JSON-RPC──► falog-mcp ──┐
                                           ├──► SQLite (WAL)  %APPDATA%\Falog\falog.db
            falog (desktop) ◄── polls ─────┘
```

Two processes share one database file. Neither talks to the other: the database is the integration
point. `falog-core` holds every rule so both binaries behave identically.

## Crates

| Crate | Kind | Depends on | Responsibility |
|---|---|---|---|
| `falog-core` | lib | rusqlite, chrono | Domain types, validation, date parsing, agenda rules, persistence |
| `falog-mcp` | bin `falog-mcp` | core, serde_json | MCP protocol, tool schemas, argument parsing, text output |
| `falog-desktop` | bin `falog` | core, eframe/egui | UI, theming, OS integration |

Dependencies point inward only: the binaries depend on `falog-core`, never on each other, and
`falog-core` knows nothing about UI or MCP.

## falog-core

```
src/
  domain/      entities and value types (Task, Company, Note, Status, Priority, Rgb, ids)
  store/       Store (connection) + one file per aggregate: tasks, companies, notes, schema
  date.rs      today/now, natural-language due dates, urgency and formatting
  agenda.rs    buckets, sorting, summaries
  text.rs      accent/case-insensitive matching
  error.rs     Error enum (thiserror)
```

- `Store` is the only way to touch the database. Methods take and return domain types.
- All reads load whole tables (`tasks()`, `companies()`); filtering happens in memory. This is
  deliberate: data is small and it keeps queries trivial. Revisit only with measurements.
- Schema changes are **append-only migrations** in `store/schema.rs`, tracked by `PRAGMA user_version`.
  Never edit a released migration.

## falog-mcp

```
src/
  main.rs       opens the store, serves stdin/stdout
  server.rs     JSON-RPC dispatch (initialize, tools/list, tools/call, ping)
  protocol.rs   message type and response helpers
  tools/        catalog.rs (schemas), args.rs (typed arguments), handlers.rs (logic)
  format.rs     text renderings returned to the model
```

The protocol is implemented by hand (newline-delimited JSON-RPC 2.0); it is small and avoids an async
runtime. See [mcp.md](mcp.md).

## falog-desktop

```
src/
  main.rs        single instance, window options, run loop
  app.rs         FalogApp: state, per-frame layout, action handling, persistence of prefs
  action.rs      Action enum: user intents emitted by widgets
  prefs.rs       persisted UI preferences
  theme.rs       Zed color tokens -> egui visuals
  fonts.rs, icons.rs
  components/    reusable widgets (buttons, chips, modal, pickers, switch, text helpers)
  workspace/     window chrome: sidebar, tab bar, toolbar, status bar, task panel
  views/         board, list, agenda (+ ViewCx, SortOrder)
  overlays/      command palette, companies, settings, confirm
  platform/      autostart (registry), single instance (loopback port), title bar colors (DWM)
```

### Frame flow

1. `poll()` checks `PRAGMA data_version` every 800 ms and reloads when another process wrote.
2. Panels are laid out in a fixed order: status bar, sidebar (left dock), task panel (right dock),
   tab bar, toolbar, central view, then overlays.
3. Widgets never mutate app data. They push `Action`s (or return events such as `PanelEvent`); the app
   applies them after drawing. This keeps rendering free of borrow conflicts and side effects.
4. Every write goes through `FalogApp::write`, which reloads data on success and shows a toast on error.

### OS integration

- **Single instance:** binding `127.0.0.1:47613`; a second launch sends `show` and exits.
- **Launch at sign-in:** `HKCU\...\Run\Falog`, toggled in Settings and set by the installer.
- **Title bar:** DWM caption/text/border colors follow the theme (Windows 11).

## Concurrency and consistency

- WAL mode + 5 s busy timeout; writes are short single statements.
- Last write wins. The task panel keeps a draft; if the task changes underneath while the draft is
  clean, the panel refreshes; if dirty, only the notes refresh.
- Migrations take `BEGIN IMMEDIATE` and re-check the version, so two processes starting on a fresh
  install do not race.

## Files on disk

| Path | Content |
|---|---|
| `%APPDATA%\Falog\falog.db` | Tasks (override with `FALOG_DB`) |
| `%APPDATA%\Falog\data\app.ron` | Window and UI preferences (eframe) |
| `%LOCALAPPDATA%\Falog\*.exe` | Installed binaries |
