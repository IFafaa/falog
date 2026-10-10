//! Puts a downloaded update in place and starts the new version once this one has quit. Each system
//! installs its own download: the Windows setup program, the macOS disk image, the Linux archive.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Whether this copy of Falog was installed from a download (or the install scripts), so it can
/// update itself. A build run from a source checkout (`target/debug`, `target/release`) cannot, and a
/// demo database (`FALOG_DB`) never updates the real app.
pub fn can_update() -> bool {
    let from_checkout =
        std::env::current_exe().is_ok_and(|exe| exe.components().any(|part| part.as_os_str() == "target"));
    let demo = std::env::var_os(falog_core::store::DB_PATH_ENV).is_some_and(|path| !path.is_empty());
    !from_checkout && !demo && super::feed::asset_name().is_some()
}

/// Installs `download` and arranges for the new version to start; the caller then quits Falog.
pub fn install(download: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        windows(download)
    }
    #[cfg(target_os = "macos")]
    {
        macos(download)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        linux(download)
    }
}

/// Runs the setup program without its wizard. It waits for Falog to quit (it closes it if needed),
/// keeps the choices made at the first install, and starts Falog again (`/update=1`, see
/// installer/falog.iss).
#[cfg(windows)]
fn windows(setup: &Path) -> io::Result<()> {
    Command::new(setup)
        .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/update=1"])
        .spawn()
        .map(drop)
}

/// Copies the app out of the disk image next to the running one, swaps them, and opens the new app
/// once this process has quit.
#[cfg(target_os = "macos")]
fn macos(image: &Path) -> io::Result<()> {
    let exe = std::env::current_exe()?;
    let bundle = exe
        .ancestors()
        .find(|dir| dir.extension().is_some_and(|ext| ext == "app"))
        .ok_or_else(|| io::Error::other("Falog is not running from an app in Applications"))?
        .to_path_buf();
    let mount = std::env::temp_dir().join(format!("falog-update-{}", std::process::id()));
    std::fs::create_dir_all(&mount)?;
    run(Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly", "-quiet", "-mountpoint"])
        .arg(&mount)
        .arg(image))?;
    let staged = bundle.with_extension("app.new");
    let _ = std::fs::remove_dir_all(&staged);
    let copied = run(Command::new("ditto").arg(mount.join("Falog.app")).arg(&staged));
    let _ = run(Command::new("hdiutil").args(["detach", "-quiet"]).arg(&mount));
    copied?;
    swap(&bundle, &staged)?;
    relaunch(Command::new("open").arg(&bundle))
}

/// Unpacks the archive and replaces `falog` and `falog-mcp` next to the running binary, then starts
/// the new `falog` once this process has quit.
#[cfg(not(any(windows, target_os = "macos")))]
fn linux(archive: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let exe = std::env::current_exe()?;
    let dir = exe
        .parent()
        .ok_or_else(|| io::Error::other("Falog's folder was not found"))?;
    let unpacked = std::env::temp_dir().join(format!("falog-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&unpacked);
    std::fs::create_dir_all(&unpacked)?;
    run(Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("-C")
        .arg(&unpacked))?;
    let source = unpacked.join("falog-linux-x86_64");
    for name in ["falog", "falog-mcp"] {
        // A running binary cannot be written to, but it can be replaced: copy next to it, rename over.
        let target = dir.join(name);
        let staged = dir.join(format!(".{name}.new"));
        std::fs::copy(source.join(name), &staged)?;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
        std::fs::rename(&staged, &target)?;
    }
    let _ = std::fs::remove_dir_all(&unpacked);
    relaunch(&mut Command::new(&exe))
}

/// Replaces the app bundle `current` with `staged`, keeping the old one until the swap succeeded.
#[cfg(target_os = "macos")]
fn swap(current: &Path, staged: &Path) -> io::Result<()> {
    let old = current.with_extension("app.old");
    let _ = std::fs::remove_dir_all(&old);
    std::fs::rename(current, &old)?;
    if let Err(err) = std::fs::rename(staged, current) {
        let _ = std::fs::rename(&old, current);
        return Err(err);
    }
    let _ = std::fs::remove_dir_all(&old);
    Ok(())
}

/// Starts `command` once this process has exited: a second Falog would otherwise find this one still
/// running and only bring its window to the front.
#[cfg(unix)]
fn relaunch(command: &mut Command) -> io::Result<()> {
    let program = command.get_program().to_owned();
    let args: Vec<_> = command.get_args().map(ToOwned::to_owned).collect();
    Command::new("sh")
        .arg("-c")
        .arg("while kill -0 \"$0\" 2>/dev/null; do sleep 0.2; done; exec \"$@\"")
        .arg(std::process::id().to_string())
        .arg(program)
        .args(args)
        .spawn()
        .map(drop)
}

#[cfg(unix)]
fn run(command: &mut Command) -> io::Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{:?} failed ({status})",
            command.get_program()
        )))
    }
}

/// Where downloaded updates are kept until they are installed.
pub fn downloads_dir() -> PathBuf {
    falog_core::paths::data_home().join("updates")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_build_from_a_checkout_does_not_update_itself() {
        // Tests run from target/debug/deps.
        assert!(!can_update());
    }
}
