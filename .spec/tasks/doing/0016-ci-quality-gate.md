# 0016: CI quality gate

**Status:** doing
**Area:** ci, scripts, docs

## Goal

Every push and pull request to `main` goes through one strict quality gate that says whether the code
is correct: formatted, lint-free with every feature set, documented without warnings, tested on
Windows, macOS and Linux, covered above a minimum, building on the declared minimum Rust, with
dependencies that are safe, licensed for MIT distribution and from crates.io, and a repository free of
secrets, conflict markers, typos and names of other apps used as design models. Contributors run the
same gate locally before pushing.

## Context

`.github/workflows/ci.yml` already runs fmt, shellcheck, clippy (with and without voice) and tests on
the three OSes, and clippy with the GPU feature on macOS and Ubuntu. It does not check docs,
coverage, the minimum Rust version, dependencies, secrets or commit messages, its actions are pinned
only to major tags, and branch protection would have to require each job by name. The repository is
about to be published on GitHub (`IFafaa/falog`), so the gate has to be in place first.

Validation only: no releases, packaging or publishing jobs (the Windows installer is separate work).
Rules being enforced come from [conventions.md](../../conventions.md) (language, naming other
products, code, commits).

## Scope

- In: a rewritten `ci.yml` (format, lint, docs, test, coverage, msrv, dependencies, repo policy,
  typos, commit messages, a single `quality-gate` job); `deny.toml`, `_typos.toml`;
  `scripts/check-policy.sh`, `scripts/check-commits.sh`; local runners `scripts/check.ps1` and
  `scripts/check.sh`; fixes for whatever the new checks find; README (badge, Development) and
  conventions (CI section).
- Out: CD, release builds, installers, publishing to crates.io, scheduled jobs, branch protection
  settings on GitHub (done by the owner once the repository exists).

## Acceptance criteria

- [ ] The workflow runs on push and pull request to `main`, cancels superseded runs, has
      `permissions: contents: read`, and pins every action to a commit SHA
- [ ] `format`: `cargo fmt --all -- --check`
- [ ] `lint`: clippy `-D warnings` on all targets for no default features, default features and the
      GPU feature where the runner can build it; `cargo doc` with `-D warnings`
- [ ] `test`: unit tests on Windows, macOS and Ubuntu, without default features and with them
- [ ] `coverage`: cargo-llvm-cov on Ubuntu, lcov uploaded as an artifact, fails under a documented
      minimum line coverage a few points below today's
- [ ] `msrv`: builds with the `rust-version` from `Cargo.toml`, which is the real minimum
- [ ] `dependencies`: cargo-deny (advisories, licenses, bans, sources) with a committed `deny.toml`
- [ ] `policy`: `scripts/check-policy.sh` fails on secrets, design-model app names and conflict
      markers; typos with a `_typos.toml` that accepts the Portuguese parsing words
- [ ] `commits`: Conventional Commits checked on the commits of a pull request
- [ ] `quality-gate` needs every job and fails if any failed or was cancelled
- [ ] `scripts/check.ps1` and `scripts/check.sh` run the same checks locally
- [ ] README (badge, Development) and conventions (CI: what each job checks, how to fix it) updated
- [ ] Everything that can run on Windows verified locally; the rest listed in the outcome

## Plan

1. Measure the baseline locally: fmt, clippy, tests, docs, coverage, `cargo +1.88` builds, cargo-deny
   and typos; fix what they find in their own commits.
2. Fix `rust-version` (1.85 today, but let-chains need 1.88) to the real minimum.
3. `deny.toml`, `_typos.toml`, `scripts/check-policy.sh`, `scripts/check-commits.sh`.
4. Rewrite `.github/workflows/ci.yml`; validate it with actionlint.
5. `scripts/check.ps1`, `scripts/check.sh`; README, conventions and `.spec/CLAUDE.md`.

Risks: whisper.cpp builds are slow on every OS, so the jobs rely on `Swatinem/rust-cache`; Windows
has no Vulkan SDK on the runner, so the GPU feature is linted on macOS (Metal) and Ubuntu (Vulkan)
only; advisories can start failing without a code change when a new one is published, which is the
point, and is fixed by updating or explicitly ignoring the crate with a reason.

## Outcome

Filled when done.
