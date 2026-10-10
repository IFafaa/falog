# Install on Windows

## Download

1. Download [**Falog-Setup-x64.exe**](https://github.com/IFafaa/falog/releases/latest/download/Falog-Setup-x64.exe)
   (Windows 10 or 11, 64-bit) and run it.
2. The installer is not code-signed yet, so Windows SmartScreen may say it "protected your PC": choose
   *More info → Run anyway*.

It installs for your user only (no administrator rights) into `%LOCALAPPDATA%\Falog`, with a Start menu
shortcut and, if you keep the box ticked, launch at sign-in. When
[Claude Code](https://claude.com/claude-code) is installed, it also connects Falog to it. Speech
recognition runs on the CPU (it needs a processor with AVX2, from 2013 on) and downloads its model
(~574 MB) the first time you dictate. To update, download and run the newest installer.

Uninstall from Windows Settings › Apps; your tasks stay in `%USERPROFILE%\.falog`. Other versions and
release notes: [releases](https://github.com/IFafaa/falog/releases).

## Build from source

Requires [Rust](https://rustup.rs) and the MSVC build tools (with CMake). For voice,
[LLVM](https://llvm.org) (`winget install LLVM.LLVM`); for speech recognition on the GPU, the
[Vulkan SDK](https://vulkan.lunarg.com). The assistant uses [Claude Code](https://claude.com/claude-code),
signed in with `claude` once.

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
```

It builds in release mode, installs to `%LOCALAPPDATA%\Falog` with a Start menu shortcut, enables launch
at sign-in, registers the MCP server with Claude Code and copies the assistant workspace to
`~\falog-assistant`. `scripts\uninstall.ps1 [-RemoveData]` reverts it. `scripts\build-installer.ps1`
builds the setup program itself (needs [Inno Setup](https://jrsoftware.org/isinfo.php)).

Keep the checkout path short (like `C:\Projects\falog`) when building with the GPU: the Vulkan shader
build fails past Windows' 260-character path limit.

## Where your data lives

One SQLite file, `%USERPROFILE%\.falog\falog.db`, with the calendar settings and assistant threads next
to it. It is not in AppData because packaged apps such as Claude desktop give the processes they start a
private copy of AppData, so tasks filed from there would not show up in Falog. Older versions kept it in
`%APPDATA%\Falog`; Falog moves it on first start.
