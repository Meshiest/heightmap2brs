#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::NativeOptions;
use heightmap::gui::{HeightmapApp, logger};

// run the window with glium
fn main() -> Result<(), eframe::Error> {
    logger::init().unwrap();

    eframe::run_native(
        "heightmap2brs",
        NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_always_on_top()
                .with_decorations(true)
                .with_drag_and_drop(true)
                .with_inner_size([600.0, 600.0])
                .with_resizable(true),
            ..Default::default()
        },
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<HeightmapApp>::default())
        }),
    )
}
