extern crate brs;
extern crate image;

use crate::util::GenOptions;
use brs::*;
use image::RgbImage;
use std::cell::RefCell;
use std::cmp::{max, min};
use std::collections::HashSet;

#[derive(Debug)]
struct Tile {
    index: usize,
    center: (u32, u32),
    size: (u32, u32),
    color: [u8; 3],
    height: u32,
    neighbors: HashSet<u32>,
    parent: Option<usize>,
}

pub struct QuadTree {
    tiles: Vec<RefCell<Tile>>,
    width: u32,
    height: u32,
}

impl Tile {
    // determine if a tile can be merged with this one
    fn similar(&self, other: &Self) -> bool {
        self.size == other.size
            && self.color == other.color
            && self.height == other.height
            && self.parent.is_none()
            && other.parent.is_none()
    }

    // merge a few tiles with this one
    fn merge_quad(
        &mut self,
        top_right: &mut Self,
        bottom_left: &mut Self,
        bottom_right: &mut Self,
    ) {
        // update size
        self.size = (self.size.0 * 2, self.size.1 * 2);

        self.neighbors.extend(&top_right.neighbors);
        self.neighbors.extend(&bottom_left.neighbors);
        self.neighbors.extend(&bottom_right.neighbors);

        // update parents of merged nodes
        top_right.parent = Some(self.index);
        bottom_left.parent = Some(self.index);
        bottom_right.parent = Some(self.index);
    }

    fn merge_line(&mut self, children: Vec<&RefCell<Tile>>) {
        if children.is_empty() {
            return;
        }

        if children[0].borrow().center.0 == self.center.0 {
            // vertical merging
            self.size.1 *= 1 + children.len() as u32
        } else {
            // horizontal merging
            self.size.0 *= 1 + children.len() as u32
        }

        children.into_iter().for_each(|t| {
            t.borrow_mut().parent = Some(self.index);
            self.neighbors.extend(&t.borrow().neighbors);
        });
    }
}

