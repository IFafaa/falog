//! Slash command completion in the composer: typing `/` lists the agent's commands.

use super::agent::SlashCommand;
use crate::components::single_line;
use crate::theme::Theme;
use eframe::egui::{
    self, Align2, CursorIcon, FontId, Frame, Id, Key, Margin, Modifiers, Order, Rect, ScrollArea, Sense, Ui,
    pos2, vec2,
};

const ROW_HEIGHT: f32 = 30.0;
const VISIBLE_ROWS: f32 = 8.0;

/// The commands that complete what is typed, or `None` when the draft is not a command name being
/// typed (it does not start with `/`, or the name is already followed by input).
pub fn matching<'a>(commands: &'a [SlashCommand], draft: &str) -> Option<Vec<&'a SlashCommand>> {
    let query = draft.strip_prefix('/')?;
    if query.contains(char::is_whitespace) {
        return None;
    }
    let query = query.to_lowercase();
    let (mut starts, mut contains): (Vec<_>, Vec<_>) = (Vec::new(), Vec::new());
    for command in commands {
        let name = command.name.to_lowercase();
        if name.starts_with(&query) {
            starts.push(command);
        } else if name.contains(&query) {
            contains.push(command);
        }
    }
    starts.append(&mut contains);
    Some(starts)
}

/// Popup state, kept in egui memory next to the composer.
#[derive(Clone, Debug, Default)]
struct MenuState {
    selected: usize,
    /// Esc closes the menu until the draft stops being a command name.
    dismissed: bool,
}

/// What the keyboard asked the menu to do this frame.
pub enum MenuKey {
    None,
    /// Replace the draft with `/name ` (Tab or Enter on a partial name).
    Complete(String),
}

/// Handles the menu's keys before the text field sees them; keys are left alone when it is closed.
pub fn handle_keys(ui: &mut Ui, id: Id, commands: &[SlashCommand], draft: &str) -> MenuKey {
    let mut state = load(ui, id);
    let Some(matches) = matching(commands, draft) else {
        state.dismissed = false;
        store(ui, id, state);
        return MenuKey::None;
    };
    if state.dismissed {
        return MenuKey::None;
    }
    let (down, up, tab, escape) = ui.input_mut(|i| {
        (
            i.consume_key(Modifiers::NONE, Key::ArrowDown),
            i.consume_key(Modifiers::NONE, Key::ArrowUp),
            i.consume_key(Modifiers::NONE, Key::Tab),
            i.consume_key(Modifiers::NONE, Key::Escape),
        )
    });
    let mut key = MenuKey::None;
    if escape {
        state.dismissed = true;
    } else if !matches.is_empty() {
        let count = matches.len();
        state.selected = state.selected.min(count - 1);
        if down {
            state.selected = (state.selected + 1) % count;
        } else if up {
            state.selected = (state.selected + count - 1) % count;
        }
        let selected = &matches[state.selected].name;
        // Enter on a full name sends it; on a partial one it completes first.
        let enter = draft[1..] != **selected && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        if tab || enter {
            key = MenuKey::Complete(format!("/{selected} "));
        }
    }
    store(ui, id, state);
    key
}

