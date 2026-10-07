//! Meetings from Google Calendar next to the tasks that are due, in a week or month grid laid out
//! like Google Calendar and painted with Zed's tokens.

use super::ViewCx;
use crate::action::{Action, Actions};
use crate::calendar::{CalendarMode, CalendarState, Selected};
use crate::components::{ButtonStyle, button, icon_button, segmented, single_line};
use crate::fonts;
use crate::icons::Icon;
use crate::theme::{self, Theme};
use chrono::{Datelike, Days, Local, Months, NaiveDate, Timelike, Weekday};
use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Frame, Id, Key, Layout, Margin, Order, Rect, RichText,
    ScrollArea, Sense, Spinner, Stroke, Ui, pos2, vec2,
};
use falog_calendar::layout;
use falog_calendar::{Event, EventTime};
use falog_core::domain::Task;

const HOUR_HEIGHT: f32 = 48.0;
const GUTTER: f32 = 52.0;
const LANE_HEIGHT: f32 = 20.0;
/// Short events are drawn at least this long, so their title stays readable.
const MIN_MINUTES: u32 = 20;

pub fn show(
    ui: &mut Ui,
    cx: &ViewCx<'_>,
    cal: &mut CalendarState,
    mode: CalendarMode,
    actions: &mut Actions,
) {
    keyboard(ui, cal, mode, actions);
    header(ui, cx.theme, cal, mode, actions);
    ui.add_space(6.0);
    if !cal.has_client() || cal.config.accounts.is_empty() {
        connect_banner(ui, cx.theme, cal, actions);
        ui.add_space(6.0);
    }

    let (from, to) = range(mode, cal.anchor);
    cal.ensure(from, to);
    let events = cal.events(from, to);
    let tasks: Vec<&Task> = cx
        .tasks
        .iter()
        .filter(|t| t.is_open() && t.due.is_some_and(|d| from <= d && d < to))
        .collect();
    let mut clicked_event = false;
    match mode {
        CalendarMode::Week => week(ui, cx, cal, &events, &tasks, from, &mut clicked_event, actions),
        CalendarMode::Month => month(
            ui,
            cx,
            cal,
            &events,
            &tasks,
            from,
            to,
            &mut clicked_event,
            actions,
        ),
    }
    popover(ui.ctx(), cx.theme, cal, clicked_event);
}

/// First day shown and the day after the last one.
fn range(mode: CalendarMode, anchor: NaiveDate) -> (NaiveDate, NaiveDate) {
    match mode {
        CalendarMode::Week => {
            let monday = anchor.week(Weekday::Mon).first_day();
            (monday, monday + Days::new(7))
        }
        CalendarMode::Month => {
            let first = anchor.with_day(1).unwrap_or(anchor);
            let last = first + Months::new(1) - Days::new(1);
            let start = first.week(Weekday::Mon).first_day();
            let end = last.week(Weekday::Mon).last_day() + Days::new(1);
            (start, end)
        }
    }
}

fn step(mode: CalendarMode, anchor: NaiveDate, forward: bool) -> NaiveDate {
    match (mode, forward) {
        (CalendarMode::Week, true) => anchor + Days::new(7),
        (CalendarMode::Week, false) => anchor - Days::new(7),
        (CalendarMode::Month, true) => anchor + Months::new(1),
        (CalendarMode::Month, false) => anchor - Months::new(1),
    }
}

/// Google Calendar's keys: T today, W/M week/month, J/K or arrows next/previous.
fn keyboard(ui: &Ui, cal: &mut CalendarState, mode: CalendarMode, actions: &mut Actions) {
    if ui.ctx().memory(|m| m.focused().is_some()) {
        return;
    }
    ui.input(|input| {
        if input.key_pressed(Key::T) {
            cal.anchor = Local::now().date_naive();
        }
        if input.key_pressed(Key::W) {
            actions.push(Action::SetCalendarMode(CalendarMode::Week));
        }
        if input.key_pressed(Key::M) {
            actions.push(Action::SetCalendarMode(CalendarMode::Month));
        }
        if input.key_pressed(Key::J) || input.key_pressed(Key::ArrowRight) {
            cal.anchor = step(mode, cal.anchor, true);
        }
        if input.key_pressed(Key::K) || input.key_pressed(Key::ArrowLeft) {
            cal.anchor = step(mode, cal.anchor, false);
        }
        if input.key_pressed(Key::Escape) {
            cal.selected = None;
        }
    });
}

