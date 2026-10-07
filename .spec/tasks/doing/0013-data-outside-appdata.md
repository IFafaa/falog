# 0013: Data folder outside AppData

**Status:** doing
**Area:** core, desktop, scripts, docs

## Goal

Falog, its MCP server and every assistant that calls it read and write the same database on Windows,
even when the MCP server is started from a packaged app such as Claude desktop.

## Context

Claude desktop is an MSIX package. Every process it starts (Claude Code sessions, their `falog-mcp`
servers, shells) has `%APPDATA%`, `%LOCALAPPDATA%` and `HKCU` redirected, copy-on-write, into
`%LOCALAPPDATA%\Packages\Claude_<id>\LocalCache\...`, even for files that already exist. Tasks
created by talking to Claude in Claude desktop landed in a private copy of `%APPDATA%\Falog\falog.db`
that the Falog app never saw, so the user's areas "vanished". Folders outside AppData (the user
profile, `~/.claude`, `~/falog-assistant`) are not redirected. The installer was fixed on its own
(`install.ps1` detects the sandbox and finishes through Explorer); the data still lived in AppData.

macOS and Linux have no such redirection; they keep the platform folders from
[0012](../done/0012-macos-and-linux.md).

## Scope

- In: `falog_core::paths::data_home()`: `%USERPROFILE%\.falog` on Windows,
  `~/Library/Application Support/Falog` on macOS, `$XDG_DATA_HOME/Falog` on Linux. The database, the
  assistant folder, the Whisper model, the calendar files and eframe's window preferences (`app.ron`)
  live there. `FALOG_DB` still overrides the database, and the assistant and calendar folders follow
  it, as today. A one-time migration from the old folders when the new database does not exist yet,
  run by both binaries and safe when they start together. Uninstallers and docs.
- Out: moving the installed binaries (`%LOCALAPPDATA%\Falog`; the installer already runs outside
  the package); repairing copies already stranded in a package's `LocalCache` (fixed by hand).

## Acceptance criteria

- [ ] With no `FALOG_DB`, both binaries open `%USERPROFILE%\.falog\falog.db` on Windows
- [ ] On first start after the update, the old database (with its WAL), `assistant/`, `models/` and
      `calendar/` move to the new folder; the old database is renamed `falog.moved.db`
- [ ] The desktop app and the MCP server starting together migrate once, and neither creates an empty
      database in the new folder while the other is copying
- [ ] Window preferences survive the move (Windows `%APPDATA%\falog\data\app.ron`, Linux
      `~/.local/share/falog/app.ron`)
- [ ] Tests cover the migration, including a second run and a held lock
- [ ] Specs (`architecture.md` Files on disk) and README updated, saying why

## Plan

1. `falog-core/src/paths.rs`: `data_home()`, legacy locations, `migrate()` with a lock file
   (`create_new`) in the new folder, the database copied with `VACUUM INTO` (consistent even with a
   WAL and another reader) then renamed into place, folders moved with `rename` and a copy fallback.
2. `Store::open_default()` migrates first when `FALOG_DB` is not set, so both binaries do it.
3. Desktop: `voice::model_path()` and eframe's `persistence_path` use `data_home()`.
4. `uninstall.ps1`/`uninstall.sh` remove the new folder (and the old one) with `-RemoveData`.

Risks: an old Falog still running during the update keeps writing the old file (the installers stop
it first); a migration started inside a package sandbox that already has a private copy migrates that
copy.

## Outcome

Filled when done.
