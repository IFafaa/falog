# Falog

**A personal organizer you talk to.** Falog keeps your tasks, errands and appointments in one place, on
your machine. Tell your AI assistant what you need to do, by voice or text, and it files it for you.

## Why

Things to do show up everywhere: a call, a message, something you remember in the shower. Writing each
one down breaks your flow, so many never get written down, and the week starts with "what was I
supposed to do?". Falog removes the friction: say *"remind me to renew the car insurance, it's
important, by Friday"* and the task exists, with area, due date and priority.

## Install

- [Windows](docs/install/windows.md): installer, or build from source
- [macOS](docs/install/macos.md)
- [Linux](docs/install/linux.md)

## Google Calendar

Falog shows your meetings next to your tasks. The simplest way is each calendar's **secret address in
iCal format**, a private link Google gives you; no sign-in and nothing to set up:

1. Open [Google Calendar settings](https://calendar.google.com/calendar/r/settings) (the gear, then
   *Settings*).
2. On the left, under *Settings for my calendars*, click the calendar.
3. In *Integrate calendar*, copy the **Secret address in iCal format**.
4. In Falog, open Settings (`Ctrl+,`) → *Calendar*, paste it, and click **Add**. Repeat for each
   calendar and account (work, personal...).

The address works like a password: anyone who has it can read that calendar. Falog keeps it in
`calendar/google.json` next to the database and never shows it whole; if it leaks, *Reset* it in the same
Google page and add the new one. Falog reads the links every five minutes while the Calendar tab is open
(and on *Refresh*); Google may take a little while to publish a change in the feed.

### Sign in with Google instead (advanced)

Some work accounts hide the secret address (their admin turned it off). For those, Falog can sign in with
Google, which also shows new events right away and lets the assistant book meetings, but it needs
**your own** Google OAuth client, because Google does not let an open source app ship a shared one. Set
it up once, in about five minutes:

1. In the [Google Cloud console](https://console.cloud.google.com), create a project and enable the
   [Google Calendar API](https://console.cloud.google.com/apis/library/calendar-json.googleapis.com).
2. Open the [OAuth consent screen](https://console.cloud.google.com/auth/overview): pick *External*,
   fill in the app name and your email, and **publish** the app. While it is in *Testing*, Google
   signs you out every 7 days. Google shows an "unverified app" warning when you sign in; choose
   *Advanced → Go to …* (it is your own client).
3. In [Credentials](https://console.cloud.google.com/apis/credentials), create an *OAuth client ID*
   of type **Desktop app**.
4. In Falog, open Settings (`Ctrl+,`) → *Calendar* → *Sign in with Google instead*, paste the client ID
   and secret, save, and click **Connect Google account**. Repeat for each account.

Work accounts whose admins block third-party apps cannot connect this way either. While the app is in
*Testing*, add each email you connect under *Audience › Test users*, or Google answers "Access blocked".
Accounts connected before booking existed show *read only* in Settings until you click *Reconnect*.

Each email (account or calendar link) belongs to one area, picked in Settings › Calendar › *Area of each
account*; its meetings take the area's color and the assistant books that area's meetings in its account.

## Development

```powershell
cargo test --workspace                 # unit tests for every crate
cargo run -p falog-desktop             # the app, on your real database
.\scripts\seed-demo.ps1                # sample data in target\demo.db
$env:FALOG_DB = "$PWD\target\demo.db"; cargo run -p falog-desktop
```

On macOS and Linux:

```sh
./scripts/seed-demo.sh                 # sample data in target/demo.db
FALOG_DB="$PWD/target/demo.db" cargo run -p falog-desktop
```

`FALOG_DB` points both binaries at another database file. `cargo build --no-default-features` skips
voice input (no CMake or libclang needed); `--features falog-desktop/gpu` runs it on the GPU (Metal on
macOS, Vulkan elsewhere). The Windows installer is built with
[Inno Setup 6](https://jrsoftware.org/isinfo.php) by `.\scripts\build-installer.ps1`.

Design notes, conventions and the roadmap live in [`.spec/`](.spec/CLAUDE.md).

## Credits

Falog bundles the One Dark and One Light color values (MIT), IBM Plex Sans and Lilex (SIL Open Font
License) and Lucide icons (ISC); their licenses are in `crates/falog-desktop/assets`
(see `THIRD-PARTY-NOTICES.md`).

## License

[MIT](LICENSE)
