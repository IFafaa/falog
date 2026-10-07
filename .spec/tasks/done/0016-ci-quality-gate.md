# 0016: CI quality gate

**Status:** done
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

- [x] The workflow runs on push and pull request to `main`, cancels superseded runs, has
      `permissions: contents: read`, and pins every action to a commit SHA
- [x] `format`: `cargo fmt --all -- --check`
- [x] `lint`: clippy `-D warnings` on all targets for no default features, default features and the
      GPU feature where the runner can build it; `cargo doc` with `-D warnings`
- [x] `test`: unit tests on Windows, macOS and Ubuntu, without default features and with them
- [x] `coverage`: cargo-llvm-cov on Ubuntu, lcov uploaded as an artifact, fails under a documented
      minimum line coverage a few points below today's
- [x] `msrv`: builds with the `rust-version` from `Cargo.toml`, which is the real minimum
- [x] `dependencies`: cargo-deny (advisories, licenses, bans, sources) with a committed `deny.toml`
- [x] `policy`: `scripts/check-policy.sh` fails on secrets, design-model app names and conflict
      markers; typos with a `_typos.toml` that accepts the Portuguese parsing words
- [x] `commits`: Conventional Commits checked on the commits of a pull request
- [x] `quality-gate` needs every job and fails if any failed or was cancelled
- [x] `scripts/check.ps1` and `scripts/check.sh` run the same checks locally
- [x] README (badge, Development) and conventions (CI: what each job checks, how to fix it) updated
- [x] Everything that can run on Windows verified locally; the rest listed in the outcome

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

`.github/workflows/ci.yml` is one workflow with these jobs; branch protection should require only
`quality-gate`. The [CI section of conventions.md](../../conventions.md#ci) says how to fix each one.

| Job | Covers |
|---|---|
| `format` | `cargo fmt --all -- --check` |
| `lint` (8 jobs) | clippy `--all-targets --locked -- -D warnings`: Ubuntu and macOS with no default features, default features and `gpu` (Vulkan, Metal); Windows with no default and default features (no Vulkan SDK on the runner) |
| `docs` | `cargo doc --workspace --no-deps --document-private-items` with `RUSTDOCFLAGS=-D warnings`, default features, Ubuntu |
| `test` (6 jobs) | `cargo test --workspace --locked` on Ubuntu, macOS and Windows, with and without default features |
| `coverage` | cargo-llvm-cov without default features on Ubuntu; `lcov.info` uploaded as the `lcov` artifact; fails under `MIN_LINE_COVERAGE` = **42%** |
| `msrv` | `cargo check --workspace --all-targets --locked` with the `rust-version` read from `Cargo.toml` (**1.88**) |
| `dependencies` | `cargo deny --locked check` with `deny.toml` |
| `policy` | `scripts/check-policy.sh`, typos (`_typos.toml`), shellcheck, actionlint (checksum-verified release) |
| `commits` | pull requests only: `scripts/check-commits.sh base..head` |
| `quality-gate` | needs all of them; fails on any result other than success (`commits` may be skipped on pushes) |

All jobs run with `RUSTFLAGS=-D warnings`, `permissions: contents: read`, `persist-credentials:
false`, timeouts, cancel-in-progress concurrency and `Swatinem/rust-cache` (saved from `main` only).
Every action is pinned to a commit SHA: checkout v7.0.1, rust-toolchain (master, `toolchain:`
input), rust-cache v2.9.2, install-action v2.87.26 (cargo-llvm-cov, cargo-deny), upload-artifact
v7.0.2, typos v1.51.1. The Ubuntu packages live in the local action `.github/actions/linux-packages`.

Decisions:

- **Coverage**: 44.86% of lines (14,132 lines, 7,793 missed) on 2026-10-07, measured on Windows with
  the same flags; falog-core, falog-mcp and falog-calendar are mostly 80-100%, the egui UI pulls the
  total down. The minimum is 42% to leave room for code that differs per OS; it is a ratchet, raised
  as tests are added.
- **MSRV**: `rust-version` was 1.85 but the code uses let chains: 1.87 fails with E0658 and 1.88
  builds the workspace with and without voice (also under `-D warnings`), so it is now 1.88.
- **cargo-deny**: licenses allow-listed for MIT distribution, plus Unlicense (whisper-rs),
  CDLA-Permissive-2.0 (webpki-roots) and, for `epaint_default_fonts` only, OFL-1.1 and the Ubuntu Font
  License. RUSTSEC-2026-0194/0195 (quick-xml 0.30) are ignored with the reason: it only reaches Falog
  on Linux through eframe 0.29's accessibility stack, reading D-Bus interface files bundled with
  atspi; an eframe upgrade removes it. Unmaintained advisories fail for direct dependencies only;
  duplicate versions are warnings (about 35 in the UI stack). Path dependencies needed `publish =
  false`, which the crates now declare.
- **Policy**: the script found two spec lines naming other apps as design models (0009 and the
  calendar view in design-system.md); both were rephrased. typos needed only the Portuguese parsing
  words and a few deliberate prefixes.
- **Docs**: rustdoc found an ambiguous link in falog-mcp, fixed; private items document cleanly, so
  the job checks them too.

Verified locally on Windows: `scripts/check.ps1` (including `-NoVoice` and `-Gpu`, so the Windows
Vulkan build is linted) and `scripts/check.sh` under Git Bash pass end to end (fmt, clippy for the
three feature sets, 177 tests with and without voice, docs, policy, cargo-deny 0.20.2, typos
1.51.1, commit subjects); a planted secret fails the run; cargo-llvm-cov 0.9.1 measures the numbers
above and `--fail-under-lines` passes at 42 and fails at 50; the MSRV builds; actionlint 1.7.12
passes on the workflow (without shellcheck, which is not installed here); the gate's result logic
was exercised with sample results.

Only on GitHub, on the first run: the Ubuntu and macOS jobs (including Linux coverage, which may
differ slightly from the Windows figure), Windows on the runner image, shellcheck of `scripts/*.sh`
and of the workflow's `run:` blocks, the typos action, the `commits` job on a pull request, and the
badge. Then require `quality-gate` in the branch protection of `main`.

Follow-ups: `tools/icon` (its own crate, outside the workspace) is not built or linted by CI; raise
`MIN_LINE_COVERAGE` once the first Linux run shows its figure.
