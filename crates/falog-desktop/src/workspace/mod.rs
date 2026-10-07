//! The window chrome around the views, laid out as a workspace: a left dock (sidebar),
//! a right dock (task panel), a tab bar with a breadcrumb toolbar, and a status bar.

pub mod sidebar;
pub mod status_bar;
pub mod tab_bar;
pub mod task_panel;
pub mod toolbar;

/// Height shared by the tab bar and dock headers.
pub const BAR_HEIGHT: f32 = 32.0;
