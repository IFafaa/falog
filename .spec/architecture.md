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
| `falog-mcp` | bin `falog-mcp` | core, calendar, serde_json | MCP protocol, tool schemas, argument parsing, text output; calendar tools through the same `calendar/` files as the app |
| `falog-calendar` | lib | ureq, chrono, chrono-tz, rrule, sha2 | iCal feeds, Google OAuth (loopback + PKCE), Calendar API, event model and layout rules, files |
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
  paths.rs     data folder per platform, moving older layouts into it (see Files on disk)
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
  assistant/     assistant dock: agents (Claude Code, ACP), threads + history, voice capture + Whisper, panel UI
  calendar.rs    Google Calendar state: links, accounts, cached events, background sign-in and refresh
  prefs.rs       persisted UI preferences
  theme.rs       Zed color tokens -> egui visuals
  fonts.rs, icons.rs
  components/    reusable widgets (buttons, chips, modal, pickers, switch, text helpers)
  workspace/     window chrome: sidebar, tab bar, toolbar, status bar, task panel
  views/         board, list, focus, calendar (+ ViewCx, SortOrder)
  overlays/      command palette, areas, settings/ (one page per tab), confirm
  platform/      autostart (registry, LaunchAgent, XDG autostart), single instance (loopback port),
                 title bar colors (DWM), regional format, reveal in Explorer/Finder/Files
