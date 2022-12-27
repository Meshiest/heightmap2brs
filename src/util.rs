use brickadia::save::{Brick, BrickOwner, Header1, Header2, SaveData, User};
use std::ffi::OsStr;
use std::path::Path;
use uuid::Uuid;

pub struct GenOptions {
    pub size: u32,
    pub scale: u32,
    pub asset: u32,
    pub cull: bool,
    pub tile: bool,
    pub micro: bool,
    pub stud: bool,
    pub snap: bool,
    pub img: bool,
    pub glow: bool,
    pub hdmap: bool,
    pub lrgb: bool,
    pub nocollide: bool,
    pub quadtree: bool,
}

// convert gamma to linear gamma
pub fn to_linear_gamma(c: u8) -> u8 {
    let cf = (c as f64) / 255.0;
    (if cf > 0.04045 {
        (cf / 1.055 + 0.0521327).powf(2.4) * 255.0
    } else {
        cf / 12.192 * 255.0
    }) as u8
}

// convert sRGB to linear rgb
pub fn to_linear_rgb(rgb: [u8; 4]) -> [u8; 4] {
    [
        to_linear_gamma(rgb[0]),
        to_linear_gamma(rgb[1]),
        to_linear_gamma(rgb[2]),
        rgb[3],
    ]
}

// given an array of bricks, create a save
#[allow(unused)]
pub fn bricks_to_save(bricks: Vec<Brick>, owner_id: String, owner_name: String) -> SaveData {
    let default_id = Uuid::parse_str("a1b16aca-9627-4a16-a160-67fa9adbb7b6").unwrap();

    let author = User {
        id: Uuid::parse_str(&owner_id).unwrap_or(default_id),
        name: owner_name.clone(),
    };

    let brick_owners = vec![BrickOwner {
        id: Uuid::parse_str(&owner_id).unwrap_or(default_id),
        name: owner_name,
        bricks: bricks.len() as u32,
    }];

    SaveData {
        header1: Header1 {
            map: String::from("https://github.com/brickadia-community"),
            author,
            description: String::from("Save generated from heightmap file"),
            ..Default::default()
        },
        header2: Header2 {
            brick_assets: vec![
                String::from("PB_DefaultBrick"),
                String::from("PB_DefaultTile"),
                String::from("PB_DefaultMicroBrick"),
                String::from("PB_DefaultStudded"),
            ],
            materials: vec!["BMC_Plastic".into(), "BMC_Glow".into()],
            brick_owners,
            ..Default::default()
        },
        bricks,
        ..Default::default()
    }
}

// get extension from filename
#[allow(unused)]
pub fn file_ext(filename: &str) -> Option<&str> {
    Path::new(filename).extension().and_then(OsStr::to_str)
}
