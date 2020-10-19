mod map;
mod old;
mod quad;
mod util;

use crate::map::*;
use crate::old::gen_heightmap;
use crate::quad::*;
use crate::util::*;
use brs::*;
use clap::clap_app;
use std::boxed::Box;
use std::fs::File;

fn main() {
    let matches = clap_app!(heightmap =>
        (version: "0.3.6")
        (author: "github.com/Meshiest")
        (about: "Converts heightmap png files to Brickadia save files")
        (@arg INPUT: +required +multiple "Input heightmap PNG images")
        (@arg output: -o --output +takes_value "Output BRS file")
        (@arg colormap: -c --colormap +takes_value "Input colormap PNG image")
        (@arg vertical: -v --vertical +takes_value "Vertical scale multiplier (default 1)")
        (@arg size: -s --size +takes_value "Brick stud size (default 1)")
        (@arg cull: --cull "Automatically remove bottom level bricks")
        (@arg tile: --tile "Render bricks as tiles")
        (@arg micro: --micro "Render bricks as micro bricks")
        (@arg stud: --stud "Render bricks as stud cubes")
        (@arg snap: --snap "Snap bricks to the brick grid")
        (@arg img: -i --img "Make the heightmap flat and render an image")
        (@arg old: --old "Use old unoptimized heightmap code")
        (@arg hdmap: --hdmap "Using a high detail rgb color encoded heightmap")
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
        .unwrap_or("../autogen.brs")
        .to_string();

    // determine generator mode
    let old_mode = matches.is_present("old");

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
        hdmap: matches.is_present("hdmap"),
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

    // colormap file parsing
    let colormap = match file_ext(&colormap_file.to_lowercase()) {
        Some("png") => match ColormapPNG::new(&colormap_file) {
            Ok(map) => map,
            Err(error) => {
                return println!("Error reading colormap: {:?}", error);
            }
        },
        Some(ext) => {
            return println!("Unsupported colormap format '{}'", ext);
        }
        None => {
            return println!("Missing colormap format");
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

    let bricks = if old_mode {
        gen_heightmap(&*heightmap, &colormap, options)
    } else {
        gen_opt_heightmap(&*heightmap, &colormap, options)
    };

    println!("Writing Save to {}", out_file);
    let data = bricks_to_save(bricks);
    let mut write_dest = File::create(out_file).unwrap();
    write_save(&mut write_dest, &data).expect("Could not save file");
    println!("Done!");
}
