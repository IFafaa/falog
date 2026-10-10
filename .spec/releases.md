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
- A version with a suffix (`0.3.0-rc.1`) is published as a pre-release: it gets its own download page,
  and the `latest` links keep pointing at the last stable release.
- Tags are the version with a `v`: `v0.2.0`. The workflow refuses a tag that does not match
  `Cargo.toml`.

## Cutting a release

1. `main` is green in CI.
2. Bump `version` in the root `Cargo.toml`, then run `cargo update --workspace` so `Cargo.lock` follows
   (release builds use `--locked`).
3. Commit both as `chore: release vX.Y.Z` and land it on `main` like any change.
4. Tag that commit and push the tag:

   ```sh
   git tag -a vX.Y.Z -m "Falog X.Y.Z"
   git push origin vX.Y.Z
   ```

5. The Release workflow checks the tag, builds on Windows, macOS (Apple silicon and Intel) and Ubuntu,
   and creates the GitHub Release with notes generated from the merged pull requests. Edit the notes on
   the release page if they need a summary.

If a build fails before anything is published, fix it on `main`, delete the tag
(`git push --delete origin vX.Y.Z`, `git tag -d vX.Y.Z`) and tag again. Once a release is public, ship
the fix as the next patch version instead of replacing it.

## Downloads

Asset names carry no version, so `https://github.com/IFafaa/falog/releases/latest/download/<name>`
always serves the newest release; the landing page and the install guides link there.

| Asset | Built by | What it is |
|---|---|---|
| `Falog-Setup-x64.exe` | `scripts/build-installer.ps1 -OutputName Falog-Setup-x64` | Inno Setup installer (`installer/falog.iss`), per user, CPU speech recognition (AVX2) |
| `Falog-macOS-arm64.dmg`, `Falog-macOS-x64.dmg` | `scripts/package-macos.sh` | `Falog.app` (`scripts/macos-app.sh`, `falog-mcp` inside) and an Applications shortcut; speech recognition on Metal |
| `falog-linux-x86_64.tar.gz` | `scripts/package-linux.sh` | `falog`, `falog-mcp`, desktop entry, icons, assistant workspace; built on Ubuntu 22.04 (glibc 2.35 or newer) |
| `install.sh` | `scripts/install-release.sh`, pinned to the release's tag | `curl … \| sh` installer: downloads the archive above, checks `SHA256SUMS`, installs to `~/.local/bin` with menu entry, autostart, MCP registration and assistant workspace; `--uninstall` |
| `SHA256SUMS` | the workflow | SHA-256 of every other asset |

whisper.cpp is built with `GGML_NATIVE=OFF`, so the downloads run on any x86-64 CPU with AVX2 (2013 on)
instead of only on CPUs like the build machine's.

## Not signed yet

The Windows installer has no code signature (SmartScreen shows "Windows protected your PC"), and the
macOS app is signed ad hoc but not notarized (Gatekeeper blocks the first launch until it is allowed in
System Settings › Privacy & Security). The install guides in `docs/install/` walk through both.
Signing needs a paid certificate on each platform; it is on the [roadmap](roadmap.md).
