use crate::state::{AppData, AppState};
use crate::tr;
use eframe::egui;

/// 项目列表组件
pub struct ProjectList;

impl ProjectList {
    /// 显示项目列表
    pub fn show(
        ui: &mut egui::Ui,
        data: &mut AppData,
        state: &AppState,
        on_start_clean: &mut bool,
    ) {
        if data.projects.is_empty() {
            ui.label(tr!("projects.empty_message"));
            return;
        }

        // 统计信息
        let selected_count = data.get_selected_count();
        let total_cleanable_size = data.get_total_cleanable_size();

        // 项目列表
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label(tr!("projects.found_message", count = data.projects.len()));

                // 滚动区域
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for (i, project) in data.projects.iter().enumerate() {
                            ui.horizontal(|ui| {
                                // 复选框
                                let mut selected =
                                    data.selected_projects.get(i).copied().unwrap_or(false);
                                if ui.checkbox(&mut selected, "").changed() {
                                    if let Some(sel) = data.selected_projects.get_mut(i) {
                                        *sel = selected;
                                    }
                                }

                                // 项目信息
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(&project.name);
                                        if project.is_workspace {
                                            ui.colored_label(egui::Color32::BLUE, "workspace");
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(format!("路径: {}", project.path.display()));
                                    });

                                    if project.has_target {
                                        ui.horizontal(|ui| {
                                            ui.label(format!(
                                                "Target大小: {}",
                                                project.formatted_size()
                                            ));
                                            ui.colored_label(egui::Color32::GREEN, "可清理");
                                        });
                                    } else {
                                        ui.colored_label(egui::Color32::GRAY, "无target目录");
                                    }
                                });
                            });
                            ui.separator();
                        }
                    });

                // 统计信息和操作按钮
                ui.horizontal(|ui| {
                    if selected_count > 0 {
                        ui.label(tr!("projects.selected_message", count = selected_count));
                    }

                    if total_cleanable_size > 0 {
                        ui.label(tr!(
                            "projects.cleanable_size",
                            size = purger_core::format_bytes(total_cleanable_size)
                        ));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let can_clean = *state == AppState::Idle && selected_count > 0;
                        if ui
                            .add_enabled(can_clean, egui::Button::new(tr!("projects.clean_button")))
                            .clicked()
                        {
                            *on_start_clean = true;
                        }
                    });
                });

                // 选择操作按钮
                ui.horizontal(|ui| {
                    if ui.button(tr!("projects.select_all")).clicked() {
                        data.select_all();
                    }
                    if ui.button(tr!("projects.select_none")).clicked() {
                        data.select_none();
                    }
                    if ui.button(tr!("projects.invert_selection")).clicked() {
                        data.invert_selection();
                    }
                });
            });
        });
    }
}
