//! Command palette (Ctrl+Shift+P) and task finder (Ctrl+P).

use crate::action::Action;
use crate::components::{Placement, modal, paint_keybinding, single_line};
use crate::i18n::{area_name, tr, trf};
use crate::icons::Icon;
use crate::overlays::settings::SettingsTab;
use crate::theme::{self, Theme, ThemeMode};
use crate::views::View;
use eframe::egui::{
    self, Color32, CursorIcon, FontId, Id, Key, Margin, Modifiers, Rect, ScrollArea, Sense, Stroke, TextEdit,
    Ui, pos2, vec2,
};
use falog_core::domain::{Area, Status, Task};
use falog_core::text::fuzzy_contains;

const WIDTH: f32 = 560.0;
const ROW_HEIGHT: f32 = 30.0;
const MAX_TASKS: usize = 50;
const INPUT_ID: &str = "command-palette-input";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Commands,
    Tasks,
}

#[derive(Debug)]
pub struct CommandPalette {
    mode: Mode,
    query: String,
    selected: usize,
    opened: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Pending,
    Dismissed,
    Run(Action),
}

struct Item {
    icon: Icon,
    icon_color: Color32,
    label: String,
    detail: Option<String>,
    keys: Option<&'static str>,
    action: Action,
}

impl CommandPalette {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            query: String::new(),
            selected: 0,
            opened: true,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, tasks: &[Task], areas: &[Area]) -> Outcome {
        let theme = Theme::current(ctx);
        let items = match self.mode {
            Mode::Commands => self.commands(theme, areas),
            Mode::Tasks => self.tasks(theme, tasks),
        };
        self.selected = self.selected.min(items.len().saturating_sub(1));

        // Handle navigation before the text field sees the keys.
        let (down, up, enter) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::Enter),
            )
        });
        if down && !items.is_empty() {
            self.selected = (self.selected + 1) % items.len();
        }
        if up && !items.is_empty() {
            self.selected = (self.selected + items.len() - 1) % items.len();
        }
        if enter && let Some(item) = items.get(self.selected) {
            return Outcome::Run(item.action);
        }

        let (clicked, dismissed) = modal(ctx, "command-palette", WIDTH, Placement::Top, |ui| {
            self.input(ui, theme);
            ui.painter().hline(
                ui.min_rect().x_range(),
                ui.cursor().top(),
                Stroke::new(1.0_f32, theme.border_variant),
            );
            self.list(ui, theme, &items)
        });
        match (clicked, dismissed) {
            (Some(action), _) => Outcome::Run(action),
            (None, true) => Outcome::Dismissed,
            (None, false) => Outcome::Pending,
        }
    }

    fn input(&mut self, ui: &mut Ui, theme: &Theme) {
        let hint = match self.mode {
            Mode::Commands => tr("Execute a command…"),
            Mode::Tasks => tr("Search tasks by title, #id, area or requester…"),
        };
        let response = ui.add(
            TextEdit::singleline(&mut self.query)
                .id(Id::new(INPUT_ID))
                .frame(false)
                .font(FontId::proportional(15.0))
                .margin(Margin::symmetric(12.0, 10.0))
                .desired_width(f32::INFINITY)
                .hint_text(egui::RichText::new(hint).color(theme.text_placeholder)),
        );
        if std::mem::take(&mut self.opened) {
            response.request_focus();
        }
        if response.changed() {
            self.selected = 0;
        }
    }

    fn list(&mut self, ui: &mut Ui, theme: &Theme, items: &[Item]) -> Option<Action> {
        let mut clicked = None;
        egui::Frame::none()
            .inner_margin(Margin::same(4.0))
            .show(ui, |ui| {
                if items.is_empty() {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!("  {}", tr("No matches")))
                            .size(13.5)
                            .color(theme.text_placeholder),
                    );
                    ui.add_space(6.0);
                    return;
                }
                ScrollArea::vertical()
                    .max_height(ROW_HEIGHT * 12.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for (index, item) in items.iter().enumerate() {
                            let response = row(ui, theme, item, index == self.selected);
                            if index == self.selected
                                && (response.changed()
                                    || ui.input(|i| {
                                        i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::ArrowUp)
                                    }))
                            {
                                response.scroll_to_me(None);
                            }
                            if response.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                                self.selected = index;
                            }
                            if response.clicked() {
                                clicked = Some(item.action);
                            }
                        }
                    });
            });
        clicked
    }

    fn commands(&self, theme: &Theme, areas: &[Area]) -> Vec<Item> {
        let command = |icon: Icon, label: String, keys: Option<&'static str>, action: Action| Item {
            icon,
            icon_color: theme.icon_muted,
            label,
            detail: None,
            keys,
            action,
        };
        let mut items = vec![
            command(
                Icon::Plus,
                tr("task: new").into(),
                Some("Ctrl+N"),
                Action::NewTask(Status::Todo),
            ),
            command(
                Icon::Search,
                tr("file finder: find task").into(),
                Some("Ctrl+P"),
                Action::OpenTaskFinder,
            ),
        ];
        for view in View::ALL {
            let label = trf("view: {view}", &[("view", &view.label().to_lowercase())]);
            items.push(command(
                view.icon(),
                label,
                Some(view.shortcut()),
                Action::SetView(view),
            ));
        }
        items.push(command(
            Icon::Archive,
            tr("task: archive done tasks").into(),
            None,
            Action::ArchiveDone,
        ));
        items.push(command(
            Icon::SidebarLeft,
            tr("workspace: toggle sidebar").into(),
            Some("Ctrl+B"),
            Action::ToggleSidebar,
        ));
        items.push(command(
            Icon::Sparkle,
            tr("assistant: toggle panel").into(),
            Some("Ctrl+Shift+A"),
            Action::ToggleAssistant,
        ));
        items.push(command(
            Icon::Maximize,
            tr("assistant: toggle zoom").into(),
            Some("Shift+Esc"),
            Action::ToggleAssistantZoom,
        ));
        items.push(command(
            Icon::Mic,
            tr("assistant: dictate").into(),
            Some("Ctrl+Space"),
            Action::ToggleDictation,
        ));
        items.push(command(
            Icon::Plus,
            tr("assistant: new thread").into(),
            None,
            Action::NewAssistantThread,
        ));
        items.push(command(
            Icon::Clock,
            tr("assistant: show history").into(),
            None,
            Action::ShowAssistantHistory,
        ));
        items.push(command(
            Icon::Calendar,
            tr("calendar: add calendar link").into(),
            None,
            Action::OpenSettingsTab(SettingsTab::Calendar),
        ));
        items.push(command(
            Icon::Calendar,
            tr("calendar: connect google account").into(),
            None,
            Action::ConnectGoogle,
        ));
        items.push(command(
            Icon::Refresh,
            tr("calendar: refresh").into(),
            None,
            Action::RefreshCalendar,
        ));
        items.push(command(
            Icon::Search,
            tr("workspace: focus search").into(),
            Some("Ctrl+F"),
            Action::FocusSearch,
        ));
        items.push(command(
            Icon::FolderOpen,
            tr("areas: show all").into(),
            None,
            Action::FilterArea(None),
        ));
        for area in areas {
            items.push(Item {
                icon: Icon::Folder,
                icon_color: theme::color(area.color),
                label: trf("areas: filter by {name}", &[("name", &area.name)]),
                detail: None,
                keys: None,
                action: Action::FilterArea(Some(area.id)),
            });
        }
        items.push(command(
            Icon::Settings,
            tr("areas: manage").into(),
            None,
            Action::ManageAreas,
        ));
        for mode in ThemeMode::ALL {
            let label = trf("theme: {theme}", &[("theme", &mode.label().to_lowercase())]);
            items.push(command(Icon::Settings, label, None, Action::SetTheme(mode)));
        }
        items.push(command(
            Icon::Settings,
            tr("settings: open").into(),
            Some("Ctrl+,"),
            Action::OpenSettings,
        ));
        items.push(command(
            Icon::Database,
            tr("data: reload").into(),
            Some("F5"),
            Action::Reload,
        ));

        let terms: Vec<&str> = self.query.split_whitespace().collect();
        items.retain(|item| terms.iter().all(|term| fuzzy_contains(&item.label, term)));
        items
    }

    fn tasks(&self, theme: &Theme, tasks: &[Task]) -> Vec<Item> {
        let mut matches: Vec<&Task> = tasks.iter().filter(|t| t.matches(&self.query)).collect();
        matches.sort_by_key(|t| (!t.is_open(), std::cmp::Reverse(t.updated_at)));
        matches
            .into_iter()
            .take(MAX_TASKS)
            .map(|task| Item {
                icon: Icon::for_status(task.status),
                icon_color: theme.status_color(task.status),
                label: format!("#{}  {}", task.id, task.title),
                detail: Some(area_name(task).to_owned()),
                keys: None,
                action: Action::OpenTask(task.id),
            })
            .collect()
    }
}

