//! `falchion-gui` binary entry point.

// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use falchion_gui::app::FalchionApp;
use falchion_hid::HidProvider;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([420.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ROG Falchion Control",
        options,
        Box::new(|cc| {
            let app = FalchionApp::new(cc.egui_ctx.clone(), || HidProvider);
            Ok(Box::new(app))
        }),
    )
}
