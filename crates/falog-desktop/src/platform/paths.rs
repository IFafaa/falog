//! Finding command-line tools the app runs, such as `claude`.

use std::path::{Path, PathBuf};

/// Finds the executable `name` (without extension) on `PATH`, then in the folders installers commonly
/// use. Apps started from Finder, the Dock, a LaunchAgent or a desktop autostart do not inherit the
/// `PATH` of the user's shell, so tools installed in `~/.local/bin` or Homebrew are often missing
/// from it.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    let path = std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .unwrap_or_default();
    path.into_iter()
        .chain(fallback_dirs(dirs::home_dir().as_deref()))
        .map(|dir| dir.join(&file))
        .find(|candidate| candidate.is_file())
}

/// Install folders searched after `PATH`, in order.
fn fallback_dirs(home: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = home {
        // Claude Code's native installer, on every platform.
        dirs.push(home.join(".local").join("bin"));
        if cfg!(unix) {
            // Claude Code's older per-user install, and npm's user prefix.
            dirs.push(home.join(".claude").join("local"));
            dirs.push(home.join(".npm-global").join("bin"));
        }
    }
    if cfg!(unix) {
        // Homebrew on Apple silicon, then Homebrew on Intel and manual installs.
        dirs.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searches_the_native_installer_folder_first() {
        let dirs = fallback_dirs(Some(Path::new("/home/ana")));
        assert_eq!(dirs[0], Path::new("/home/ana").join(".local").join("bin"));
        if cfg!(unix) {
            assert!(dirs.contains(&PathBuf::from("/opt/homebrew/bin")));
        }
    }

    #[test]
    fn needs_no_home_folder() {
        assert_eq!(fallback_dirs(None).is_empty(), !cfg!(unix));
    }

    #[test]
    fn finds_executables_on_path() {
        // `cargo` runs the tests, so it is on PATH or in ~/.cargo/bin, which rustup puts on PATH.
        let found = find_executable("cargo");
        assert!(found.is_some_and(|path| path.is_file()));
    }
}
