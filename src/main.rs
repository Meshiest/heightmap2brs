extern crate brs;
extern crate image;

use brs::*;
use brs::{chrono::DateTime, uuid::Uuid};
use clap::{App, Arg};
use std::cmp::{max, min};

fn image_from_file(file: String) -> image::RgbImage {
    image::open(file).expect("No heightmap provided").to_rgb()
}

type Pos = (i32, i32, i32);
type Col = (u8, u8, u8);

fn ez_brick(size: u32, position: Pos, height: u32, color: Col) -> brs::Brick {
    brs::Brick {
        asset_name_index: 0,
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
        color: ColorMode::Custom(Color::from_rgba(color.0, color.1, color.2, 255)),
        owner_index: 0u32,
    }
}

fn bricks_to_save(bricks: Vec<brs::Brick>) -> brs::WriteData {
    let author = User {
        id: Uuid::parse_str("a1b16aca-9627-4a16-a160-67fa9adbb7b6").unwrap(),
        name: String::from("Generator"),
    };

    let brick_owners = vec![author.clone()];

    WriteData {
        map: String::from("Plate"),
        author,
        description: String::from("None"),
        save_time: DateTime::from(std::time::SystemTime::now()),
        mods: vec![],
        brick_assets: vec![String::from("PB_DefaultBrick")],
        colors: vec![],
        materials: vec![String::from("BMC_Plastic")],
        brick_owners,
        bricks,
    }
}

fn gen_heightmap(
    heightmap_file: String,
    colormap_file: String,
    size: u32,
    scale: u32,
    cull: bool,
) -> brs::WriteData {
    println!("Reading files");
    let heightmap = image_from_file(heightmap_file);
    let colormap = if colormap_file.is_empty() {
        heightmap.clone()
    } else {
        image_from_file(colormap_file)
    };

    let get_height = |x: i32, y: i32| {
        if x < 0 || x >= heightmap.width() as i32 || y < 0 || y >= heightmap.height() as i32 {
            0
        } else {
            heightmap.get_pixel(x as u32, y as u32).0[0]
        }
    };

    let get_color = |x: i32, y: i32| colormap.get_pixel(x as u32, y as u32).0;

    let neighbors = |x: i32, y: i32| vec![(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)];
    let brick_height = |x: i32, y: i32| {
        let top = get_height(x, y);
        let min = neighbors(x, y)
            .into_iter()
            .map(|(x, y)| get_height(x, y))
            .min();

        top as i32 - min.unwrap_or(0) as i32 + 1
    };

    let mut bricks: Vec<Brick> = Vec::new();

    println!("Iterating pixels");
    for y in 0..heightmap.height() as i32 {
        for x in 0..heightmap.width() as i32 {
            let raw_height = brick_height(x, y);

            if cull && raw_height == 0 {
                continue;
            }

            let raw_z = get_height(x, y);

            let height = min(max(raw_height * scale as i32 / 2, 2), 500) as u32;
            let color = get_color(x, y);
            let z = raw_z as i32 * scale as i32;
            // println!("{} {} {} {}", x, y, z, height);

            let brick = ez_brick(
                size,
                (x as i32, y as i32, z as i32),
                (height + height % 2) as u32,
                (color[0], color[1], color[2]),
            );

            bricks.push(brick);
        }
    }

    bricks_to_save(bricks)
}

fn main() {
    let matches = App::new("Heightmap2BRS")
        .version("0.2.0")
        .author("cake")
        .about("Converts png 2 brs file")
        .arg(Arg::with_name("INPUT").required(true).index(1))
        .arg(Arg::with_name("output").short("o").takes_value(true))
        .arg(Arg::with_name("colormap").short("c").takes_value(true))
        .arg(Arg::with_name("size").short("s").takes_value(true))
        .arg(Arg::with_name("scale").short("x").takes_value(true))
        .arg(Arg::with_name("cull").short("z"))
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
        .value_of("scale")
        .unwrap_or("1")
        .parse::<u32>()
        .expect("Scale must be integer");

    let cull = matches.is_present("cull");

    let data = gen_heightmap(heightmap_file, colormap_file, size, scale, cull);

    println!("Writing Save to {}", out_file);
    let mut write_dest = std::fs::File::create(out_file).unwrap();
    write_save(&mut write_dest, &data).unwrap();
}
