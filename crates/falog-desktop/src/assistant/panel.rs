//! The assistant dock, modeled on Zed's agent panel: thread on top, composer at the bottom.

use super::voice;
use super::{Assistant, AssistantOptions, Item, ToolCall, VoiceState};
use crate::action::{Action, Actions};
use crate::components::{ButtonStyle, button, icon_button, icon_toggle, single_line};
use crate::fonts;
use crate::icons::Icon;
use crate::theme::Theme;
use crate::workspace::BAR_HEIGHT;
use eframe::egui::{
    self, Align, Align2, CentralPanel, Color32, CursorIcon, FontId, Frame, Id, Key, KeyboardShortcut, Layout,
    Margin, Modifiers, ProgressBar, Rect, RichText, ScrollArea, Sense, SidePanel, Spinner, Stroke, TextEdit,
    TopBottomPanel, Ui, pos2, vec2,
};
use falog_core::domain::TaskId;

const COMPOSER_ID: &str = "assistant-composer";

const EXAMPLES: [&str; 3] = [
    "What's on my plate today?",
    "Ana from Acme asked me to fix the Google sign-in on Android, it's urgent, due Friday",
    "I finished the payments PR review",
];

pub fn show(
    ctx: &egui::Context,
    theme: &Theme,
    assistant: &mut Assistant,
    options: AssistantOptions,
    actions: &mut Actions,
) {
    SidePanel::right("assistant_panel")
        .resizable(true)
        .default_width(380.0)
        .width_range(320.0..=640.0)
        .frame(Frame::none().fill(theme.panel))
        .show(ctx, |ui| {
            header(ui, theme, assistant, actions);
            TopBottomPanel::bottom("assistant_composer_panel")
                .show_separator_line(false)
                .frame(Frame::none().inner_margin(Margin::same(10.0)))
                .show_inside(ui, |ui| composer(ui, theme, assistant, options));
            CentralPanel::default()
                .frame(Frame::none().inner_margin(Margin::symmetric(14.0, 8.0)))
                .show_inside(ui, |ui| thread(ui, theme, assistant, options, actions));
        });
}

fn header(ui: &mut Ui, theme: &Theme, assistant: &mut Assistant, actions: &mut Actions) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), BAR_HEIGHT), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme.tab_bar);
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0_f32, theme.border),
    );
    let icon_rect = Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), vec2(14.0, 14.0));
    Icon::Sparkle.paint(ui, icon_rect, 14.0, theme.text_accent);
    ui.painter().text(
        pos2(icon_rect.right() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        "Assistant",
        fonts::semibold(13.5),
        theme.text,
    );
    if let Some(model) = &assistant.model_name {
        ui.painter().text(
            pos2(icon_rect.right() + 80.0, rect.center().y),
            Align2::LEFT_CENTER,
            model,
            FontId::proportional(11.5),
            theme.text_placeholder,
        );
    }
    let buttons = Rect::from_min_max(pos2(rect.right() - 64.0, rect.top()), rect.max);
    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(buttons)
            .layout(Layout::right_to_left(Align::Center)),
        |ui| {
            ui.add_space(6.0);
            ui.spacing_mut().item_spacing.x = 2.0;
            if icon_button(ui, Icon::Close, "Close (Ctrl+Shift+A)").clicked() {
                actions.push(Action::ToggleAssistant);
            }
            if icon_button(ui, Icon::Plus, "New thread").clicked() {
                assistant.new_thread();
            }
        },
    );
}

// ---- thread ----------------------------------------------------------------------------------

fn thread(
    ui: &mut Ui,
    theme: &Theme,
    assistant: &mut Assistant,
    options: AssistantOptions,
    actions: &mut Actions,
) {
    if assistant.items.is_empty() && !assistant.busy {
        empty_state(ui, theme, assistant, options);
        return;
    }
    ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            for item in &assistant.items {
                match item {
                    Item::User(text) => user_message(ui, theme, text),
                    Item::Reply(text) => reply(ui, theme, text),
                    Item::Tool(call) => tool_call(ui, theme, call, actions),
                    Item::Error(text) => error(ui, theme, text),
                    Item::Notice(text) => {
                        ui.label(
                            RichText::new(text)
                                .size(12.5)
                                .italics()
                                .color(theme.text_placeholder),
                        );
                    }
                }
            }
            if !assistant.streaming.is_empty() {
                reply(ui, theme, &format!("{}▍", assistant.streaming));
            } else if assistant.busy && !has_pending_tool(assistant) {
                ui.horizontal(|ui| {
                    ui.add(Spinner::new().size(12.0).color(theme.text_muted));
                    ui.label(RichText::new("Thinking…").size(13.0).color(theme.text_muted));
                });
            }
            ui.add_space(4.0);
        });
}

