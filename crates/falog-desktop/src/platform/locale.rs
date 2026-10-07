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

#[cfg(not(windows))]
pub fn is_portuguese() -> bool {
    std::env::var("LANG").is_ok_and(|lang| lang.starts_with("pt"))
}
