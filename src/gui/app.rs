#![allow(dead_code, unused_variables)]
use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use super::{logger, util::load_image_from_path};
use crate::gui::util::maps_from_files;
use brickadia::write::SaveWriter;
use eframe::App;
use egui::{
    vec2, Button, CentralPanel, Color32, Context, Id, ProgressBar, ScrollArea, TextureHandle,
    TopBottomPanel, Ui,
};
use log::{error, info};
use poll_promise::Promise;
use {
    heightmap::{quad::*, util::*},
    std::{fs::File, path::Path},
};

#[derive(PartialEq, Clone)]
enum BrickMode {
    Default,
    Tile,
    Stud,
    Micro,
}

type Progress = (&'static str, f32);

pub struct HeightmapApp {
    // options for the generator
    heightmaps: Vec<String>,
    colormap: Option<String>,
    owner_name: String,
    owner_id: String,
    out_file: String,
    vertical_scale: u32,
    horizontal_size: u32,
    opt_cull: bool,
    opt_nocollide: bool,
    opt_lrgb: bool,
    opt_hdmap: bool,
    opt_snap: bool,
    opt_glow: bool,
    mode: BrickMode,
    progress: Progress,
    progress_channel: (Sender<Progress>, Receiver<Progress>),
    promise: Option<Promise<Result<(), String>>>,
    texture_handles: HashMap<String, TextureHandle>,
}

impl Default for HeightmapApp {
    fn default() -> Self {
        Self {
            // default generator options
            heightmaps: vec![],
            colormap: None,
            owner_name: "Generator".to_string(),
            owner_id: "a1b16aca-9627-4a16-a160-67fa9adbb7b6".to_string(),
            out_file: "out.brs".to_string(),
            vertical_scale: 1,
            horizontal_size: 1,
            opt_cull: false,
            opt_nocollide: false,
            opt_lrgb: false,
            opt_snap: false,
            opt_glow: false,
            opt_hdmap: false,
            mode: BrickMode::Default,
            promise: None,
            progress: ("Pending", 0.),
            progress_channel: mpsc::channel(),
            texture_handles: HashMap::new(),
        }
    }
}

impl HeightmapApp {
    fn options(&self) -> GenOptions {
        // output options
        let mut options = GenOptions {
            size: self.horizontal_size * 5,
            scale: self.vertical_scale,
            cull: self.opt_cull,
            asset: 0,
            tile: self.mode == BrickMode::Tile,
            micro: self.mode == BrickMode::Micro,
            stud: self.mode == BrickMode::Stud,
            snap: self.opt_snap,
            img: self.heightmaps.is_empty() && self.colormap.is_some(),
            glow: self.opt_glow,
            hdmap: self.opt_hdmap,
            lrgb: self.opt_lrgb,
            nocollide: self.opt_nocollide,
        };

        if options.tile {
            options.asset = 1
        } else if options.micro {
            options.size /= 5;
            options.asset = 2;
        }
        if options.stud {
            options.asset = 3
        }

        options
    }

    fn run_converter(&mut self) {
        let out_file = self.out_file.clone();
        let owner_id = self.owner_id.clone();
        let owner_name = self.owner_name.clone();
        let options = self.options();
        let heightmap_files = self.heightmaps.clone();
        let colormap_file = self.colormap.clone();
        let progress = self.progress_channel.0.clone();

        self.promise.get_or_insert_with(|| {
            info!("Preparing converter...");
            let (sender, promise) = Promise::new();
            progress.send(("Reading", 0.)).unwrap();

            thread::spawn(move || {
                info!("Reading image files...");
                let (heightmap, colormap) =
                    match maps_from_files(&options, heightmap_files, colormap_file) {
                        Ok(hc) => hc,
                        Err(err) => {
                            error!("{err}");
                            return sender.send(Err(err));
                        }
                    };

                progress.send(("Generating", 0.10)).unwrap();

                let bricks = match gen_opt_heightmap(&*heightmap, &*colormap, options, |p| {
                    progress.send(("Generating", 0.1 + 0.85 * p)).unwrap();
                }) {
                    Ok(b) => b,
                    Err(err) => {
                        error!("{err}");
                        return sender.send(Err(err));
                    }
                };

                info!("Writing Save to {}", out_file);
                progress.send(("Writing", 0.95)).unwrap();
                let data = bricks_to_save(bricks, owner_id, owner_name);
                if let Err(e) = SaveWriter::new(File::create(&out_file).unwrap(), data).write() {
                    let err = format!("failed to write file: {e}");
                    error!("{err}");
                    return sender.send(Err(err));
                }
                progress.send(("Finshed", 1.0)).unwrap();

                info!("Done!");
                sender.send(Ok(()));
                thread::sleep(Duration::from_millis(500));
                progress.send(("", 2.0)).unwrap();
            });
            promise
        });
    }

    fn draw_header(&self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("heightmap2brs");
            ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
        });
        ui.hyperlink("https://github.com/brickadia-community/heightmap2brs");
        ui.label("Converts heightmap png files to Brickadia save files, also works as img2brick");
        egui::warn_if_debug_build(ui);
    }

    fn draw_settings(&mut self, ui: &mut Ui) {
        ui.heading("Settings");
        ui.label("Configure how the generator outputs the saves as bricks");

        // list of settings
        egui::Grid::new("settings_grid")
            .striped(true)
            .spacing([40.0, 4.0])
            .show(ui, |ui| {
                ui.set_enabled(true);

                ui.label("Save Path")
                    .on_hover_text("The save will be created relative to the location of the exe.");
                ui.add(egui::TextEdit::singleline(&mut self.out_file).hint_text("File Name"));
                ui.end_row();

                ui.label("Brick Owner");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.owner_name)
                            .hint_text("Name")
                            .desired_width(100.0),
                    );
                    ui.add(egui::TextEdit::singleline(&mut self.owner_id).hint_text("Id"));
                });
                ui.end_row();

                ui.label("Horizontal Scale")
                    .on_hover_text("The size of each pixel in studs (or microbricks)");
                ui.add(egui::Slider::new(&mut self.horizontal_size, 1..=100).text("studs"));
                ui.end_row();
                ui.label("Vertical Size")
                    .on_hover_text("The height of each shade of grey from the heightmap");
                ui.add(egui::Slider::new(&mut self.vertical_scale, 1..=100).text("units"));
                ui.end_row();

                ui.label("Options")
                    .on_hover_text("A list of options for modifying how the generator works");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.opt_snap, "Snap")
                        .on_hover_text("Snap bricks to the brick grid");
                    ui.checkbox(&mut self.opt_cull, "Cull").on_hover_text(
                        "Automatically remove bottom level bricks and fully transparent bricks\n\
                            In image mode, only transparent bricks are removed",
                    );
                    ui.checkbox(&mut self.opt_nocollide, "No Collide")
                        .on_hover_text("Disable brick collision");
                    ui.checkbox(&mut self.opt_lrgb, "LRGB")
                        .on_hover_text("Use linear rgb input color instead of sRGB");
                    ui.checkbox(&mut self.opt_hdmap, "HD Map")
                        .on_hover_text("Using a high detail rgb color encoded heightmap");
                    ui.checkbox(&mut self.opt_glow, "Glow")
                        .on_hover_text("Glow bricks at lowest intensity");
                });
                ui.end_row();

                ui.label("Brick Type")
                    .on_hover_text("Change which brick type is used for the save file");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.mode, BrickMode::Default, "Default")
                        .on_hover_text("Use default bricks");
                    ui.radio_value(&mut self.mode, BrickMode::Tile, "Tile")
                        .on_hover_text("Use tile bricks");
                    ui.radio_value(&mut self.mode, BrickMode::Stud, "Stud")
                        .on_hover_text("Use studded bricks");
                    ui.radio_value(&mut self.mode, BrickMode::Micro, "Micro")
                        .on_hover_text("Use micro bricks");
                });
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.separator();

        ui.heading("Heightmap Images");
        ui.label("Select image files to use for save generation.");

        // handle heightmap multiple file selection
        if ui.button("Select images").clicked() {
            let result = nfd::dialog_multiple()
                .filter("png")
                .open()
                .unwrap_or_else(|e| {
                    panic!("{}", e);
                });

            match result {
                nfd::Response::Okay(_) => unreachable!(),
                nfd::Response::OkayMultiple(files) => {
                    info!("Selected heightmap files: {:?}", files);
                    self.heightmaps = files;
                }
                nfd::Response::Cancel => {
                    self.heightmaps = vec![];
                }
            }
        }

        egui::Grid::new("heightmap_grid")
            .striped(true)
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                for img in &self.heightmaps.clone() {
                    self.thumb(ui, img);
                    ui.label(Path::new(&img).file_name().unwrap().to_str().unwrap());
                    ui.end_row();
                }
            });

        ui.separator();
        ui.add_space(4.0);

        ui.heading("Colormap Image");
        ui.label("Select image file to use for heightmap coloring. Select only a colormap for img2brick mode.");

        // handle colormap single file selection
        if ui.button("Select colormap image").clicked() {
            let result = nfd::dialog().filter("png").open().unwrap_or_else(|e| {
                panic!("{}", e);
            });

            match result {
                nfd::Response::Okay(file_path) => {
                    info!("Selected colormap file: {:?}", file_path);
                    self.colormap = Some(file_path);
                }
                nfd::Response::OkayMultiple(files) => unreachable!(),
                nfd::Response::Cancel => {
                    self.colormap = None;
                }
            }
        }

        if let Some(path) = self.colormap.clone() {
            ui.horizontal(|ui| {
                self.thumb(ui, &path);
                ui.label(Path::new(&path).file_name().unwrap().to_str().unwrap());
            });
        }
    }

    fn draw_progress(&mut self, ctx: &Context, ui: &mut Ui) {
        while let Ok(p) = self.progress_channel.1.try_recv() {
            self.progress = p;
        }
        let (progress_text, progress) = self.progress;

        let mut clear_promise = progress > 1.0;

        if let Some(p) = &self.promise {
            match p.ready() {
                Some(Ok(())) => {
                    ui.add(
                        ProgressBar::new(ctx.animate_value_with_time(
                            Id::new("progress"),
                            1.0,
                            0.1,
                        ))
                        .text("Finished"),
                    );
                }
                Some(Err(e)) => {
                    ui.horizontal(|ui| {
                        if ui.button("ok").clicked() {
                            clear_promise = true;
                        }
                        ui.colored_label(Color32::RED, format!("Error: {e}"));
                    });
                }
                None => {
                    ui.add(
                        ProgressBar::new(ctx.animate_value_with_time(
                            Id::new("progress"),
                            progress,
                            0.1,
                        ))
                        .text(progress_text)
                        .animate(true),
                    );
                }
            }
        }

        if clear_promise {
            self.promise = None
        }
    }

    fn draw_submit(&mut self, ui: &mut Ui) {
        // display different text based on the selected image files
        let heightmap_ok = !self.heightmaps.is_empty();
        let colormap_ok = self.colormap.is_some();

        if self.promise.is_some() {
            return;
        }

        if heightmap_ok || colormap_ok {
            if ui
                .add(
                    Button::new(match (heightmap_ok, colormap_ok) {
                        (true, true) => "Generate save",
                        (true, false) => "Generate colorless save",
                        (false, true) => "Generate image2brick save",
                        (false, false) => unreachable!(),
                    })
                    .fill(Color32::DARK_GREEN),
                )
                .clicked()
            {
                self.run_converter();
            }
        } else {
            ui.label("Select some image files to continue...");
        }
    }

    fn thumb(&mut self, ui: &mut Ui, image: &str) {
        let texture: &egui::TextureHandle = self
            .texture_handles
            .entry(image.to_string())
            .or_insert_with(|| {
                let default_image = egui::ColorImage::new([32, 32], Color32::from_rgb(255, 0, 255));

                let data = load_image_from_path(Path::new(image)).unwrap_or(default_image);

                ui.ctx().load_texture(image, data, Default::default())
            });

        ui.image(texture, vec2(32.0, 32.0));
    }
}

impl App for HeightmapApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            self.draw_header(ui);
            ScrollArea::vertical().show(ui, |ui| {
                ui.separator();
                self.draw_settings(ui);
                ui.separator();
                self.draw_progress(ctx, ui);
                self.draw_submit(ui);
            });

            TopBottomPanel::bottom(Id::new("logs"))
                .min_height(30.0)
                .resizable(true)
                .frame(egui::Frame {
                    fill: Color32::BLACK,
                    inner_margin: 4.0.into(),
                    outer_margin: 0.0.into(),
                    ..Default::default()
                })
                .show_inside(ui, |ui| {
                    logger::draw(ui);
                });
        });
    }
}
