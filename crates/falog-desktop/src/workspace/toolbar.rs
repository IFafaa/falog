use crate::components::{icon_button, icon_toggle};
use crate::icons::Icon;
use crate::prefs::Prefs;
use crate::theme::Theme;
use crate::views::{SortOrder, View};
use eframe::egui::{
    self, Align, ComboBox, Frame, Id, Layout, Margin, RichText, Stroke, TextEdit, TopBottomPanel, vec2,
};

pub const SEARCH_ID: &str = "toolbar-search";

/// Breadcrumb on the left, view options and search on the right.
pub fn show(ctx: &egui::Context, theme: &Theme, prefs: &mut Prefs, search: &mut String, scope: &str) {
    TopBottomPanel::top("toolbar")
        .exact_height(38.0)
        .show_separator_line(false)
        .frame(
            Frame::none()
                .fill(theme.editor)
                .inner_margin(Margin::symmetric(12.0, 0.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(RichText::new(scope).size(13.0).color(theme.text_muted));
                ui.label(RichText::new("›").size(13.0).color(theme.text_placeholder));
                ui.label(RichText::new(prefs.view.label()).size(13.0).color(theme.text));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    search_field(ui, theme, search);
                    match prefs.view {
                        View::Board => {
                            let tip = "Show tasks completed more than a week ago";
                            if icon_toggle(ui, Icon::Check, prefs.show_old_completed, tip).clicked() {
                                prefs.show_old_completed = !prefs.show_old_completed;
                            }
                        }
                        View::List => {
                            let tip = "Show completed tasks";
                            if icon_toggle(ui, Icon::Check, prefs.show_completed, tip).clicked() {
                                prefs.show_completed = !prefs.show_completed;
                            }
                        }
                        View::Focus | View::Calendar | View::Archive => {}
                    }
                    if !matches!(prefs.view, View::Focus | View::Calendar | View::Archive) {
                        ComboBox::from_id_salt("sort")
                            .selected_text(format!("Sort: {}", prefs.sort.label()))
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                for order in SortOrder::ALL {
                                    ui.selectable_value(&mut prefs.sort, order, order.label());
                                }
                            });
                    }
                });
            });
        });
}

fn search_field(ui: &mut egui::Ui, theme: &Theme, search: &mut String) {
    let id = Id::new(SEARCH_ID);
    let focused = ui.memory(|m| m.has_focus(id));
    // Fixed-size box with its own left-to-right layout: the toolbar flows right to left.
    ui.allocate_ui_with_layout(vec2(250.0, 26.0), Layout::left_to_right(Align::Center), |ui| {
        Frame::none()
            .fill(theme.element)
            .stroke(Stroke::new(
                1.0_f32,
                if focused {
                    theme.border_focused
                } else {
                    theme.border_variant
                },
            ))
            .rounding(4.0)
            .inner_margin(Margin::symmetric(6.0, 1.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.add(Icon::Search.image(13.0, theme.icon_muted));
                    ui.add(
                        TextEdit::singleline(search)
                            .id(id)
                            .frame(false)
                            .hint_text("Search tasks…  Ctrl+F")
                            .desired_width(190.0),
                    );
                    if !search.is_empty() && icon_button(ui, Icon::Close, "Clear search").clicked() {
                        search.clear();
                    }
                });
            });
    });
}
