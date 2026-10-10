# Install on macOS

## Download

1. Download the disk image for your Mac (macOS 11 or newer):
   - Apple silicon (M1 and later):
     [**Falog-macOS-arm64.dmg**](https://github.com/IFafaa/falog/releases/latest/download/Falog-macOS-arm64.dmg)
   - Intel: [**Falog-macOS-x64.dmg**](https://github.com/IFafaa/falog/releases/latest/download/Falog-macOS-x64.dmg)

   Not sure which? Apple menu › About This Mac: "Chip: Apple M…" is Apple silicon.
2. Open it and drag **Falog** onto **Applications**.
3. Open Falog from Applications. The app is not notarized by Apple yet, so the first launch is blocked
   ("Apple could not verify…"): click *Done*, open System Settings › Privacy & Security, scroll to
   *Security*, click **Open Anyway** next to Falog and confirm. Later launches open normally.

macOS asks for the microphone the first time you dictate; speech recognition runs on the GPU through
Metal. Turn on launch at sign-in in Falog's Settings › General. To use Falog from
[Claude Code](https://claude.com/claude-code), copy the command in Settings › Data and run it once in a
terminal. To update, download the newest image and replace the app.

Uninstall by moving Falog from Applications to the Trash; your tasks stay in
`~/Library/Application Support/Falog/falog.db`. Other versions, release notes and Preview builds
(pre-releases, to follow progress): [releases](https://github.com/IFafaa/falog/releases).

## Build from source

Requires [Rust](https://rustup.rs) and the Xcode command line tools (`xcode-select --install`); for
voice, CMake (`brew install cmake`). The assistant uses [Claude Code](https://claude.com/claude-code),
signed in with `claude` once.

```sh
./scripts/install.sh
```

It builds in release mode and installs `~/Applications/Falog.app` (in Launchpad, Spotlight and the Dock)
with `falog-mcp` inside it, a LaunchAgent for launch at sign-in, the MCP server registered with Claude
Code and the assistant workspace in `~/falog-assistant`. Without CMake, Falog is built without voice.
The app is signed ad hoc on your Mac, so macOS asks for the microphone again after a reinstall.

`scripts/uninstall.sh [--remove-data]` reverts it.
