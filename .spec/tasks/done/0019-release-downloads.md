# 0019: Release downloads

**Status:** done
**Area:** scripts | docs

## Goal

Anyone installs Falog from a download, without cloning or building: a setup program on Windows, a disk
image on macOS, a `curl | sh` line on Linux, always at the same `releases/latest/download/<name>` links
the landing page uses.

## Context

Until now each install built Falog from source (`scripts/install.*`), except the Windows installer,
which was built by hand. Decisions: releases are published by GitHub Actions only when a version tag is
pushed; the macOS app is signed ad hoc, not notarized, for now; the CI quality gate (`ci.yml`) stays a
separate workflow. How it works now: [releases.md](../../releases.md).

## Scope

- In: `.github/workflows/release.yml`; packaging scripts for the three platforms; the Linux installer
  with checksums and `--uninstall` and `--preview`; Stable and Preview channels; release notes from
  the commits; the version in Settings; install guides with the download first.
- Out: code signing and notarization, Linux packages (`.deb`, Flatpak), ARM Linux, automatic updates.

## Acceptance criteria

- [x] Pushing `vX.Y.Z` fails unless it matches `Cargo.toml`, then publishes `Falog-Setup-x64.exe`,
      `Falog-macOS-arm64.dmg`, `Falog-macOS-x64.dmg`, `falog-linux-x86_64.tar.gz`, `install.sh` and
      `SHA256SUMS`
- [x] `install.sh` from a release installs that release, refuses an archive that does not match
      `SHA256SUMS`, and `--uninstall` removes what it installed
- [x] Settings › General shows the version
- [x] Two channels by tag: `vX.Y.Z` is Stable (latest), `vX.Y.Z-beta.N`/`-rc.N` is Preview (pre-release);
      release notes grouped from the Conventional Commits; `install.sh --preview`
- [x] Specs and install guides updated; shellcheck and actionlint clean

## Outcome

Verified on Windows: the installer builds as `Falog-Setup-x64.exe`; `package-linux.sh` and
`install-release.sh` ran end to end in a shell with stub `uname`, `curl` and `claude` (install,
tampered checksum, `--version`, `--uninstall`, piped). The macOS and Linux build jobs only run on
GitHub's runners, so the first tag is their first real run. whisper.cpp is built with
`GGML_NATIVE=OFF` so the downloads do not require the build machine's CPU features.
