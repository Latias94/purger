use crate::state::{AppData, AppState};
use crate::tr;
use eframe::egui;

/// Bottom action bar
pub struct ActionBar;

impl ActionBar {
    pub fn show(
        ui: &mut egui::Ui,
        data: &mut AppData,
        state: &AppState,
        on_request_clean: &mut bool,
    ) {
        let selected_count = data.get_selected_count();
        let total_selected_size = data.get_total_cleanable_size();
        let selection_enabled = *state == AppState::Idle;

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if selected_count > 0 {
                    ui.label(tr!("projects.selected_message", count = selected_count));
                } else {
                    ui.label(tr!("actions.no_selection"));
                }

                if total_selected_size > 0 {
                    ui.separator();
                    ui.label(tr!(
                        "projects.cleanable_size",
                        size = purger_core::format_bytes(total_selected_size)
                    ));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_clean = *state == AppState::Idle && selected_count > 0;
                    if ui
                        .add_enabled(can_clean, egui::Button::new(tr!("projects.clean_button")))
                        .clicked()
                    {
                        *on_request_clean = true;
                    }
                });
            });

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        selection_enabled,
                        egui::Button::new(tr!("projects.select_all")),
                    )
                    .clicked()
                {
                    data.select_all();
                }
                if ui
                    .add_enabled(
                        selection_enabled,
                        egui::Button::new(tr!("actions.select_cleanable")),
                    )
                    .clicked()
                {
                    data.select_all_cleanable();
                }
                if ui
                    .add_enabled(
                        selection_enabled,
                        egui::Button::new(tr!("projects.select_none")),
                    )
                    .clicked()
                {
                    data.select_none();
                }
                if ui
                    .add_enabled(
                        selection_enabled,
                        egui::Button::new(tr!("projects.invert_selection")),
                    )
                    .clicked()
                {
                    data.invert_selection();
                }
            });
        });
    }
}
