#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod model;

use app::MarkupApp;
use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let image_path = std::env::args_os().nth(1).map(PathBuf::from);
    let app_icon = image::load_from_memory(include_bytes!("../assets/app-icon.png"))
        .expect("embedded app icon is a valid PNG")
        .to_rgba8();
    let icon = egui::IconData {
        rgba: app_icon.as_raw().clone(),
        width: app_icon.width(),
        height: app_icon.height(),
    };
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 720.0])
            .with_min_inner_size([560.0, 360.0])
            .with_icon(icon)
            .with_title("Screenshot Markup"),
        ..Default::default()
    };

    eframe::run_native(
        "Screenshot Markup",
        native_options,
        Box::new(move |cc| Ok(Box::new(MarkupApp::new(cc, image_path)))),
    )
}
