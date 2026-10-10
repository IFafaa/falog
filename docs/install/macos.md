# Install on macOS

Requires [Rust](https://rustup.rs) and the Xcode command line tools (`xcode-select --install`); for
voice, CMake (`brew install cmake`). The assistant uses [Claude Code](https://claude.com/claude-code),
signed in with `claude` once.

```sh
./scripts/install.sh
```

It builds in release mode and installs `~/Applications/Falog.app` (in Launchpad, Spotlight and the Dock)
with `falog-mcp` inside it, a LaunchAgent for launch at sign-in, the MCP server registered with Claude
Code and the assistant workspace in `~/falog-assistant`. Speech recognition runs on the GPU through
Metal. Without CMake, Falog is built without voice.

macOS asks for the microphone the first time you dictate; the app is signed ad hoc on your Mac, so it
asks again after a reinstall.

`scripts/uninstall.sh [--remove-data]` reverts it. Your tasks live in
`~/Library/Application Support/Falog/falog.db`.
