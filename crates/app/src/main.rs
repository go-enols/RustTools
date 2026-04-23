use eframe::egui;

mod app;
mod models;
mod services;
mod theme;
mod ui;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RustTools",
        options,
        Box::new(|cc| Ok(Box::new(app::RustToolsApp::new(cc)))),
    )
}
