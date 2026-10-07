# Architecture

## Overview

```
 assistant ──stdio JSON-RPC──► falog-mcp ──┐
                                           ├──► SQLite (WAL)  <data dir>/falog.db (see Files on disk)
            falog (desktop) ◄── polls ─────┘
```

Two processes share one database file. Neither talks to the other: the database is the integration
point. `falog-core` holds every rule so both binaries behave identically.

## Crates

| Crate | Kind | Depends on | Responsibility |
|---|---|---|---|
| `falog-core` | lib | rusqlite, chrono | Domain types, validation, date parsing, agenda rules, persistence |
| `falog-mcp` | bin `falog-mcp` | core, serde_json | MCP protocol, tool schemas, argument parsing, text output |
| `falog-calendar` | lib | ureq, chrono, sha2 | Google OAuth (loopback + PKCE), Calendar API, event model and layout rules, files |
| `falog-desktop` | bin `falog` | core, calendar, eframe/egui | UI, theming, OS integration |

Dependencies point inward only: the binaries depend on `falog-core`, never on each other, and
`falog-core` knows nothing about UI or MCP.

## falog-core

```
src/
  domain/      entities and value types (Task, Area, Note, Status, Priority, Rgb, ids)
  store/       Store (connection) + one file per aggregate: tasks, areas, notes, schema
  date.rs      today/now, natural-language due dates, urgency and formatting
  agenda.rs    buckets, sorting, summaries
  text.rs      accent/case-insensitive matching
  error.rs     Error enum (thiserror)
```

- `Store` is the only way to touch the database. Methods take and return domain types.
- All reads load whole tables (`tasks()`, `areas()`); filtering happens in memory. This is
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
  assistant/     assistant dock: Claude Code sessions, threads + history, voice capture + Whisper, panel UI
  calendar.rs    Google Calendar state: accounts, cached events, background sign-in and refresh
  prefs.rs       persisted UI preferences
  theme.rs       Zed color tokens -> egui visuals
  fonts.rs, icons.rs
  components/    reusable widgets (buttons, chips, modal, pickers, switch, text helpers)
  workspace/     window chrome: sidebar, tab bar, toolbar, status bar, task panel
  views/         board, list, focus, calendar (+ ViewCx, SortOrder)
  overlays/      command palette, areas, settings, confirm
  platform/      autostart (registry, LaunchAgent, XDG autostart), single instance (loopback port),
                 title bar colors (DWM), regional format, reveal in Explorer/Finder/Files
