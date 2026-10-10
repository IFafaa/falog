# Releases

People install Falog from a download: a setup program on Windows, a disk image on macOS, a one-line
installer on Linux. Those downloads are built and published by
[`.github/workflows/release.yml`](../.github/workflows/release.yml) when a version tag is pushed; normal
pushes only run CI.

## Versions

- One version for the whole workspace: `version` in `[workspace.package]` of the root `Cargo.toml`.
  Every crate inherits it; the app shows it in Settings › General › About and the MCP server reports it
  in `serverInfo` (both from `env!("CARGO_PKG_VERSION")`).
- [Semantic versioning](https://semver.org). While Falog is 0.x, the minor number goes up for new
  features or anything that changes behavior people rely on, the patch number for fixes only.
- Tags are the version with a `v`: `v0.2.0`. The workflow refuses a tag that does not match
  `Cargo.toml`.

## Channels

The tag decides the channel; both publish the same assets.

| Channel | Tag | On GitHub | Who it is for |
|---|---|---|---|
| **Stable** | `vX.Y.Z` | a normal release, marked latest: `releases/latest` serves it | everyone; the landing page offers it first |
| **Preview** | `vX.Y.Z-beta.N` or `vX.Y.Z-rc.N` | a pre-release, never latest | people who want to follow progress |

- Pre-release versions are valid semver and go in `Cargo.toml` as is (`version = "0.3.0-beta.1"`); the
  tag must match them like any other.
- Previews lead to a stable release of the same number: `0.3.0-beta.1`, `0.3.0-beta.2`, `0.3.0-rc.1`,
  then `0.3.0`. Fixes for the current stable release ship as its next patch on the Stable channel.
- The Linux installer installs Stable; `--preview` installs the newest release of either channel (the
  newest pre-release while one is ahead of Stable, so following Preview never goes back to an older
  build). The Windows and macOS downloads of a preview are on its release page.

## Release notes

The release body is [`scripts/release-notes.sh`](../scripts/release-notes.sh): the
[Conventional Commits](conventions.md#commits) since the previous release, grouped into Features
(`feat`), Fixes (`fix`), Performance (`perf`) and Other (`refactor`, `build`, `revert` and subjects that
are not conventional). `chore`, `ci`, `docs`, `style` and `test` commits are left out unless nothing
else changed. A stable release is compared with the previous stable tag, so its notes include what its
previews shipped; a preview with the previous tag of either channel. Breaking changes (`!`) are marked.
The landing page shows these notes as the changelog, so commit subjects are written for the people who
use Falog. Edit the body on the release page to add a summary.

## Cutting a release

1. `main` is green in CI. To try the downloads first, run the Release workflow from the Actions tab
   (*Run workflow* on `main`): it builds all of them and publishes nothing.
2. Bump `version` in the root `Cargo.toml`, then run `cargo update --workspace` so `Cargo.lock` follows
   (release builds use `--locked`).
3. Commit both as `chore: release vX.Y.Z` and land it on `main` like any change.
4. Tag that commit and push the tag:

   ```sh
   git tag -a vX.Y.Z -m "Falog X.Y.Z"
   git push origin vX.Y.Z
   ```

5. The Release workflow checks the tag, builds on Windows, macOS (Apple silicon and Intel) and Ubuntu,
   and creates the GitHub Release in the tag's channel with the release notes. It then starts the Pages
   workflow, which rebuilds the public site with the new release on its download and release pages.
   An edited release body reaches the site with the daily Pages run, or run Pages by hand.

If a build fails before anything is published, fix it on `main`, delete the tag
(`git push --delete origin vX.Y.Z`, `git tag -d vX.Y.Z`) and tag again. Once a release is public, ship
the fix as the next patch version instead of replacing it.

## Downloads

Asset names carry no version, so `https://github.com/IFafaa/falog/releases/latest/download/<name>`
always serves the newest Stable release; the landing page and the install guides link there.
A preview's files are at `https://github.com/IFafaa/falog/releases/download/<tag>/<name>`; the newest tag of
either channel is the first `tag_name` of `https://api.github.com/repos/IFafaa/falog/releases`.

| Asset | Built by | What it is |
|---|---|---|
| `Falog-Setup-x64.exe` | `scripts/build-installer.ps1 -OutputName Falog-Setup-x64` | Inno Setup installer (`installer/falog.iss`), per user, CPU speech recognition (AVX2) |
| `Falog-macOS-arm64.dmg`, `Falog-macOS-x64.dmg` | `scripts/package-macos.sh` | `Falog.app` (`scripts/macos-app.sh`, `falog-mcp` inside) and an Applications shortcut; speech recognition on Metal |
| `falog-linux-x86_64.tar.gz` | `scripts/package-linux.sh` | `falog`, `falog-mcp`, desktop entry, icons, assistant workspace; built on Ubuntu 22.04 (glibc 2.35 or newer) |
| `install.sh` | `scripts/install-release.sh`, pinned to the release's tag | `curl … \| sh` installer: downloads the archive above, checks `SHA256SUMS`, installs to `~/.local/bin` with menu entry, autostart, MCP registration and assistant workspace; `--preview`, `--uninstall` |
| `SHA256SUMS` | the workflow | SHA-256 of every other asset |

whisper.cpp is built with `GGML_NATIVE=OFF`, so the downloads run on any x86-64 CPU with AVX2 (2013 on)
instead of only on CPUs like the build machine's.

## Not signed yet

The Windows installer has no code signature (SmartScreen shows "Windows protected your PC"), and the
macOS app is signed ad hoc but not notarized (Gatekeeper blocks the first launch until it is allowed in
System Settings › Privacy & Security). The install guides in `docs/install/` walk through both.
Signing needs a paid certificate on each platform; it is on the [roadmap](roadmap.md).
