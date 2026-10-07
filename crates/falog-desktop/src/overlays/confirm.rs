use crate::action::Action;
use crate::components::{ButtonStyle, Placement, button, modal};
use crate::fonts;
use crate::theme::Theme;
use eframe::egui::{self, Align, Frame, Key, Layout, Margin, RichText};
use falog_core::domain::{AreaId, TaskId};

/// A destructive or lossy operation waiting for confirmation.
#[derive(Clone, Debug)]
pub enum Confirm {
    DeleteTask {
        id: TaskId,
        title: String,
    },
    DeleteArea {
        id: AreaId,
        name: String,
    },
    /// Leave the task panel with unsaved edits, then run `then` (if any).
    DiscardDraft {
        then: Option<Action>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Answer {
    Pending,
    Yes,
    No,
}

impl Confirm {
    fn copy(&self) -> (String, String, &'static str) {
        match self {
            Self::DeleteTask { id, title } => (
                format!("Delete task #{id}?"),
                format!("\"{title}\" and its activity log will be permanently deleted."),
                "Delete",
            ),
            Self::DeleteArea { name, .. } => (
                format!("Delete \"{name}\"?"),
                "Its tasks are kept, without an area.".to_owned(),
                "Delete",
            ),
            Self::DiscardDraft { .. } => (
                "Discard unsaved changes?".to_owned(),
                "The changes in the task panel will be lost.".to_owned(),
                "Discard",
            ),
        }
    }

    pub fn show(&self, ctx: &egui::Context) -> Answer {
        let theme = Theme::current(ctx);
        let (title, message, confirm_label) = self.copy();
        let (answer, dismissed) = modal(ctx, "confirm", 400.0, Placement::Center, |ui| {
            let mut answer = Answer::Pending;
            Frame::none().inner_margin(Margin::same(16.0)).show(ui, |ui| {
                ui.label(RichText::new(title).font(fonts::semibold(15.0)).color(theme.text));
                ui.add_space(4.0);
                ui.label(RichText::new(message).size(13.5).color(theme.text_muted));
                ui.add_space(14.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if button(ui, ButtonStyle::Danger, None, confirm_label).clicked() {
                        answer = Answer::Yes;
                    }
                    if button(ui, ButtonStyle::Ghost, None, "Cancel").clicked() {
                        answer = Answer::No;
                    }
                });
            });
            answer
        });
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            return Answer::Yes;
        }
        if dismissed { Answer::No } else { answer }
    }
}
