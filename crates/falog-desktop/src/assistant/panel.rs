//! The assistant dock, modeled on Zed's agent panel: thread on top, composer at the bottom, and a
//! history view listing past threads.

use super::agent::claude_code::FAST_ON;
use super::agent::{OptionKind, Usage, registry};
use super::thread::{self, Thread};
use super::{Assistant, AssistantOptions, Item, ToolCall, VoiceState};
use super::{slash, voice};
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
/// Width of the conversation column when the assistant fills the window.
const ZOOMED_WIDTH: f32 = 820.0;

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
    // Zoomed, the dock takes the whole width and keeps the conversation to a readable column.
    let panel = if options.zoomed {
        SidePanel::right("assistant_panel_zoomed")
            .resizable(false)
            .exact_width(ctx.available_rect().width())
    } else {
        SidePanel::right("assistant_panel")
            .resizable(true)
            .default_width(380.0)
            .width_range(320.0..=640.0)
    };
    panel.frame(Frame::none().fill(theme.panel)).show(ctx, |ui| {
        header(ui, theme, assistant, options.zoomed, actions);
        let side = if options.zoomed {
            ((ui.available_width() - ZOOMED_WIDTH) / 2.0).max(0.0)
        } else {
            0.0
        };
        if assistant.show_history {
            CentralPanel::default()
                .frame(Frame::none().inner_margin(Margin::symmetric(8.0 + side, 8.0)))
                .show_inside(ui, |ui| history(ui, theme, assistant));
            return;
        }
        TopBottomPanel::bottom("assistant_composer_panel")
            .show_separator_line(false)
            .frame(Frame::none().inner_margin(Margin::symmetric(10.0 + side, 10.0)))
            .show_inside(ui, |ui| composer(ui, theme, assistant, options));
        CentralPanel::default()
            .frame(Frame::none().inner_margin(Margin::symmetric(14.0 + side, 8.0)))
            .show_inside(ui, |ui| thread_view(ui, theme, assistant, actions));
    });
}

/// Zed's agent panel header: the thread title on the left, thread actions on the right.
fn header(ui: &mut Ui, theme: &Theme, assistant: &mut Assistant, zoomed: bool, actions: &mut Actions) {
    const BUTTONS_WIDTH: f32 = 122.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), BAR_HEIGHT), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme.tab_bar);
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0_f32, theme.border),
    );
    let icon_rect = Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), vec2(14.0, 14.0));
    let (icon, title) = if assistant.show_history {
        (Icon::Clock, "History".to_owned())
    } else {
        (Icon::Sparkle, assistant.active().title())
    };
    icon.paint(ui, icon_rect, 14.0, theme.text_accent);
    let title_left = icon_rect.right() + 8.0;
    let galley = single_line(
        ui,
        &title,
        fonts::semibold(13.5),
        theme.text,
        rect.right() - BUTTONS_WIDTH - title_left,
        false,
    );
    let title_rect = Rect::from_min_size(
        pos2(title_left, rect.center().y - galley.size().y / 2.0),
        galley.size(),
    );
    ui.painter().galley(title_rect.min, galley, theme.text);
    if let (false, Some(model)) = (assistant.show_history, &assistant.active().model_name) {
        ui.interact(title_rect, Id::new("assistant-title"), Sense::hover())
            .on_hover_text(format!("{title}\n{model}"));
    }

    let buttons = Rect::from_min_max(pos2(rect.right() - BUTTONS_WIDTH, rect.top()), rect.max);
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
            let (icon, tip) = if zoomed {
                (Icon::Minimize, "Zoom out (Shift+Esc)")
            } else {
                (Icon::Maximize, "Zoom in (Shift+Esc)")
            };
            if icon_button(ui, icon, tip).clicked() {
                actions.push(Action::ToggleAssistantZoom);
            }
            if icon_toggle(ui, Icon::Clock, assistant.show_history, "History").clicked() {
                assistant.show_history = !assistant.show_history;
            }
            if icon_button(ui, Icon::Plus, "New thread").clicked() {
                assistant.new_thread();
            }
        },
    );
}

