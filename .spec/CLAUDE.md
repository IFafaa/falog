# Falog: guide for AI assistants and contributors

Falog is a local-first personal organizer: tasks, errands and appointments in one place. They are filed
by talking to an AI assistant through an MCP server, and shown in a fast, keyboard-friendly desktop app.
Everything is Rust; data lives in one SQLite file.

This folder (`.spec/`) is the source of truth for how the project works and how to change it.
Read the spec for the area you are touching **before** writing code, and update it in the same change
when behavior or structure changes.

| Spec | Read it when you... |
|---|---|
| [product.md](product.md) | need the why: users, goals, non-goals, vocabulary |
| [architecture.md](architecture.md) | add a module or crate, touch persistence, sync or startup |
| [domain.md](domain.md) | change tasks, statuses, priorities, dates or the Focus buckets |
| [mcp.md](mcp.md) | add or change an MCP tool or its output |
| [design-system.md](design-system.md) | build or restyle any UI |
| [conventions.md](conventions.md) | write code, tests or commits (always) |
| [roadmap.md](roadmap.md) | pick the next thing to build |
| [tasks/](tasks/README.md) | plan a feature: one spec file per task |

## Commands

```powershell
cargo fmt --all                          # format
cargo clippy --workspace --all-targets   # lint (CI denies warnings)
cargo test --workspace                   # tests
cargo run -p falog-desktop               # run the app (FALOG_DB=path to use another database)
.\scripts\seed-demo.ps1                  # demo data in target\demo.db
.\scripts\install.ps1                    # release build + install for the current user
```

On macOS and Linux the scripts are `./scripts/seed-demo.sh`, `./scripts/install.sh` and
`./scripts/uninstall.sh`. `--no-default-features` builds without voice input (no CMake or libclang).

CI (`.github/workflows/ci.yml`) runs fmt and shellcheck once; clippy (with and without voice) and tests on
Windows, macOS and Ubuntu; and clippy with `--features falog-desktop/gpu` on macOS (Metal) and Ubuntu
(Vulkan). Code behind `cfg(windows)`/`cfg(target_os = ...)` must stay warning-free on all three.

## Rules that always apply

@conventions.md
