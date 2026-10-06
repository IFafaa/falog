use crate::components::{ButtonStyle, Placement, button, icon_button, modal, modal_header};
use crate::icons::Icon;
use crate::theme::Theme;
use eframe::egui::{self, Align, Frame, Key, Layout, Margin, RichText, TextEdit, Ui};
use falog_core::domain::{Company, CompanyId, PALETTE, Rgb};
use std::collections::HashMap;

#[derive(Debug)]
pub enum CompanyEvent {
    Create { name: String, color: Rgb },
    Update { id: CompanyId, name: String, color: Rgb },
    Delete { id: CompanyId, name: String },
}

#[derive(Debug)]
pub struct CompaniesDialog {
    edits: HashMap<CompanyId, (String, [u8; 3])>,
    new_name: String,
    new_color: [u8; 3],
}

impl CompaniesDialog {
    pub fn new(existing: usize) -> Self {
        Self {
            edits: HashMap::new(),
            new_name: String::new(),
            new_color: PALETTE[existing % PALETTE.len()].0,
        }
    }

    /// Call after a successful create so the next company gets a fresh color.
    pub fn reset_new(&mut self, existing: usize) {
        self.new_name.clear();
        self.new_color = PALETTE[existing % PALETTE.len()].0;
    }

    pub fn forget(&mut self, id: CompanyId) {
        self.edits.remove(&id);
    }

    /// Returns the requested changes and whether the dialog should close.
    pub fn show(&mut self, ctx: &egui::Context, companies: &[Company]) -> (Vec<CompanyEvent>, bool) {
        let theme = Theme::current(ctx);
        let mut events = Vec::new();
        let (closed, dismissed) = modal(ctx, "companies", 460.0, Placement::Center, |ui| {
            let closed = modal_header(ui, "Companies");
            Frame::none()
                .inner_margin(Margin::symmetric(16.0, 12.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("The companies you work for. The color marks their tasks everywhere.")
                            .size(13.0)
                            .color(theme.text_muted),
                    );
                    ui.add_space(10.0);
                    for company in companies {
                        self.company_row(ui, company, &mut events);
                    }
                    if !companies.is_empty() {
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                    }
                    self.new_row(ui, &mut events);
                });
            closed
        });
        (events, closed || dismissed)
    }

    fn company_row(&mut self, ui: &mut Ui, company: &Company, events: &mut Vec<CompanyEvent>) {
        let (name, color) = self
            .edits
            .entry(company.id)
            .or_insert_with(|| (company.name.clone(), company.color.0));
        ui.horizontal(|ui| {
            ui.color_edit_button_srgb(color);
            ui.add(TextEdit::singleline(name).desired_width(220.0));
            let changed = name.trim() != company.name || Rgb(*color) != company.color;
            let save = ui
                .add_enabled_ui(changed, |ui| button(ui, ButtonStyle::Filled, None, "Save"))
                .inner;
            if save.clicked() {
                events.push(CompanyEvent::Update {
                    id: company.id,
                    name: name.clone(),
                    color: Rgb(*color),
                });
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, Icon::Trash, "Delete company").clicked() {
                    events.push(CompanyEvent::Delete {
                        id: company.id,
                        name: company.name.clone(),
                    });
                }
            });
        });
    }

    fn new_row(&mut self, ui: &mut Ui, events: &mut Vec<CompanyEvent>) {
        ui.horizontal(|ui| {
            ui.color_edit_button_srgb(&mut self.new_color);
            let response = ui.add(
                TextEdit::singleline(&mut self.new_name)
                    .hint_text("New company")
                    .desired_width(220.0),
            );
            let enter = response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            let add = button(ui, ButtonStyle::Accent, Some(Icon::Plus), "Add").clicked();
            if (add || enter) && !self.new_name.trim().is_empty() {
                events.push(CompanyEvent::Create {
                    name: self.new_name.clone(),
                    color: Rgb(self.new_color),
                });
            }
        });
    }
}