// ---- history ---------------------------------------------------------------------------------

fn history(ui: &mut Ui, theme: &Theme, assistant: &mut Assistant) {
    if assistant.recent_threads().is_empty() {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("No threads yet").size(13.0).color(theme.text_muted));
        });
        return;
    }
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| thread_list(ui, theme, assistant, usize::MAX));
}

/// Rows for the most recent threads; clicking one opens it.
fn thread_list(ui: &mut Ui, theme: &Theme, assistant: &mut Assistant, limit: usize) {
    let now = thread::now();
    let active = assistant.active().id;
    let (mut open, mut delete) = (None, None);
    ui.spacing_mut().item_spacing.y = 2.0;
    for thread in assistant.recent_threads().into_iter().take(limit) {
        match thread_row(ui, theme, thread, thread.id == active, now) {
            RowAction::Open => open = Some(thread.id),
            RowAction::Delete => delete = Some(thread.id),
            RowAction::None => {}
        }
    }
    if let Some(id) = delete {
        assistant.delete_thread(id);
    } else if let Some(id) = open {
        assistant.open_thread(id);
    }
}

enum RowAction {
    None,
    Open,
    Delete,
}

/// One past thread: title, then "N messages · 2h ago"; a delete button shows on hover.
fn thread_row(ui: &mut Ui, theme: &Theme, thread: &Thread, active: bool, now: i64) -> RowAction {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
    let hovered = ui.rect_contains_pointer(rect);
    let fill = if active {
        theme.element_selected
    } else if hovered {
        theme.ghost_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 6.0, fill);

    let left = rect.left() + 10.0;
    let text_width = rect.width() - 48.0;
    let title = single_line(
        ui,
        &thread.title(),
        FontId::proportional(13.5),
        theme.text,
        text_width,
        false,
    );
    ui.painter()
        .galley(pos2(left, rect.top() + 6.0), title, theme.text);
    let count = thread.message_count();
    let detail = format!(
        "{} · {count} message{} · {}",
        thread.agent_name,
        if count == 1 { "" } else { "s" },
        thread::relative_time(thread.updated_at, now)
    );
    let detail = single_line(
        ui,
        &detail,
        FontId::proportional(12.0),
        theme.text_muted,
        text_width,
        false,
    );
    ui.painter()
        .galley(pos2(left, rect.top() + 25.0), detail, theme.text_muted);

    let side = Rect::from_center_size(pos2(rect.right() - 20.0, rect.center().y), vec2(24.0, 24.0));
    if thread.busy {
        ui.put(side, Spinner::new().size(12.0).color(theme.text_muted));
    } else if hovered {
        let delete = ui.put(side, |ui: &mut Ui| icon_button(ui, Icon::Trash, "Delete thread"));
        if delete.clicked() {
            return RowAction::Delete;
        }
    }
    if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
        RowAction::Open
    } else {
        RowAction::None
    }
}

// ---- thread ----------------------------------------------------------------------------------

fn thread_view(ui: &mut Ui, theme: &Theme, assistant: &mut Assistant, actions: &mut Actions) {
    let thread = assistant.active();
    if thread.items.is_empty() && !thread.busy {
        empty_state(ui, theme, assistant);
        return;
    }
    ScrollArea::vertical()
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            for item in &thread.items {
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
            if !thread.streaming.is_empty() {
                reply(ui, theme, &format!("{}▍", thread.streaming));
            } else if thread.busy && !has_pending_tool(thread) {
                ui.horizontal(|ui| {
                    ui.add(Spinner::new().size(12.0).color(theme.text_muted));
                    ui.label(RichText::new("Thinking…").size(13.0).color(theme.text_muted));
                });
            }
            ui.add_space(4.0);
        });
}

fn has_pending_tool(thread: &Thread) -> bool {
    matches!(thread.items.last(), Some(Item::Tool(call)) if call.result.is_none())
}

fn empty_state(ui: &mut Ui, theme: &Theme, assistant: &mut Assistant) {
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
            assistant.send(ui.ctx(), example.to_owned());
        }
        ui.add_space(4.0);
    }
    recent(ui, theme, assistant);
}

