# Install on Windows

## Installer (no build needed)

Download `Falog-Setup-<version>.exe` and run it. It installs for your user only (no administrator
rights) into `%LOCALAPPDATA%\Falog`, with a Start menu shortcut and, if you keep the box ticked, launch
at sign-in. When [Claude Code](https://claude.com/claude-code) is installed, it also connects Falog to it.

The installer is not code-signed yet, so Windows SmartScreen may say it "protected your PC": choose
*More info → Run anyway*. Speech recognition runs on the CPU (it needs a processor with AVX2, from 2013
on) and downloads its model (~574 MB) the first time you dictate.

Uninstall from Windows Settings › Apps; your tasks stay in `%USERPROFILE%\.falog`.

## From source

Requires [Rust](https://rustup.rs) and the MSVC build tools (with CMake). For voice,
[LLVM](https://llvm.org) (`winget install LLVM.LLVM`); for speech recognition on the GPU, the
[Vulkan SDK](https://vulkan.lunarg.com). The assistant uses [Claude Code](https://claude.com/claude-code),
signed in with `claude` once.

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
```

It builds in release mode, installs to `%LOCALAPPDATA%\Falog` with a Start menu shortcut, enables launch
at sign-in, registers the MCP server with Claude Code and copies the assistant workspace to
`~\falog-assistant`. `scripts\uninstall.ps1 [-RemoveData]` reverts it.

Keep the checkout path short (like `C:\Projects\falog`) when building with the GPU: the Vulkan shader
build fails past Windows' 260-character path limit.

## Where your data lives

One SQLite file, `%USERPROFILE%\.falog\falog.db`, with the calendar settings and assistant threads next
to it. It is not in AppData because packaged apps such as Claude desktop give the processes they start a
private copy of AppData, so tasks filed from there would not show up in Falog. Older versions kept it in
`%APPDATA%\Falog`; Falog moves it on first start.
