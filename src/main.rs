mod old;
mod quad;
mod util;

use crate::old::gen_heightmap;
use crate::quad::*;
use crate::util::*;
use brs::*;
use clap::clap_app;

fn main() {
    let matches = clap_app!(heightmap =>
        (version: "0.3.0")
        (author: "github.com/Meshiest")
        (about: "Converts heightmap png files to Brickadia save files")
        (@arg INPUT: +required "Input heightmap PNG image")
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

    let heightmap_file = matches.value_of("INPUT").unwrap().to_string();
    let colormap_file = matches.value_of("colormap").unwrap_or("").to_string();
    let out_file = matches
        .value_of("output")
        .unwrap_or("../autogen.brs")
        .to_string();

    let old_mode = matches.is_present("old");

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

    let bricks = if old_mode {
        gen_heightmap(heightmap_file, colormap_file, options)
    } else {
        println!("Reading image files");
        let heightmap = image_from_file(heightmap_file);
        let colormap = if colormap_file.is_empty() {
            heightmap.clone()
        } else {
            image_from_file(colormap_file)
        };

        println!("Building initial quadtree");
        let area = heightmap.width() * heightmap.height();
        let mut quad = QuadTree::from_images(heightmap, colormap);

        println!("Optimizing quadtree");
        let mut scale = 0;
        // loop until the bricks would be too wide or we stop optimizing bricks
        while 2_i32.pow(scale + 1) * (options.size as i32) < 500 {
            let count = quad.quad_optimize_level(scale);
            if count == 0 {
                break;
            } else {
                println!("  Removed {:?} {}x{} bricks", count, scale + 1, scale + 1);
            }
            scale += 1;
        }

        println!("Optimizing linear");
        quad.line_optimize(options.size);
        quad.line_optimize(options.size);

        let bricks = quad.into_bricks(options);
        let brick_count = bricks.len();
        println!(
            "Reduced {} to {} ({}%; -{} bricks)",
            area,
            brick_count,
            (100. - brick_count as f64 / area as f64 * 100.).floor(),
            area as i32 - brick_count as i32,
        );
        bricks
    };

    println!("Writing Save to {}", out_file);
    let data = bricks_to_save(bricks);
    let mut write_dest = std::fs::File::create(out_file).unwrap();
    write_save(&mut write_dest, &data).unwrap();
    println!("Done!");
}
