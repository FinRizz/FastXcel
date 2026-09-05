use eframe::NativeOptions;
use fastxcel::app::UltraFastApp;

fn main() -> eframe::Result<()> {
    env_logger::init();

    if std::env::args().any(|arg| arg == "--smoke-test") {
        let _ = (
            fastxcel::data::DataEngine::new(),
            fastxcel::ui::state::AppState::new(),
        );
        return Ok(());
    }

    let options = NativeOptions::default();
    eframe::run_native(
        "FastXcel",
        options,
        Box::new(|cc| Ok(Box::new(UltraFastApp::new(cc)))),
    )
}
