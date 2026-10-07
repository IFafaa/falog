//! Lucide icons as shipped in Zed's `assets/icons` (ISC license), recolored to white so they
//! can be tinted with any theme color.

use eframe::egui::{self, Color32, Image, ImageSource, Rect, Ui, include_image, vec2};
use falog_core::domain::{Priority, Status};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    ArrowDown,
    ArrowUp,
    ArrowUpRight,
    Board,
    Calendar,
    Check,
    ChevronDown,
    ChevronLeft,
    ChevronRight,
    Circle,
    Clock,
    Close,
    Command,
    Copy,
    Dash,
    Database,
    Flame,
    Folder,
    FolderOpen,
    List,
    MapPin,
    Maximize,
    Mic,
    Minimize,
    Notepad,
    Person,
    Plus,
    Reader,
    Refresh,
    Search,
    Send,
    Server,
    Settings,
    SidebarLeft,
    Sparkle,
    Stop,
    TodoComplete,
    TodoPending,
    TodoProgress,
    TodoWaiting,
    Trash,
    Video,
    Warning,
}

impl Icon {
    fn source(self) -> ImageSource<'static> {
        match self {
            Self::ArrowDown => include_image!("../assets/icons/arrow_down.svg"),
            Self::ArrowUp => include_image!("../assets/icons/arrow_up.svg"),
            Self::ArrowUpRight => include_image!("../assets/icons/arrow_up_right.svg"),
            Self::Board => include_image!("../assets/icons/blocks.svg"),
            Self::Calendar => include_image!("../assets/icons/calendar.svg"),
            Self::Check => include_image!("../assets/icons/check.svg"),
            Self::ChevronDown => include_image!("../assets/icons/chevron_down.svg"),
            Self::ChevronLeft => include_image!("../assets/icons/chevron_left.svg"),
            Self::ChevronRight => include_image!("../assets/icons/chevron_right.svg"),
            Self::Circle => include_image!("../assets/icons/circle.svg"),
            Self::Clock => include_image!("../assets/icons/clock.svg"),
            Self::Close => include_image!("../assets/icons/close.svg"),
            Self::Command => include_image!("../assets/icons/command.svg"),
            Self::Copy => include_image!("../assets/icons/copy.svg"),
            Self::Dash => include_image!("../assets/icons/dash.svg"),
            Self::Database => include_image!("../assets/icons/database_zap.svg"),
            Self::Flame => include_image!("../assets/icons/flame.svg"),
            Self::Folder => include_image!("../assets/icons/folder.svg"),
            Self::FolderOpen => include_image!("../assets/icons/folder_open.svg"),
            Self::List => include_image!("../assets/icons/list_todo.svg"),
            Self::MapPin => include_image!("../assets/icons/map_pin.svg"),
            Self::Maximize => include_image!("../assets/icons/maximize.svg"),
            Self::Mic => include_image!("../assets/icons/mic.svg"),
            Self::Minimize => include_image!("../assets/icons/minimize.svg"),
            Self::Notepad => include_image!("../assets/icons/notepad.svg"),
            Self::Person => include_image!("../assets/icons/person.svg"),
            Self::Plus => include_image!("../assets/icons/plus.svg"),
            Self::Reader => include_image!("../assets/icons/reader.svg"),
            Self::Refresh => include_image!("../assets/icons/rotate_cw.svg"),
            Self::Search => include_image!("../assets/icons/magnifying_glass.svg"),
            Self::Send => include_image!("../assets/icons/send.svg"),
            Self::Server => include_image!("../assets/icons/server.svg"),
            Self::Settings => include_image!("../assets/icons/settings.svg"),
            Self::SidebarLeft => include_image!("../assets/icons/threads_sidebar_left_open.svg"),
            Self::Sparkle => include_image!("../assets/icons/sparkle.svg"),
            Self::Stop => include_image!("../assets/icons/stop.svg"),
            Self::TodoComplete => include_image!("../assets/icons/todo_complete.svg"),
            Self::TodoPending => include_image!("../assets/icons/todo_pending.svg"),
            Self::TodoProgress => include_image!("../assets/icons/todo_progress.svg"),
            Self::TodoWaiting => include_image!("../assets/icons/countdown_timer.svg"),
            Self::Trash => include_image!("../assets/icons/trash.svg"),
            Self::Video => include_image!("../assets/icons/video.svg"),
            Self::Warning => include_image!("../assets/icons/warning.svg"),
        }
    }

    pub fn image(self, size: f32, tint: Color32) -> Image<'static> {
        Image::new(self.source())
            .fit_to_exact_size(vec2(size, size))
            .tint(tint)
    }

    /// Paints the icon centered in `rect`.
    pub fn paint(self, ui: &Ui, rect: Rect, size: f32, tint: Color32) {
        self.image(size, tint)
            .paint_at(ui, Rect::from_center_size(rect.center(), vec2(size, size)));
    }

    pub fn for_status(status: Status) -> Self {
        match status {
            Status::Todo => Self::TodoPending,
            Status::InProgress => Self::TodoProgress,
            Status::Waiting => Self::TodoWaiting,
            Status::Done => Self::TodoComplete,
        }
    }

    pub fn for_priority(priority: Priority) -> Self {
        match priority {
            Priority::Urgent => Self::Flame,
            Priority::High => Self::ArrowUp,
            Priority::Medium => Self::Dash,
            Priority::Low => Self::ArrowDown,
        }
    }
}

/// The window and taskbar icon: `assets/icon/falog.svg` as rasterized by `tools/icon`. eframe scales
/// it down to the sizes the OS asks for.
pub fn app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon/png/falog-256.png")).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_icon_decodes() {
        let icon = app_icon();
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
    }
}
