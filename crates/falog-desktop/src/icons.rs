//! Lucide icons as shipped in Zed's `assets/icons` (ISC license), recolored to white so they
//! can be tinted with any theme color.

use eframe::egui::{self, Color32, Image, ImageSource, Rect, Ui, include_image, vec2};
use falog_core::domain::{Priority, Status};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    ArrowDown,
    ArrowUp,
    Board,
    Check,
    ChevronDown,
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
    Maximize,
    Mic,
    Minimize,
    Notepad,
    Person,
    Plus,
    Reader,
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
    Warning,
}

impl Icon {
    fn source(self) -> ImageSource<'static> {
        match self {
            Self::ArrowDown => include_image!("../assets/icons/arrow_down.svg"),
            Self::ArrowUp => include_image!("../assets/icons/arrow_up.svg"),
            Self::Board => include_image!("../assets/icons/blocks.svg"),
            Self::Check => include_image!("../assets/icons/check.svg"),
            Self::ChevronDown => include_image!("../assets/icons/chevron_down.svg"),
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
            Self::Maximize => include_image!("../assets/icons/maximize.svg"),
            Self::Mic => include_image!("../assets/icons/mic.svg"),
            Self::Minimize => include_image!("../assets/icons/minimize.svg"),
            Self::Notepad => include_image!("../assets/icons/notepad.svg"),
            Self::Person => include_image!("../assets/icons/person.svg"),
            Self::Plus => include_image!("../assets/icons/plus.svg"),
            Self::Reader => include_image!("../assets/icons/reader.svg"),
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

/// The window/taskbar icon, drawn in code: a rounded One Dark tile with an accent check mark.
pub fn app_icon() -> egui::IconData {
    const SIZE: usize = 64;
    fn segment_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
    }

    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    let (half, radius) = (SIZE as f32 / 2.0, 14.0f32);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let qx = (p.0 - half).abs() - (half - radius - 1.0);
            let qy = (p.1 - half).abs() - (half - radius - 1.0);
            let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() - radius;
            if outside > 0.0 {
                continue;
            }
            let alpha = ((-outside).min(1.0) * 255.0) as u8;
            let check = segment_distance(p, (18.0, 33.0), (28.0, 43.0)).min(segment_distance(
                p,
                (28.0, 43.0),
                (47.0, 22.0),
            ));
            let pixel = if check < 4.0 {
                [0x74, 0xad, 0xe8, alpha]
            } else {
                [0x28, 0x2c, 0x33, alpha]
            };
            rgba[(y * SIZE + x) * 4..][..4].copy_from_slice(&pixel);
        }
    }
    egui::IconData {
        rgba,
        width: SIZE as u32,
        height: SIZE as u32,
    }
}