```

### Frame flow

1. `poll()` checks `PRAGMA data_version` every 800 ms and reloads when another process wrote.
2. Panels are laid out in a fixed order: status bar, sidebar (left dock), assistant (outer right
   dock), task panel (right dock), tab bar, toolbar, central view, then overlays. When the assistant
   is zoomed (`Prefs::assistant_zoomed`), it is laid out at the full width and the workspace panels
   and views are skipped.
3. Widgets never mutate app data. They push `Action`s (or return events such as `PanelEvent`); the app
   applies them after drawing. This keeps rendering free of borrow conflicts and side effects.
4. Every write goes through `FalogApp::write`, which reloads data on success and shows a toast on error.

### Assistant

The assistant dock talks to an **agent** on the user's own subscription (no API key). Threads only
see the `agent::Session` trait (send, cancel, set an option) and the `AgentEvent`s it yields (text
deltas, tool use and results, turn end, slash commands, settings, notices, exit), so any agent can
sit behind a thread. Every session gets the same `agent::Environment`: the tools of a `falog-mcp`
child with `FALOG_DB` set to the app's database as its only tools, the system prompt
`assets/assistant-prompt.md`, and `<data dir>/assistant` as working directory, so no project
instructions leak in. When a tool result arrives the app syncs immediately instead of waiting for
the poll.

Agents (`assistant/agent/registry.rs`) are `{id, name, command, args, env}` plus a protocol. Presets:
**Claude Code** (default), **Claude (ACP)** (`npx -y @agentclientprotocol/claude-agent-acp`, Node
22+), **Gemini CLI** (`gemini --acp`) and **Codex** (`npx -y @agentclientprotocol/codex-acp`).
Commands are looked up on `PATH` with `PATHEXT` on Windows (`npx.cmd`); Claude Code is also found in
`~/.local/bin`.

- **Claude Code** (`agent/claude_code.rs`): one long-lived `claude --print --input-format
  stream-json --output-format stream-json` process per conversation. Built-in tools are disabled
  (`--tools ""`), MCP comes from a generated config (`--strict-mcp-config --allowedTools mcp__falog
  --permission-mode dontAsk --setting-sources ""`). A reader thread maps stream-json lines to events;
  slash commands come from `init` (names) and `system/commands_changed` (with descriptions) and are
  sent as plain user messages. Model, effort and mode are `--model`, `--effort` and
  `--permission-mode` (Falog's default stays `dontAsk`; `--allowedTools mcp__falog` keeps falog
  tools allowed in every mode), fast mode is `--settings {"fastMode":true}` (Opus with extra usage;
  when Claude Code reports `fast_mode_disabled_reason`, the thread gets a notice saying why). It
  cannot switch these live, so the thread restarts it with `--resume <session>` once the current
  turn ends. Context use comes from `result`: the last model call in `usage.iterations` (input,
  cache and output tokens) against `modelUsage[model].contextWindow`. Ultracode is not offered: it
  runs multi-agent workflows with the built-in tools the assistant turns off. Stopping kills the
  process; the next message resumes the conversation.
- **ACP** (`agent/acp.rs`): a client for the [Agent Client Protocol](https://agentclientprotocol.com)
  v1, newline-delimited JSON-RPC over the agent's stdio, written by hand like the MCP server. A
  reader thread runs the handshake (`initialize` with no fs/terminal capabilities, then
  `session/resume` if the agent supports it, else `session/load` with the replayed history dropped,
  else `session/new`; a session that cannot be reopened starts fresh with a notice), queues
  messages typed meanwhile, maps `session/update` (`agent_message_chunk`, `tool_call`,
  `tool_call_update`, `available_commands_update`, `config_option_update`) to events and answers
  `session/request_permission`, allowing falog tools only (`falog_tool` recognizes
  `mcp__falog__x`, `falog.x`, `x (falog MCP Server)`...). Model, effort, mode and fast mode are the
  session config options with category `model`, `thought_level`, `mode` and (Claude's adapter)
  `model_config` with id `fast`, switched live with `session/set_config_option`; agents that still
  use the older `modes` get them as a mode option switched with `session/set_mode` and updated by
  `current_mode_update`. Context use comes from `usage_update` (`used`, `size`). Stop sends `session/cancel` and drops what still streams for that
  turn. Instructions go in `_meta.systemPrompt` (read by Claude's adapter, which also gets
  `_meta.claudeCode.options` with no built-in tools, no user settings and `strictMcpConfig`, so the
  user's own MCP servers stay out); other agents get them in
  front of the first prompt of a new session. Dropping a session closes the agent's stdin before
  killing it, because `npx` agents run under `cmd.exe` on Windows and the Node process would
  otherwise linger.

Threads (`assistant/thread.rs`): each conversation is a `Thread` owning its items, draft, settings
(agent, model, effort; `thread::Settings`) and agent session; `Assistant` keeps a list of them plus
the active one and polls them all, so a turn keeps running while another thread is open. The agent
of a thread can change only before its first message (the conversation lives in the agent). Switching
threads ends the idle processes of the others; the next message resumes them. Threads with messages
are saved to `<data dir>/assistant/threads.json` (`assistant/history.rs`: newest first, at most 100,
written through a temporary file; an unreadable file is kept as `threads.json.bak`; threads saved
before agents existed load as Claude Code threads) a couple of seconds after they change and when
eframe saves its state. Falog opens on a fresh thread, as Zed does.

`<data dir>/assistant/agents.json` (`assistant/agents.rs`) keeps the user's custom agents, the last
agent/model/effort picked (new threads start from it), and what each agent reported last time (its
settings and commands), so the pickers and `/` work before the agent starts again.

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

A calendar comes from one of two sources, both saved in `calendar/google.json`:

- **Links** (the usual way): a calendar's secret iCal address. `ics` downloads the whole feed (capped
  at 64 MB; long-lived Google calendars are several MB), parses VEVENTs (folding, escapes, `TZID` with
  `chrono-tz`, UTC, floating and all-day times, `DURATION`), drops cancelled events and invitations the
  owner declined (the owner is the calendar id in Google's address; shared `group.calendar.google.com`
  calendars never count as declining), and expands `RRULE` with the `rrule` crate only inside the asked
  range, minus `EXDATE`s and the occurrences a `RECURRENCE-ID` override replaces. `UNTIL` is applied by
  hand because `rrule` rejects Google's date-only rules. A rule `rrule` cannot read falls back to its
  first occurrence. Link events have an empty `account` and the link's `link-…` id as `calendar_id`.
  The address is a credential: errors never contain it, `Debug` and the UI show
  `ics::masked` (`calendar.google.com/…/basic.ics`).
- **Accounts** (advanced): `oauth` signs in the way Google wants installed apps to (a one-shot listener
  on `127.0.0.1`, PKCE, `access_type=offline`), `google` reads the calendar list and events
  (`singleEvents=true`, paged; cancelled, declined and working-location entries dropped). The user brings
  their own OAuth client (Desktop app type).

`model` holds the `Event`s in local time, and `layout` places overlapping events in columns.

`falog-desktop/src/calendar.rs` owns that state. Checking a pasted link, sign-in and fetching run on
short-lived threads that report over a channel drained each frame; access tokens stay in memory. A link
is saved only after one successful fetch, and its events for the cached range show right away. The view
asks for its visible range and the state fetches a month-aligned window around it, links and accounts in
the same thread, when it is not cached or older than five minutes, only for visible calendars. A link or
account that fails keeps its cached events and puts its error in the header; a rejected refresh token
marks the account "needs to sign in". When a meeting comes through both a link and an account, the
link's copy is kept (deduplicated by iCal UID and start).

Each account and link has one area (an area may have several). `falog_calendar::service` holds what the
app and the MCP server share: access tokens, fetching an account, and creating, changing and deleting
events (signed-in accounts that granted `calendar.events`; others get a "reconnect" error). The MCP
server reads the same `calendar/` files and writes `calendar/changed` after a change; the app checks
that marker every second and fetches again, so a meeting booked by voice shows up at once.

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

`<data dir>` is `falog_core::paths::data_home()`: `%USERPROFILE%\.falog` on Windows,
`~/Library/Application Support/Falog` on macOS, `$XDG_DATA_HOME/Falog` (`~/.local/share/Falog`) on Linux.
The assistant and calendar folders sit next to the database, so they follow `FALOG_DB`; the Whisper
model and the window preferences stay in `<data dir>` for every database.

| Content | Path |
|---|---|
| Tasks (override with `FALOG_DB`) | `<data dir>/falog.db` |
| Assistant threads (`threads.json`), agents (`agents.json`), MCP config, agents' working directory | `<data dir>/assistant/` |
| Whisper model | `<data dir>/models/` |
| Calendar links (secret iCal addresses), Google OAuth client, accounts with refresh tokens, calendar choices | `<data dir>/calendar/google.json` |
| Last fetched events, shown at startup and offline | `<data dir>/calendar/events.json` |
| Window and UI preferences (eframe `persistence_path`) | `<data dir>/app.ron` |

| Content | Windows | macOS | Linux |
|---|---|---|---|
| Installed binaries | `%LOCALAPPDATA%\Falog\*.exe` | `~/Applications/Falog.app/Contents/MacOS/` | `~/.local/bin/` |
| Launch at sign-in | `HKCU\...\Run\Falog` | `~/Library/LaunchAgents/app.falog.Falog.plist` | `~/.config/autostart/falog.desktop` |
| App menu entry | Start menu `Falog.lnk` | the bundle itself | `~/.local/share/applications/falog.desktop` |

**Why not AppData on Windows.** Packaged (MSIX) apps such as Claude desktop redirect `%APPDATA%`,
`%LOCALAPPDATA%` and `HKCU`, copy-on-write, for every process they start, into
`%LOCALAPPDATA%\Packages\<package>\LocalCache\...`. A `falog-mcp` started by Claude Code inside Claude
desktop wrote to a private copy of the database that the Falog app never saw. The user profile is not
redirected. (The binaries stay in `%LOCALAPPDATA%`: the installer detects the sandbox and finishes the
install through Explorer.)

**Moving older data.** Older versions kept the data in `%APPDATA%\Falog` (Windows) and let eframe keep
`app.ron` in its own folder (`%APPDATA%\falog\data`, `~/.local/share/falog`). Without `FALOG_DB`,
`Store::open_default` runs `paths::migrate_legacy_data` first, in both binaries: whatever the new folder
lacks is moved there under a `migration.lock` file (a second process waits, then re-checks), the
database through `VACUUM INTO` and a rename, then the old file is renamed `falog.moved.db`. Assistant
threads keep their history, but Claude Code stores sessions per working directory, so `--resume` of a
thread from before the move may not find its session (not verified).
