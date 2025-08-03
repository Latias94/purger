use crate::state::{AppSettings, AppState};
use crate::tr;
use eframe::egui;

/// 扫描配置面板组件
pub struct ScanPanel;

impl ScanPanel {
    /// 渲染扫描配置面板
    pub fn show(
        ui: &mut egui::Ui,
        scan_path: &mut String,
        max_depth: &mut String,
        settings: &mut AppSettings,
        state: &AppState,
        on_select_folder: &mut bool,
        on_start_scan: &mut bool,
    ) {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(tr!("scan.path_label"));
                    ui.add(egui::TextEdit::singleline(scan_path).desired_width(400.0));

                    if ui.button("📁").clicked() {
                        *on_select_folder = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(tr!("scan.max_depth_label"));
                    ui.add(egui::TextEdit::singleline(max_depth).desired_width(100.0));

                    ui.separator();

                    ui.label(tr!("scan.strategy_label"));
                    egui::ComboBox::from_id_salt("clean_strategy")
                        .selected_text(match settings.clean_strategy {
                            purger_core::CleanStrategy::CargoClean => tr!("strategy.cargo_clean"),
                            purger_core::CleanStrategy::DirectDelete => {
                                tr!("strategy.direct_delete")
                            }
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
                });

                // 过滤选项
                ui.collapsing("过滤选项", |ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut settings.target_only, "只显示有target目录的项目");
                    });

                    ui.horizontal(|ui| {
                        ui.label("保留最近编译的项目 (天数):");
                        let mut keep_days_str =
                            settings.keep_days.map_or(String::new(), |d| d.to_string());
                        if ui
                            .add(egui::TextEdit::singleline(&mut keep_days_str).desired_width(80.0))
                            .changed()
                        {
                            settings.keep_days = keep_days_str.parse().ok();
                        }
                        ui.label("(留空表示不过滤)");
                    });

                    ui.horizontal(|ui| {
                        ui.label("保留小项目 (MB):");
                        let mut keep_size_str = settings
                            .keep_size_mb
                            .map_or(String::new(), |s| s.to_string());
                        if ui
                            .add(egui::TextEdit::singleline(&mut keep_size_str).desired_width(80.0))
                            .changed()
                        {
                            settings.keep_size_mb = keep_size_str.parse().ok();
                        }
                        ui.label("(留空表示不过滤)");
                    });

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut settings.keep_executable, "保留可执行文件");
                        if settings.keep_executable {
                            ui.label("备份目录:");
                            let mut backup_dir =
                                settings.executable_backup_dir.clone().unwrap_or_default();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut backup_dir)
                                        .desired_width(200.0),
                                )
                                .changed()
                            {
                                settings.executable_backup_dir = if backup_dir.is_empty() {
                                    None
                                } else {
                                    Some(backup_dir)
                                };
                            }
                        }
                    });

                    // 忽略路径管理
                    ui.horizontal(|ui| {
                        ui.label("忽略路径:");
                        if ui.button("添加").clicked() {
                            settings.ignore_paths.push(String::new());
                        }
                    });

                    let mut to_remove = None;
                    for (i, ignore_path) in settings.ignore_paths.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(ignore_path).desired_width(300.0));
                            if ui.button("删除").clicked() {
                                to_remove = Some(i);
                            }
                        });
                    }
                    if let Some(index) = to_remove {
                        settings.ignore_paths.remove(index);
                    }
                });

                // 最近使用的路径
                if !settings.recent_paths.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(tr!("scan.recent_paths_label"));
                        egui::ComboBox::from_id_salt("recent_paths")
                            .selected_text(tr!("scan.recent_paths_placeholder"))
                            .show_ui(ui, |ui| {
                                for path in &settings.recent_paths {
                                    if ui.selectable_label(false, path).clicked() {
                                        *scan_path = path.clone();
                                    }
                                }
                            });
                    });
                }

                ui.horizontal(|ui| {
                    let can_scan = *state == AppState::Idle && !scan_path.trim().is_empty();
                    if ui
                        .add_enabled(can_scan, egui::Button::new(tr!("scan.start_button")))
                        .clicked()
                    {
                        *on_start_scan = true;
                    }

                    if *state == AppState::Scanning {
                        ui.label(tr!("scan.scanning_status"));
                    }
                });
            });
        });
    }
}
