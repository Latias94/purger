use crate::simple_i18n::{set_language, Language};
use crate::state::AppSettings;
use crate::tr;
use eframe::egui;

/// 对话框组件
pub struct Dialogs;

impl Dialogs {
    /// 显示设置对话框
    pub fn show_settings(
        ctx: &egui::Context,
        show_settings: &mut bool,
        settings: &mut AppSettings,
    ) {
        if !*show_settings {
            return;
        }

        egui::Window::new(tr!("dialog.settings_title"))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                // 语言设置
                ui.horizontal(|ui| {
                    ui.label(tr!("language.label"));
                    let current_lang = settings.language;
                    egui::ComboBox::from_id_source("language_selector")
                        .selected_text(current_lang.display_name())
                        .show_ui(ui, |ui| {
                            for lang in Language::all() {
                                if ui
                                    .selectable_value(
                                        &mut settings.language,
                                        lang,
                                        lang.display_name(),
                                    )
                                    .clicked()
                                {
                                    set_language(lang);
                                }
                            }
                        });
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(tr!("dialog.max_recent_paths"));
                    ui.add(egui::DragValue::new(&mut settings.max_recent_paths).range(1..=20));
                });

                ui.checkbox(
                    &mut settings.auto_save_settings,
                    tr!("dialog.auto_save_settings"),
                );

                ui.horizontal(|ui| {
                    ui.label(tr!("scan.strategy_label"));
                    egui::ComboBox::from_label("")
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
                                tr!("strategy.cargo_clean"),
                            );
                            ui.selectable_value(
                                &mut settings.clean_strategy,
                                purger_core::CleanStrategy::DirectDelete,
                                tr!("strategy.direct_delete"),
                            );
                        });
                });

                ui.horizontal(|ui| {
                    if ui.button(tr!("dialog.clear_recent_paths")).clicked() {
                        settings.clear_recent_paths();
                    }

                    if ui.button(tr!("dialog.reset_defaults")).clicked() {
                        *settings = AppSettings::default();
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(tr!("dialog.ok")).clicked() {
                        *show_settings = false;
                    }
                    if ui.button(tr!("dialog.cancel")).clicked() {
                        *show_settings = false;
                    }
                });
            });
    }

    /// 显示关于对话框
    pub fn show_about(ctx: &egui::Context, show_about: &mut bool) {
        if !*show_about {
            return;
        }

        egui::Window::new(tr!("dialog.about_title"))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(tr!("app.title"));
                    ui.label(tr!("about.version"));
                    ui.separator();
                    ui.label(tr!("about.description1"));
                    ui.label(tr!("about.description2"));
                    ui.separator();
                    ui.label(tr!("about.footer"));

                    if ui.button(tr!("dialog.ok")).clicked() {
                        *show_about = false;
                    }
                });
            });
    }
}