fn header(ui: &mut Ui, theme: &Theme, cal: &mut CalendarState, mode: CalendarMode, actions: &mut Actions) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if button(ui, ButtonStyle::Filled, None, "Today")
            .on_hover_text("Go to today (T)")
            .clicked()
        {
            cal.anchor = Local::now().date_naive();
        }
        if icon_button(ui, Icon::ChevronLeft, "Previous (K)").clicked() {
            cal.anchor = step(mode, cal.anchor, false);
        }
        if icon_button(ui, Icon::ChevronRight, "Next (J)").clicked() {
            cal.anchor = step(mode, cal.anchor, true);
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new(title(mode, cal.anchor))
                .font(fonts::semibold(16.0))
                .color(theme.text),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let mut value = mode;
            let options: Vec<(CalendarMode, &str)> =
                CalendarMode::ALL.iter().map(|m| (*m, m.label())).collect();
            if segmented(ui, &mut value, &options) {
                actions.push(Action::SetCalendarMode(value));
            }
            ui.add_space(6.0);
            if cal.is_busy() {
                ui.add(Spinner::new().size(14.0).color(theme.text_muted));
            } else if !cal.config.accounts.is_empty() && icon_button(ui, Icon::Refresh, "Refresh").clicked() {
                cal.refresh();
            }
            status(ui, theme, cal);
        });
    });
}

fn title(mode: CalendarMode, anchor: NaiveDate) -> String {
    let (from, to) = range(mode, anchor);
    match mode {
        CalendarMode::Month => anchor.format("%B %Y").to_string(),
        CalendarMode::Week => {
            let last = to - Days::new(1);
            if from.month() == last.month() {
                format!("{} – {}, {}", from.format("%b %-d"), last.day(), last.year())
            } else if from.year() == last.year() {
                format!(
                    "{} – {}, {}",
                    from.format("%b %-d"),
                    last.format("%b %-d"),
                    last.year()
                )
            } else {
                format!("{} – {}", from.format("%b %-d, %Y"), last.format("%b %-d, %Y"))
            }
        }
    }
}

fn status(ui: &mut Ui, theme: &Theme, cal: &CalendarState) {
    if let Some(error) = &cal.error {
        let galley = single_line(ui, error, FontId::proportional(12.5), theme.warning, 360.0, false);
        let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
        ui.painter().galley(rect.min, galley, theme.warning);
        response.on_hover_text(error.as_str());
    } else if let Some(at) = cal.updated_at {
        ui.label(
            RichText::new(format!("Updated {}", at.format("%H:%M")))
                .size(12.5)
                .color(theme.text_placeholder),
        );
    }
}

fn connect_banner(ui: &mut Ui, theme: &Theme, cal: &mut CalendarState, actions: &mut Actions) {
    Frame::none()
        .fill(theme.elevated_surface)
        .stroke(Stroke::new(1.0_f32, theme.border_variant))
        .rounding(6.0)
        .inner_margin(Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(Icon::Calendar.image(16.0, theme.text_accent));
                let text = if cal.has_client() {
                    "Connect a Google account to see your meetings next to your tasks."
                } else {
                    "See your Google Calendar meetings here: add your Google OAuth client in Settings, then connect your accounts."
                };
                ui.label(RichText::new(text).size(13.0).color(theme.text_muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !cal.has_client() {
                        if button(ui, ButtonStyle::Accent, Some(Icon::Settings), "Set up").clicked() {
                            actions.push(Action::OpenSettings);
                        }
                    } else if cal.is_connecting() {
                        if button(ui, ButtonStyle::Ghost, None, "Cancel").clicked() {
                            cal.cancel_connect();
                        }
                        ui.label(
                            RichText::new("Finish signing in in your browser…")
                                .size(12.5)
                                .color(theme.text_placeholder),
                        );
                    } else if button(ui, ButtonStyle::Accent, Some(Icon::Plus), "Connect Google account").clicked() {
                        actions.push(Action::ConnectGoogle);
                    }
                });
            });
        });
}

// ---- event colors --------------------------------------------------------------------------

fn calendar_color(cal: &CalendarState, event: &Event) -> Color32 {
    let [r, g, b] = cal.calendar_of(event).map_or([0x74, 0xad, 0xe8], |c| c.rgb());
    Color32::from_rgb(r, g, b)
}

