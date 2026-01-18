use crate::state::AppData;
use crate::tr;
use eframe::egui;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime};

/// Right details panel
pub struct ProjectDetails;

impl ProjectDetails {
    pub fn show(ui: &mut egui::Ui, data: &mut AppData) {
        ui.strong(tr!("details.title"));
        ui.separator();

        let Some(focused) = data.focused_project.clone() else {
            ui.label(tr!("details.empty"));
            return;
        };

        let Some(project) = data.projects.iter().find(|p| p.path == focused).cloned() else {
            ui.label(tr!("details.not_found"));
            return;
        };

        ui.label(&project.name);
        if project.is_workspace {
            ui.colored_label(egui::Color32::BLUE, tr!("projects.tag_workspace"));
        }

        ui.add_space(8.0);
        ui.label(tr!("details.path_label"));
        let path_text = project.path.display().to_string();
        let path_resp = ui
            .add(
                egui::Label::new(path_text.clone())
                    .truncate()
                    .sense(egui::Sense::click()),
            )
            .on_hover_text(path_text.clone());
        if path_resp.clicked() {
            ui.ctx().copy_text(path_text);
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button(tr!("details.copy_path")).clicked() {
                ui.ctx().copy_text(project.path.display().to_string());
            }

            if ui.button(tr!("details.open_project")).clicked() {
                if let Err(e) = open_in_file_manager(&project.path) {
                    data.error_message = Some(format!("{}: {e}", tr!("details.open_failed")));
                }
            }

            if project.has_target && ui.button(tr!("details.open_target")).clicked() {
                let target = project.target_path();
                if let Err(e) = open_in_file_manager(&target) {
                    data.error_message = Some(format!("{}: {e}", tr!("details.open_failed")));
                }
            }
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(tr!("details.size_label"));
            if project.has_target {
                if project.target_size == 0 {
                    ui.colored_label(egui::Color32::GRAY, "…");
                } else {
                    ui.monospace(purger_core::format_bytes(project.target_size));
                }
            } else {
                ui.colored_label(egui::Color32::GRAY, "-");
            }
        });
        ui.horizontal(|ui| {
            ui.label(tr!("details.modified_label"));
            if project.has_target {
                ui.monospace(format_relative_time(project.last_modified));
            } else {
                ui.colored_label(egui::Color32::GRAY, "-");
            }
        });

        ui.add_space(8.0);
        if project.has_target {
            let mut selected = data.is_selected(&project);
            let checkbox = ui.checkbox(&mut selected, tr!("details.selected"));
            if checkbox.changed() {
                data.set_selected(&project, selected);
            }

            if ui.button(tr!("details.select_only")).clicked() {
                data.select_only(&project);
            }
        } else {
            ui.colored_label(egui::Color32::GRAY, tr!("projects.no_target"));
        }
    }
}

fn open_in_file_manager(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer").arg(path).spawn()?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(path).spawn()?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(path).spawn()?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    {
        let _ = path;
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "unsupported platform",
        ));
    }
}

fn format_relative_time(time: SystemTime) -> String {
    let Ok(elapsed) = SystemTime::now().duration_since(time) else {
        return tr!("details.time_unknown");
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
