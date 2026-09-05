#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::NativeOptions;
use fastxcel::app::UltraFastApp;

fn app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .unwrap_or_else(|error| {
            log::warn!("Failed to load app icon: {error}");
            egui::IconData::default()
        })
}

fn main() -> eframe::Result<()> {
    env_logger::init();

    if std::env::args().any(|arg| arg == "--smoke-test") {
        let _ = (
            fastxcel::data::DataEngine::new(),
            fastxcel::ui::state::AppState::new(),
        );
        return Ok(());
    }

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("fastxcel")
            .with_icon(app_icon())
            .with_inner_size([1440.0, 960.0]),
        ..NativeOptions::default()
    };
    eframe::run_native(
        "FastXcel",
        options,
        Box::new(|cc| Ok(Box::new(UltraFastApp::new(cc)))),
    )
}
