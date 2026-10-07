use crate::theme::{self, Theme};
use eframe::egui::{ComboBox, RichText, Ui};
use falog_core::domain::{Area, AreaId, Priority, Status, UNASSIGNED};

pub fn company_picker(ui: &mut Ui, id: &str, value: &mut Option<AreaId>, companies: &[Area], width: f32) {
    let selected = value
        .and_then(|id| companies.iter().find(|c| c.id == id))
        .map_or(UNASSIGNED, |c| c.name.as_str());
    ComboBox::from_id_salt(id)
        .selected_text(selected)
        .width(width)
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, UNASSIGNED);
            for company in companies {
                let label = RichText::new(format!("● {}", company.name)).color(theme::color(company.color));
                ui.selectable_value(value, Some(company.id), label);
            }
        });
}

pub fn status_picker(ui: &mut Ui, id: &str, value: &mut Status, width: f32) {
    let theme = Theme::current(ui.ctx());
    ComboBox::from_id_salt(id)
        .selected_text(value.label())
        .width(width)
        .show_ui(ui, |ui| {
            for status in Status::ALL {
                let label = RichText::new(status.label()).color(theme.status_color(status));
                ui.selectable_value(value, status, label);
            }
        });
}

pub fn priority_picker(ui: &mut Ui, id: &str, value: &mut Priority, width: f32) {
    let theme = Theme::current(ui.ctx());
    let selected = RichText::new(value.label()).color(theme.priority_color(*value));
    ComboBox::from_id_salt(id)
        .selected_text(selected)
        .width(width)
        .show_ui(ui, |ui| {
            for priority in Priority::ALL {
                let label = RichText::new(priority.label()).color(theme.priority_color(priority));
                ui.selectable_value(value, priority, label);
            }
        });
}
