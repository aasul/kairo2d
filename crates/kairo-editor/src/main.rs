#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use eframe::egui;
use kairo_editor::KairoApp;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let project = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("Kairo {} | Feature UI", env!("CARGO_PKG_VERSION")))
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        renderer: eframe::Renderer::Glow,
        persist_window: true,
        follow_system_theme: false,
        default_theme: eframe::Theme::Dark,
        ..Default::default()
    };
    eframe::run_native(
        "Kairo",
        options,
        Box::new(move |cc| Ok(Box::new(KairoApp::new(cc, project)))),
    )
}
