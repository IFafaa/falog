# 0012: macOS and Linux

**Status:** done
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

- [x] `cargo build` works on macOS and Linux with default features; `gpu` builds Metal on macOS and
      Vulkan on Linux and Windows (cross-checked without voice here; voice and gpu builds by CI)
- [x] The Settings toggle for launch at sign-in works on all three OSes (macOS/Linux: unit-tested
      file contents; not yet run on a Mac or a Linux desktop)
- [x] A second launch brings the running window forward instead of opening another one (unchanged
      portable code; not yet run on macOS/Linux)
- [x] `scripts/install.sh` installs Falog on macOS (`~/Applications/Falog.app`) and Linux
      (`~/.local/bin`, desktop entry, icon), enables launch at sign-in, registers the MCP server and
      starts the app; `uninstall.sh` reverts it (syntax-checked; not yet run on macOS/Linux)
- [x] CI runs fmt, clippy (`-D warnings`) and tests on Windows, macOS and Ubuntu (workflow written;
      runs on the first push)
- [x] Windows keeps working: fmt, clippy and tests green with `--no-default-features`, default and
      `--features falog-desktop/gpu`
- [x] Specs updated (`architecture.md`, `CLAUDE.md`), README has install steps per OS

## Plan

1. Scope the whisper.cpp MSVC flags to MSVC with a CMake project include
   (`cmake/whisper.cmake`), so `.cargo/config.toml` holds nothing compiler-specific.
2. `gpu` feature: a dependency declared per target (Metal on macOS, Vulkan elsewhere) that only adds
   backend features.
3. `platform/autostart.rs`: LaunchAgent plist and XDG `.desktop` writers, pure builders with tests.
4. `platform/reveal.rs` (Explorer / Finder / file manager) and the Settings label; locale from
   `AppleLocale` on macOS and `LC_ALL`/`LC_TIME`/`LANG` on Linux.
5. `platform/paths.rs`: executable lookup that also searches the usual install directories, used to
   find `claude` (keep the change to `assistant/claude.rs` to one call, the assistant is being
   rewritten in parallel).
6. `scripts/install.sh`, `uninstall.sh`, `seed-demo.sh`. (The icon files came from the parallel app
   icon work instead of a new SVG here.)
7. CI matrix; README and specs.

Risks: no Mac here, so macOS is verified by CI only (build, clippy, tests); the bundle, the
LaunchAgent and Metal need a run on a real Mac. Linux can be built and tested in a container.

## Outcome

What changed:

- **whisper.cpp flags.** `.cargo/config.toml` now only sets `CMAKE_PROJECT_INCLUDE` to
  `cmake/whisper.cmake`, which applies the MSVC `/O2` and AVX2 settings inside `if(MSVC)`. GCC and
  Clang keep ggml's defaults (native CPU tuning).
- **`gpu` feature.** Enables `whisper-rs-sys`, declared per target with `metal` (macOS) or `vulkan`
  (elsewhere). A renamed second copy of `whisper-rs` was the first idea; Cargo rejects a crate that
  depends on the same package under two names. whisper-rs's own backend features only forward to
  whisper-rs-sys (and set a default we override), so nothing is lost.
- **platform/.** `autostart.rs` (LaunchAgent `app.falog.Falog.plist`, XDG `falog.desktop`, with the
  desktop-entry `Exec` quoting), `reveal.rs` (Explorer, `open -R`, FileManager1 over D-Bus with an
  `xdg-open` fallback), `locale.rs` (`AppleLocale`; `LC_ALL`/`LC_TIME`/`LANG`), `paths.rs` (finding
  `claude` outside a login shell). `Theme::title_bar` is Windows-only and says so.
- **Scripts.** `install.sh` (macOS bundle with `Info.plist`, microphone usage description, ad hoc
  signature, LaunchAgent; Linux binaries, desktop entry, hicolor icons, autostart; both register the MCP
  server, copy the assistant workspace and start the app), `uninstall.sh`, `seed-demo.sh`. Icons come
  from the parallel app icon work (`assets/icon/`); without them the scripts skip the icon.
- **CI.** fmt + shellcheck once; clippy (no voice, default) and tests on Windows, macOS, Ubuntu; gpu
  clippy on macOS (Metal) and Ubuntu (Vulkan). Windows gpu is not in CI (no Vulkan SDK on the runner).

Verified, per OS:

| | Windows (this machine) | macOS | Linux |
|---|---|---|---|
| fmt | yes | n/a | n/a |
| clippy `-D warnings`, no voice | yes | yes, `--target aarch64-apple-darwin` with a stand-in C compiler | yes, `--target x86_64-unknown-linux-gnu`, stand-in C compiler and pkg-config |
| clippy, default (voice on CPU) | yes | CI | CI |
| clippy, `gpu` | yes (Vulkan) | CI (Metal) | CI (Vulkan) |
| tests | yes, all three feature sets | CI | CI |
| MSVC flags reach whisper.cpp | yes (`build.ninja`: `/O2 /Ob2`, `/arch:AVX2`) | n/a | n/a |
| `seed-demo.sh` | ran under Git Bash against `falog-mcp` | not run | not run |
| `install.sh`, `uninstall.sh` | `bash -n` only | not run | not run |
| App, autostart, reveal, locale, microphone | Windows paths unchanged | not run | not run |

There is no Mac here, and the only WSL distribution is Docker Desktop's, which did not start, so
nothing ran on macOS or Linux. The cross-target clippy found one real failure (dead code on
`Theme::title_bar`), fixed before CI was added.

Decisions:

- Launch at sign-in writes files (LaunchAgent, XDG autostart) rather than calling `SMAppService` or
  a portal: no extra dependencies, no admin, and the installer writes the same files.
- The macOS bundle is assembled by `install.sh` and signed ad hoc, not packaged; the microphone
  permission is asked again after a reinstall because the signature changes.
- On Windows, `--features falog-desktop/gpu` needs a short checkout path: in a worktree 10 characters
  longer than `C:\Projects\falog`, `rc.exe` in the Vulkan shader sub-build failed on the 260-character
  limit. README says so; use a short `CARGO_TARGET_DIR` for worktrees.

Follow-ups:

- Run `install.sh` on the Mac and the Linux machine; check the bundle icon, the LaunchAgent, the
  microphone prompt, Metal speed, and the D-Bus reveal.
- macOS shortcuts: `Cmd+Space` (dictation) is Spotlight's, so it never reaches the app; labels and
  tooltips say `Ctrl` where macOS uses `Cmd`.
- Wayland: a second launch cannot raise the window without an activation token.
- Signed and notarized macOS builds, `.dmg`, Linux packages (roadmap, "Later").
