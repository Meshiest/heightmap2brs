extern crate brs;
use brs::*;
use brs::{chrono::DateTime, uuid::Uuid};

type Pos = (i32, i32, i32);
type Col = [u8; 3];

pub struct GenOptions {
    pub size: u32,
    pub scale: u32,
    pub cull: bool,
    pub tile: bool,
    pub snap: bool,
}

// Open an image file
pub fn image_from_file(file: String) -> image::RgbImage {
    image::open(file).expect("Invalid image file").to_rgb()
}

// Brick creation helper
pub fn ez_brick(size: u32, position: Pos, height: u32, color: Col, tile: bool) -> brs::Brick {
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
pub fn bricks_to_save(bricks: Vec<brs::Brick>) -> brs::WriteData {
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
