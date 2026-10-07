//! Launch at sign-in, for the current user only (no admin rights needed):
//!
//! - Windows: the `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Falog` registry value;
//! - macOS: a LaunchAgent, `~/Library/LaunchAgents/app.falog.Falog.plist`;
//! - Linux and other XDG desktops: `~/.config/autostart/falog.desktop`.
//!
//! Each one starts the running executable, so enable it from the installed copy.

#[cfg(windows)]
mod imp {
    use std::io;
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE: &str = "Falog";

    pub fn is_enabled() -> bool {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(RUN_KEY)
            .and_then(|key| key.get_value::<String, _>(VALUE))
            .is_ok()
    }

    pub fn set_enabled(enabled: bool) -> io::Result<()> {
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY)?;
        if enabled {
            let exe = std::env::current_exe()?;
            key.set_value(VALUE, &format!("\"{}\"", exe.display()))
        } else {
            match key.delete_value(VALUE) {
                Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
                result => result,
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use std::io;
    use std::path::{Path, PathBuf};

    fn missing_home() -> io::Error {
        io::Error::new(io::ErrorKind::NotFound, "the home folder could not be found")
    }

    /// The file that makes the OS start Falog at sign-in.
    #[cfg(target_os = "macos")]
    fn entry_path() -> io::Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(missing_home)?;
        let file = format!("{}.plist", super::LAUNCH_AGENT_LABEL);
        Ok(home.join("Library").join("LaunchAgents").join(file))
    }

    #[cfg(not(target_os = "macos"))]
    fn entry_path() -> io::Result<PathBuf> {
        let config = dirs::config_dir().ok_or_else(missing_home)?;
        Ok(config.join("autostart").join("falog.desktop"))
    }

    #[cfg(target_os = "macos")]
    fn entry_contents(exe: &Path) -> String {
        super::launch_agent_plist(exe)
    }

    #[cfg(not(target_os = "macos"))]
    fn entry_contents(exe: &Path) -> String {
        super::autostart_desktop_entry(exe)
    }

    pub fn is_enabled() -> bool {
        entry_path().is_ok_and(|path| path.is_file())
    }

    pub fn set_enabled(enabled: bool) -> io::Result<()> {
        let path = entry_path()?;
        if enabled {
            let exe = std::env::current_exe()?;
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, entry_contents(&exe))
        } else {
            match std::fs::remove_file(&path) {
                Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
                result => result,
            }
        }
    }
}

pub use imp::{is_enabled, set_enabled};

/// LaunchAgent label, also the plist file name. Matches the bundle id `install.sh` writes.
#[cfg(any(target_os = "macos", test))]
const LAUNCH_AGENT_LABEL: &str = "app.falog.Falog";

/// A LaunchAgent that starts `exe` once at sign-in. launchd does not restart it after it quits.
#[cfg(any(target_os = "macos", test))]
fn launch_agent_plist(exe: &std::path::Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LAUNCH_AGENT_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>ProcessType</key>
    <string>Interactive</string>
</dict>
</plist>
"#,
        xml_escape(&exe.to_string_lossy())
    )
}

#[cfg(any(target_os = "macos", test))]
fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// An XDG autostart entry that starts `exe` when the desktop session starts.
#[cfg(any(not(any(windows, target_os = "macos")), test))]
fn autostart_desktop_entry(exe: &std::path::Path) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Falog\n\
         Comment=Task board you talk to\n\
         Exec={}\n\
         Icon=falog\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n",
        desktop_exec_quote(&exe.to_string_lossy())
    )
}

/// Quotes a path for the `Exec` key of a desktop entry: inside double quotes, `"`, `` ` ``, `$` and `\`
/// are escaped with a backslash, and the string format then doubles every backslash.
#[cfg(any(not(any(windows, target_os = "macos")), test))]
fn desktop_exec_quote(path: &str) -> String {
    let mut quoted = String::from("\"");
    for c in path.chars() {
        match c {
            '"' | '`' | '$' => {
                quoted.push_str("\\\\");
                quoted.push(c);
            }
            '\\' => quoted.push_str("\\\\\\\\"),
            '%' => quoted.push_str("%%"),
            _ => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn launch_agent_runs_the_executable_at_load() {
        let plist = launch_agent_plist(Path::new(
            "/Users/ana/Applications/Falog.app/Contents/MacOS/falog",
        ));
        assert!(plist.contains("<string>app.falog.Falog</string>"));
        assert!(plist.contains("<string>/Users/ana/Applications/Falog.app/Contents/MacOS/falog</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>\n    <true/>"));
    }

    #[test]
    fn launch_agent_escapes_the_path() {
        let plist = launch_agent_plist(Path::new("/Users/a&b/<falog>"));
        assert!(plist.contains("<string>/Users/a&amp;b/&lt;falog&gt;</string>"));
    }

    #[test]
    fn desktop_entry_quotes_the_executable() {
        let entry = autostart_desktop_entry(Path::new("/home/ana/.local/bin/falog"));
        assert!(entry.starts_with("[Desktop Entry]\n"));
        assert!(entry.contains("\nExec=\"/home/ana/.local/bin/falog\"\n"));
    }

    #[test]
    fn desktop_exec_escapes_reserved_characters() {
        assert_eq!(
            desktop_exec_quote("/opt/my apps/falog"),
            r#""/opt/my apps/falog""#
        );
        assert_eq!(desktop_exec_quote("/a$b"), r#""/a\\$b""#);
        assert_eq!(desktop_exec_quote(r#"/a"b"#), r#""/a\\"b""#);
        assert_eq!(desktop_exec_quote(r"/a\b"), r#""/a\\\\b""#);
        assert_eq!(desktop_exec_quote("/100%/falog"), r#""/100%%/falog""#);
    }
}
