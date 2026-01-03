use crate::state::{AppData, AppState};
use crate::tr;
use eframe::egui;
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

        let selection_enabled = *state == AppState::Idle;

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                egui::Grid::new("projects_table")
                    .striped(true)
                    .min_col_width(12.0)
                    .spacing([6.0, 2.0])
                    .show(ui, |ui| {
                        ui.strong("");
                        ui.strong(tr!("projects.column_name"));
                        ui.strong(tr!("projects.column_size"));
                        ui.strong(tr!("projects.column_modified"));
                        ui.strong(tr!("projects.column_path"));
                        ui.strong(tr!("projects.column_tags"));
                        ui.end_row();

                        for &index in visible {
                            let (name, path, target_size, last_modified, is_workspace, cleanable) = {
                                let project = &data.projects[index];
                                (
                                    project.name.clone(),
                                    project.path.clone(),
                                    project.target_size,
                                    project.last_modified,
                                    project.is_workspace,
                                    project.has_target,
                                )
                            };

                            let mut selected =
                                cleanable && data.selected_projects.contains(&path);
                            let checkbox = ui.add_enabled(
                                selection_enabled && cleanable,
                                egui::Checkbox::new(&mut selected, ""),
                            );
                            if checkbox.changed() {
                                if selected {
                                    data.selected_projects.insert(path.clone());
                                } else {
                                    data.selected_projects.remove(&path);
                                }
                            }

                            let focused = data
                                .focused_project
                                .as_ref()
                                .is_some_and(|p| p == &path);
                            let name_resp = ui.selectable_label(focused, &name);
                            if name_resp.clicked() {
                                data.focused_project = Some(path.clone());
                            }

                            if cleanable {
                                ui.monospace(purger_core::format_bytes(target_size));
                            } else {
                                ui.colored_label(egui::Color32::GRAY, "-");
                            }

                            ui.monospace(format_compact_relative_time(last_modified, cleanable));

                            let path_text = path.display().to_string();
                            let path_resp = ui.add(
                                egui::Label::new(path_text.clone())
                                    .truncate()
                                    .sense(egui::Sense::click()),
                            );
                            if path_resp.clicked() {
                                data.focused_project = Some(path.clone());
                            }
                            path_resp.on_hover_text(path_text);

                            ui.horizontal(|ui| {
                                if is_workspace {
                                    ui.colored_label(
                                        egui::Color32::BLUE,
                                        tr!("projects.tag_workspace"),
                                    );
                                }
                                if !cleanable {
                                    ui.colored_label(
                                        egui::Color32::GRAY,
                                        tr!("projects.no_target"),
                                    );
                                }
                            });

                            ui.end_row();
                        }
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
