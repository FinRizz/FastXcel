use eframe::NativeOptions;
use fastxcel::app::UltraFastApp;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = NativeOptions::default();
    eframe::run_native(
        "FastXcel",
        options,
        Box::new(|cc| Ok(Box::new(UltraFastApp::new(cc)))),
    )
}
