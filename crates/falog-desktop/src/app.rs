//! Application state and the per-frame orchestration of the workspace.

use crate::action::{Action, Actions};
use crate::assistant::{self, Assistant};
use crate::overlays::command_palette::{CommandPalette, Mode, Outcome};
use crate::overlays::companies::{CompaniesDialog, CompanyEvent};
use crate::overlays::confirm::{Answer, Confirm};
use crate::overlays::settings::{SettingsDialog, SettingsEvent};
use crate::platform::{autostart, title_bar};
use crate::prefs::Prefs;
use crate::theme::{self, Theme};
use crate::views::agenda::AgendaState;
use crate::views::board::BoardState;
use crate::views::{self, View, ViewCx};
use crate::workspace::status_bar::{self, StatusBar, Toast, ToastKind};
use crate::workspace::task_panel::{self, PanelEvent, TaskPanel};
use crate::workspace::{sidebar, tab_bar, toolbar};
use crate::{fonts, icons::Icon};
use chrono::NaiveDate;
use eframe::egui::{self, Frame, Id, Key, Margin, Modifiers, RichText, ViewportCommand};
use falog_core::domain::{Company, Status, Task, TaskId, TaskPatch};
use falog_core::{Store, agenda, date};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// How often to check whether the MCP server (or another process) changed the database.
const POLL_INTERVAL: Duration = Duration::from_millis(800);
const TOAST_DURATION: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct FalogApp {
    store: Result<Store, String>,
    tasks: Vec<Task>,
    companies: Vec<Company>,
    data_version: i64,
    last_poll: Instant,
    today: NaiveDate,
    prefs: Prefs,
    search: String,
    board: BoardState,
    agenda: AgendaState,
    task_panel: Option<TaskPanel>,
    assistant: Assistant,
    palette: Option<CommandPalette>,
    companies_dialog: Option<CompaniesDialog>,
    settings: Option<SettingsDialog>,
    confirm: Option<Confirm>,
    toast: Option<Toast>,
    show_requested: Arc<AtomicBool>,
    title_bar_dark: Option<bool>,
}

