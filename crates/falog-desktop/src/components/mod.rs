//! Reusable widgets of the design system (see `.spec/design-system.md`).

mod button;
mod chip;
mod modal;
mod picker;
mod switch;
mod text;

pub use button::{ButtonStyle, button, icon_button, icon_toggle};
pub use chip::{badge, meta};
pub use modal::{Placement, modal, modal_header};
pub use picker::{area_picker, priority_picker, status_picker};
pub use switch::{segmented, switch};
pub use text::{paint_keybinding, single_line};
