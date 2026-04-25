mod api;
mod app;
mod dao;
mod model;

use app::NotesApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 600.0])
            .with_title("Gestionnaire de Notes"),
        ..Default::default()
    };

    eframe::run_native(
        "Gestionnaire de Notes",
        options,
        Box::new(|_cc| Ok(Box::new(NotesApp::nouveau()))),
    )
}