/// The tinted fill of an event block: the calendar color over the editor background.
fn tint(color: Color32, theme: &Theme, past: bool) -> Color32 {
    let alpha = match (theme.dark, past) {
        (true, false) => 0.30,
        (true, true) => 0.14,
        (false, false) => 0.22,
        (false, true) => 0.10,
    };
    let mix = |a: u8, b: u8| (a as f32 * alpha + b as f32 * (1.0 - alpha)).round() as u8;
    Color32::from_rgb(
        mix(color.r(), theme.editor.r()),
        mix(color.g(), theme.editor.g()),
        mix(color.b(), theme.editor.b()),
    )
}

fn time_range(event: &Event) -> String {
    match (event.start, event.end) {
        (EventTime::At(start), EventTime::At(end)) if start.date() == end.date() => {
            format!("{} – {}", start.format("%H:%M"), end.format("%H:%M"))
        }
        (EventTime::At(start), EventTime::At(end)) => {
            format!(
                "{} – {}",
                start.format("%a %b %-d, %H:%M"),
                end.format("%a %b %-d, %H:%M")
            )
        }
        _ => {
            let (first, last) = event.first_and_last_day();
            if first == last {
                format!("{}, all day", first.format("%a, %b %-d"))
            } else {
                format!("{} – {}", first.format("%a, %b %-d"), last.format("%a, %b %-d"))
            }
        }
    }
}

// ---- week ------------------------------------------------------------------------------------

