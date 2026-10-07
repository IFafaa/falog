# 0012: macOS and Linux

**Status:** doing
**Area:** desktop, scripts, docs

## Goal

Falog builds, installs and runs on macOS and Linux as well as on Windows: the same app, the same MCP
server and the same assistant, with an install script per OS and CI that checks all three.

## Context

Falog was built Windows-first: launch at sign-in writes the `HKCU\...\Run` registry key, the title bar
is painted through DWM, the installers are PowerShell, and `.cargo/config.toml` forces MSVC compiler
flags on whisper.cpp for every target. The user also works on a Mac and a Linux machine and wants
their board on all of them. See [architecture.md](../../architecture.md#os-integration).

Audit of what is Windows-only today:

| Piece | Today | macOS / Linux |
|---|---|---|
| Launch at sign-in (`platform/autostart.rs`) | `HKCU\...\Run\Falog` | stub that errors |
| Title bar (`platform/title_bar.rs`) | DWM colors | no-op (native title bar) |
| Single instance (`platform/single_instance.rs`) | loopback port | same code, portable |
| Regional format (`platform/locale.rs`) | `GetUserDefaultLCID` | `$LANG` only; unset for macOS apps |
| "Show in Explorer" (`overlays/settings.rs`) | `explorer /select,` | spawns a missing `explorer` |
| Finding `claude` (`assistant/claude.rs`) | `PATH`, `~/.local/bin` | apps started from Finder, the Dock or a LaunchAgent get a bare `PATH` |
| `CREATE_NO_WINDOW` (`assistant/claude.rs`) | `cfg(windows)` | not needed |
| whisper.cpp flags (`.cargo/config.toml`) | MSVC `/O2`, AVX2 forced for every target | `/O2` breaks gcc/clang; AVX2 is wrong on Apple silicon |
| `gpu` feature | `whisper-rs/vulkan` | Vulkan on Linux; macOS should use Metal |
| Data and binaries | `%APPDATA%\Falog`, `%LOCALAPPDATA%\Falog` | `dirs::data_dir()` already portable; install location undefined |
| Installers | `install.ps1`, `uninstall.ps1`, `seed-demo.ps1` | none |
| CI | `windows-latest` only | none |
| Microphone | no permission needed | macOS asks once and needs `NSMicrophoneUsageDescription` in the bundle |

Fonts are bundled and there are no file dialogs, so neither needs work. Shortcuts already use egui's
`Modifiers::COMMAND`, which is Cmd on macOS.

## Scope

- In: launch at sign-in on macOS (LaunchAgent) and Linux (XDG autostart); reveal the database in
  Finder or the file manager; regional format on macOS and Linux; finding `claude` outside a login
  shell; whisper.cpp flags scoped to MSVC; `gpu` = Metal on macOS, Vulkan elsewhere; `install.sh`,
  `uninstall.sh`, `seed-demo.sh` (a minimal `Falog.app` on macOS, a desktop entry and icon on Linux);
  a CI matrix on Windows, macOS and Ubuntu; README and specs per OS.
- Out: signed or notarized macOS builds, `.dmg`, Flatpak/AppImage/`.deb` packages, Windows installer
  changes, a native macOS menu bar, Wayland window activation for the single-instance "show" request.

## Acceptance criteria

- [ ] `cargo build` works on macOS and Linux with default features; `gpu` builds Metal on macOS and
      Vulkan on Linux and Windows
- [ ] The Settings toggle for launch at sign-in works on all three OSes
- [ ] A second launch brings the running window forward instead of opening another one
- [ ] `scripts/install.sh` installs Falog on macOS (`~/Applications/Falog.app`) and Linux
      (`~/.local/bin`, desktop entry, icon), enables launch at sign-in, registers the MCP server and
      starts the app; `uninstall.sh` reverts it
- [ ] CI runs fmt, clippy (`-D warnings`) and tests on Windows, macOS and Ubuntu
- [ ] Windows keeps working: fmt, clippy and tests green with `--no-default-features`, default and
      `--features falog-desktop/gpu`
- [ ] Specs updated (`architecture.md`, `CLAUDE.md`), README has install steps per OS

## Plan

1. Scope the whisper.cpp MSVC flags to MSVC with a CMake project include
   (`cmake/whisper.cmake`), so `.cargo/config.toml` holds nothing compiler-specific.
2. `gpu` feature: a `whisper-rs-gpu` dependency declared per target (Metal on macOS, Vulkan
   elsewhere) that only adds whisper-rs features.
3. `platform/autostart.rs`: LaunchAgent plist and XDG `.desktop` writers, pure builders with tests.
4. `platform/reveal.rs` (Explorer / Finder / file manager) and the Settings label; locale from
   `AppleLocale` on macOS and `LC_ALL`/`LC_TIME`/`LANG` on Linux.
5. `platform/paths.rs`: executable lookup that also searches the usual install directories, used to
   find `claude` (keep the change to `assistant/claude.rs` to one call, the assistant is being
   rewritten in parallel).
6. `scripts/install.sh`, `uninstall.sh`, `seed-demo.sh`, and an SVG app icon matching `app_icon`.
7. CI matrix; README and specs.

Risks: no Mac here, so macOS is verified by CI only (build, clippy, tests); the bundle, the
LaunchAgent and Metal need a run on a real Mac. Linux can be built and tested in a container.

## Outcome

What changed, decisions taken, follow-ups. Filled when done.
