//! Reusable widgets styled after Zed's `ui` crate.

mod button;
mod chip;
mod modal;
mod text;

pub use button::{ButtonStyle, button, icon_button, icon_toggle};
pub use chip::{badge, meta};
pub use modal::{Placement, modal, modal_header};
pub use text::{paint_keybinding, single_line};
