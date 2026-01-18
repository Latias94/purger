use crate::state::AppSettings;
use crate::tr;
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectSort {
    SizeDesc,
    SizeAsc,
    ModifiedDesc,
    ModifiedAsc,
    NameAsc,
    NameDesc,
    PathAsc,
    PathDesc,
}

impl ProjectSort {
    fn label_key(&self) -> &'static str {
        match self {
            ProjectSort::SizeDesc => "filters.sort.size_desc",
            ProjectSort::SizeAsc => "filters.sort.size_asc",
            ProjectSort::ModifiedDesc => "filters.sort.modified_desc",
            ProjectSort::ModifiedAsc => "filters.sort.modified_asc",
            ProjectSort::NameAsc => "filters.sort.name_asc",
            ProjectSort::NameDesc => "filters.sort.name_desc",
            ProjectSort::PathAsc => "filters.sort.path_asc",
            ProjectSort::PathDesc => "filters.sort.path_desc",
        }
    }
}

/// Left filters panel
pub struct FiltersPanel;

impl FiltersPanel {
    pub fn show(
        ui: &mut egui::Ui,
        settings: &mut AppSettings,
        search_query: &mut String,
        sort: &mut ProjectSort,
        show_selected_only: &mut bool,
        show_workspace_only: &mut bool,
    ) {
        ui.strong(tr!("filters.title"));
        ui.separator();

        ui.horizontal(|ui| {
            ui.label(tr!("filters.search_label"));
            ui.add(
                egui::TextEdit::singleline(search_query)
                    .hint_text(tr!("filters.search_placeholder"))
                    .desired_width(f32::INFINITY),
            );
            if ui.button(tr!("filters.clear_search")).clicked() {
                search_query.clear();
            }
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(tr!("filters.sort_label"));
            egui::ComboBox::from_id_salt("project_sort")
                .selected_text(tr!(sort.label_key()))
                .show_ui(ui, |ui| {
                    for option in [
                        ProjectSort::SizeDesc,
                        ProjectSort::SizeAsc,
                        ProjectSort::ModifiedDesc,
                        ProjectSort::ModifiedAsc,
                        ProjectSort::NameAsc,
                        ProjectSort::NameDesc,
                        ProjectSort::PathAsc,
                        ProjectSort::PathDesc,
                    ] {
                        ui.selectable_value(sort, option, tr!(option.label_key()));
                    }
                });
        });

        ui.add_space(6.0);
        ui.checkbox(show_selected_only, tr!("filters.selected_only"));
        ui.checkbox(&mut settings.target_only, tr!("filters.target_only"));
        ui.checkbox(show_workspace_only, tr!("filters.workspace_only"));

        ui.add_space(6.0);
        ui.collapsing(tr!("filters.advanced"), |ui| {
            ui.label(tr!("filters.advanced_hint"));
            ui.add_space(4.0);

            // keep_days
            ui.horizontal(|ui| {
                let mut enabled = settings.keep_days.is_some();
                if ui
                    .checkbox(&mut enabled, tr!("filters.keep_days_label"))
                    .changed()
                {
                    settings.keep_days = if enabled { Some(7) } else { None };
                }
                if enabled {
                    let mut value = settings.keep_days.unwrap_or(7);
                    ui.add(egui::DragValue::new(&mut value).range(1..=3650));
                    settings.keep_days = Some(value);
                } else {
                    ui.colored_label(egui::Color32::GRAY, tr!("filters.keep_days_hint"));
                }
            });

            // keep_size_mb
            ui.horizontal(|ui| {
                let mut enabled = settings.keep_size_mb.is_some();
                if ui
                    .checkbox(&mut enabled, tr!("filters.keep_size_label"))
                    .changed()
                {
                    settings.keep_size_mb = if enabled { Some(100.0) } else { None };
                }
                if enabled {
                    let mut value = settings.keep_size_mb.unwrap_or(100.0);
                    ui.add(egui::DragValue::new(&mut value).range(0.0..=1_000_000.0));
                    settings.keep_size_mb = Some(value);
                } else {
                    ui.colored_label(egui::Color32::GRAY, tr!("filters.keep_size_hint"));
                }
            });

            // keep_executable
            ui.checkbox(
                &mut settings.keep_executable,
                tr!("filters.keep_executable"),
            );
            if settings.keep_executable {
                ui.horizontal(|ui| {
                    ui.label(tr!("filters.backup_dir"));
                    let mut backup_dir = settings.executable_backup_dir.clone().unwrap_or_default();
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut backup_dir)
                            .desired_width(f32::INFINITY)
                            .hint_text(tr!("filters.backup_dir_hint")),
                    );
                    if response.changed() {
                        settings.executable_backup_dir = if backup_dir.trim().is_empty() {
                            None
                        } else {
                            Some(backup_dir)
                        };
                    }
                });
            }

            ui.add_space(8.0);
            ui.label(tr!("filters.ignore_paths"));
            ui.horizontal(|ui| {
                if ui.button(tr!("filters.ignore_add")).clicked() {
                    settings.ignore_paths.push(String::new());
                }
            });

            let mut to_remove = None;
            for (i, ignore_path) in settings.ignore_paths.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(ignore_path).desired_width(f32::INFINITY));
                    if ui.button(tr!("filters.ignore_remove")).clicked() {
                        to_remove = Some(i);
                    }
                });
            }
            if let Some(index) = to_remove {
                settings.ignore_paths.remove(index);
            }
        });
    }
}