/// Draws the menu above `composer` while the composer has focus (or the pointer is on the menu,
/// since clicking a row takes the focus away). Returns the command clicked, as the new draft.
pub fn show(
    ctx: &egui::Context,
    theme: &Theme,
    id: Id,
    composer: Rect,
    focused: bool,
    commands: &[SlashCommand],
    draft: &str,
) -> Option<String> {
    let matches = matching(commands, draft)?;
    let mut state = ctx.data(|d| d.get_temp::<MenuState>(id).unwrap_or_default());
    let menu = id.with("menu");
    let pointed = ctx
        .memory(|m| m.area_rect(menu))
        .zip(ctx.pointer_hover_pos())
        .is_some_and(|(rect, pointer)| rect.contains(pointer));
    if state.dismissed || !(focused || pointed) {
        return None;
    }
    let mut clicked = None;
    egui::Area::new(menu)
        .order(Order::Foreground)
        .pivot(Align2::LEFT_BOTTOM)
        .fixed_pos(pos2(composer.left(), composer.top() - 4.0))
        .show(ctx, |ui| {
            Frame::none()
                .fill(theme.elevated_surface)
                .stroke((1.0, theme.border))
                .rounding(6.0)
                .shadow(theme.popover_shadow())
                .inner_margin(Margin::same(4.0))
                .show(ui, |ui| {
                    ui.set_width(composer.width() - 8.0);
                    if matches.is_empty() {
                        let text = if commands.is_empty() {
                            "The agent lists its commands once it starts. Send a message first."
                        } else {
                            "No matching commands"
                        };
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(text).size(13.0).color(theme.text_placeholder));
                        ui.add_space(4.0);
                        return;
                    }
                    ScrollArea::vertical()
                        .max_height(ROW_HEIGHT * VISIBLE_ROWS)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            for (index, command) in matches.iter().enumerate() {
                                let selected = index == state.selected;
                                let response = row(ui, theme, command, selected);
                                if selected
                                    && ui.input(|i| {
                                        i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::ArrowUp)
                                    })
                                {
                                    response.scroll_to_me(None);
                                }
                                if response.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                                    state.selected = index;
                                }
                                if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                                    clicked = Some(format!("/{} ", command.name));
                                }
                            }
                        });
                });
        });
    ctx.data_mut(|d| d.insert_temp(id, state));
    clicked
}

/// `/name  hint` on the left, the description muted after it.
fn row(ui: &mut Ui, theme: &Theme, command: &SlashCommand, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if selected {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_selected);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_hover);
    }
    let y = rect.center().y;
    let mut left = rect.left() + 8.0;
    let right = rect.right() - 8.0;
    let mut text = |text: &str, font: FontId, color, max: f32| {
        if text.is_empty() || left >= right {
            return;
        }
        let galley = single_line(ui, text, font, color, max.min(right - left), false);
        let width = galley.size().x;
        ui.painter()
            .galley(pos2(left, y - galley.size().y / 2.0), galley, color);
        left += width + 10.0;
    };
    text(
        &format!("/{}", command.name),
        FontId::monospace(13.0),
        theme.text,
        f32::INFINITY,
    );
    text(
        &command.hint,
        FontId::proportional(12.5),
        theme.text_placeholder,
        160.0,
    );
    text(
        &command.description,
        FontId::proportional(12.5),
        theme.text_muted,
        f32::INFINITY,
    );
    response
}

fn load(ui: &Ui, id: Id) -> MenuState {
    ui.data(|d| d.get_temp::<MenuState>(id).unwrap_or_default())
}

fn store(ui: &Ui, id: Id, state: MenuState) {
    ui.data_mut(|d| d.insert_temp(id, state));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commands(names: &[&str]) -> Vec<SlashCommand> {
        names
            .iter()
            .map(|name| SlashCommand {
                name: (*name).into(),
                description: String::new(),
                hint: String::new(),
            })
            .collect()
    }

    fn names(matches: Option<Vec<&SlashCommand>>) -> Option<Vec<&str>> {
        matches.map(|list| list.into_iter().map(|c| c.name.as_str()).collect())
    }

    #[test]
    fn lists_commands_while_typing_a_name() {
        let all = commands(&["compact", "context", "review", "pr-comments"]);
        assert_eq!(
            names(matching(&all, "/")),
            Some(vec!["compact", "context", "review", "pr-comments"])
        );
        assert_eq!(
            names(matching(&all, "/co")),
            Some(vec!["compact", "context", "pr-comments"])
        );
        assert_eq!(names(matching(&all, "/REV")), Some(vec!["review"]));
        assert_eq!(names(matching(&all, "/zzz")), Some(vec![]));
    }

    #[test]
    fn closes_outside_a_command_name() {
        let all = commands(&["compact"]);
        assert_eq!(names(matching(&all, "hello")), None);
        assert_eq!(names(matching(&all, "/compact now")), None);
        assert_eq!(names(matching(&all, "")), None);
    }
}