fn has_pending_tool(assistant: &Assistant) -> bool {
    matches!(assistant.items.last(), Some(Item::Tool(call)) if call.result.is_none())
}

fn empty_state(ui: &mut Ui, theme: &Theme, assistant: &mut Assistant, options: AssistantOptions) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        ui.add(Icon::Sparkle.image(28.0, theme.text_accent));
        ui.add_space(8.0);
        ui.label(RichText::new("Talk to your tasks").font(fonts::semibold(16.0)).color(theme.text));
        ui.label(
            RichText::new("Create, update and review tasks in your own words. Hold Ctrl+Space or click the mic to speak.")
                .size(13.0)
                .color(theme.text_muted),
        );
    });
    ui.add_space(16.0);
    for example in EXAMPLES {
        let response = Frame::none()
            .stroke(Stroke::new(1.0_f32, theme.border_variant))
            .rounding(6.0)
            .inner_margin(Margin::symmetric(10.0, 6.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(example).size(13.0).color(theme.text_muted));
            })
            .response;
        let response = ui.interact(response.rect, response.id.with("example"), Sense::click());
        if response.hovered() {
            ui.painter()
                .rect_stroke(response.rect, 6.0, Stroke::new(1.0_f32, theme.border));
        }
        if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
            assistant.send(ui.ctx(), example.to_owned(), options);
        }
        ui.add_space(4.0);
    }
}

fn user_message(ui: &mut Ui, theme: &Theme, text: &str) {
    Frame::none()
        .fill(theme.element)
        .stroke(Stroke::new(1.0_f32, theme.border_variant))
        .rounding(6.0)
        .inner_margin(Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(egui::Label::new(RichText::new(text).size(14.0).color(theme.text)).selectable(true));
        });
}

fn reply(ui: &mut Ui, theme: &Theme, text: &str) {
    // Replies are asked to be plain text; drop stray Markdown emphasis just in case.
    let text = text.replace("**", "");
    ui.add(egui::Label::new(RichText::new(text).size(14.0).color(theme.text)).selectable(true));
}

fn error(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.horizontal_top(|ui| {
        ui.add(Icon::Warning.image(14.0, theme.error));
        ui.add(egui::Label::new(RichText::new(text).size(13.0).color(theme.error)).wrap());
    });
}

/// A compact card for one tool call: status icon, action, and the first line of the result.
fn tool_call(ui: &mut Ui, theme: &Theme, call: &ToolCall, actions: &mut Actions) {
    let (icon, color) = match &call.result {
        None => (None, theme.text_muted),
        Some(outcome) if outcome.is_error => (Some(Icon::Warning), theme.error),
        Some(_) => (Some(Icon::Check), theme.success),
    };
    let detail = call
        .result
        .as_ref()
        .map(|r| r.text.lines().next().unwrap_or_default().to_owned());
    let task = call
        .result
        .as_ref()
        .filter(|r| !r.is_error)
        .and_then(|r| first_task_id(&r.text));
    let opens_task = task.is_some() && call.name != "delete_task";

    let height = if detail.is_some() { 44.0 } else { 28.0 };
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    let hovered = opens_task && response.hovered();
    ui.painter().rect(
        rect,
        6.0,
        if hovered {
            theme.ghost_hover
        } else {
            Color32::TRANSPARENT
        },
        Stroke::new(1.0_f32, theme.border_variant),
    );

    let first_line = rect.top() + 14.0;
    let icon_rect = Rect::from_center_size(pos2(rect.left() + 16.0, first_line), vec2(14.0, 14.0));
    match icon {
        Some(icon) => icon.paint(ui, icon_rect, 13.0, color),
        None => {
            ui.put(icon_rect, Spinner::new().size(12.0).color(theme.text_muted));
        }
    }
    ui.painter().text(
        pos2(icon_rect.right() + 8.0, first_line),
        Align2::LEFT_CENTER,
        tool_label(&call.name, call.result.is_none()),
        FontId::proportional(13.0),
        theme.text,
    );
    if let Some(detail) = detail {
        let galley = single_line(
            ui,
            &detail,
            FontId::proportional(12.0),
            theme.text_muted,
            rect.width() - 40.0,
            false,
        );
        ui.painter().galley(
            pos2(icon_rect.right() + 8.0, rect.top() + 26.0),
            galley,
            theme.text_muted,
        );
    }
    // The exact arguments the assistant sent, for when a result looks off.
    let arguments = serde_json::to_string_pretty(&call.input).unwrap_or_default();
    let response = response.on_hover_text(RichText::new(arguments).monospace().size(11.5));
    if let (true, Some(id)) = (opens_task, task)
        && response.on_hover_cursor(CursorIcon::PointingHand).clicked()
    {
        actions.push(Action::OpenTask(id));
    }
}

