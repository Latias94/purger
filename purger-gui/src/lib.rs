use anyhow::Result;
use eframe::egui;

mod app;
mod handlers;
mod simple_i18n;
mod state;
mod ui;

use app::PurgerApp;
use simple_i18n::translate;

pub fn run_gui() -> Result<()> {
    // Logging is initialized by the caller binary or by the GUI-only binary.
    // If it is already initialized, this will return an error, so we use `try_init`.
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        &translate("app.title"),
        options,
        Box::new(|cc| {
            setup_custom_fonts(&cc.egui_ctx);
            let app = PurgerApp::new(cc);
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run GUI: {}", e))
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "noto_sans_sc".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/NotoSansSC-Regular.ttf")).into(),
    );

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "noto_sans_sc".to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "noto_sans_sc".to_owned());

    ctx.set_fonts(fonts);
}
