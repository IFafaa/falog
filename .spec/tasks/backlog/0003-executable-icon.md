# 0003: Executable icon

**Status:** backlog
**Area:** desktop, scripts

## Goal

`falog.exe` carries the Falog icon, so Explorer, the Start menu shortcut and the taskbar pin all show it.

## Context

The window icon is drawn in code (`icons::app_icon`), but the executable has no icon resource.

## Scope

- In: an `.ico` (16–256 px) in `crates/falog-desktop/assets`, embedded at build time on Windows.
- Out: an MSI/MSIX installer.

## Acceptance criteria

- [ ] Explorer and the Start menu shortcut show the icon
- [ ] The window icon and the `.ico` come from the same artwork
- [ ] Builds on non-Windows targets are unaffected

## Plan

`build.rs` with the `winresource` crate, gated on `cfg(windows)`. Generate the `.ico` from the same
geometry as `app_icon`, or draw it once as SVG and rasterize.