fn tool_label(name: &str, pending: bool) -> &'static str {
    match (name, pending) {
        ("create_task", true) => "Creating task…",
        ("create_task", false) => "Created task",
        ("update_task", true) => "Updating task…",
        ("update_task", false) => "Updated task",
        ("add_note", true) => "Adding note…",
        ("add_note", false) => "Added note",
        ("delete_task", true) => "Deleting task…",
        ("delete_task", false) => "Deleted task",
        ("create_company", true) => "Adding company…",
        ("create_company", false) => "Added company",
        ("get_agenda", _) => "Read the agenda",
        ("list_tasks", _) => "Searched tasks",
        ("get_task", _) => "Read task",
        ("list_companies", _) => "Listed companies",
        (_, true) => "Working…",
        (_, false) => "Done",
    }
}

/// The first `#123` in a tool result (not the start of a hex color like `#74ade8`).
pub fn first_task_id(text: &str) -> Option<TaskId> {
    text.match_indices('#').find_map(|(index, _)| {
        let rest = &text[index + 1..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let ends_word = rest[digits.len()..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric());
        if digits.is_empty() || !ends_word {
            None
        } else {
            digits.parse().ok().map(TaskId)
        }
    })
}

// ---- composer --------------------------------------------------------------------------------

fn composer(ui: &mut Ui, theme: &Theme, assistant: &mut Assistant, options: AssistantOptions) {
    let id = Id::new(COMPOSER_ID);
    let focused = ui.memory(|m| m.has_focus(id));
    Frame::none()
        .fill(theme.editor)
        .stroke(Stroke::new(
            1.0_f32,
            if focused {
                theme.border_focused
            } else {
                theme.border_variant
            },
        ))
        .rounding(6.0)
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            match &assistant.voice {
                VoiceState::Recording(recorder) => {
                    let (level, elapsed) = (recorder.level(), recorder.elapsed());
                    live_preview(ui, theme, &assistant.live_text, "Listening…");
                    recording(ui, theme, assistant, level, elapsed, options);
                }
                VoiceState::Transcribing => {
                    live_preview(ui, theme, &assistant.live_text, "");
                    status_line(ui, theme, "Finishing the transcript…");
                }
                VoiceState::NeedsModel => model_prompt(ui, theme, assistant),
                VoiceState::Downloading(download) => {
                    let text = format!("Downloading the voice model… {} MB", download.received_mb());
                    ui.label(RichText::new(text).size(13.0).color(theme.text_muted));
                    ui.add(ProgressBar::new(download.progress().unwrap_or(0.0)).desired_height(6.0));
                }
                VoiceState::Idle => text_input(ui, theme, assistant, options, focused),
            }
        });
}

