#![allow(dead_code, unused_variables)]

use {
    brs::write_save,
    heightmap::{map::*, quad::*, util::*},
    std::{boxed::Box, cell::RefCell, fs::File, path::Path, rc::Rc},
};

#[derive(Debug, PartialEq, Clone)]
enum BrickMode {
    Default,
    Tile,
    Stud,
    Micro,
}

#[derive(Debug, Clone)]
pub struct HeightmapApp {
    // options for the generator
    heightmaps: Rc<RefCell<Vec<String>>>,
    colormap: Rc<RefCell<Option<String>>>,
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
    mode: BrickMode,
}

impl HeightmapApp {
    fn run_converter(&mut self) {
        println!("Running converter");
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
            img: self.heightmaps.borrow().len() == 0 && self.colormap.borrow().is_some(),
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

        println!("Reading image files");

        let heightmap_str = self.heightmaps.borrow();
        let heightmap_files = heightmap_str
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<&str>>();
        let first_heightmap = heightmap_files.first().map(|s| s.to_owned()).unwrap_or("");
        let colormap_file = self
            .colormap
            .borrow()
            .as_ref()
            .unwrap_or(&first_heightmap.to_string())
            .to_string();

        // colormap file parsing
        let colormap = match file_ext(&colormap_file.to_lowercase()) {
            Some("png") => match ColormapPNG::new(&colormap_file, options.lrgb) {
                Ok(map) => map,
                Err(error) => {
                    return println!("Error reading colormap: {:?}", error);
                }
            },
            Some(ext) => {
                return println!("Unsupported colormap format '{}'", ext);
            }
            None => {
                return println!("Missing colormap format for '{}'", colormap_file);
            }
        };

        // heightmap file parsing
        let heightmap: Box<dyn Heightmap> =
            if heightmap_files.iter().all(|f| file_ext(f) == Some("png")) {
                if options.img {
                    Box::new(HeightmapFlat::new(colormap.size()).unwrap())
                } else {
                    match HeightmapPNG::new(heightmap_files, options.hdmap) {
                        Ok(map) => Box::new(map),
                        Err(error) => {
                            return println!("Error reading heightmap: {:?}", error);
                        }
                    }
                }
            } else {
                return println!("Unsupported heightmap format");
            };

        let bricks = gen_opt_heightmap(&*heightmap, &colormap, options);

        println!("Writing Save to {}", self.out_file);
        let data = bricks_to_save(bricks, self.owner_id.clone(), self.owner_name.clone());
        let mut write_dest = File::create(self.out_file.clone()).unwrap();
        write_save(&mut write_dest, &data).expect("Could not save file");
        println!("Done!");
    }
}

impl Default for HeightmapApp {
    fn default() -> Self {
        Self {
            // default generator options
            heightmaps: Rc::new(RefCell::new(vec![])),
            colormap: Rc::new(RefCell::new(None)),
            owner_name: "Generator".to_string(),
            owner_id: "a1b16aca-9627-4a16-a160-67fa9adbb7b6".to_string(),
            out_file: "out.brs".to_string(),
            vertical_scale: 1,
            horizontal_size: 1,
            opt_cull: false,
            opt_nocollide: false,
            opt_lrgb: false,
            opt_snap: false,
            opt_hdmap: false,
            mode: BrickMode::Default,
        }
    }
}

impl epi::App for HeightmapApp {
    fn name(&self) -> &str {
        "heightmap2brs"
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // header
            ui.horizontal(|ui|{
                ui.heading("heightmap2brs");
                ui.label("v0.4.2");
            });
            ui.hyperlink("https://github.com/brickadia-community/heightmap2brs");
            ui.label("Converts heightmap png files to Brickadia save files, also works as img2brick");
            egui::warn_if_debug_build(ui);

            ui.separator();

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
                        ui.add(egui::TextEdit::singleline(&mut self.owner_name).hint_text("Name").desired_width(100.0));
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

            ui.separator();

            ui.heading("Heightmap Images");
            ui.label("Select image files to use for save generation.");

            // handle heightmap multiple file selection
            if ui.button("select images").clicked() {
                let result = nfd::dialog_multiple().filter("png").open().unwrap_or_else(|e| {
                    panic!("{}", e);
                });

                match result {
                    nfd::Response::Okay(_) => unreachable!(),
                    nfd::Response::OkayMultiple(files) => {
                        println!("Selected heightmap files: {:?}", files);
                        self.heightmaps.replace(files);
                    },
                    nfd::Response::Cancel => {
                        self.heightmaps.replace(vec![]);
                    }
                }
            }

            let maps = self.heightmaps.borrow();

            egui::Grid::new("heightmap_grid")
                .striped(true)
                .spacing([40.0, 4.0])
                .show(ui, |ui| {
                    for img in maps.iter() {
                        ui.label(Path::new(&img).file_name().unwrap().to_str().unwrap());
                        ui.end_row();
                    }
                });

            ui.separator();

            ui.heading("Colormap Image");
            ui.label("Select image file to use for heightmap coloring. Select only a colormap for img2brick mode.");

            // handle colormap single file selection
            if ui.button("select colormap image").clicked() {
                let result = nfd::dialog().filter("png").open().unwrap_or_else(|e| {
                    panic!("{}", e);
                });

                match result {
                    nfd::Response::Okay(file_path) => {
                        println!("Selected colormap file: {:?}", file_path);
                        self.colormap.replace(Some(file_path));
                    },
                    nfd::Response::OkayMultiple(files) => unreachable!(),
                    nfd::Response::Cancel => {
                        self.colormap.replace(None);
                    },
                }
            }

            if let Some(path) = self.colormap.borrow().as_ref() {
                ui.label(Path::new(path).file_name().unwrap().to_str().unwrap());
            }
            ui.separator();

            // display different text based on the selected image files
            let heightmap_ok = self.heightmaps.borrow().len() > 0;
            let colormap_ok = self.colormap.borrow().is_some();
            if heightmap_ok || colormap_ok {
                if ui.button(
                    match (heightmap_ok, colormap_ok) {
                        (true, true) => "generate save",
                        (true, false) => "generate colorless save",
                        (false, true) => "generate image2brick save",
                        (false, false) => unreachable!(),
                    }).clicked() {
                    self.clone().run_converter();
                }
            } else {
                ui.label("Select some image files to continue...");
            }

        });
    }
}

// run the window with glium
fn main() {
    let app = HeightmapApp::default();
    egui_glium::run(
        Box::new(app),
        epi::NativeOptions {
            always_on_top: false,
            decorated: true,
            drag_and_drop_support: true,
            icon_data: None,
            initial_window_size: Some(egui::Vec2 { x: 600.0, y: 600.0 }),
            resizable: true,
            transparent: false,
        },
    )
}
