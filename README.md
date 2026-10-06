# Falog

**A task board you talk to.** Falog is a local-first desktop app for developers who juggle work for
several companies at once. You tell your AI assistant about a request, by voice or text, and it files
the task for you; Falog shows everything on a Zed-inspired board, list and agenda.

![Falog board with the task panel open](docs/screenshots/board.png)

## Why

Requests arrive everywhere: a call, a chat message, a hallway comment. Writing each one down breaks your
flow, so many never get written down, and Monday morning starts with an hour of "what was I supposed to
do?". Falog removes the friction: say *"Ana from Acme asked me to fix the Google sign-in on Android,
it's urgent, needs to ship Friday"* and the task exists, with company, requester, due date and priority.

## Features

- **Board, List and Agenda views.** Drag cards between *To do*, *In progress*, *Waiting* and *Done*.
  The agenda groups open work into *Overdue*, *Due today*, *Due this week*, and so on.
- **Built for talking to an assistant.** `falog-mcp` exposes your tasks through the
  [Model Context Protocol](https://modelcontextprotocol.io), so Claude can create, update, annotate and
  summarize them. Due dates like "friday", "next week" or "amanhã" are understood.
- **Several companies, one place.** Each company has a color and a sidebar entry to filter by.
- **Zed's look and feel.** Same One Dark / One Light palettes, IBM Plex Sans, Lucide icons, a command
  palette (`Ctrl+Shift+P`) and a task finder (`Ctrl+P`).
- **Local and private.** A single SQLite file on your machine. No account, no server.
- **Always there.** Opens at sign-in and keeps a single window.

| Agenda | Command palette | Light theme |
|---|---|---|
| ![Agenda](docs/screenshots/agenda.png) | ![Command palette](docs/screenshots/command-palette.png) | ![List in light theme](docs/screenshots/list-light.png) |

## How it works

```
 you (voice / text) ──► Claude ──► falog-mcp ──┐
                                               ├──► SQLite  (%APPDATA%\Falog\falog.db)
                  falog (desktop app)  ◄───────┘    the app reloads when the file changes
```

The workspace has three crates:

| Crate | What it is |
|---|---|
| `falog-core` | Domain model, natural-language due dates, agenda rules and the SQLite store |
| `falog-mcp` | MCP server over stdio (`falog-mcp.exe`) |
| `falog-desktop` | The egui desktop app (`falog.exe`) |

Design notes, conventions and the roadmap live in [`.spec/`](.spec/CLAUDE.md).

## Install (Windows)

Requires [Rust](https://rustup.rs) and the MSVC build tools.

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
```

This builds in release mode, installs to `%LOCALAPPDATA%\Falog`, enables launch at sign-in, adds a Start
menu shortcut, registers the MCP server with Claude Code (`claude mcp add --scope user falog`) and copies
the assistant workspace to `~\falog-assistant`. `scripts\uninstall.ps1` reverts it.

## Talking to your tasks

Open a Claude Code session in `~\falog-assistant` (its `CLAUDE.md` teaches Claude how to file tasks) or
just mention tasks in any session, since the server is registered for your user. Dictate with your
client's microphone or **Win+H**.

- *"Got two things from Globex: review Carlos's payments PR today, and upgrade the API to Node 22 by Friday."*
- *"I'm blocked on the navbar until Lia signs off on the design."*
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