/// The last few threads under the examples, like Zed's empty agent panel.
fn recent(ui: &mut Ui, theme: &Theme, assistant: &mut Assistant) {
    const SHOWN: usize = 3;
    let total = assistant.recent_threads().len();
    if total == 0 {
        return;
    }
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Recent").size(12.0).color(theme.text_muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if total > SHOWN && button(ui, ButtonStyle::Ghost, None, &format!("View all ({total})")).clicked()
            {
                assistant.show_history = true;
            }
        });
    });
    thread_list(ui, theme, assistant, SHOWN);
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
        ("create_area", true) => "Adding area…",
        ("create_area", false) => "Added area",
        ("get_agenda", _) => "Read the agenda",
        ("list_tasks", _) => "Searched tasks",
        ("get_task", _) => "Read task",
        ("list_areas", _) => "Listed areas",
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
    let frame = Frame::none()
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
    if matches!(assistant.voice, VoiceState::Idle) {
        let thread = assistant.active();
        if let Some(draft) = slash::show(
            ui.ctx(),
            theme,
            id,
            frame.response.rect,
            focused,
            &thread.commands,
            &thread.draft,
        ) {
            set_draft(ui, assistant, draft);
        }
    }
}

/// Replaces the draft and puts the cursor at its end, as after picking a completion.
fn set_draft(ui: &Ui, assistant: &mut Assistant, draft: String) {
    let id = Id::new(COMPOSER_ID);
    let end = draft.chars().count();
    assistant.active_mut().draft = draft;
    if let Some(mut state) = TextEdit::load_state(ui.ctx(), id) {
        let cursor = egui::text::CCursorRange::one(egui::text::CCursor::new(end));
        state.cursor.set_char_range(Some(cursor));
        state.store(ui.ctx(), id);
    }
    ui.memory_mut(|m| m.request_focus(id));
}

fn text_input(
    ui: &mut Ui,
    theme: &Theme,
    assistant: &mut Assistant,
    options: AssistantOptions,
    focused: bool,
) {
    let id = Id::new(COMPOSER_ID);
    // The command menu takes arrows, Tab, Esc and (on a partial name) Enter first.
    if focused {
        let thread = assistant.active();
        if let slash::MenuKey::Complete(draft) = slash::handle_keys(ui, id, &thread.commands, &thread.draft) {
            set_draft(ui, assistant, draft);
        }
    }
    // Enter sends, Shift+Enter breaks the line; consume Enter before the text field sees it.
    let enter = focused && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
    let response = ui.add(
        TextEdit::multiline(&mut assistant.active_mut().draft)
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
    // Zed's composer footer, on two rows so it fits a narrow dock: what the thread asks the agent
    // for, then the actions.
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        agent_picker(ui, theme, assistant);
        option_picker(ui, theme, assistant, OptionKind::Model);
        option_picker(ui, theme, assistant, OptionKind::Effort);
        option_picker(ui, theme, assistant, OptionKind::Mode);
    });
    ui.horizontal(|ui| {
        if icon_toggle(ui, Icon::Mic, false, "Talk (Ctrl+Space)").clicked() {
            assistant.toggle_dictation(ui.ctx(), options);
        }
        fast_toggle(ui, assistant);
        ultracode(ui, theme, assistant);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if assistant.active().busy {
                if button(ui, ButtonStyle::Filled, Some(Icon::Stop), "")
                    .on_hover_text("Stop")
                    .clicked()
                {
                    assistant.stop();
                }
            } else {
                let ready = !assistant.active().draft.trim().is_empty();
                let send =
                    ui.add_enabled_ui(ready, |ui| button(ui, ButtonStyle::Accent, Some(Icon::Send), ""));
                if (send.inner.on_hover_text("Send (Enter)").clicked() || enter) && ready {
                    let text = std::mem::take(&mut assistant.active_mut().draft);
                    assistant.send(ui.ctx(), text);
                    response.request_focus();
                }
            }
            if let Some(usage) = assistant.active().usage {
                context_ring(ui, theme, usage);
            }
        });
    });
}

