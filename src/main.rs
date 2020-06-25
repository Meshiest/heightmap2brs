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
use std::fs::File;

fn main() {
    let matches = clap_app!(heightmap =>
        (version: "0.3.4")
        (author: "github.com/Meshiest")
        (about: "Converts heightmap png files to Brickadia save files")
        (@arg INPUT: +required +multiple "Input heightmap PNG images")
        (@arg output: -o --output +takes_value "Output BRS file")
        (@arg colormap: -c --colormap +takes_value "Input colormap PNG image")
        (@arg vertical: -v --vertical +takes_value "Vertical scale multiplier (default 1)")
        (@arg size: -s --size +takes_value "Brick stud size (default 1)")
        (@arg cull: --cull "Automatically remove bottom level bricks")
        (@arg tile: --tile "Render bricks as tiles")
        (@arg snap: --snap "Snap bricks to the brick grid")
        (@arg old: --old "Use old unoptimized heightmap code")
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
    let options = GenOptions {
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
        tile: matches.is_present("tile"),
        snap: matches.is_present("snap"),
    };

    println!("Reading image files");

    // heightmap file parsing
    let heightmap = if heightmap_files.iter().all(|f| file_ext(f) == Some("png")) {
        match HeightmapPNG::new(heightmap_files) {
            Ok(map) => map,
            Err(error) => {
                return println!("Error reading colormap: {:?}", error);
            }
        }
    } else {
        return println!("Unsupported heightmap format");
    };

    // colormap file parsing
    let colormap = match file_ext(&colormap_file) {
        Some("png") => match ColormapPNG::new(&colormap_file) {
            Ok(map) => map,
            Err(error) => {
                return println!("Error reading colormap: {:?}", error);
            }
        },
        Some(ext) => {
            return println!("Unsupported colormap format '{}'", ext);
        }
        _ => {
            return println!("Unexpected colormap format");
        }
    };

    let bricks = if old_mode {
        gen_heightmap(&heightmap, &colormap, options)
    } else {
        gen_opt_heightmap(&heightmap, &colormap, options)
    };

    println!("Writing Save to {}", out_file);
    let data = bricks_to_save(bricks);
    let mut write_dest = File::create(out_file).unwrap();
    write_save(&mut write_dest, &data).unwrap();
    println!("Done!");
}