```

### Frame flow

1. `poll()` checks `PRAGMA data_version` every 800 ms and reloads when another process wrote.
2. Panels are laid out in a fixed order: status bar, sidebar (left dock), assistant (outer right
   dock), task panel (right dock), tab bar, toolbar, central view, then overlays.
3. Widgets never mutate app data. They push `Action`s (or return events such as `PanelEvent`); the app
   applies them after drawing. This keeps rendering free of borrow conflicts and side effects.
4. Every write goes through `FalogApp::write`, which reloads data on success and shows a toast on error.

### Assistant

The assistant dock talks to **Claude Code in headless mode** on the user's own login (no API key):
one long-lived `claude --print --input-format stream-json --output-format stream-json` process per
conversation (`assistant/claude.rs`). Built-in tools are disabled (`--tools ""`); the only tools are
those of a `falog-mcp` child with `FALOG_DB` set to the app's database (`--strict-mcp-config
--allowedTools mcp__falog --permission-mode dontAsk --setting-sources ""`). The system prompt is
`assets/assistant-prompt.md`; the working directory is `<data dir>/assistant`, so no project
`CLAUDE.md` leaks in. A reader thread maps stream-json lines to `ClaudeEvent`s (text deltas, tool use and
results, turn end, exit). Stopping kills the process; the next message resumes the conversation with
`--resume <session>`. When a tool result arrives the app syncs immediately instead of waiting for the poll.

Threads (`assistant/thread.rs`): each conversation is a `Thread` owning its items, draft and Claude
Code process; `Assistant` keeps a list of them plus the active one and polls them all, so a turn keeps
running while another thread is open. Switching threads ends the idle processes of the others; the
next message resumes them with `--resume`. Threads with messages are saved to
`<data dir>/assistant/threads.json` (`assistant/history.rs`: newest first, at most 100, written
through a temporary file; an unreadable file is kept as `threads.json.bak`) a couple of seconds after
they change and when eframe saves its state. Falog opens on a fresh thread, as Zed does.

Voice (`assistant/voice.rs`): `cpal` records the default microphone, mixes to mono and resamples to
16 kHz; a worker thread runs whisper.cpp (`whisper-rs`, model `ggml-large-v3-turbo-q5_0.bin` in
`<data dir>/models`, downloaded on first use) and unloads it after 5 idle minutes. Whisper sits behind the
`whisper` cargo feature (default on) because it needs CMake and libclang to build. The `gpu` feature runs
it on the GPU (Settings has a switch): Metal on macOS, Vulkan on Windows and Linux. Cargo features cannot
depend on the target, so `gpu` enables whisper-rs's `whisper-rs-sys` dependency, declared once per
target with the backend feature.

Performance notes (Ryzen 7 5700X, CPU busy with other apps, 6 s clip): `cmake/whisper.cmake` (a CMake
project include that `.cargo/config.toml` points whisper-rs-sys at) forces `/O2` and AVX2 for whisper.cpp
under MSVC only (without it: ~290 s; GCC and Clang keep ggml's native CPU tuning); `audio_ctx` is sized
to the clip instead of the fixed 30 s window (~20 s → ~8 s). The default voice language follows the
regional format, because `Auto` adds a full-window language detection pass (~+20 s).
OpenMP and flash attention made no measurable difference.

### Calendar

`falog-calendar` has no UI: `oauth` signs in the way Google wants installed apps to (a one-shot
listener on `127.0.0.1`, PKCE, `access_type=offline`), `google` reads the calendar list and events
(`singleEvents=true`, paged; cancelled, declined and working-location entries dropped), `model` turns
them into `Event`s in local time, and `layout` places overlapping events in columns. The user brings
their own OAuth client (Desktop app type), saved with the accounts in `calendar/google.json`.

`falog-desktop/src/calendar.rs` owns that state. Sign-in and fetching run on short-lived threads that
report over a channel drained each frame; access tokens stay in memory. The view asks for its visible
range and the state fetches a month-aligned window around it when it is not cached or older than five
minutes, only for visible calendars. A rejected refresh token marks the account "needs to sign in"
instead of dropping its cached events.

### OS integration

- **Single instance:** binding `127.0.0.1:47613`; a second launch sends `show` and exits. Portable as
  is: on macOS and Linux a second bind of the same address fails while the first process listens
  (`SO_REUSEADDR` only reuses ports in `TIME_WAIT`). Wayland compositors may refuse to raise the window.
- **Launch at sign-in:** toggled in Settings and set by the installers. Windows: the
  `HKCU\...\Run\Falog` registry value; macOS: a LaunchAgent,
  `~/Library/LaunchAgents/app.falog.Falog.plist`; Linux: an XDG autostart entry,
  `~/.config/autostart/falog.desktop`. Each starts the executable that wrote it.
- **Title bar:** DWM caption/text/border colors follow the theme (Windows 11). macOS and Linux keep the
  native title bar.
- **Microphone (macOS):** the bundle's `Info.plist` carries `NSMicrophoneUsageDescription`; without it
  macOS ends the app when it opens the microphone. Run from a terminal, the terminal's permission applies.
- **Shortcuts:** `Modifiers::COMMAND` is Ctrl on Windows and Linux and Cmd on macOS.
- **Finding `claude`** (`platform/paths.rs`): `PATH`, then `~/.local/bin` (Claude Code's native
  installer) and, on macOS and Linux, `~/.claude/local`, `~/.npm-global/bin`, `/opt/homebrew/bin` and
  `/usr/local/bin`. Apps started from Finder, the Dock, a LaunchAgent or a desktop autostart do not get
  the shell's `PATH`. `falog-mcp` is always the one next to the running executable.

## Concurrency and consistency

- WAL mode + 5 s busy timeout; writes are short single statements.
- Last write wins. The task panel keeps a draft; if the task changes underneath while the draft is
  clean, the panel refreshes; if dirty, only the notes refresh.
- Migrations take `BEGIN IMMEDIATE` and re-check the version, so two processes starting on a fresh
  install do not race.

## Files on disk

`<data dir>` is `dirs::data_dir()/Falog`: `%APPDATA%\Falog` on Windows,
`~/Library/Application Support/Falog` on macOS, `$XDG_DATA_HOME/Falog` (`~/.local/share/Falog`) on Linux.

| Content | Windows | macOS | Linux |
|---|---|---|---|
| Tasks (override with `FALOG_DB`) | `<data dir>\falog.db` | `<data dir>/falog.db` | `<data dir>/falog.db` |
| Assistant threads, MCP config | `<data dir>\assistant\` | `<data dir>/assistant/` | `<data dir>/assistant/` |
| Whisper model | `<data dir>\models\` | `<data dir>/models/` | `<data dir>/models/` |
| Google OAuth client, accounts with refresh tokens, calendar choices | `<data dir>\calendar\google.json` | `<data dir>/calendar/google.json` | `<data dir>/calendar/google.json` |
| Last fetched events, shown at startup and offline | `<data dir>\calendar\events.json` | `<data dir>/calendar/events.json` | `<data dir>/calendar/events.json` |
| Window and UI preferences (eframe, app id `falog`) | `%APPDATA%\falog\data\app.ron` | `~/Library/Application Support/falog/app.ron` | `~/.local/share/falog/app.ron` |
| Installed binaries | `%LOCALAPPDATA%\Falog\*.exe` | `~/Applications/Falog.app/Contents/MacOS/` | `~/.local/bin/` |
| Launch at sign-in | `HKCU\...\Run\Falog` | `~/Library/LaunchAgents/app.falog.Falog.plist` | `~/.config/autostart/falog.desktop` |
| App menu entry | Start menu `Falog.lnk` | the bundle itself | `~/.local/share/applications/falog.desktop` |

Windows and macOS file systems ignore case, so eframe's `falog` folder is the same as `Falog` there; on
Linux they are two folders.
