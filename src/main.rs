extern crate brs;
extern crate image;

use brs::*;
use brs::{chrono::DateTime, uuid::Uuid};
use clap::clap_app;
use std::cmp::{max, min};

type Pos = (i32, i32, i32);
type Col = [u8; 3];

#[derive(Debug)]
struct GenOptions {
    size: u32,
    scale: u32,
    cull: bool,
    tile: bool,
    snap: bool,
}

// Open an image file
fn image_from_file(file: String) -> image::RgbImage {
    image::open(file).expect("Invalid image file").to_rgb()
}

// Brick creation helper
fn ez_brick(size: u32, position: Pos, height: u32, color: Col, tile: bool) -> brs::Brick {
    // require brick height to be even (gen doesn't allow odd height bricks)
    let height = height + height % 2;

    brs::Brick {
        asset_name_index: tile.into(),
        size: (size, size, height),
        position: (
            position.0 * size as i32 * 2 + 5,
            position.1 * size as i32 * 2 + 5,
            position.2 - height as i32 + 2,
        ),
        direction: Direction::ZPositive,
        rotation: Rotation::Deg0,
        collision: true,
        visibility: true,
        material_index: 0,
        color: ColorMode::Custom(Color::from_rgba(color[0], color[1], color[2], 255)),
        owner_index: 1u32,
    }
}

// given an array of bricks, create a save
fn bricks_to_save(bricks: Vec<brs::Brick>) -> brs::WriteData {
    let author = User {
        id: Uuid::parse_str("a1b16aca-9627-4a16-a160-67fa9adbb7b6").unwrap(),
        name: String::from("Generator"),
    };

    let brick_owners = vec![User {
        id: Uuid::parse_str("a1b16aca-9627-4a16-a160-67fa9adbb7b6").unwrap(),
        name: String::from("Generator"),
    }];

    WriteData {
        map: String::from("Plate"),
        author,
        description: String::from("Save generated from heightmap file"),
        save_time: DateTime::from(std::time::SystemTime::now()),
        mods: vec![],
        brick_assets: vec![
            String::from("PB_DefaultBrick"),
            String::from("PB_DefaultTile"),
        ],
        colors: vec![],
        materials: vec![String::from("BMC_Plastic")],
        brick_owners,
        bricks,
    }
}

fn gen_heightmap(
    heightmap_file: String,
    colormap_file: String,
    options: GenOptions,
) -> brs::WriteData {
    println!("Reading files");
    let heightmap = image_from_file(heightmap_file);
    let colormap = if colormap_file.is_empty() {
        heightmap.clone()
    } else {
        image_from_file(colormap_file)
    };

    // get the height of a pixel if it is in bounds
    let get_height = |x: i32, y: i32| {
        if x < 0 || x >= heightmap.width() as i32 || y < 0 || y >= heightmap.height() as i32 {
            0
        } else {
            heightmap.get_pixel(x as u32, y as u32).0[0]
        }
    };

    // get the color of a pixel
    let get_color = |x: i32, y: i32| colormap.get_pixel(x as u32, y as u32).0;

    // determine how tall a brick should be based on its neighbors
    let brick_height = |x: i32, y: i32| {
        let top = get_height(x, y);
        let min = vec![(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
            .into_iter()
            .map(|(x, y)| get_height(x, y))
            .min();

        top as i32 - min.unwrap_or(0) as i32 + 1
    };

    let mut bricks: Vec<Brick> = Vec::new();

    println!("Iterating pixels");
    for y in 0..heightmap.height() as i32 {
        for x in 0..heightmap.width() as i32 {
            let raw_z = get_height(x, y);

            // cull bricks with 0 height
            if options.cull && raw_z == 0 {
                continue;
            }

            let raw_height = brick_height(x, y);
            let color = get_color(x, y);

            let mut desired_height = max(raw_height * options.scale as i32 / 2, 2);
            let mut z = raw_z as i32 * options.scale as i32;

            if options.snap {
                z += 4 - z % 4;
                desired_height += 4 - desired_height % 4;
            }

            // until we've made enough bricks to fill the height
            // add a brick with a max height of 500
            while desired_height > 0 {
                // pick height for this brick
                let height = min(max(desired_height, 2), 500) as u32;

                // make the brick
                bricks.push(ez_brick(
                    options.size,
                    (x as i32, y as i32, z as i32),
                    height,
                    color,
                    options.tile,
                ));

                // update Z and remaining height
                desired_height -= height as i32;
                z -= height as i32 * 2;
            }
        }
    }

    bricks_to_save(bricks)
}

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
    )
    .get_matches();

    let heightmap_file = matches.value_of("INPUT").unwrap().to_string();
    let colormap_file = matches.value_of("colormap").unwrap_or("").to_string();
    let out_file = matches
        .value_of("output")
        .unwrap_or("../autogen.brs")
        .to_string();

    let size = matches
        .value_of("size")
        .unwrap_or("1")
        .parse::<u32>()
        .expect("Size must be integer")
        * 5;

    let scale = matches
        .value_of("vertical")
        .unwrap_or("1")
        .parse::<u32>()
        .expect("Scale must be integer");

    let cull = matches.is_present("cull");
    let tile = matches.is_present("tile");
    let snap = matches.is_present("snap");

    let data = gen_heightmap(
        heightmap_file,
        colormap_file,
        GenOptions {
            size,
            scale,
            cull,
            tile,
            snap,
        },
    );

    println!("Writing Save to {}", out_file);
    let mut write_dest = std::fs::File::create(out_file).unwrap();
    write_save(&mut write_dest, &data).unwrap();
}
