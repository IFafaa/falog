//! Shows a file in the system file manager.

use std::io;
use std::path::Path;
use std::process::Command;

/// Label of the button that calls [`reveal`], naming the platform's file manager.
pub const LABEL: &str = if cfg!(windows) {
    "Show in Explorer"
} else if cfg!(target_os = "macos") {
    "Show in Finder"
} else {
    "Show in Files"
};

/// Opens a file manager window with `path` selected (on Linux, its folder when the file manager does
/// not support selecting).
pub fn reveal(path: &Path) -> io::Result<()> {
    imp::reveal(path)
}

#[cfg(windows)]
mod imp {
    use super::*;

    pub fn reveal(path: &Path) -> io::Result<()> {
        Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()
            .map(drop)
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;

    pub fn reveal(path: &Path) -> io::Result<()> {
        Command::new("open").arg("-R").arg(path).spawn().map(drop)
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    use super::*;

    /// Asks the file manager over D-Bus to select the file (Nautilus, Dolphin, Nemo, Thunar...), and
    /// falls back to opening the folder. Runs on a thread because it waits for `dbus-send`.
    pub fn reveal(path: &Path) -> io::Result<()> {
        let path = path.to_path_buf();
        std::thread::spawn(move || {
            let selected = Command::new("dbus-send")
                .args([
                    "--session",
                    "--print-reply",
                    "--dest=org.freedesktop.FileManager1",
                    "--type=method_call",
                    "/org/freedesktop/FileManager1",
                    "org.freedesktop.FileManager1.ShowItems",
                ])
                .arg(format!("array:string:{}", super::file_uri(&path)))
                .arg("string:")
                .output()
                .is_ok_and(|output| output.status.success());
            if !selected && let Some(dir) = path.parent() {
                let _ = Command::new("xdg-open").arg(dir).spawn();
            }
        });
        Ok(())
    }
}

/// `file://` URI for an absolute path, percent-encoding everything but unreserved characters and `/`.
#[cfg(any(not(any(windows, target_os = "macos")), test))]
fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                uri.push(char::from(byte));
            }
            _ => uri.push_str(&format!("%{byte:02X}")),
        }
    }
    uri
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uri_percent_encodes_the_path() {
        assert_eq!(
            file_uri(Path::new("/home/ana/.local/share/Falog/falog.db")),
            "file:///home/ana/.local/share/Falog/falog.db"
        );
        assert_eq!(
            file_uri(Path::new("/tmp/my tasks/ç,1.db")),
            "file:///tmp/my%20tasks/%C3%A7%2C1.db"
        );
    }
}
