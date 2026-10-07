# Falog

**A task board you talk to.** Falog is a local-first desktop app for developers who juggle work for
several companies at once and still have a life to run. You tell your AI assistant about a request or
an appointment, by voice or text, and it files the task for you; Falog shows everything on a Zed-inspired board, list and agenda.

![Falog board with the task panel open](docs/screenshots/board.png)

## Why

Requests arrive everywhere: a call, a chat message, a hallway comment. Writing each one down breaks your
flow, so many never get written down, and Monday morning starts with an hour of "what was I supposed to
do?". Falog removes the friction: say *"Ana from Acme asked me to fix the Google sign-in on Android,
it's urgent, needs to ship Friday"* and the task exists, with area, requester, due date and priority.

## Features

- **Board, List and Agenda views.** Drag cards between *To do*, *In progress*, *Waiting* and *Done*.
  The agenda groups open work into *Overdue*, *Due today*, *Due this week*, and so on.
- **An assistant you talk to, built in.** Open the assistant dock (`Ctrl+Shift+A`), press `Ctrl+Space`
  and say what changed: Claude creates, updates and summarizes tasks while the board updates live.
  Speech is transcribed locally with Whisper, and Claude runs on your own Claude Code login.
- **Works with any MCP client too.** `falog-mcp` exposes your tasks through the
  [Model Context Protocol](https://modelcontextprotocol.io). Due dates like "friday", "next week" or
  "amanhã" are understood.
- **Work and life, one place.** Tasks belong to areas: each employer or client, plus personal ones like
  *Personal* or *Health*. Each area has a color and a sidebar entry to filter by.
- **Zed's look and feel.** Same One Dark / One Light palettes, IBM Plex Sans, Lucide icons, a command
  palette (`Ctrl+Shift+P`) and a task finder (`Ctrl+P`).
- **Local and private.** A single SQLite file on your machine. No account, no server.
- **Always there.** Opens at sign-in and keeps a single window.

| Agenda | Command palette | Light theme |
|---|---|---|
| ![Agenda](docs/screenshots/agenda.png) | ![Command palette](docs/screenshots/command-palette.png) | ![List in light theme](docs/screenshots/list-light.png) |

## How it works

```
 assistant dock ──► Claude Code (headless) ──┐
 (speech: Whisper, on device)                ├──► falog-mcp ──┐
 any MCP client ──► Claude ──────────────────┘                ├──► SQLite (%APPDATA%\Falog\falog.db)
                       falog (desktop app)  ◄─────────────────┘    the app reloads when the file changes
```

The workspace has three crates:

| Crate | What it is |
|---|---|
| `falog-core` | Domain model, natural-language due dates, agenda rules and the SQLite store |
| `falog-mcp` | MCP server over stdio (`falog-mcp.exe`) |
| `falog-desktop` | The egui desktop app (`falog.exe`) |

Design notes, conventions and the roadmap live in [`.spec/`](.spec/CLAUDE.md).

## Install (Windows)

Requires [Rust](https://rustup.rs) and the MSVC build tools (with CMake). Voice input also needs
[LLVM](https://llvm.org) at build time (`winget install LLVM.LLVM`); without it the installer builds
Falog without voice. The assistant dock uses [Claude Code](https://claude.com/claude-code), signed in
with `claude` once. It can also talk to any agent that speaks the
[Agent Client Protocol](https://agentclientprotocol.com): Claude through its ACP adapter (Node.js 22+),
[Gemini CLI](https://github.com/google-gemini/gemini-cli) and Codex are built in, and Settings › Agents
adds your own (`name`, command, environment). Each one runs on your own login or subscription.

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
```

This builds in release mode, installs to `%LOCALAPPDATA%\Falog`, enables launch at sign-in, adds a Start
menu shortcut, registers the MCP server with Claude Code (`claude mcp add --scope user falog`) and copies
the assistant workspace to `~\falog-assistant`. `scripts\uninstall.ps1` reverts it.

## Talking to your tasks

**In Falog:** open the assistant dock with the ✦ button in the status bar or `Ctrl+Shift+A`. Type, or
press `Ctrl+Space` (or the mic button), speak, and press it again: your words are transcribed on this
computer and sent. The first time, Falog downloads the Whisper model (~574 MB). Pick your voice
language and whether dictation sends right away in Settings. Like Zed, each conversation is a thread:
`+` starts a new one and the clock button lists past threads, so you can keep one per area and pick up
any of them later, even after a restart. The composer footer picks the model (Opus, Sonnet, Haiku...),
effort and permission mode of the thread, toggles fast mode, and shows how full the context is;
typing `/` lists the agent's commands (`/compact`, `/context`...). `Shift+Esc` zooms the assistant
to fill the window.

**From Claude Code or another MCP client:** open a session in `~\falog-assistant` (its `CLAUDE.md`
teaches Claude how to file tasks) or mention tasks in any session, since the server is registered for
your user.

- *"Got two things from Globex: review Carlos's payments PR today, and upgrade the API to Node 22 by Friday."*
- *"I'm blocked on the navbar until Lia signs off on the design."*
- *"Dentist on Thursday at 3, put it in Personal."*
- *"Good morning, what's on my plate this week?"*

Any MCP client works; point it at `falog-mcp.exe`.

## Keyboard shortcuts

| Keys | Action |
|---|---|
| `Ctrl+Shift+P` | Command palette |
| `Ctrl+P` | Find a task |
| `Ctrl+N` | New task |
| `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | Board / List / Agenda |
| `Ctrl+F` | Search |
| `Ctrl+B` | Toggle sidebar |
| `Ctrl+Shift+A` | Toggle the assistant |
| `Shift+Esc` | Zoom the assistant to fill the window, and back |
| `Ctrl+Space` | Start / finish dictating to the assistant |
| `Ctrl+S` | Save the task being edited |
| `Ctrl+,` | Settings |
| `Esc` | Close the task panel or a dialog |

## Development

```powershell
cargo test --workspace                 # unit tests for every crate
cargo run -p falog-desktop             # the app, on your real database
.\scripts\seed-demo.ps1                # sample data in target\demo.db
$env:FALOG_DB = "$PWD\target\demo.db"; cargo run -p falog-desktop
```

`FALOG_DB` points both binaries at another database file.

## Credits

Falog's visual design follows [Zed](https://zed.dev). It bundles the One Dark and One Light color values
(MIT), IBM Plex Sans and Lilex (SIL Open Font License) and Lucide icons (ISC) as shipped in the Zed
repository; their licenses are next to the assets in `crates/falog-desktop/assets`.

## License

[MIT](LICENSE)