impl FalogApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        store: falog_core::Result<Store>,
        show_requested: Arc<AtomicBool>,
    ) -> Self {
        fonts::install(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let prefs: Prefs = cc
            .storage
            .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
            .unwrap_or_default();
        theme::install(&cc.egui_ctx, prefs.theme);
        let assistant = Assistant::new(store.as_ref().ok().and_then(Store::path));

        let mut app = Self {
            store: store.map_err(|err| err.to_string()),
            tasks: Vec::new(),
            companies: Vec::new(),
            data_version: 0,
            last_poll: Instant::now(),
            today: date::today(),
            prefs,
            search: String::new(),
            board: BoardState::default(),
            agenda: AgendaState::default(),
            task_panel: None,
            assistant,
            palette: None,
            companies_dialog: None,
            settings: None,
            confirm: None,
            toast: None,
            show_requested,
            title_bar_dark: None,
        };
        app.reload();
        app
    }

    // ---- data ------------------------------------------------------------------------------

    fn reload(&mut self) {
        let Ok(store) = &self.store else { return };
        let loaded = store.tasks().and_then(|tasks| Ok((tasks, store.companies()?)));
        self.data_version = store.data_version().unwrap_or_default();
        match loaded {
            Ok((tasks, companies)) => {
                self.tasks = tasks;
                self.companies = companies;
                if let Some(id) = self.prefs.company
                    && !self.companies.iter().any(|c| c.id == id)
                {
                    self.prefs.company = None;
                }
            }
            Err(err) => self.notify(format!("Could not load tasks: {err}"), ToastKind::Error),
        }
    }

    /// Checks for outside writes at most every [`POLL_INTERVAL`].
    fn poll(&mut self) {
        if self.last_poll.elapsed() >= POLL_INTERVAL {
            self.sync();
        }
    }

    /// Picks up writes made by the MCP server, refreshing the open task unless it has edits.
    fn sync(&mut self) {
        self.last_poll = Instant::now();
        self.today = date::today();
        let current = self
            .store
            .as_ref()
            .ok()
            .and_then(|store| store.data_version().ok());
        if current.is_none_or(|version| version == self.data_version) {
            return;
        }
        self.reload();
        if let Some(id) = self.task_panel.as_ref().and_then(|p| p.id) {
            if self.task_panel.as_ref().is_some_and(TaskPanel::is_dirty) {
                self.refresh_notes(id);
            } else {
                self.open_task(id);
            }
        }
    }

    /// Runs a write; on success reloads the data, on failure shows the error.
    fn write<T>(&mut self, op: impl FnOnce(&Store) -> falog_core::Result<T>) -> Option<T> {
        let result = match &self.store {
            Ok(store) => op(store),
            Err(_) => return None,
        };
        match result {
            Ok(value) => {
                self.reload();
                Some(value)
            }
            Err(err) => {
                self.notify(err.to_string(), ToastKind::Error);
                None
            }
        }
    }

    fn notify(&mut self, message: impl Into<String>, kind: ToastKind) {
        self.toast = Some(Toast {
            message: message.into(),
            kind,
            shown_at: Instant::now(),
        });
    }

    /// Tasks after the company filter and the search box.
    fn visible_tasks(&self) -> Vec<Task> {
        self.tasks
            .iter()
            .filter(|t| self.prefs.company.is_none() || t.company_id() == self.prefs.company)
            .filter(|t| t.matches(&self.search))
            .cloned()
            .collect()
    }

    fn scope_label(&self) -> String {
        self.prefs
            .company
            .and_then(|id| self.companies.iter().find(|c| c.id == id))
            .map_or_else(|| "All companies".to_owned(), |c| c.name.clone())
    }

    // ---- task panel ------------------------------------------------------------------------

    fn open_task(&mut self, id: TaskId) {
        let Ok(store) = &self.store else { return };
        let loaded = store
            .require_task(id)
            .and_then(|task| Ok((task, store.notes(id)?)));
        match loaded {
            Ok((task, notes)) => {
                let new_note = self.task_panel.as_mut().map(|p| std::mem::take(&mut p.new_note));
                let mut panel = TaskPanel::edit(&task, notes);
                if self.task_panel.as_ref().and_then(|p| p.id) == Some(id) {
                    panel.new_note = new_note.unwrap_or_default();
                }
                self.task_panel = Some(panel);
            }
            Err(err) => self.notify(err.to_string(), ToastKind::Error),
        }
    }

    fn refresh_notes(&mut self, id: TaskId) {
        let notes = self.store.as_ref().ok().and_then(|store| store.notes(id).ok());
        if let (Some(panel), Some(notes)) = (self.task_panel.as_mut(), notes) {
            panel.notes = notes;
        }
    }

    fn save_panel(&mut self) {
        let today = self.today;
        let Some(panel) = self.task_panel.as_mut() else {
            return;
        };
        let new = match panel.draft.to_new_task(today) {
            Ok(new) => new,
            Err(message) => {
                panel.error = Some(message);
                return;
            }
        };
        let id = panel.id;
        let saved = self.write(|store| match id {
            Some(id) => store.update_task(id, &TaskPatch::from(new)),
            None => store.create_task(&new),
        });
        if let Some(task) = saved {
            self.notify(format!("Saved #{}", task.id), ToastKind::Success);
            self.open_task(task.id);
        }
    }

    fn handle_panel_events(&mut self, events: Vec<PanelEvent>) {
        for event in events {
            let id = self.task_panel.as_ref().and_then(|p| p.id);
            match event {
                PanelEvent::Save => self.save_panel(),
                PanelEvent::Close => {
                    self.close_panel(None);
                }
                PanelEvent::Delete => {
                    if let Some(id) = id {
                        self.ask_delete_task(id);
                    }
                }
                PanelEvent::AddNote(body) => {
                    if let Some(id) = id
                        && self.write(|store| store.add_note(id, &body)).is_some()
                    {
                        self.refresh_notes(id);
                    }
                }
                PanelEvent::DeleteNote(note) => {
                    if let Some(id) = id
                        && self.write(|store| store.delete_note(note)).is_some()
                    {
                        self.refresh_notes(id);
                    }
                }
            }
        }
    }

    /// Closes the panel, asking first if it has unsaved edits; then runs `then`.
    fn close_panel(&mut self, then: Option<Action>) -> bool {
        if self.task_panel.as_ref().is_some_and(TaskPanel::is_dirty) {
            self.confirm = Some(Confirm::DiscardDraft { then });
            return false;
        }
        self.task_panel = None;
        true
    }

    fn ask_delete_task(&mut self, id: TaskId) {
        if let Some(task) = self.tasks.iter().find(|t| t.id == id) {
            self.confirm = Some(Confirm::DeleteTask {
                id,
                title: task.title.clone(),
            });
        }
    }

    // ---- actions ---------------------------------------------------------------------------

    fn shortcuts(&self, ctx: &egui::Context, actions: &mut Actions) -> bool {
        let ctrl = Modifiers::COMMAND;
        let ctrl_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        let mut save = false;
        ctx.input_mut(|input| {
            // Check Ctrl+Shift+P before Ctrl+P, which would also match it.
            if input.consume_key(ctrl_shift, Key::P) {
                actions.push(Action::OpenCommandPalette);
            } else if input.consume_key(ctrl, Key::P) {
                actions.push(Action::OpenTaskFinder);
            }
            if input.consume_key(ctrl_shift, Key::A) {
                actions.push(Action::ToggleAssistant);
            }
            if input.consume_key(ctrl, Key::Space) {
                actions.push(Action::ToggleDictation);
            }
            let bindings = [
                (Key::N, Action::NewTask(Status::Todo)),
                (Key::Num1, Action::SetView(View::Board)),
                (Key::Num2, Action::SetView(View::List)),
                (Key::Num3, Action::SetView(View::Agenda)),
                (Key::B, Action::ToggleSidebar),
                (Key::F, Action::FocusSearch),
                (Key::Comma, Action::OpenSettings),
            ];
            for (key, action) in bindings {
                if input.consume_key(ctrl, key) {
                    actions.push(action);
                }
            }
            if input.consume_key(Modifiers::NONE, Key::F5) {
                actions.push(Action::Reload);
            }
            save = self.task_panel.is_some() && input.consume_key(ctrl, Key::S);
        });
        save
    }

    fn overlay_open(&self) -> bool {
        self.palette.is_some()
            || self.companies_dialog.is_some()
            || self.settings.is_some()
            || self.confirm.is_some()
    }

    fn apply(&mut self, ctx: &egui::Context, action: Action) {
        let replaces_panel = match action {
            Action::OpenTask(id) => self.task_panel.as_ref().and_then(|p| p.id) != Some(id),
            Action::NewTask(_) => true,
            _ => false,
        };
        if replaces_panel && !self.close_panel(Some(action)) {
            return;
        }
        match action {
            Action::OpenTask(id) => self.open_task(id),
            Action::NewTask(status) => self.task_panel = Some(TaskPanel::new(self.prefs.company, status)),
            Action::MoveTask(id, status) => self.move_task(id, status),
            Action::DeleteTask(id) => self.ask_delete_task(id),
            Action::SetView(view) => self.prefs.view = view,
            Action::FilterCompany(company) => self.prefs.company = company,
            Action::ToggleSidebar => self.prefs.sidebar_open = !self.prefs.sidebar_open,
            Action::FocusSearch => ctx.memory_mut(|m| m.request_focus(Id::new(toolbar::SEARCH_ID))),
            Action::OpenCommandPalette => self.palette = Some(CommandPalette::new(Mode::Commands)),
            Action::OpenTaskFinder => self.palette = Some(CommandPalette::new(Mode::Tasks)),
            Action::ManageCompanies => {
                self.companies_dialog = Some(CompaniesDialog::new(self.companies.len()))
            }
            Action::OpenSettings => {
                let path = self.store.as_ref().ok().and_then(Store::path);
                self.settings = Some(SettingsDialog::new(autostart::is_enabled(), path));
            }
            Action::SetTheme(mode) => {
                self.prefs.theme = mode;
                theme::set_mode(ctx, mode);
            }
            Action::Reload => {
                self.reload();
                self.notify("Reloaded", ToastKind::Success);
            }
            Action::ToggleAssistant => {
                self.prefs.assistant_open = !self.prefs.assistant_open;
                self.assistant.focus_composer = self.prefs.assistant_open;
            }
            Action::ToggleDictation => {
                self.prefs.assistant_open = true;
                self.assistant
                    .toggle_dictation(ctx, self.prefs.assistant_options());
            }
            Action::NewAssistantThread => {
                self.prefs.assistant_open = true;
                self.assistant.new_thread();
            }
        }
    }

    fn move_task(&mut self, id: TaskId, status: Status) {
        if self.tasks.iter().any(|t| t.id == id && t.status == status) {
            return;
        }
        if let Some(task) = self.write(|store| store.set_status(id, status)) {
            self.notify(
                format!("#{} moved to {}", task.id, status.label()),
                ToastKind::Success,
            );
            let panel_shows_it = self
                .task_panel
                .as_ref()
                .is_some_and(|p| p.id == Some(id) && !p.is_dirty());
            if panel_shows_it {
                self.open_task(id);
            }
        }
    }

    // ---- overlays --------------------------------------------------------------------------

    fn overlays(&mut self, ctx: &egui::Context, actions: &mut Actions) {
        if let Some(palette) = &mut self.palette {
            match palette.show(ctx, &self.tasks, &self.companies) {
                Outcome::Pending => {}
                Outcome::Dismissed => self.palette = None,
                Outcome::Run(action) => {
                    self.palette = None;
                    actions.push(action);
                }
            }
        }

        if let Some(dialog) = &mut self.companies_dialog {
            let (events, close) = dialog.show(ctx, &self.companies);
            if close {
                self.companies_dialog = None;
            }
            self.company_events(events);
        }

        if let Some(dialog) = &mut self.settings {
            let (events, close) = dialog.show(ctx, &mut self.prefs);
            if close {
                self.settings = None;
            }
            for event in events {
                self.settings_event(ctx, event);
            }
        }

        if let Some(confirm) = &self.confirm {
            match confirm.show(ctx) {
                Answer::Pending => {}
                Answer::No => self.confirm = None,
                Answer::Yes => {
                    if let Some(confirm) = self.confirm.take() {
                        self.confirmed(ctx, confirm);
                    }
                }
            }
        }
    }

    fn company_events(&mut self, events: Vec<CompanyEvent>) {
        for event in events {
            match event {
                CompanyEvent::Create { name, color } => {
                    if let Some(company) = self.write(|store| store.create_company(&name, Some(color))) {
                        self.notify(format!("Added {}", company.name), ToastKind::Success);
                        let count = self.companies.len();
                        if let Some(dialog) = &mut self.companies_dialog {
                            dialog.reset_new(count);
                        }
                    }
                }
                CompanyEvent::Update { id, name, color } => {
                    if let Some(company) = self.write(|store| store.update_company(id, &name, color)) {
                        self.notify(format!("Saved {}", company.name), ToastKind::Success);
                        if let Some(dialog) = &mut self.companies_dialog {
                            dialog.forget(id);
                        }
                    }
                }
                CompanyEvent::Delete { id, name } => self.confirm = Some(Confirm::DeleteCompany { id, name }),
            }
        }
    }

    fn settings_event(&mut self, ctx: &egui::Context, event: SettingsEvent) {
        match event {
            SettingsEvent::SetAutostart(enabled) => match autostart::set_enabled(enabled) {
                Ok(()) => {
                    let message = if enabled {
                        "Falog will open at startup"
                    } else {
                        "Launch at startup disabled"
                    };
                    self.notify(message, ToastKind::Success);
                }
                Err(err) => {
                    self.notify(
                        format!("Could not change launch at startup: {err}"),
                        ToastKind::Error,
                    );
                    if let Some(dialog) = &mut self.settings {
                        dialog.autostart = !enabled;
                    }
                }
            },
            SettingsEvent::SetTheme(mode) => theme::set_mode(ctx, mode),
            SettingsEvent::AssistantModelChanged => self.assistant.restart_session(),
            SettingsEvent::VoiceEngineChanged => self.assistant.reset_voice_engine(),
            SettingsEvent::Copied => self.notify("Copied to clipboard", ToastKind::Success),
        }
    }

    fn confirmed(&mut self, ctx: &egui::Context, confirm: Confirm) {
        match confirm {
            Confirm::DeleteTask { id, .. } => {
                if let Some(task) = self.write(|store| store.delete_task(id)) {
                    self.notify(format!("Deleted #{}", task.id), ToastKind::Success);
                    if self.task_panel.as_ref().is_some_and(|p| p.id == Some(id)) {
                        self.task_panel = None;
                    }
                }
            }
            Confirm::DeleteCompany { id, name } => {
                if self.write(|store| store.delete_company(id)).is_some() {
                    self.notify(format!("Deleted {name}"), ToastKind::Success);
                    if let Some(dialog) = &mut self.companies_dialog {
                        dialog.forget(id);
                    }
                }
            }
            Confirm::DiscardDraft { then } => {
                self.task_panel = None;
                if let Some(action) = then {
                    self.apply(ctx, action);
                }
            }
        }
    }
}