/// A bar in the all-day row: an event or a task due that day.
enum Bar<'a> {
    Event(&'a Event),
    Task(&'a Task),
}

#[allow(clippy::too_many_arguments)]
fn week(
    ui: &mut Ui,
    cx: &ViewCx<'_>,
    cal: &mut CalendarState,
    events: &[Event],
    tasks: &[&Task],
    monday: NaiveDate,
    clicked: &mut bool,
    actions: &mut Actions,
) {
    let theme = cx.theme;
    let days: Vec<NaiveDate> = (0..7).map(|i| monday + Days::new(i)).collect();
    let width = ui.available_width();
    let column = (width - GUTTER) / 7.0;
    let x_of = |index: usize, left: f32| left + GUTTER + column * index as f32;

    // Day names.
    let (head, _) = ui.allocate_exact_size(vec2(width, 46.0), Sense::hover());
    for (i, day) in days.iter().enumerate() {
        let x = x_of(i, head.left()) + column / 2.0;
        let today = *day == cx.today;
        let name_color = if today {
            theme.text_accent
        } else {
            theme.text_muted
        };
        ui.painter().text(
            pos2(x, head.top() + 9.0),
            Align2::CENTER_CENTER,
            day.format("%a").to_string().to_uppercase(),
            fonts::semibold(11.0),
            name_color,
        );
        let center = pos2(x, head.top() + 30.0);
        if today {
            ui.painter().circle_filled(center, 13.0, theme.text_accent);
        }
        ui.painter().text(
            center,
            Align2::CENTER_CENTER,
            day.day().to_string(),
            FontId::proportional(17.0),
            if today { theme.editor } else { theme.text },
        );
    }

    // All-day row: multi-day events span their days, one lane each when they overlap.
    let mut bars: Vec<(usize, usize, Bar<'_>)> = Vec::new();
    for event in events.iter().filter(|e| e.in_all_day_row()) {
        let (first, last) = event.first_and_last_day();
        let start = days.iter().position(|d| *d >= first).unwrap_or(0);
        let end = days.iter().rposition(|d| *d <= last).unwrap_or(6);
        bars.push((start, end, Bar::Event(event)));
    }
    for task in tasks {
        if let Some(index) = task.due.and_then(|due| days.iter().position(|d| *d == due)) {
            bars.push((index, index, Bar::Task(task)));
        }
    }
    let mut lane_ends: Vec<usize> = Vec::new();
    let mut lanes = Vec::with_capacity(bars.len());
    for (start, end, _) in &bars {
        let lane = match lane_ends.iter().position(|&lane_end| lane_end < *start) {
            Some(free) => free,
            None => {
                lane_ends.push(0);
                lane_ends.len() - 1
            }
        };
        lane_ends[lane] = *end;
        lanes.push(lane);
    }
    let row_height = (lane_ends.len().max(1) as f32 * (LANE_HEIGHT + 2.0) + 6.0).min(LANE_HEIGHT * 5.0);
    let (all_day, _) = ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
    ui.painter().text(
        pos2(all_day.left() + GUTTER - 8.0, all_day.top() + 12.0),
        Align2::RIGHT_CENTER,
        "all-day",
        FontId::proportional(10.5),
        theme.text_placeholder,
    );
    for ((start, end, bar), lane) in bars.iter().zip(&lanes) {
        let rect = Rect::from_min_max(
            pos2(
                x_of(*start, all_day.left()) + 2.0,
                all_day.top() + 3.0 + *lane as f32 * (LANE_HEIGHT + 2.0),
            ),
            pos2(
                x_of(*end + 1, all_day.left()) - 2.0,
                all_day.top() + 3.0 + *lane as f32 * (LANE_HEIGHT + 2.0) + LANE_HEIGHT,
            ),
        );
        if rect.bottom() > all_day.bottom() {
            continue;
        }
        match bar {
            Bar::Event(event) => {
                let color = calendar_color(cal, event);
                let past = event.first_and_last_day().1 < cx.today;
                let id = Id::new(("all-day", &event.account, &event.id, event.start));
                if event_chip(ui, theme, rect, id, &event.title, color, past).clicked() {
                    cal.selected = Some(Selected {
                        event: (*event).clone(),
                        anchor: rect.left_bottom(),
                    });
                    *clicked = true;
                }
            }
            Bar::Task(task) => {
                if task_chip(ui, cx, rect, task).clicked() {
                    actions.push(Action::OpenTask(task.id));
                }
            }
        }
    }
    let line = Stroke::new(1.0_f32, theme.border_variant);
    ui.painter().hline(
        all_day.x_range(),
        all_day.bottom(),
        Stroke::new(1.0_f32, theme.border),
    );

    // Hours.
    let now = Local::now().naive_local();
    let initial = if cal.scrolled {
        None
    } else {
        cal.scrolled = true;
        let hour = if days.contains(&cx.today) {
            now.hour().saturating_sub(1).min(16)
        } else {
            8
        };
        Some(hour as f32 * HOUR_HEIGHT)
    };
    let mut scroll = ScrollArea::vertical()
        .id_salt("calendar-week")
        .auto_shrink([false, false]);
    if let Some(offset) = initial {
        scroll = scroll.vertical_scroll_offset(offset);
    }
    scroll.show(ui, |ui| {
        let (grid, _) = ui.allocate_exact_size(vec2(width, HOUR_HEIGHT * 24.0), Sense::hover());
        let painter = ui.painter_at(grid);
        if let Some(index) = days.iter().position(|d| *d == cx.today) {
            let left = x_of(index, grid.left());
            painter.rect_filled(
                Rect::from_x_y_ranges(left..=left + column, grid.y_range()),
                0.0,
                theme.ghost_hover.gamma_multiply(0.5),
            );
        }
        for hour in 0..24 {
            let y = grid.top() + hour as f32 * HOUR_HEIGHT;
            if hour > 0 {
                painter.hline(grid.left() + GUTTER - 4.0..=grid.right(), y, line);
                painter.text(
                    pos2(grid.left() + GUTTER - 8.0, y),
                    Align2::RIGHT_CENTER,
                    format!("{hour:02}:00"),
                    FontId::proportional(11.0),
                    theme.text_placeholder,
                );
            }
        }
        for i in 0..=7 {
            painter.vline(x_of(i, grid.left()), grid.y_range(), line);
        }

        for (index, day) in days.iter().enumerate() {
            let timed: Vec<&Event> = events
                .iter()
                .filter(|e| !e.in_all_day_row() && e.touches(*day))
                .collect();
            let minutes: Vec<(u32, u32)> = timed.iter().map(|e| e.minutes_on(*day)).collect();
            let slots = layout::columns(&minutes, MIN_MINUTES);
            let left = x_of(index, grid.left()) + 2.0;
            let inner = column - 6.0;
            for ((event, (start, end)), slot) in timed.iter().zip(&minutes).zip(&slots) {
                let slot_width = inner / slot.columns as f32;
                let x = left + slot_width * slot.column as f32;
                let top = grid.top() + *start as f32 / 60.0 * HOUR_HEIGHT;
                let height = ((end - start).max(MIN_MINUTES) as f32 / 60.0 * HOUR_HEIGHT).max(16.0);
                let rect = Rect::from_min_size(pos2(x, top + 1.0), vec2(slot_width - 2.0, height - 2.0));
                let past = matches!(event.end, EventTime::At(end) if end < now);
                if timed_event(ui, cal, theme, rect, event, past).clicked() {
                    cal.selected = Some(Selected {
                        event: (*event).clone(),
                        anchor: rect.right_top(),
                    });
                    *clicked = true;
                }
            }
        }

        if let Some(index) = days.iter().position(|d| *d == cx.today) {
            let y = grid.top() + (now.hour() * 60 + now.minute()) as f32 / 60.0 * HOUR_HEIGHT;
            let left = x_of(index, grid.left());
            painter.hline(left..=left + column, y, Stroke::new(2.0_f32, theme.error));
            painter.circle_filled(pos2(left, y), 5.0, theme.error);
        }
    });
}

fn timed_event(
    ui: &mut Ui,
    cal: &CalendarState,
    theme: &Theme,
    rect: Rect,
    event: &Event,
    past: bool,
) -> egui::Response {
    let response = ui
        .interact(
            rect,
            Id::new(("event", &event.account, &event.id, event.start)),
            Sense::click(),
        )
        .on_hover_cursor(CursorIcon::PointingHand);
    let color = calendar_color(cal, event);
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, tint(color, theme, past));
    painter.rect_filled(
        Rect::from_min_size(rect.min, vec2(3.0, rect.height())),
        egui::Rounding {
            nw: 4.0,
            sw: 4.0,
            ..Default::default()
        },
        if past { color.gamma_multiply(0.5) } else { color },
    );
    if response.hovered() {
        painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, color));
    }
    let text_color = if past { theme.text_muted } else { theme.text };
    let text_width = rect.width() - 10.0;
    let x = rect.left() + 7.0;
    if rect.height() >= 34.0 {
        let title = single_line(
            ui,
            &event.title,
            fonts::semibold(12.0),
            text_color,
            text_width,
            false,
        );
        painter.galley(pos2(x, rect.top() + 3.0), title, text_color);
        let time = single_line(
            ui,
            &time_range(event),
            FontId::proportional(11.0),
            theme.text_muted,
            text_width,
            false,
        );
        painter.galley(pos2(x, rect.top() + 18.0), time, theme.text_muted);
    } else {
        let start = match event.start {
            EventTime::At(at) => at.format("%H:%M").to_string(),
            EventTime::Date(_) => String::new(),
        };
        let text = format!("{}, {start}", event.title);
        let line = single_line(
            ui,
            &text,
            FontId::proportional(11.5),
            text_color,
            text_width,
            false,
        );
        painter.galley(pos2(x, rect.center().y - line.size().y / 2.0), line, text_color);
    }
    response
}

