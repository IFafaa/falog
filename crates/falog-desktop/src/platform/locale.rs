//! The user's regional language, used to pick sensible defaults.

/// Whether the user's regional format (not necessarily the display language) is Portuguese.
#[cfg(windows)]
pub fn is_portuguese() -> bool {
    const LANG_PORTUGUESE: u32 = 0x16;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserDefaultLCID() -> u32;
    }
    // SAFETY: takes no arguments and only returns the current user's locale identifier.
    let lcid = unsafe { GetUserDefaultLCID() };
    lcid & 0x3ff == LANG_PORTUGUESE
}

/// Whether the user's regional format (not necessarily the display language) is Portuguese.
///
/// Apps started from Finder, the Dock or a LaunchAgent get no `LANG`, so this reads the regional
/// format from the user defaults (`AppleLocale`, e.g. `pt_BR` or `en_US@rg=brzzzz`).
#[cfg(target_os = "macos")]
pub fn is_portuguese() -> bool {
    let apple_locale = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLocale"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned());
    match apple_locale {
        Some(locale) => is_portuguese_locale(&locale),
        None => posix_locale(|name| std::env::var(name).ok()).is_some_and(|l| is_portuguese_locale(&l)),
    }
}

/// Whether the user's regional format (not necessarily the display language) is Portuguese.
#[cfg(not(any(windows, target_os = "macos")))]
pub fn is_portuguese() -> bool {
    posix_locale(|name| std::env::var(name).ok()).is_some_and(|l| is_portuguese_locale(&l))
}

/// The locale that formats dates and times, by POSIX precedence: `LC_ALL`, `LC_TIME`, then `LANG`.
#[cfg(any(not(windows), test))]
fn posix_locale(var: impl Fn(&str) -> Option<String>) -> Option<String> {
    ["LC_ALL", "LC_TIME", "LANG"]
        .into_iter()
        .filter_map(var)
        .find(|value| !value.trim().is_empty())
}

/// `pt`, `pt_BR`, `pt_PT.UTF-8`, `pt-BR`...; not `C`, `POSIX` or other languages.
#[cfg(any(not(windows), test))]
fn is_portuguese_locale(locale: &str) -> bool {
    let language = locale
        .trim()
        .split(['_', '-', '.', '@'])
        .next()
        .unwrap_or_default();
    language.eq_ignore_ascii_case("pt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_portuguese_locales() {
        for locale in ["pt", "pt_BR", "pt_PT.UTF-8", "pt-BR", "pt_BR@rg=brzzzz\n"] {
            assert!(is_portuguese_locale(locale), "{locale}");
        }
        for locale in ["", "C", "POSIX", "en_US.UTF-8", "en_BR", "ptx"] {
            assert!(!is_portuguese_locale(locale), "{locale}");
        }
    }

    #[test]
    fn posix_locale_follows_precedence() {
        let env = |vars: &'static [(&str, &str)]| {
            move |name: &str| {
                vars.iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| (*value).to_owned())
            }
        };
        assert_eq!(
            posix_locale(env(&[("LANG", "en_US.UTF-8")])).as_deref(),
            Some("en_US.UTF-8")
        );
        assert_eq!(
            posix_locale(env(&[("LANG", "en_US.UTF-8"), ("LC_TIME", "pt_BR.UTF-8")])).as_deref(),
            Some("pt_BR.UTF-8")
        );
        assert_eq!(
            posix_locale(env(&[("LC_ALL", "C"), ("LC_TIME", "pt_BR.UTF-8")])).as_deref(),
            Some("C")
        );
        assert_eq!(
            posix_locale(env(&[("LC_ALL", ""), ("LANG", "pt_BR")])).as_deref(),
            Some("pt_BR")
        );
        assert_eq!(posix_locale(env(&[])), None);
    }
}
