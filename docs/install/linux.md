# Install on Linux

## Download

On x86-64 (glibc 2.35 or newer: Ubuntu 22.04, Debian 12, Fedora 36 and later), run:

```sh
curl -fsSL https://github.com/IFafaa/falog/releases/latest/download/install.sh | sh
```

It downloads the latest Stable release, checks it against the release's `SHA256SUMS` and installs,
for your user only: `falog` and `falog-mcp` in `~/.local/bin`, an app menu entry and icon, launch at
sign-in, the MCP server registered with [Claude Code](https://claude.com/claude-code) when `claude` is
installed, and the assistant workspace in `~/falog-assistant`. Then it starts Falog. Run it again to update.

Options go after `sh -s --`, for example `… | sh -s -- --no-start`:

| Option | Does |
|---|---|
| `--preview` | install the Preview channel: the newest release, pre-releases included |
| `--version X.Y.Z` | install that release |
| `--assistant-dir DIR` | put the assistant workspace somewhere else |
| `--no-start` | do not start Falog afterwards |
| `--uninstall [--remove-data]` | remove Falog; with `--remove-data`, your tasks too |

Prefer to look first? Download
[`install.sh`](https://github.com/IFafaa/falog/releases/latest/download/install.sh), read it, and run
`sh install.sh`. The archive it installs is
[`falog-linux-x86_64.tar.gz`](https://github.com/IFafaa/falog/releases/latest/download/falog-linux-x86_64.tar.gz).

The app needs the ALSA library for the microphone (`libasound2`, present on desktop installs); the
installer says what is missing. Speech recognition runs on the CPU.

## Build from source

Requires [Rust](https://rustup.rs), a C toolchain, `pkg-config` and the ALSA headers; for voice, CMake
and libclang. On Debian and Ubuntu:

```sh
sudo apt install build-essential pkg-config libasound2-dev cmake libclang-dev
./scripts/install.sh
```

It builds in release mode and installs the same things as the download. Without CMake or libclang,
Falog is built without voice. Speech recognition runs on the GPU through Vulkan when the Vulkan headers
and `glslc` are present at build time (`sudo apt install libvulkan-dev glslc`); pass `--no-gpu` to skip
it. `scripts/uninstall.sh [--remove-data]` reverts it.

## Notes

Falog runs on X11 and Wayland; on Wayland a second launch may not be able to bring the existing window to
the front. Your tasks live in `~/.local/share/Falog/falog.db`.
