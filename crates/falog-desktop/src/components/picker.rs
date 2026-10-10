use crate::i18n::{priority_label, status_label, tr};
use crate::theme::{self, Theme};
use eframe::egui::{ComboBox, RichText, Ui};
use falog_core::domain::{Area, AreaId, Priority, Status};

pub fn area_picker(ui: &mut Ui, id: &str, value: &mut Option<AreaId>, areas: &[Area], width: f32) {
    let selected = value
        .and_then(|id| areas.iter().find(|a| a.id == id))
        .map_or(tr("No area"), |a| a.name.as_str());
    ComboBox::from_id_salt(id)
        .selected_text(selected)
        .width(width)
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, tr("No area"));
            for area in areas {
                let label = RichText::new(format!("● {}", area.name)).color(theme::color(area.color));
                ui.selectable_value(value, Some(area.id), label);
            }
        });
}

pub fn status_picker(ui: &mut Ui, id: &str, value: &mut Status, width: f32) {
    let theme = Theme::current(ui.ctx());
    ComboBox::from_id_salt(id)
        .selected_text(status_label(*value))
        .width(width)
        .show_ui(ui, |ui| {
            for status in Status::ALL {
                let label = RichText::new(status_label(status)).color(theme.status_color(status));
                ui.selectable_value(value, status, label);
            }
        });
}

pub fn priority_picker(ui: &mut Ui, id: &str, value: &mut Priority, width: f32) {
    let theme = Theme::current(ui.ctx());
    let selected = RichText::new(priority_label(*value)).color(theme.priority_color(*value));
    ComboBox::from_id_salt(id)
        .selected_text(selected)
        .width(width)
        .show_ui(ui, |ui| {
            for priority in Priority::ALL {
                let label = RichText::new(priority_label(priority)).color(theme.priority_color(priority));
                ui.selectable_value(value, priority, label);
            }
        });
}
