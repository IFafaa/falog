# 0005: Quick capture hotkey

**Status:** backlog
**Area:** desktop

## Goal

A global shortcut (e.g. `Ctrl+Alt+Space`) opens a small capture box from any app; typing or dictating
(Win+H) a line creates a task with the same natural-language parsing used for due dates and areas.

## Context

The assistant is the main way to file tasks, but sometimes it is not open. This keeps capture under
two seconds. Parsing should reuse `falog-core` (`date::parse_due`, `Store::find_area`).

## Scope

- In: global hotkey, borderless always-on-top capture window, inline hints for `@area`, `!priority`,
  `due friday`, Enter to save, Esc to cancel.
- Out: editing existing tasks from the capture box.

## Acceptance criteria

- [ ] The hotkey works while Falog is in the background (or in the tray, see 0001)
- [ ] "Fix login @work !urgent due friday" creates the expected task
- [ ] Parsing lives in `falog-core` with tests
- [ ] Hotkey configurable in Settings
