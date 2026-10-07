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
  assistant/     assistant dock: agents (Claude Code, ACP), threads + history, voice capture + Whisper, panel UI
  prefs.rs       persisted UI preferences
  theme.rs       Zed color tokens -> egui visuals
  fonts.rs, icons.rs
  components/    reusable widgets (buttons, chips, modal, pickers, switch, text helpers)
  workspace/     window chrome: sidebar, tab bar, toolbar, status bar, task panel
  views/         board, list, agenda (+ ViewCx, SortOrder)
  overlays/      command palette, areas, settings, confirm
  platform/      autostart (registry), single instance (loopback port), title bar colors (DWM)
```

### Frame flow

1. `poll()` checks `PRAGMA data_version` every 800 ms and reloads when another process wrote.
2. Panels are laid out in a fixed order: status bar, sidebar (left dock), assistant (outer right
   dock), task panel (right dock), tab bar, toolbar, central view, then overlays.
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
  sent as plain user messages. Model and effort are `--model` and `--effort`; it cannot switch them
  live, so the thread restarts it with `--resume <session>` once the current turn ends. Stopping
  kills the process; the next message resumes the conversation.
- **ACP** (`agent/acp.rs`): a client for the [Agent Client Protocol](https://agentclientprotocol.com)
  v1, newline-delimited JSON-RPC over the agent's stdio, written by hand like the MCP server. A
  reader thread runs the handshake (`initialize` with no fs/terminal capabilities, then
  `session/resume` if the agent supports it, else `session/load` with the replayed history dropped,
  else `session/new`; a session that cannot be reopened starts fresh with a notice), queues
  messages typed meanwhile, maps `session/update` (`agent_message_chunk`, `tool_call`,
  `tool_call_update`, `available_commands_update`, `config_option_update`) to events and answers
  `session/request_permission`, allowing falog tools only (`falog_tool` recognizes
  `mcp__falog__x`, `falog.x`, `x (falog MCP Server)`...). Model and effort are the session config
  options with category `model` and `thought_level`, switched live with
  `session/set_config_option`. Stop sends `session/cancel` and drops what still streams for that
  turn. Instructions go in `_meta.systemPrompt` (read by Claude's adapter, which also gets
  `_meta.claudeCode.options` with no built-in tools and no user settings); other agents get them in
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
`whisper` cargo feature (default on) because it needs CMake and libclang to build.

Performance notes (Ryzen 7 5700X, CPU busy with other apps, 6 s clip): `.cargo/config.toml` forces `/O2` and
AVX2 for whisper.cpp under MSVC (without it: ~290 s); `audio_ctx` is sized to the clip instead of the fixed
30 s window (~20 s → ~8 s). The default voice language follows the regional format, because `Auto` adds a
full-window language detection pass (~+20 s). OpenMP and flash attention made no measurable difference.

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
| `%APPDATA%\Falog\assistant\` | Assistant threads (`threads.json`), agents (`agents.json`), agents' working directory |
| `%LOCALAPPDATA%\Falog\*.exe` | Installed binaries |