impl QuadTree {
    // create a heightmap grid from two images
    pub fn from_images(heightmap: RgbImage, colormap: RgbImage) -> Self {
        let height = heightmap.height();
        let width = heightmap.width();

        if width != colormap.width() || height != colormap.height() {
            panic!("Heightmap and colormap must have same dimensions");
        }

        let mut tiles = vec![];

        // add all the tiles to the heightmap
        for x in 0..width as i32 {
            for y in 0..height as i32 {
                tiles.push(RefCell::new(Tile {
                    index: (x + y * height as i32) as usize,
                    center: (x as u32, y as u32),
                    // store a set of the neighbor's heights with each tile
                    // they will be joined when the tiles merge
                    neighbors: vec![(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                        .into_iter()
                        .filter(|(x, y)| {
                            *x >= 0 && *x < width as i32 && *y >= 0 && *y < height as i32
                        })
                        .map(|(x, y)| heightmap.get_pixel(x as u32, y as u32).0[0] as u32)
                        .fold(HashSet::new(), |mut set, height| {
                            set.insert(height);
                            set
                        }),
                    size: (1, 1),
                    color: colormap.get_pixel(x as u32, y as u32).0,
                    height: heightmap.get_pixel(x as u32, y as u32).0[0] as u32,
                    parent: None,
                }))
            }
        }

        QuadTree {
            tiles,
            width,
            height,
        }
    }

    // optimize bricks with size (level+1)
    pub fn quad_optimize_level(&mut self, level: u32) -> u32 {
        let mut count = 0;
        macro_rules! get_at {
            ($x:expr, $y:expr) => {
                &self.tiles[($y + $x * self.height) as usize];
            };
        }

        // step amounts
        let space = 2_u32.pow(level);
        let step_amt = space as usize * 2;

        for x in (0..self.width - space).step_by(step_amt) {
            for y in (0..self.height - space).step_by(step_amt) {
                // get neighboring tiles
                let top_left = get_at!(x, y);
                let top_right = get_at!(x + space, y);
                let bottom_left = get_at!(x, y + space);
                let bottom_right = get_at!(x + space, y + space);

                // if these are not similar tiles, skip them
                if !top_left.borrow().similar(&top_right.borrow())
                    || !top_left.borrow().similar(&bottom_left.borrow())
                    || !top_left.borrow().similar(&bottom_right.borrow())
                {
                    continue;
                }

                count += 3;

                // merge the tiles into the first one
                top_left.borrow_mut().merge_quad(
                    &mut top_right.borrow_mut(),
                    &mut bottom_left.borrow_mut(),
                    &mut bottom_right.borrow_mut(),
                );
            }
        }

        count
    }

    // optimize by nearby bricks in line
    pub fn line_optimize(&mut self, size: u32) {
        macro_rules! get_at {
            ($x:expr, $y:expr) => {
                &self.tiles[($y + $x * self.height) as usize];
            };
        }

        for x in 0..self.width {
            for y in 0..self.height {
                let start = get_at!(x, y);
                let shift = start.borrow().size;
                let mut sx = 0;
                let mut horiz_tiles = vec![];
                let mut sy = 0;
                let mut vert_tiles = vec![];

                // determine longest horizontal merge
                while x + sx + shift.0 < self.width && (sx + 1) * shift.0 * size < 500 {
                    let t = get_at!(x + sx + shift.0, y);
                    if !start.borrow().similar(&t.borrow()) {
                        break;
                    }
                    horiz_tiles.push(t);
                    sx += shift.0;
                }

                // determine longest vertical merge
                while y + sy + shift.1 < self.height && (sy + 1) * shift.1 * size < 500 {
                    let t = get_at!(x, y + sy + shift.1);
                    if !start.borrow().similar(&t.borrow()) {
                        break;
                    }
                    vert_tiles.push(t);
                    sy += shift.1;
                }

                // merge whichever is largest
                start
                    .borrow_mut()
                    .merge_line(if sx > sy { horiz_tiles } else { vert_tiles });
            }
        }
    }

    // convert quadtree state into bricks
    pub fn into_bricks(self, options: GenOptions) -> Vec<Brick> {
        self.tiles
            .into_iter()
            .map(|t| {
                let t = t.borrow();
                if t.parent.is_some() || options.cull && t.height == 0 {
                    return vec![];
                }

                let mut z = (options.scale * t.height) as i32;

                // determine the height of this brick (difference of self and smallest neighbor)
                let raw_height = max(
                    t.height as i32 - t.neighbors.iter().cloned().min().unwrap_or(0) as i32 + 1,
                    2,
                );
                let mut desired_height = max(raw_height as i32 * options.scale as i32 / 2, 2);

                // snap bricks to grid
                if options.snap {
                    z += 4 - z % 4;
                    desired_height += 4 - desired_height % 4;
                }

                let mut bricks = vec![];
                // until we've made enough bricks to fill the height
                // add a brick with a max height of 500
                while desired_height > 0 {
                    // pick height for this brick

                    let height = min(max(desired_height, 2), 500) as u32;
                    let height = height + height % 2;

                    bricks.push(brs::Brick {
                        asset_name_index: options.tile.into(),
                        size: (t.size.0 * options.size, t.size.1 * options.size, height),
                        position: (
                            ((t.center.0 * 2 + t.size.0) * options.size) as i32,
                            ((t.center.1 * 2 + t.size.1) * options.size) as i32,
                            z as i32 - height as i32 + 2,
                        ),
                        direction: Direction::ZPositive,
                        rotation: Rotation::Deg0,
                        collision: true,
                        visibility: true,
                        material_index: 0,
                        color: ColorMode::Custom(Color::from_rgba(
                            t.color[0], t.color[1], t.color[2], 255,
                        )),
                        owner_index: 1u32,
                    });

                    // update Z and remaining height
                    desired_height -= height as i32;
                    z -= height as i32 * 2;
                }
                bricks
            })
            .flatten()
            .collect()
    }
}
