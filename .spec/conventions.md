# Conventions

## Language

Everything in the repository is in **English**, no exceptions: file and folder names, crates,
modules, identifiers, comments, UI text, docs, specs, scripts, test data and commit messages. When
you find something in another language, rename it in its own commit.

- Prefer words that read as English to an English speaker and are not ambiguous in the product:
  the urgency view is called *Focus* so it is not confused with the calendar.
- User data (task titles, area names) is whatever the user writes. Input parsing may accept other
  languages (status synonyms, weekday names, "amanhã") because users dictate in their own language;
  those words live only in parsing tables and their tests.
- Conversation with the user may happen in any language; what lands in the repository is English.

## Naming other products

Describe Falog on its own terms. Code, comments, UI text and docs do not name other apps as the model
for a design ("like X", "X-style", "X's panel"); say what the thing is instead. Third-party assets are
credited only where their licenses require it, in `crates/falog-desktop/assets/THIRD-PARTY-NOTICES.md`.
Services Falog integrates with (Google Calendar, Claude Code, ACP agents) are named, since they are
what the feature is about.

## Code

- Rust 2024 edition, `cargo fmt` with the repo's `rustfmt.toml`, `cargo clippy` clean (CI denies
  warnings).
- Domain rules live in `falog-core`. Binaries orchestrate; they do not re-implement rules.
- Name things after the domain (`Task`, `due`, `requester`), not the storage or the widget.
- Newtypes for ids; enums for closed sets; `Option` for absence. No stringly-typed state.
- Errors: `falog_core::Error` (thiserror) in the library, `anyhow` at binary edges. User-facing
  messages are full sentences without trailing punctuation in toasts.
- No `unwrap()`/`expect()` outside tests, except for invariants documented in a comment.
- `unsafe` only for FFI, with a `// SAFETY:` comment.
- Comments explain *why*. Doc comments on public items say what they promise.
- UI code emits `Action`s instead of mutating state (see [architecture.md](architecture.md#frame-flow)).

## Tests

- Unit tests next to the code (`#[cfg(test)] mod tests`).
- Store tests use `Store::open_in_memory()`; date tests pin `today` explicitly.
- MCP behavior is tested through `Server::handle_line`, the same path real clients use.
- Every bug fix comes with a test that fails without it.

## Commits

- **Atomic**: one logical change per commit, and every commit builds and passes tests.
- [Conventional Commits](https://www.conventionalcommits.org): `type(scope): summary`
  - types: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `style`, `build`, `ci`, `chore`,
    `revert`; `!` after the type or scope marks a breaking change
  - scopes (optional, lowercase): `core`, `mcp`, `calendar`, `desktop`, `assistant`, `scripts`, `spec`
  - imperative, lowercase, no trailing period, ≤ 72 chars for the whole subject; body explains why
    when not obvious. CI checks the subjects of every pull request ([CI](#ci)).
- Specs change in the same commit as the behavior they describe.

## CI

Every push and pull request to `main` runs [`.github/workflows/ci.yml`](../.github/workflows/ci.yml).
Branch protection requires one check, **quality-gate**, which fails when any job below failed or was
cancelled. Run the same checks before pushing with `.\scripts\check.ps1` or `./scripts/check.sh`
(`-NoVoice` / `--no-voice` skips the builds that need CMake and libclang; cargo-deny and typos run
when installed: `cargo install --locked cargo-deny typos-cli`).

| Job | Checks | When it fails |
|---|---|---|
| `format` | `cargo fmt --all -- --check` | Run `cargo fmt --all` |
| `lint` | `cargo clippy --workspace --all-targets -- -D warnings` on Windows, macOS and Ubuntu, with `--no-default-features`, default features and `--features falog-desktop/gpu` (macOS and Ubuntu only: the Windows runner has no Vulkan SDK) | Fix the warning; `#[allow]` only with a comment saying why. Code behind `cfg(windows)` / `cfg(target_os)` is only linted on its OS, so read the log of the failing one |
| `docs` | `cargo doc --workspace --no-deps --document-private-items` with `RUSTDOCFLAGS=-D warnings` | Usually a broken or ambiguous intra-doc link: link `` [`name()`] `` for a function, `` [`mod@name`] `` for a module |
| `test` | `cargo test --workspace` on the three OSes, with and without default features | Reproduce with the same features locally; on another OS, read the test's assertion in the log |
| `coverage` | `cargo llvm-cov` on Ubuntu without default features; the lcov report is the `lcov` artifact of the run; fails under `MIN_LINE_COVERAGE` (42%, set at 44.9% measured on 2026-10-07) | Add tests for the code you changed. The minimum only goes up: raise it when coverage grows, never lower it to let a change pass |
| `msrv` | `cargo check --workspace --all-targets` with the `rust-version` of `Cargo.toml` (1.88: let chains) | Avoid the newer API or language feature, or raise `rust-version` on purpose in its own commit |
| `dependencies` | `cargo deny check` with [`deny.toml`](../deny.toml): advisories (vulnerabilities, yanked, unmaintained direct dependencies), licenses compatible with MIT distribution, wildcard versions, sources other than crates.io; duplicate versions are warnings | Update the crate (`cargo update -p name`). An advisory that does not apply goes in `ignore` with the reason; a new license only after checking it allows shipping Falog under MIT |
| `policy` | [`scripts/check-policy.sh`](../scripts/check-policy.sh): secrets (Google client secrets, OAuth tokens and secret iCal addresses, private keys, GitHub, Anthropic, AWS and Slack tokens), database and credential files, other apps named as design models ([Naming other products](#naming-other-products)), merge conflict markers; typos with [`_typos.toml`](../_typos.toml); shellcheck; actionlint | Remove the secret and rotate it (it is public once pushed); rephrase the text on Falog's own terms; for a word typos gets wrong on purpose (a Portuguese parsing word), add it to `_typos.toml` with the reason |
| `commits` | Pull requests only: [`scripts/check-commits.sh`](../scripts/check-commits.sh) on the commits the PR adds ([Commits](#commits)) | `git rebase -i` and reword the commit |

Coverage, the minimum Rust version and the operating systems other than yours run only in CI.

## Workflow for a task

1. Write or update a task spec in [tasks/](tasks/README.md) (goal, scope, acceptance criteria).
2. Read the area specs it touches.
3. Implement in small commits; keep `fmt`, `clippy` and `test` green.
4. Verify behavior for real (run the app or call the MCP server), not only tests.
5. Update the specs and move the task to `done/`.