fn text_input(
    ui: &mut Ui,
    theme: &Theme,
    assistant: &mut Assistant,
    options: AssistantOptions,
    focused: bool,
) {
    let id = Id::new(COMPOSER_ID);
    // Enter sends, Shift+Enter breaks the line; consume Enter before the text field sees it.
    let enter = focused && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
    let response = ui.add(
        TextEdit::multiline(&mut assistant.draft)
            .id(id)
            .frame(false)
            .desired_rows(2)
            .desired_width(f32::INFINITY)
            .return_key(KeyboardShortcut::new(Modifiers::SHIFT, Key::Enter))
            .hint_text(RichText::new("Tell the assistant what changed…").color(theme.text_placeholder)),
    );
    if std::mem::take(&mut assistant.focus_composer) {
        response.request_focus();
    }
    ui.horizontal(|ui| {
        if icon_toggle(ui, Icon::Mic, false, "Talk (Ctrl+Space)").clicked() {
            assistant.toggle_dictation(ui.ctx(), options);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if assistant.busy {
                if button(ui, ButtonStyle::Filled, Some(Icon::Stop), "Stop").clicked() {
                    assistant.stop();
                }
            } else {
                let ready = !assistant.draft.trim().is_empty();
                let send = ui.add_enabled_ui(ready, |ui| {
                    button(ui, ButtonStyle::Accent, Some(Icon::Send), "Send")
                });
                if (send.inner.on_hover_text("Enter").clicked() || enter) && ready {
                    let text = std::mem::take(&mut assistant.draft);
                    assistant.send(ui.ctx(), text, options);
                    response.request_focus();
                }
            }
        });
    });
}

fn recording(
    ui: &mut Ui,
    theme: &Theme,
    assistant: &mut Assistant,
    level: f32,
    elapsed: std::time::Duration,
    options: AssistantOptions,
) {
    ui.horizontal(|ui| {
        let pulse = (ui.input(|i| i.time) * 3.0).sin() as f32 * 0.25 + 0.75;
        let (dot, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
        ui.painter()
            .circle_filled(dot.center(), 5.0, theme.error.gamma_multiply(pulse));
        let seconds = elapsed.as_secs();
        ui.label(
            RichText::new(format!("Listening  {}:{:02}", seconds / 60, seconds % 60))
                .size(13.0)
                .color(theme.text),
        );
        let (meter, _) = ui.allocate_exact_size(vec2(90.0, 6.0), Sense::hover());
        ui.painter().rect_filled(meter, 3.0, theme.element_selected);
        let filled = Rect::from_min_size(
            meter.min,
            vec2(meter.width() * level.clamp(0.02, 1.0), meter.height()),
        );
        ui.painter().rect_filled(filled, 3.0, theme.text_accent);
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if icon_toggle(ui, Icon::Mic, true, "Stop and transcribe (Ctrl+Space)").clicked() {
            assistant.toggle_dictation(ui.ctx(), options);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let label = if options.send_after_dictation {
                "Send"
            } else {
                "Done"
            };
            if button(ui, ButtonStyle::Accent, Some(Icon::Check), label).clicked() {
                assistant.toggle_dictation(ui.ctx(), options);
            }
            if button(ui, ButtonStyle::Ghost, None, "Cancel").clicked() {
                assistant.cancel_dictation();
            }
        });
    });
}

/// The words recognized so far, greyed out like a placeholder until the final transcript lands.
fn live_preview(ui: &mut Ui, theme: &Theme, text: &str, empty: &str) {
    let text = if text.trim().is_empty() {
        empty
    } else {
        text.trim()
    };
    if text.is_empty() {
        return;
    }
    ui.add(
        egui::Label::new(
            RichText::new(text)
                .size(14.0)
                .italics()
                .color(theme.text_placeholder),
        )
        .wrap(),
    );
    ui.add_space(6.0);
}

fn status_line(ui: &mut Ui, theme: &Theme, text: &str) {
    ui.horizontal(|ui| {
        ui.add(Spinner::new().size(12.0).color(theme.text_muted));
        ui.label(RichText::new(text).size(13.0).color(theme.text_muted));
    });
}

fn model_prompt(ui: &mut Ui, theme: &Theme, assistant: &mut Assistant) {
    ui.label(
        RichText::new(format!(
            "Voice input runs Whisper on this computer. It needs a one-time download of about {} MB.",
            voice::MODEL_SIZE_MB
        ))
        .size(13.0)
        .color(theme.text_muted),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if button(ui, ButtonStyle::Accent, Some(Icon::Plus), "Download").clicked() {
            assistant.download_model(ui.ctx());
        }
        if button(ui, ButtonStyle::Ghost, None, "Not now").clicked() {
            assistant.cancel_dictation();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_first_task_id() {
        assert_eq!(
            first_task_id("Created #12 [Acme] Fix login — High"),
            Some(TaskId(12))
        );
        assert_eq!(first_task_id("Color #74ade8 and task #3"), Some(TaskId(3)));
        assert_eq!(first_task_id("No tasks found."), None);
    }
}
