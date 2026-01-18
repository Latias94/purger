use crate::state::{AppData, AppState};
use crate::tr;
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::time::{Duration, SystemTime};

/// Project list (table view)
pub struct ProjectList;

impl ProjectList {
    pub fn show(ui: &mut egui::Ui, data: &mut AppData, state: &AppState, visible: &[usize]) {
        if data.projects.is_empty() {
            ui.label(tr!("projects.empty_message"));
            return;
        }

        ui.horizontal(|ui| {
            ui.label(tr!("projects.found_message", count = data.projects.len()));
            if visible.len() != data.projects.len() {
                ui.separator();
                ui.label(tr!(
                    "projects.showing_message",
                    visible = visible.len(),
                    total = data.projects.len()
                ));
            }
        });
        ui.separator();

        if visible.is_empty() {
            if data.size_progress.is_some() {
                ui.label(tr!("projects.waiting_sizes"));
            } else {
                ui.label(tr!("projects.no_match"));
            }
            return;
        }

        let selection_enabled = *state == AppState::Idle;

        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::initial(160.0).resizable(true))
            .column(Column::initial(90.0))
            .column(Column::initial(80.0))
            .column(Column::remainder().at_least(240.0))
            .column(Column::initial(140.0))
            .header(22.0, |mut header| {
                header.col(|ui| {
                    ui.strong("");
                });
                header.col(|ui| {
                    ui.strong(tr!("projects.column_name"));
                });
                header.col(|ui| {
                    ui.strong(tr!("projects.column_size"));
                });
                header.col(|ui| {
                    ui.strong(tr!("projects.column_modified"));
                });
                header.col(|ui| {
                    ui.strong(tr!("projects.column_path"));
                });
                header.col(|ui| {
                    ui.strong(tr!("projects.column_tags"));
                });
            })
            .body(|body| {
                body.rows(20.0, visible.len(), |mut row| {
                    let row_index = row.index();
                    let index = visible[row_index];
                    let project = &data.projects[index];

                    let cleanable = project.has_target;

                    row.col(|ui| {
                        let mut selected =
                            cleanable && data.selected_projects.contains(&project.path);
                        let resp = ui.add_enabled(
                            selection_enabled && cleanable,
                            egui::Checkbox::new(&mut selected, ""),
                        );
                        if resp.changed() {
                            if selected {
                                data.selected_projects.insert(project.path.clone());
                            } else {
                                data.selected_projects.remove(&project.path);
                            }
                        }
                    });

                    row.col(|ui| {
                        let focused = data
                            .focused_project
                            .as_ref()
                            .is_some_and(|p| p == &project.path);
                        let resp = ui.selectable_label(focused, &project.name);
                        if resp.clicked() {
                            data.focused_project = Some(project.path.clone());
                        }
                    });

                    row.col(|ui| {
                        if cleanable {
                            if project.target_size == 0 {
                                ui.colored_label(egui::Color32::GRAY, "…");
                            } else {
                                ui.monospace(purger_core::format_bytes(project.target_size));
                            }
                        } else {
                            ui.colored_label(egui::Color32::GRAY, "-");
                        }
                    });

                    row.col(|ui| {
                        ui.monospace(format_compact_relative_time(
                            project.last_modified,
                            cleanable,
                        ));
                    });

                    row.col(|ui| {
                        let path_text = project.path.display().to_string();
                        let resp = ui.add(
                            egui::Label::new(path_text.clone())
                                .truncate()
                                .sense(egui::Sense::click()),
                        );
                        if resp.clicked() {
                            data.focused_project = Some(project.path.clone());
                        }
                        resp.on_hover_text(path_text);
                    });

                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            if project.is_workspace {
                                ui.colored_label(
                                    egui::Color32::BLUE,
                                    tr!("projects.tag_workspace"),
                                );
                            }
                            if !cleanable {
                                ui.colored_label(egui::Color32::GRAY, tr!("projects.no_target"));
                            }
                        });
                    });
                });
            });
    }
}

fn format_compact_relative_time(time: SystemTime, enabled: bool) -> String {
    if !enabled {
        return "-".to_string();
    }

    let Ok(elapsed) = SystemTime::now().duration_since(time) else {
        return "-".to_string();
    };

    if elapsed < Duration::from_secs(60) {
        return tr!("details.time_just_now");
    }
    if elapsed < Duration::from_secs(60 * 60) {
        return tr!("details.time_minutes", n = elapsed.as_secs() / 60);
    }
    if elapsed < Duration::from_secs(24 * 60 * 60) {
        return tr!("details.time_hours", n = elapsed.as_secs() / (60 * 60));
    }

    tr!("details.time_days", n = elapsed.as_secs() / (24 * 60 * 60))
}
