# Falog

**A personal organizer you talk to.** Falog keeps your tasks, errands and appointments in one place, on
your machine. Tell your AI assistant what you need to do, by voice or text, and it files it for you.

Downloads, features and roadmap: **<https://ifafaa.github.io/falog/>**

## Why

Things to do show up everywhere: a call, a message, something you remember in the shower. Writing each
one down breaks your flow, so many never get written down, and the week starts with "what was I
supposed to do?". Falog removes the friction: say *"remind me to renew the car insurance, it's
important, by Friday"* and the task exists, with area, due date and priority.

## Install

- [Windows](docs/install/windows.md): installer, or build from source
- [macOS](docs/install/macos.md)
- [Linux](docs/install/linux.md)

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

Before pushing, `.scriptseck.ps1` (or `./scripts/check.sh`) runs what the CI quality gate checks.

Before pushing, `.\scripts\check.ps1` (or `./scripts/check.sh`) runs what the CI quality gate checks.

Design notes, conventions and the roadmap live in [`.spec/`](.spec/CLAUDE.md).

## Credits

Falog bundles the One Dark and One Light color values (MIT), IBM Plex Sans and Lilex (SIL Open Font
License) and Lucide icons (ISC); their licenses are in `crates/falog-desktop/assets`
(see `THIRD-PARTY-NOTICES.md`).

## License

[MIT](LICENSE)
