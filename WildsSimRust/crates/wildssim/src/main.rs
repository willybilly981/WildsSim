#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod data;

use eframe::egui;

// The release build has no console, so an error printed to stderr is never seen.
fn show_error(message: String) -> ! {
    rfd::MessageDialog::new()
        .set_title("エラー")
        .set_description(message)
        .show();
    std::process::exit(1);
}

fn main() {
    let data_root = data::resolve_data_root();
    let simulator = match simmodel::Simulator::load(&data_root) {
        Ok(simulator) => simulator,
        Err(error) => show_error(format!(
            "{}\n\nデータフォルダ: {}",
            error,
            data_root.display()
        )),
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 860.0])
            .with_title("MHWildsSkillSimurator"),
        ..Default::default()
    };
    if let Err(error) = eframe::run_native(
        "MHWildsSkillSimurator",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, simulator, data_root)))),
    ) {
        show_error(format!("画面を起動できませんでした。: {}", error));
    }
}
