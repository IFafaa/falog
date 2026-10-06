//! Reusable widgets styled after Zed's `ui` crate.

mod button;
mod chip;
mod text;

pub use button::{ButtonStyle, button, icon_button, icon_toggle};
pub use chip::{badge, meta};
pub use text::{paint_keybinding, single_line};
