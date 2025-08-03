use crate::tr;
use eframe::egui;

/// 菜单栏组件
pub struct MenuBar;

impl MenuBar {
    /// 渲染菜单栏
    pub fn show(
        ctx: &egui::Context,
        show_settings: &mut bool,
        show_about: &mut bool,
        on_select_folder: &mut bool,
    ) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button(tr!("menu.file"), |ui| {
                    if ui.button(tr!("menu.select_folder")).clicked() {
                        *on_select_folder = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(tr!("menu.exit")).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button(tr!("menu.settings"), |ui| {
                    if ui.button(tr!("menu.preferences")).clicked() {
                        *show_settings = true;
                        ui.close();
                    }
                });

                ui.menu_button(tr!("menu.help"), |ui| {
                    if ui.button(tr!("menu.about")).clicked() {
                        *show_about = true;
                        ui.close();
                    }
                });
            });
        });
    }
}
