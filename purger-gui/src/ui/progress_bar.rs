use crate::state::{AppData, AppState};
use crate::tr;
use eframe::egui;

/// Progress display
pub struct ProgressBar;

impl ProgressBar {
    /// Show scan progress
    pub fn show_scan_progress(ui: &mut egui::Ui, data: &AppData) {
        if let Some((current, total)) = data.scan_progress {
            ui.horizontal(|ui| {
                ui.label(tr!("progress.scan_label"));
                let progress = if total > 0 {
                    current as f32 / total as f32
                } else {
                    0.0
                };
                ui.add(egui::ProgressBar::new(progress).text(format!("{current}/{total}")));
            });
        }
    }

    /// Show clean progress
    pub fn show_clean_progress(ui: &mut egui::Ui, data: &AppData) {
        if let Some((current, total, size_freed)) = data.clean_progress {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(tr!("progress.clean_label"));
                    let progress = if total > 0 {
                        current as f32 / total as f32
                    } else {
                        0.0
                    };
                    ui.add(egui::ProgressBar::new(progress).text(format!("{current}/{total}")));
                });

                if let Some(ref project_name) = data.current_cleaning_project {
                    ui.horizontal(|ui| {
                        ui.label(tr!("progress.current_project"));
                        ui.label(project_name);
                    });
                }

                if size_freed > 0 {
                    ui.horizontal(|ui| {
                        ui.label(tr!("progress.freed_size"));
                        ui.label(purger_core::format_bytes(size_freed));
                    });
                }

                if !data.clean_errors.is_empty() {
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 80, 80),
                        tr!("progress.failed_so_far", count = data.clean_errors.len()),
                    );
                }
            });
        }
    }

    /// Show all progress information
    pub fn show_all_progress(ui: &mut egui::Ui, state: &AppState, data: &AppData) {
        match state {
            AppState::Scanning => {
                Self::show_scan_progress(ui, data);
            }
            AppState::Cleaning => {
                Self::show_clean_progress(ui, data);
            }
            AppState::Idle => {
                // 显示最后的清理结果
                if let Some(ref result) = data.last_clean_result {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.label(tr!("progress.last_result"));
                            ui.horizontal(|ui| {
                                ui.label(tr!(
                                    "progress.cleaned_projects",
                                    count = result.cleaned_projects
                                ));
                                ui.label(tr!("progress.freed_space", size = result.format_size()));
                                ui.label(tr!("progress.duration", ms = result.duration_ms));
                            });

                            if !result.failed_projects.is_empty() {
                                ui.label(tr!(
                                    "progress.failed_projects",
                                    count = result.failed_projects.len()
                                ));
                            }

                            if !data.clean_errors.is_empty() {
                                ui.collapsing(
                                    tr!("progress.failed_details", count = data.clean_errors.len()),
                                    |ui| {
                                        ui.horizontal(|ui| {
                                            if ui.button(tr!("progress.copy_failed")).clicked() {
                                                let text = data
                                                    .clean_errors
                                                    .iter()
                                                    .map(|(name, error)| format!("{name}: {error}"))
                                                    .collect::<Vec<_>>()
                                                    .join("\n");
                                                ui.ctx().copy_text(text);
                                            }
                                        });

                                        egui::ScrollArea::vertical()
                                            .max_height(180.0)
                                            .auto_shrink([false; 2])
                                            .show(ui, |ui| {
                                                for (name, error) in &data.clean_errors {
                                                    ui.horizontal_wrapped(|ui| {
                                                        ui.colored_label(
                                                            egui::Color32::from_rgb(220, 80, 80),
                                                            name,
                                                        );
                                                        ui.label(error);
                                                    });
                                                }
                                            });
                                    },
                                );
                            }
                        });
                    });
                }
            }
        }

        // 显示错误信息
        if let Some(ref error) = data.error_message {
            ui.colored_label(egui::Color32::RED, format!("错误: {error}"));
        }
    }
}
