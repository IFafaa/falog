# Install on Linux

Requires [Rust](https://rustup.rs), a C toolchain, `pkg-config` and the ALSA headers; for voice, CMake
and libclang. The assistant uses [Claude Code](https://claude.com/claude-code), signed in with `claude`
once. On Debian and Ubuntu:

```sh
sudo apt install build-essential pkg-config libasound2-dev cmake libclang-dev
./scripts/install.sh
```

It builds in release mode and installs `falog` and `falog-mcp` to `~/.local/bin`, an app menu entry and
icon, an XDG autostart entry, the MCP server registered with Claude Code and the assistant workspace in
`~/falog-assistant`. Without CMake or libclang, Falog is built without voice.

Speech recognition runs on the GPU through Vulkan when the Vulkan headers and `glslc` are present at
build time (`sudo apt install libvulkan-dev glslc`); pass `--no-gpu` to skip it.

Falog runs on X11 and Wayland; on Wayland a second launch may not be able to bring the existing window to
the front.

`scripts/uninstall.sh [--remove-data]` reverts it. Your tasks live in `~/.local/share/Falog/falog.db`.