/// A filled one-line bar for all-day events.
fn event_chip(
    ui: &mut Ui,
    theme: &Theme,
    rect: Rect,
    id: Id,
    title: &str,
    color: Color32,
    past: bool,
) -> egui::Response {
    let response = ui
        .interact(rect, id, Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    let fill = if past { tint(color, theme, false) } else { color };
    ui.painter().rect_filled(rect, 4.0, fill);
    if response.hovered() {
        ui.painter()
            .rect_stroke(rect, 4.0, Stroke::new(1.0_f32, theme.border_focused));
    }
    let text_color = if past {
        theme.text_muted
    } else {
        readable_on(color)
    };
    let galley = single_line(
        ui,
        title,
        FontId::proportional(12.0),
        text_color,
        rect.width() - 10.0,
        false,
    );
    ui.painter().galley(
        pos2(rect.left() + 6.0, rect.center().y - galley.size().y / 2.0),
        galley,
        text_color,
    );
    response
}

/// Black or white text, whichever reads better on `fill`.
fn readable_on(fill: Color32) -> Color32 {
    let luma = 0.299 * fill.r() as f32 + 0.587 * fill.g() as f32 + 0.114 * fill.b() as f32;
    if luma > 150.0 {
        Color32::from_rgb(0x1e, 0x21, 0x27)
    } else {
        Color32::WHITE
    }
}

/// A task due that day: status icon, title, area color.
fn task_chip(ui: &mut Ui, cx: &ViewCx<'_>, rect: Rect, task: &Task) -> egui::Response {
    let theme = cx.theme;
    let response = ui
        .interact(rect, Id::new(("calendar-task", task.id)), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(format!("#{} {} · {}", task.id, task.title, task.area_name()));
    let area = task
        .area
        .as_ref()
        .map_or(theme.text_placeholder, |a| theme::color(a.color));
    let selected = cx.selected == Some(task.id);
    let fill = if selected || response.hovered() {
        theme.ghost_hover
    } else {
        theme.element
    };
    ui.painter()
        .rect(rect, 4.0, fill, Stroke::new(1.0_f32, theme.border_variant));
    ui.painter().rect_filled(
        Rect::from_min_size(rect.min + vec2(0.0, 0.0), vec2(3.0, rect.height())),
        egui::Rounding {
            nw: 4.0,
            sw: 4.0,
            ..Default::default()
        },
        area,
    );
    let icon_rect = Rect::from_center_size(pos2(rect.left() + 13.0, rect.center().y), vec2(12.0, 12.0));
    Icon::for_status(task.status).paint(ui, icon_rect, 12.0, theme.status_color(task.status));
    let galley = single_line(
        ui,
        &task.title,
        FontId::proportional(12.0),
        theme.text,
        rect.width() - 28.0,
        false,
    );
    ui.painter().galley(
        pos2(rect.left() + 22.0, rect.center().y - galley.size().y / 2.0),
        galley,
        theme.text,
    );
    response
}

// ---- month -----------------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn month(
    ui: &mut Ui,
    cx: &ViewCx<'_>,
    cal: &mut CalendarState,
    events: &[Event],
    tasks: &[&Task],
    from: NaiveDate,
    to: NaiveDate,
    clicked: &mut bool,
    actions: &mut Actions,
) {
    let theme = cx.theme;
    let weeks = ((to - from).num_days() / 7).max(1) as usize;
    let width = ui.available_width();
    let column = width / 7.0;
    let (head, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
    for i in 0..7 {
        let day = from + Days::new(i as u64);
        ui.painter().text(
            pos2(head.left() + column * (i as f32 + 0.5), head.center().y),
            Align2::CENTER_CENTER,
            day.format("%a").to_string().to_uppercase(),
            fonts::semibold(11.0),
            theme.text_muted,
        );
    }
    let height = ui.available_height().max(weeks as f32 * 80.0);
    let (grid, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let row = grid.height() / weeks as f32;
    let line = Stroke::new(1.0_f32, theme.border_variant);
    for w in 0..=weeks {
        ui.painter()
            .hline(grid.x_range(), grid.top() + row * w as f32, line);
    }
    for i in 0..=7 {
        ui.painter()
            .vline(grid.left() + column * i as f32, grid.y_range(), line);
    }

    let month = cal.anchor.month();
    for w in 0..weeks {
        for i in 0..7 {
            let day = from + Days::new((w * 7 + i) as u64);
            let cell = Rect::from_min_size(
                pos2(grid.left() + column * i as f32, grid.top() + row * w as f32),
                vec2(column, row),
            );
            month_cell(
                ui,
                cx,
                cal,
                events,
                tasks,
                day,
                cell,
                day.month() == month,
                clicked,
                actions,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn month_cell(
    ui: &mut Ui,
    cx: &ViewCx<'_>,
    cal: &mut CalendarState,
    events: &[Event],
    tasks: &[&Task],
    day: NaiveDate,
    cell: Rect,
    in_month: bool,
    clicked: &mut bool,
    actions: &mut Actions,
) {
    let theme = cx.theme;
    let today = day == cx.today;
    let number = pos2(cell.left() + 14.0, cell.top() + 13.0);
    if today {
        ui.painter().circle_filled(number, 10.0, theme.text_accent);
    }
    let number_color = match (today, in_month) {
        (true, _) => theme.editor,
        (false, true) => theme.text,
        (false, false) => theme.text_placeholder,
    };
    ui.painter().text(
        number,
        Align2::CENTER_CENTER,
        day.day().to_string(),
        FontId::proportional(12.5),
        number_color,
    );
    let number_hit = Rect::from_center_size(number, vec2(22.0, 22.0));
    if ui
        .interact(number_hit, Id::new(("month-day", day)), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text("Open this week")
        .clicked()
    {
        cal.anchor = day;
        actions.push(Action::SetCalendarMode(CalendarMode::Week));
    }

    let day_events: Vec<&Event> = events.iter().filter(|e| e.touches(day)).collect();
    let day_tasks: Vec<&&Task> = tasks.iter().filter(|t| t.due == Some(day)).collect();
    let total = day_events.len() + day_tasks.len();
    let line_height = 18.0;
    let top = cell.top() + 26.0;
    let fits = (((cell.bottom() - 4.0 - top) / line_height).floor() as usize).max(1);
    let shown = if total > fits { fits - 1 } else { total };
    let width = cell.width() - 8.0;
    let now = Local::now().naive_local();

    for (n, event) in day_events.iter().take(shown).enumerate() {
        let rect = Rect::from_min_size(
            pos2(cell.left() + 4.0, top + n as f32 * line_height),
            vec2(width, line_height - 2.0),
        );
        let color = calendar_color(cal, event);
        let id = Id::new(("month-event", day, &event.account, &event.id, event.start));
        let response = if event.in_all_day_row() {
            let past = event.first_and_last_day().1 < cx.today;
            event_chip(ui, theme, rect, id, &event.title, color, past)
        } else {
            let past = matches!(event.end, EventTime::At(end) if end < now);
            month_line(ui, theme, rect, id, event, color, past)
        };
        if response.clicked() {
            cal.selected = Some(Selected {
                event: (*event).clone(),
                anchor: rect.right_top(),
            });
            *clicked = true;
        }
    }
    for (n, task) in day_tasks
        .iter()
        .take(shown.saturating_sub(day_events.len().min(shown)))
        .enumerate()
    {
        let index = day_events.len().min(shown) + n;
        let rect = Rect::from_min_size(
            pos2(cell.left() + 4.0, top + index as f32 * line_height),
            vec2(width, line_height - 2.0),
        );
        if task_chip(ui, cx, rect, task).clicked() {
            actions.push(Action::OpenTask(task.id));
        }
    }
    if total > shown {
        let rect = Rect::from_min_size(
            pos2(cell.left() + 4.0, top + shown as f32 * line_height),
            vec2(width, line_height - 2.0),
        );
        let response = ui
            .interact(rect, Id::new(("month-more", day)), Sense::click())
            .on_hover_cursor(CursorIcon::PointingHand);
        let color = if response.hovered() {
            theme.text
        } else {
            theme.text_muted
        };
        ui.painter().text(
            pos2(rect.left() + 6.0, rect.center().y),
            Align2::LEFT_CENTER,
            format!("+{} more", total - shown),
            fonts::semibold(11.5),
            color,
        );
        if response.clicked() {
            cal.anchor = day;
            actions.push(Action::SetCalendarMode(CalendarMode::Week));
        }
    }
}

/// A timed event in the month grid: colored dot, start time, title.
fn month_line(
    ui: &mut Ui,
    theme: &Theme,
    rect: Rect,
    id: Id,
    event: &Event,
    color: Color32,
    past: bool,
) -> egui::Response {
    let response = ui
        .interact(rect, id, Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    if response.hovered() {
        ui.painter().rect_filled(rect, 4.0, theme.ghost_hover);
    }
    let dot = if past { color.gamma_multiply(0.5) } else { color };
    ui.painter()
        .circle_filled(pos2(rect.left() + 6.0, rect.center().y), 3.5, dot);
    let start = match event.start {
        EventTime::At(at) => at.format("%H:%M ").to_string(),
        EventTime::Date(_) => String::new(),
    };
    let text_color = if past { theme.text_muted } else { theme.text };
    let galley = single_line(
        ui,
        &format!("{start}{}", event.title),
        FontId::proportional(12.0),
        text_color,
        rect.width() - 16.0,
        false,
    );
    ui.painter().galley(
        pos2(rect.left() + 14.0, rect.center().y - galley.size().y / 2.0),
        galley,
        text_color,
    );
    response
}

// ---- details -------------------------------------------------------------------------------

const POPOVER_WIDTH: f32 = 340.0;

fn popover(ctx: &egui::Context, theme: &Theme, cal: &mut CalendarState, clicked_event: bool) {
    let Some(selected) = cal.selected.clone() else {
        return;
    };
    let screen = ctx.screen_rect();
    let mut pos = selected.anchor + vec2(8.0, 0.0);
    if pos.x + POPOVER_WIDTH > screen.right() - 8.0 {
        pos.x = (selected.anchor.x - POPOVER_WIDTH - 8.0).max(screen.left() + 8.0);
    }
    pos.y = pos.y.min(screen.bottom() - 260.0).max(screen.top() + 8.0);
    let event = &selected.event;
    let calendar = cal.calendar_of(event).cloned();
    let color = calendar_color(cal, event);
    let mut close = false;
    let area = egui::Area::new(Id::new("calendar-event-details"))
        .order(Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ui| {
            Frame::none()
                .fill(theme.elevated_surface)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .rounding(8.0)
                .shadow(theme.popover_shadow())
                .inner_margin(Margin::same(14.0))
                .show(ui, |ui| {
                    ui.set_width(POPOVER_WIDTH - 28.0);
                    ui.horizontal(|ui| {
                        let (square, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
                        ui.painter().rect_filled(square, 3.0, color);
                        ui.add(
                            egui::Label::new(
                                RichText::new(&event.title)
                                    .font(fonts::semibold(15.0))
                                    .color(theme.text),
                            )
                            .wrap(),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                            if icon_button(ui, Icon::Close, "Close (Esc)").clicked() {
                                close = true;
                            }
                        });
                    });
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(time_range(event))
                            .size(13.0)
                            .color(theme.text_muted),
                    );
                    if let Some(calendar) = &calendar {
                        ui.label(
                            RichText::new(format!("{} · {}", calendar.name, event.account))
                                .size(12.0)
                                .color(theme.text_placeholder),
                        );
                    }
                    if !event.location.is_empty() {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.add(Icon::MapPin.image(13.0, theme.icon_muted));
                            ui.add(
                                egui::Label::new(RichText::new(&event.location).size(13.0).color(theme.text))
                                    .wrap(),
                            );
                        });
                    }
                    let description = event.plain_description();
                    if !description.is_empty() {
                        ui.add_space(6.0);
                        ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(description).size(12.5).color(theme.text_muted),
                                )
                                .wrap(),
                            );
                        });
                    }
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if let Some(link) = &event.join_link
                            && button(ui, ButtonStyle::Accent, Some(Icon::Video), "Join").clicked()
                        {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(link));
                        }
                        if let Some(link) = &event.html_link
                            && button(
                                ui,
                                ButtonStyle::Ghost,
                                Some(Icon::ArrowUpRight),
                                "Open in Google Calendar",
                            )
                            .clicked()
                        {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(link));
                        }
                    });
                });
        });
    let clicked_outside = ctx.input(|i| i.pointer.any_pressed())
        && ctx
            .pointer_interact_pos()
            .is_some_and(|p| !area.response.rect.contains(p));
    if close || (clicked_outside && !clicked_event) {
        cal.selected = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn weeks_start_on_monday() {
        assert_eq!(
            range(CalendarMode::Week, day(2026, 10, 7)),
            (day(2026, 10, 5), day(2026, 10, 12))
        );
        assert_eq!(
            range(CalendarMode::Week, day(2026, 10, 11)),
            (day(2026, 10, 5), day(2026, 10, 12))
        );
    }

    #[test]
    fn months_cover_whole_weeks() {
        // October 2026 starts on a Thursday and ends on a Saturday.
        assert_eq!(
            range(CalendarMode::Month, day(2026, 10, 7)),
            (day(2026, 9, 28), day(2026, 11, 2))
        );
    }

    #[test]
    fn titles_read_like_google_calendar() {
        assert_eq!(title(CalendarMode::Week, day(2026, 10, 7)), "Oct 5 – 11, 2026");
        assert_eq!(
            title(CalendarMode::Week, day(2026, 9, 30)),
            "Sep 28 – Oct 4, 2026"
        );
        assert_eq!(
            title(CalendarMode::Week, day(2026, 12, 30)),
            "Dec 28, 2026 – Jan 3, 2027"
        );
        assert_eq!(title(CalendarMode::Month, day(2026, 10, 7)), "October 2026");
    }

    #[test]
    fn steps_by_week_or_month() {
        assert_eq!(
            step(CalendarMode::Week, day(2026, 10, 7), true),
            day(2026, 10, 14)
        );
        assert_eq!(
            step(CalendarMode::Month, day(2026, 1, 31), true),
            day(2026, 2, 28)
        );
    }
}
