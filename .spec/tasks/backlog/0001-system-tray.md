# 0001: System tray

**Status:** backlog
**Area:** desktop

## Goal

Closing the window hides Falog to the system tray instead of quitting; the tray icon shows the number
of overdue tasks and reopens the window on click.

## Context

Falog starts at sign-in and is meant to be always available. Today, closing the window ends the
process. See [architecture.md](../../architecture.md#os-integration) for the single-instance mechanism,
which already knows how to bring the window back.

## Scope

- In: tray icon with menu (Open, New task, Quit), hide on close, overdue badge or tooltip.
- Out: start minimized option (separate task if wanted).

## Acceptance criteria

- [ ] Closing the window keeps the process alive and the tray icon visible
- [ ] Clicking the icon or "Open" restores and focuses the window
- [ ] "Quit" exits and saves preferences
- [ ] Tooltip shows "N overdue, M due today"
- [ ] Specs updated

## Plan

Evaluate the `tray-icon` crate with winit 0.30 (eframe 0.29). Hide via
`ViewportCommand::Visible(false)` and intercept close with `ViewportCommand::CancelClose`.
