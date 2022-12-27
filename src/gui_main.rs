#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::NativeOptions;
use gui::{logger, HeightmapApp};

mod gui;

// run the window with glium
fn main() {
    logger::init().unwrap();

    eframe::run_native(
        "heightmap2brs",
        NativeOptions {
            always_on_top: false,
            decorated: true,
            drag_and_drop_support: true,
            icon_data: None,
            initial_window_size: Some(egui::Vec2 { x: 600.0, y: 600.0 }),
            resizable: true,
            ..Default::default()
        },
        Box::new(|_cc| Box::<HeightmapApp>::default()),
    );
}
