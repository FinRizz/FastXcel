mod app;
mod data;
mod columns;
mod ui;

use eframe::NativeOptions;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = NativeOptions::default();
    eframe::run_native(
        "FastXcel",
        options,
        Box::new(|cc| Ok(Box::new(app::UltraFastApp::new(cc)))),
    )
}