impl eframe::App for FalogApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let theme = Theme::current(ctx);
        if self.title_bar_dark != Some(theme.dark) {
            title_bar::apply(frame, theme);
            self.title_bar_dark = Some(theme.dark);
        }
        if self.show_requested.swap(false, Ordering::Relaxed) {
            ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(ViewportCommand::Focus);
        }
        if let Err(err) = &self.store {
            startup_error(ctx, theme, err);
            return;
        }

        self.poll();
        if self.assistant.poll(ctx, self.prefs.assistant_options()) {
            // A tool just ran: show its effect now instead of waiting for the next poll.
            self.sync();
        }
        let repaint = if self.assistant.is_active() {
            Duration::from_millis(50)
        } else {
            Duration::from_secs(1)
        };
        ctx.request_repaint_after(repaint);
        if self
            .toast
            .as_ref()
            .is_some_and(|t| t.shown_at.elapsed() > TOAST_DURATION)
        {
            self.toast = None;
        }

        let mut actions = Actions::default();
        let save = self.shortcuts(ctx, &mut actions);
        let mut escape = !self.overlay_open() && ctx.input(|i| i.key_pressed(Key::Escape));
        if escape && self.assistant.is_recording() {
            self.assistant.cancel_dictation();
            escape = false;
        }

        let bar = StatusBar {
            summary: agenda::summary(&self.tasks, self.today),
            date: self.today.format("%a, %b %-d").to_string(),
            sidebar_open: self.prefs.sidebar_open,
            assistant_open: self.prefs.assistant_open,
            listening: self.assistant.is_recording(),
            toast: self.toast.as_ref(),
        };
        status_bar::show(ctx, theme, bar, &mut actions);
        if self.prefs.sidebar_open {
            sidebar::show(
                ctx,
                theme,
                &self.tasks,
                &self.companies,
                self.prefs.company,
                &mut actions,
            );
        }
        if self.prefs.assistant_open {
            let options = self.prefs.assistant_options();
            assistant::panel::show(ctx, theme, &mut self.assistant, options, &mut actions);
        }
        let mut panel_events = match &mut self.task_panel {
            Some(panel) => task_panel::show(ctx, theme, panel, &self.companies, self.today),
            None => Vec::new(),
        };
        if save {
            panel_events.push(PanelEvent::Save);
        }
        if escape && self.task_panel.is_some() {
            panel_events.push(PanelEvent::Close);
        }
        tab_bar::show(ctx, theme, self.prefs.view, &mut actions);
        let scope = self.scope_label();
        toolbar::show(ctx, theme, &mut self.prefs, &mut self.search, &scope);

        let visible = self.visible_tasks();
        let margin = Margin {
            left: 12.0,
            right: 12.0,
            top: 4.0,
            bottom: 8.0,
        };
        egui::CentralPanel::default()
            .frame(Frame::none().fill(theme.editor).inner_margin(margin))
            .show(ctx, |ui| {
                let cx = ViewCx {
                    theme,
                    today: self.today,
                    tasks: &visible,
                    companies: &self.companies,
                    prefs: &self.prefs,
                    selected: self.task_panel.as_ref().and_then(|p| p.id),
                    has_any_task: !self.tasks.is_empty(),
                };
                match self.prefs.view {
                    View::Board => views::board::show(ui, &cx, &mut self.board, &mut actions),
                    View::List => views::list::show(ui, &cx, &mut actions),
                    View::Agenda => views::agenda::show(ui, &cx, &mut self.agenda, &mut actions),
                }
            });

        self.overlays(ctx, &mut actions);
        self.handle_panel_events(panel_events);
        for action in actions.take() {
            self.apply(ctx, action);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.prefs);
    }
}

fn startup_error(ctx: &egui::Context, theme: &Theme, error: &str) {
    egui::CentralPanel::default()
        .frame(Frame::none().fill(theme.editor).inner_margin(32.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(Icon::Warning.image(18.0, theme.error));
                ui.label(
                    RichText::new("Falog could not open its database")
                        .font(fonts::semibold(18.0))
                        .color(theme.text),
                );
            });
            ui.add_space(8.0);
            ui.label(RichText::new(error).monospace().color(theme.text_muted));
            ui.add_space(8.0);
            let hint = format!("Set {} to use another location.", falog_core::store::DB_PATH_ENV);
            ui.label(RichText::new(hint).color(theme.text_placeholder));
        });
}
