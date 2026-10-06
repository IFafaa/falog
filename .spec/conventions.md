# Conventions

## Language

Everything in the repository is in **English**: code, identifiers, comments, UI text, docs, commit
messages. User data (task titles, company names) is whatever the user writes. Input parsing may accept
other languages (status synonyms, weekday names) because users dictate in their own language.

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
  - types: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `style`, `build`, `ci`, `chore`
  - scopes: `core`, `mcp`, `desktop`, `assistant`, `scripts`, `spec`
  - imperative, lowercase, no trailing period, ≤ 72 chars; body explains why when not obvious.
- Specs change in the same commit as the behavior they describe.

## Workflow for a task

1. Write or update a task spec in [tasks/](tasks/README.md) (goal, scope, acceptance criteria).
2. Read the area specs it touches.
3. Implement in small commits; keep `fmt`, `clippy` and `test` green.
4. Verify behavior for real (run the app or call the MCP server), not only tests.
5. Update the specs and move the task to `done/`.