/// Fast mode, when the agent offers it: a flame that stays lit while on, like Zed's burn mode.
fn fast_toggle(ui: &mut Ui, assistant: &mut Assistant) {
    let Some((option, value)) = assistant.active().option(OptionKind::Fast) else {
        return;
    };
    let on = value == FAST_ON;
    let tooltip = match (on, option.description.is_empty()) {
        (true, _) => "Fast mode is on".to_owned(),
        (false, true) => "Fast mode".to_owned(),
        (false, false) => format!("Fast mode: {}", option.description),
    };
    let off = option
        .choices
        .iter()
        .map(|c| c.value.clone())
        .find(|v| v != FAST_ON)
        .unwrap_or_else(|| "off".into());
    if icon_toggle(ui, Icon::Flame, on, &tooltip).clicked() {
        assistant.choose(OptionKind::Fast, if on { off } else { FAST_ON.into() });
    }
}

/// Ultracode has Claude plan and run multi-agent workflows with its built-in tools, which the
/// assistant turns off; it is shown, disabled, so it is clear why it is missing.
fn ultracode(ui: &mut Ui, theme: &Theme, assistant: &Assistant) {
    if !registry::is_claude(&assistant.active().settings.agent) {
        return;
    }
    picker_label_colored(ui, "Ultracode", theme.text_placeholder).on_hover_text(
        "Ultracode is not available here: it has Claude run multi-agent workflows with its built-in \
         tools, and Falog's assistant runs with only its task tools.",
    );
}

/// How full the context window is: a ring that fills up, amber then red near the limit.
fn context_ring(ui: &mut Ui, theme: &Theme, usage: Usage) {
    let (rect, response) = ui.allocate_exact_size(vec2(22.0, 24.0), Sense::hover());
    let center = rect.center();
    let radius = 6.5;
    let fraction = usage.fraction();
    let color = match fraction {
        f if f >= 0.9 => theme.error,
        f if f >= 0.7 => theme.warning,
        _ => theme.text_muted,
    };
    let painter = ui.painter();
    painter.circle_stroke(center, radius, Stroke::new(2.0_f32, theme.border_variant));
    if fraction > 0.0 {
        let steps = 48;
        let filled = ((steps as f32 * fraction).ceil() as usize).max(1);
        let points: Vec<_> = (0..=filled)
            .map(|i| {
                // Clockwise from twelve o'clock.
                let angle = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * i as f32 / steps as f32;
                center + vec2(angle.cos(), angle.sin()) * radius
            })
            .collect();
        painter.add(egui::Shape::line(points, Stroke::new(2.0_f32, color)));
    }
    response.on_hover_text(format!(
        "Context: {} of {} tokens ({:.0}%)",
        tokens(usage.used),
        tokens(usage.size),
        fraction * 100.0
    ));
}

/// `850`, `12.4k`, `1M`.
pub fn tokens(count: u64) -> String {
    match count {
        0..1_000 => count.to_string(),
        1_000..1_000_000 => format!("{:.1}k", count as f64 / 1_000.0).replace(".0k", "k"),
        _ => format!("{:.1}M", count as f64 / 1_000_000.0).replace(".0M", "M"),
    }
}

/// The thread's agent. It can only change before the first message, since the conversation lives
/// in the agent.
fn agent_picker(ui: &mut Ui, theme: &Theme, assistant: &mut Assistant) {
    let thread = assistant.active();
    let label = thread.agent_name.clone();
    if thread.has_messages() {
        picker_label(ui, theme, &label)
            .on_hover_text("Each thread keeps its agent. Start a new thread (+) to talk to another one.");
        return;
    }
    let current = thread.settings.agent.clone();
    let agents = assistant.agents();
    let popup = Id::new("assistant-agent");
    let response = picker_button(ui, theme, &label);
    let response = if ui.memory(|m| m.is_popup_open(popup)) {
        response
    } else {
        response.on_hover_text("Agent")
    };
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup));
    }
    let mut picked = None;
    egui::popup::popup_above_or_below_widget(
        ui,
        popup,
        &response,
        egui::AboveOrBelow::Above,
        egui::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(220.0);
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.label(RichText::new("Agent").size(12.0).color(theme.text_muted));
            ui.add_space(4.0);
            for agent in &agents {
                let row = menu_row(ui, theme, agent.name(), *agent.id() == current)
                    .on_hover_text(RichText::new(agent.command_line()).monospace().size(11.5));
                if row.clicked() {
                    picked = Some(agent.id().clone());
                }
            }
        },
    );
    if let Some(id) = picked {
        assistant.set_agent(id);
    }
}

