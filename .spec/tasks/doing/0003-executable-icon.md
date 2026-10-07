# 0003: App icon

**Status:** doing
**Area:** desktop, scripts

## Goal

Falog has an icon of its own, drawn once as SVG, and `falog.exe`, the window, the taskbar, the Start
menu shortcut and (later) the macOS and Linux launchers all show it.

## Context

The window icon was drawn in code (`icons::app_icon`): a rounded One Dark tile with an accent check
mark, which says "to-do app" but nothing about Falog, and the executable had no icon resource at all.

## Scope

- In: a new design (SVG master plus pixel-hinted 16 and 24 px copies), a committed rasterizer that
  regenerates PNGs, a multi-size `.ico` and a `.icns`; the window and taskbar icon; the `.ico`
  embedded in `falog.exe` on Windows; the Start menu shortcut; files a macOS `.app` bundle and a Linux
  desktop entry can pick up.
- Out: an MSI/MSIX installer, the `.app` bundle and desktop entry themselves (macOS/Linux port, 0012).

## Acceptance criteria

- [ ] Explorer and the Start menu shortcut show the icon
- [ ] The window icon and the `.ico` come from the same artwork
- [ ] Builds on non-Windows targets are unaffected
- [ ] Recognizable at 16 px on light and dark taskbars; not a plain check mark
- [ ] `docs/icon-preview.png` shows it at every size; `design-system.md` has an "App icon" section
- [ ] `fmt`, `clippy`, `test` green

## Plan

1. Three concepts as SVG in `docs/icon-concepts`, compared at 16–64 px on dark and light backgrounds;
   refine the strongest into `crates/falog-desktop/assets/icon/falog.svg` plus hinted
   `falog-16.svg` and `falog-24.svg`.
2. `tools/icon`: a standalone crate (outside the workspace, so release builds do not compile it) that
   rasterizes the masters with resvg and writes `png/falog-N.png`, `falog.ico` (BMP entries up to
   128 px, PNG at 256) and `falog.icns` (Apple's 824/1024 grid with a drop shadow), plus the preview.
3. `main.rs` loads the 256 px PNG through `eframe::icon_data::from_png_bytes`; `app_icon()` goes away.
4. `build.rs` embeds `falog.ico` with `winresource` when the target is Windows.
5. `install.ps1` sets the shortcut's `IconLocation` to the executable.

Risks: the `.icns` cannot be checked on Windows (written to Apple's documented format; `iconutil`
on a Mac can rebuild it from the PNGs if Finder disagrees). Explorer caches icons; a stale one after
reinstalling is the cache, not the build.
