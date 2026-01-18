use crate::simple_i18n::{Language, set_language};
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
        draft: &mut Option<AppSettings>,
    ) {
        if !*show_settings {
            return;
        }

        if draft.is_none() {
            *draft = Some(settings.clone());
        }

        let mut apply = false;
        let mut cancel = false;

        egui::Window::new(tr!("dialog.settings_title"))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let Some(draft_settings) = draft.as_mut() else {
                    return;
                };

                // 语言设置
                ui.horizontal(|ui| {
                    ui.label(tr!("language.label"));
                    let current_lang = draft_settings.language;
                    egui::ComboBox::from_id_salt("language_selector")
                        .selected_text(current_lang.display_name())
                        .show_ui(ui, |ui| {
                            for lang in Language::all() {
                                ui.selectable_value(
                                    &mut draft_settings.language,
                                    lang,
                                    lang.display_name(),
                                );
                            }
                        });
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(tr!("dialog.max_recent_paths"));
                    ui.add(
                        egui::DragValue::new(&mut draft_settings.max_recent_paths).range(1..=20),
                    );
                });

                ui.checkbox(
                    &mut draft_settings.auto_save_settings,
                    tr!("dialog.auto_save_settings"),
                );

                ui.horizontal(|ui| {
                    ui.label(tr!("scan.strategy_label"));
                    egui::ComboBox::from_label("")
                        .selected_text(match draft_settings.clean_strategy {
                            purger_core::CleanStrategy::CargoClean => tr!("strategy.cargo_clean"),
                            purger_core::CleanStrategy::DirectDelete => {
                                tr!("strategy.direct_delete")
                            }
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut draft_settings.clean_strategy,
                                purger_core::CleanStrategy::CargoClean,
                                tr!("strategy.cargo_clean"),
                            );
                            ui.selectable_value(
                                &mut draft_settings.clean_strategy,
                                purger_core::CleanStrategy::DirectDelete,
                                tr!("strategy.direct_delete"),
                            );
                        });
                });

                ui.horizontal(|ui| {
                    ui.label(tr!("dialog.clean_timeout"));
                    ui.add(
                        egui::DragValue::new(&mut draft_settings.clean_timeout_seconds)
                            .range(0..=36000)
                            .speed(10),
                    );
                });

                ui.horizontal(|ui| {
                    if ui.button(tr!("dialog.clear_recent_paths")).clicked() {
                        draft_settings.clear_recent_paths();
                    }

                    if ui.button(tr!("dialog.reset_defaults")).clicked() {
                        *draft_settings = AppSettings::default();
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button(tr!("dialog.ok")).clicked() {
                        apply = true;
                    }
                    if ui.button(tr!("dialog.cancel")).clicked() {
                        cancel = true;
                    }
                });
            });

        if apply {
            if let Some(draft_settings) = draft.take() {
                *settings = draft_settings;
            }
            set_language(settings.language);
            *show_settings = false;
        }

        if cancel {
            *draft = None;
            *show_settings = false;
        }
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
