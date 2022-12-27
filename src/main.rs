pub mod map;
pub mod quad;
pub mod util;

use crate::{map::*, quad::*, util::*};
use brickadia::write::SaveWriter;
use clap::clap_app;
use env_logger::Builder;
use log::{error, info, LevelFilter};
use std::{boxed::Box, fs::File, io::Write};

fn main() {
    Builder::new()
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .filter(None, LevelFilter::Info)
        .init();

    let matches = clap_app!(heightmap =>
        (version: env!("CARGO_PKG_VERSION"))
        (author: "github.com/Meshiest")
        (about: "Converts heightmap png files to Brickadia save files")
        (@arg INPUT: +required +multiple "Input heightmap PNG images")
        (@arg output: -o --output +takes_value "Output BRS file")
        (@arg colormap: -c --colormap +takes_value "Input colormap PNG image")
        (@arg vertical: -v --vertical +takes_value "Vertical scale multiplier (default 1)")
        (@arg size: -s --size +takes_value "Brick stud size (default 1)")
        (@arg cull: --cull "Automatically remove bottom level bricks and fully transparent bricks")
        (@arg tile: --tile "Render bricks as tiles")
        (@arg micro: --micro "Render bricks as micro bricks")
        (@arg stud: --stud "Render bricks as stud cubes")
        (@arg snap: --snap "Snap bricks to the brick grid")
        (@arg lrgb: --lrgb "Use linear rgb input color instead of sRGB")
        (@arg img: -i --img "Make the heightmap flat and render an image")
        (@arg glow: --glow "Make the heightmap glow at 0 intensity")
        (@arg hdmap: --hdmap "Using a high detail rgb color encoded heightmap")
        (@arg nocollide: --nocollide "Disable brick collision")
        (@arg owner_id: --owner_id  +takes_value "Set the owner id (default a1b16aca-9627-4a16-a160-67fa9adbb7b6)")
        (@arg owner: --owner +takes_value "Set the owner name (default Generator)")
    )
    .get_matches();

    // get files from matches
    let heightmap_files = matches.values_of("INPUT").unwrap().collect::<Vec<&str>>();
    let colormap_file = matches
        .value_of("colormap")
        .unwrap_or(heightmap_files[0])
        .to_string();
    let out_file = matches
        .value_of("output")
        .unwrap_or("./out.brs")
        .to_string();

    // owner values
    let owner_id = matches
        .value_of("owner_id")
        .unwrap_or("a1b16aca-9627-4a16-a160-67fa9adbb7b6")
        .to_string();
    let owner_name = matches.value_of("owner").unwrap_or("Generator").to_string();

    // output options
    let mut options = GenOptions {
        size: matches
            .value_of("size")
            .unwrap_or("1")
            .parse::<u32>()
            .expect("Size must be integer")
            * 5,
        scale: matches
            .value_of("vertical")
            .unwrap_or("1")
            .parse::<u32>()
            .expect("Scale must be integer"),
        cull: matches.is_present("cull"),
        asset: 0,
        tile: matches.is_present("tile"),
        micro: matches.is_present("micro"),
        stud: matches.is_present("stud"),
        snap: matches.is_present("snap"),
        img: matches.is_present("img"),
        glow: matches.is_present("glow"),
        hdmap: matches.is_present("hdmap"),
        lrgb: matches.is_present("lrgb"),
        nocollide: matches.is_present("nocollide"),
        quadtree: true,
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

    info!("Reading image files");

    // colormap file parsing
    let colormap = match file_ext(&colormap_file.to_lowercase()) {
        Some("png") => match ColormapPNG::new(&colormap_file, options.lrgb) {
            Ok(map) => map,
            Err(err) => {
                return error!("Error reading colormap: {:?}", err);
            }
        },
        Some(ext) => {
            return error!("Unsupported colormap format '{}'", ext);
        }
        None => {
            return error!("Missing colormap format for '{}'", colormap_file);
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
                        return error!("Error reading heightmap: {:?}", error);
                    }
                }
            }
        } else {
            return error!("Unsupported heightmap format");
        };

    let bricks = gen_opt_heightmap(&*heightmap, &colormap, options, |_| true)
        .expect("error during generation");

    info!("Writing Save to {}", out_file);
    let data = bricks_to_save(bricks, owner_id, owner_name);
    SaveWriter::new(File::create(out_file).unwrap(), data)
        .write()
        .expect("Failed to write file!");
    info!("Done!");
}