/// A picker that cannot be opened: just the muted label.
fn picker_label(ui: &mut Ui, theme: &Theme, label: &str) -> egui::Response {
    picker_label_colored(ui, label, theme.text_muted)
}

fn picker_label_colored(ui: &mut Ui, label: &str, color: Color32) -> egui::Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(12.5), color);
    let (rect, response) = ui.allocate_exact_size(vec2(galley.size().x + 12.0, 24.0), Sense::hover());
    ui.painter().galley(
        pos2(rect.left() + 6.0, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    response
}

/// Zed's composer selectors: muted text with a chevron that opens a menu above it.
fn option_picker(ui: &mut Ui, theme: &Theme, assistant: &mut Assistant, kind: OptionKind) {
    let Some((option, value)) = assistant.active().option(kind) else {
        return;
    };
    let current = value.to_owned();
    let label = match (kind, option.label(value)) {
        (OptionKind::Model, "Default") => "Default model".to_owned(),
        (OptionKind::Effort, name) => format!("{name} effort"),
        (OptionKind::Mode, "Default") => "Default mode".to_owned(),
        (_, name) => name.to_owned(),
    };
    let (title, choices) = (option.name.clone(), option.choices.clone());
    let popup = Id::new(("assistant-option", kind as u8));
    let response = picker_button(ui, theme, &label);
    let response = if ui.memory(|m| m.is_popup_open(popup)) {
        response
    } else {
        response.on_hover_text(&title)
    };
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup));
    }
    let mut picked = None;
    egui::popup::popup_above_or_below_widget(
        ui,
        popup,
        &response,
        egui::AboveOrBelow::Above,
        egui::PopupCloseBehavior::CloseOnClick,
        |ui| {
            ui.set_min_width(180.0);
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.label(RichText::new(&title).size(12.0).color(theme.text_muted));
            ui.add_space(4.0);
            for choice in &choices {
                if menu_row(ui, theme, &choice.name, choice.value == current).clicked() {
                    picked = Some(choice.value.clone());
                }
            }
        },
    );
    if let Some(value) = picked.filter(|value| *value != current) {
        assistant.choose(kind, value);
    }
}

fn picker_button(ui: &mut Ui, theme: &Theme, label: &str) -> egui::Response {
    let font = FontId::proportional(12.5);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, theme.text_muted);
    let size = vec2(galley.size().x + 26.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_hover);
    }
    let text = if response.hovered() {
        theme.text
    } else {
        theme.text_muted
    };
    let y = rect.center().y;
    ui.painter()
        .galley(pos2(rect.left() + 6.0, y - galley.size().y / 2.0), galley, text);
    let chevron = Rect::from_center_size(pos2(rect.right() - 10.0, y), vec2(10.0, 10.0));
    Icon::ChevronDown.paint(ui, chevron, 10.0, theme.icon_muted);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

/// A menu entry with a check on the selected one.
fn menu_row(ui: &mut Ui, theme: &Theme, label: &str, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_hover);
    }
    ui.painter().text(
        pos2(rect.left() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        theme.text,
    );
    if selected {
        let check = Rect::from_center_size(pos2(rect.right() - 14.0, rect.center().y), vec2(12.0, 12.0));
        Icon::Check.paint(ui, check, 12.0, theme.text_accent);
    }
    response.on_hover_cursor(CursorIcon::PointingHand)
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

    #[test]
    fn shortens_token_counts() {
        assert_eq!(tokens(850), "850");
        assert_eq!(tokens(12_400), "12.4k");
        assert_eq!(tokens(200_000), "200k");
        assert_eq!(tokens(1_000_000), "1M");
    }
}
