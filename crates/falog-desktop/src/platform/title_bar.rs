//! Paints the native Windows title bar with the theme's `title_bar` color, so the window
//! chrome matches the app (Windows 11; older versions only get dark mode).

use crate::theme::Theme;

#[cfg(windows)]
pub fn apply(frame: &eframe::Frame, theme: &Theme) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::c_void;

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(hwnd: isize, attribute: u32, value: *const c_void, size: u32) -> i32;
    }

    const USE_IMMERSIVE_DARK_MODE: u32 = 20;
    const BORDER_COLOR: u32 = 34;
    const CAPTION_COLOR: u32 = 35;
    const TEXT_COLOR: u32 = 36;

    let Ok(handle) = frame.window_handle() else { return };
    let RawWindowHandle::Win32(window) = handle.as_raw() else {
        return;
    };
    let hwnd = window.hwnd.get();

    let colorref =
        |c: eframe::egui::Color32| u32::from(c.r()) | u32::from(c.g()) << 8 | u32::from(c.b()) << 16;
    let attributes = [
        (USE_IMMERSIVE_DARK_MODE, u32::from(theme.dark)),
        (CAPTION_COLOR, colorref(theme.title_bar)),
        (TEXT_COLOR, colorref(theme.text_muted)),
        (BORDER_COLOR, colorref(theme.border)),
    ];
    for (attribute, value) in attributes {
        // SAFETY: `hwnd` is the live window owned by eframe, and `value` points to a u32 that
        // outlives the call, which is the layout DWM expects for these attributes.
        // Failures (e.g. unsupported attributes on Windows 10) are harmless and ignored.
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attribute,
                (&raw const value).cast(),
                size_of::<u32>() as u32,
            );
        }
    }
}

#[cfg(not(windows))]
pub fn apply(_frame: &eframe::Frame, _theme: &Theme) {}
