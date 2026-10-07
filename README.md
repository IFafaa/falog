# Falog

**A task board you talk to.** Falog is a local-first desktop app for developers who juggle work for
several companies at once and still have a life to run. You tell your AI assistant about a request or
an appointment, by voice or text, and it files the task for you; Falog shows everything on a Zed-inspired board, list, focus view and calendar.

![Falog board with the task panel open](docs/screenshots/board.png)

## Why

Requests arrive everywhere: a call, a chat message, a hallway comment. Writing each one down breaks your
flow, so many never get written down, and Monday morning starts with an hour of "what was I supposed to
do?". Falog removes the friction: say *"Ana from Acme asked me to fix the Google sign-in on Android,
it's urgent, needs to ship Friday"* and the task exists, with area, requester, due date and priority.

## Features

- **Board, List and Focus views.** Drag cards between *To do*, *In progress*, *Waiting* and *Done*.
  Focus groups open work into *Overdue*, *Due today*, *Due this week*, and so on.
- **Your meetings too.** A Calendar tab shows Google Calendar from as many accounts as you like (work
  and personal), by week or month, next to the tasks due each day.
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

| Focus | Command palette | Light theme |
|---|---|---|
| ![Focus](docs/screenshots/focus.png) | ![Command palette](docs/screenshots/command-palette.png) | ![List in light theme](docs/screenshots/list-light.png) |

## How it works

```
 assistant dock ──► Claude Code (headless) ──┐
 (speech: Whisper, on device)                ├──► falog-mcp ──┐
 any MCP client ──► Claude ──────────────────┘                ├──► SQLite (%APPDATA%\Falog\falog.db)
                       falog (desktop app)  ◄─────────────────┘    the app reloads when the file changes
```

The workspace has four crates:

| Crate | What it is |
|---|---|
| `falog-core` | Domain model, natural-language due dates, agenda rules and the SQLite store |
| `falog-mcp` | MCP server over stdio (`falog-mcp.exe`) |
| `falog-calendar` | Read-only Google Calendar: OAuth sign-in, calendars, events, local cache |
| `falog-desktop` | The egui desktop app (`falog.exe`) |

Design notes, conventions and the roadmap live in [`.spec/`](.spec/CLAUDE.md).

## Install (Windows)

Requires [Rust](https://rustup.rs) and the MSVC build tools (with CMake). Voice input also needs
[LLVM](https://llvm.org) at build time (`winget install LLVM.LLVM`); without it the installer builds
Falog without voice. The assistant dock uses [Claude Code](https://claude.com/claude-code), signed in
with `claude` once.

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install.ps1
```

This builds in release mode, installs to `%LOCALAPPDATA%\Falog`, enables launch at sign-in, adds a Start
menu shortcut, registers the MCP server with Claude Code (`claude mcp add --scope user falog`) and copies
the assistant workspace to `~\falog-assistant`. `scripts\uninstall.ps1` reverts it.

## Talking to your tasks

**In Falog:** open the assistant dock with the ✦ button in the status bar or `Ctrl+Shift+A`. Type, or
press `Ctrl+Space` (or the mic button), speak, and press it again: your words are transcribed on this
computer and sent. The first time, Falog downloads the Whisper model (~574 MB). Pick the Claude model,
your voice language and whether dictation sends right away in Settings. Like Zed, each conversation
is a thread: `+` starts a new one and the clock button lists past threads, so you can keep one per
area and pick up any of them later, even after a restart.

**From Claude Code or another MCP client:** open a session in `~\falog-assistant` (its `CLAUDE.md`
teaches Claude how to file tasks) or mention tasks in any session, since the server is registered for
your user.

- *"Got two things from Globex: review Carlos's payments PR today, and upgrade the API to Node 22 by Friday."*
- *"I'm blocked on the navbar until Lia signs off on the design."*
- *"Dentist on Thursday at 3, put it in Personal."*
- *"Good morning, what's on my plate this week?"*

Any MCP client works; point it at `falog-mcp.exe`.

## Google Calendar

![Calendar week view](docs/screenshots/calendar-week.png)

Falog reads your meetings (read-only) with **your own** Google OAuth client, because Google does not
let an open source app ship a shared one. Set it up once, in about five minutes:

1. In the [Google Cloud console](https://console.cloud.google.com), create a project and enable the
   [Google Calendar API](https://console.cloud.google.com/apis/library/calendar-json.googleapis.com).
2. Open the [OAuth consent screen](https://console.cloud.google.com/auth/overview): pick *External*,
   fill in the app name and your email, and **publish** the app. While it is in *Testing*, Google
   signs you out every 7 days. Google shows an "unverified app" warning when you sign in; choose
   *Advanced → Go to …* (it is your own client).
3. In [Credentials](https://console.cloud.google.com/apis/credentials), create an *OAuth client ID*
   of type **Desktop app**.
4. In Falog, open Settings (`Ctrl+,`) → *Google Calendar*, paste the client ID and secret, save, and
   click **Connect Google account**. Repeat for each account (work, personal...).

The sidebar of the Calendar tab lists each account's calendars to show or hide. Tokens and the event
cache live in `calendar/` next to the database; removing an account in Settings revokes its token.
Work accounts whose admins block third-party apps cannot connect.

## Keyboard shortcuts

| Keys | Action |
|---|---|
| `Ctrl+Shift+P` | Command palette |
| `Ctrl+P` | Find a task |
| `Ctrl+N` | New task |
| `Ctrl+1` … `Ctrl+4` | Board / List / Focus / Calendar |
| `T` / `J` / `K` / `W` / `M` | Calendar: today / next / previous / week / month |
| `Ctrl+F` | Search |
| `Ctrl+B` | Toggle sidebar |
| `Ctrl+Shift+A` | Toggle the assistant |
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