fn row(ui: &mut Ui, theme: &Theme, item: &Item, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if selected {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_selected);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_hover);
    }
    let y = rect.center().y;
    let icon_rect = Rect::from_center_size(pos2(rect.left() + 18.0, y), vec2(14.0, 14.0));
    item.icon.paint(ui, icon_rect, 14.0, item.icon_color);

    let mut right = rect.right() - 10.0;
    if let Some(keys) = item.keys {
        right -= paint_keybinding(ui, keys, right, y) + 10.0;
    }
    if let Some(detail) = &item.detail {
        let galley =
            ui.painter()
                .layout_no_wrap(detail.clone(), FontId::proportional(12.5), theme.text_muted);
        right -= galley.size().x;
        ui.painter()
            .galley(pos2(right, y - galley.size().y / 2.0), galley, theme.text_muted);
        right -= 12.0;
    }
    let left = icon_rect.right() + 10.0;
    let galley = single_line(
        ui,
        &item.label,
        FontId::proportional(14.0),
        theme.text,
        right - left,
        false,
    );
    ui.painter()
        .galley(pos2(left, y - galley.size().y / 2.0), galley, theme.text);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{Language, set};
    use crate::testing::{area, done, keys_pressed, noon, task};

    fn theme() -> &'static Theme {
        Theme::current(&egui::Context::default())
    }

    fn palette(mode: Mode, query: &str) -> CommandPalette {
        CommandPalette {
            query: query.into(),
            ..CommandPalette::new(mode)
        }
    }

    fn command_labels(query: &str, areas: &[Area]) -> Vec<String> {
        set(Language::English);
        let items = palette(Mode::Commands, query).commands(theme(), areas);
        items.into_iter().map(|item| item.label).collect()
    }

    #[test]
    fn commands_match_every_word_typed() {
        assert_eq!(command_labels("view arch", &[]), ["view: archive"]);
        assert_eq!(
            command_labels("theme", &[]),
            ["theme: system", "theme: dark", "theme: light"]
        );
        assert!(command_labels("zzz", &[]).is_empty());
        assert_eq!(
            command_labels("filter", &[area(1, "Work"), area(2, "Home")]),
            ["areas: filter by Work", "areas: filter by Home"]
        );
    }

    #[test]
    fn the_task_finder_lists_open_tasks_first_then_the_latest_changed() {
        let changed = |id, title, day| Task {
            updated_at: noon(day),
            ..task(id, title)
        };
        let tasks = [
            changed(1, "Pay rent", 2),
            done(2, "Pay the gym", 20),
            changed(3, "Pay taxes", 9),
            changed(4, "Call the bank", 30),
        ];
        let actions: Vec<Action> = palette(Mode::Tasks, "pay")
            .tasks(theme(), &tasks)
            .into_iter()
            .map(|item| item.action)
            .collect();
        let open = |id| Action::OpenTask(falog_core::domain::TaskId(id));
        assert_eq!(actions, [open(3), open(1), open(2)]);

        let many: Vec<Task> = (1..=60).map(|id| task(id, "Same title")).collect();
        assert_eq!(
            palette(Mode::Tasks, "same").tasks(theme(), &many).len(),
            MAX_TASKS
        );
    }

    /// Shows the palette for one frame with `keys` pressed.
    fn show_with(palette: &mut CommandPalette, ctx: &egui::Context, keys: &[Key]) -> Outcome {
        let mut outcome = Outcome::Pending;
        let _ = ctx.run(keys_pressed(keys), |ctx| outcome = palette.show(ctx, &[], &[]));
        outcome
    }

    #[test]
    fn arrows_pick_a_command_enter_runs_it_and_escape_closes() {
        let ctx = egui::Context::default();
        set(Language::English);
        let mut open = palette(Mode::Commands, "view");
        assert_eq!(show_with(&mut open, &ctx, &[]), Outcome::Pending);
        assert_eq!(
            show_with(&mut open, &ctx, &[Key::Enter]),
            Outcome::Run(Action::SetView(View::Board))
        );
        show_with(&mut open, &ctx, &[Key::ArrowUp]);
        assert_eq!(
            show_with(&mut open, &ctx, &[Key::Enter]),
            Outcome::Run(Action::SetView(View::Archive)),
            "up from the first wraps to the last"
        );
        show_with(&mut open, &ctx, &[Key::ArrowDown]);
        assert_eq!(
            show_with(&mut open, &ctx, &[Key::Enter]),
            Outcome::Run(Action::SetView(View::Board))
        );
        assert_eq!(show_with(&mut open, &ctx, &[Key::Escape]), Outcome::Dismissed);
    }
}
