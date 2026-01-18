use crate::state::{AppSettings, AppState};
use crate::tr;
use eframe::egui;

/// Top scan toolbar
pub struct ScanPanel;

impl ScanPanel {
    pub fn show(
        ui: &mut egui::Ui,
        scan_path: &mut String,
        settings: &mut AppSettings,
        state: &AppState,
        can_stop_extra: bool,
        on_select_folder: &mut bool,
        on_start_scan: &mut bool,
        on_stop: &mut bool,
    ) {
        ui.horizontal(|ui| {
            ui.label(tr!("scan.path_label"));
            ui.add(
                egui::TextEdit::singleline(scan_path)
                    .desired_width(320.0)
                    .hint_text(tr!("scan.path_hint")),
            );

            if ui.button(tr!("scan.browse_button")).clicked() {
                *on_select_folder = true;
            }

            if !settings.recent_paths.is_empty() {
                egui::ComboBox::from_id_salt("recent_paths")
                    .selected_text(tr!("scan.recent_paths_placeholder"))
                    .show_ui(ui, |ui| {
                        for path in &settings.recent_paths {
                            if ui.selectable_label(false, path).clicked() {
                                *scan_path = path.clone();
                            }
                        }
                    });
            }

            ui.separator();
            ui.label(tr!("scan.max_depth_label"));
            ui.add(egui::DragValue::new(&mut settings.max_depth).range(0..=200));

            ui.separator();
            ui.label(tr!("scan.strategy_label"));
            egui::ComboBox::from_id_salt("clean_strategy_quick")
                .selected_text(match settings.clean_strategy {
                    purger_core::CleanStrategy::CargoClean => tr!("strategy.cargo_clean"),
                    purger_core::CleanStrategy::DirectDelete => tr!("strategy.direct_delete"),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut settings.clean_strategy,
                        purger_core::CleanStrategy::CargoClean,
                        tr!("scan.strategy_cargo_clean"),
                    );
                    ui.selectable_value(
                        &mut settings.clean_strategy,
                        purger_core::CleanStrategy::DirectDelete,
                        tr!("scan.strategy_direct_delete"),
                    );
                });

            ui.separator();
            let can_scan = *state == AppState::Idle && !scan_path.trim().is_empty();
            if ui
                .add_enabled(can_scan, egui::Button::new(tr!("scan.start_button")))
                .clicked()
            {
                *on_start_scan = true;
            }

            let can_stop = *state != AppState::Idle || can_stop_extra;
            if ui
                .add_enabled(can_stop, egui::Button::new(tr!("scan.stop_button")))
                .clicked()
            {
                *on_stop = true;
            }

            if *state == AppState::Scanning {
                ui.label(tr!("scan.scanning_status"));
            } else if can_stop_extra {
                ui.label(tr!("scan.sizing_status"));
            }
        });
    }
}
