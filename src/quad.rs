extern crate brs;
extern crate image;

use crate::map::*;
use crate::util::*;
use brs::*;

use std::cell::RefCell;
use std::cmp::{max, min};
use std::collections::HashSet;

#[derive(Debug)]
struct Tile {
    index: usize,
    center: (u32, u32),
    size: (u32, u32),
    color: [u8; 4],
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
    // determine if another tile is similar in all properties
    fn similar_quad(&self, other: &Self) -> bool {
        self.size == other.size
            && self.color == other.color
            && self.height == other.height
            && self.parent.is_none()
            && other.parent.is_none()
    }

    // determine if another tile is similar in all properties except potentially width or height as long as they are in a line
    fn similar_line(&self, other: &Self) -> bool {
        let is_vertical = self.center.0 == other.center.0;
        let is_horizontal = self.center.1 == other.center.1;

        (is_vertical && self.size.0 == other.size.0 || is_horizontal && self.size.1 == other.size.1)
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

    // merge tiles that are arranged in a line
    fn merge_line(&mut self, children: Vec<&RefCell<Tile>>) {
        // there is nothing to merge, return
        if children.is_empty() {
            return;
        }

        // determine direction of this merge
        let is_vertical = children[0].borrow().center.0 == self.center.0;

        // determine the new size of the parent tile, make children point at the parent
        let new_size = children.into_iter().fold(0, |sum, t| {
            // assign parent, extend parent's neighbors
            t.borrow_mut().parent = Some(self.index);
            self.neighbors.extend(&t.borrow().neighbors);

            // sum size depending on merge direction
            sum + if is_vertical {
                t.borrow().size.1
            } else {
                t.borrow().size.0
            }
        });

        // add the size to its respective dimension
        if is_vertical {
            self.size.1 += new_size
        } else {
            self.size.0 += new_size
        }
    }
}

impl QuadTree {
    // create a heightmap grid from two images
    pub fn new(heightmap: &dyn Heightmap, colormap: &dyn Colormap) -> Self {
        let (width, height) = heightmap.size();

        if colormap.size() != heightmap.size() {
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
                        .map(|(x, y)| heightmap.at(x as u32, y as u32))
                        .fold(HashSet::new(), |mut set, height| {
                            set.insert(height);
                            set
                        }),
                    size: (1, 1),
                    color: colormap.at(x as u32, y as u32),
                    height: heightmap.at(x as u32, y as u32),
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
    pub fn quad_optimize_level(&mut self, level: u32) -> usize {
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
                if !top_left.borrow().similar_quad(&top_right.borrow())
                    || !top_left.borrow().similar_quad(&bottom_left.borrow())
                    || !top_left.borrow().similar_quad(&bottom_right.borrow())
                    || top_left.borrow().size.0 != space
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
    pub fn line_optimize(&mut self, tile_scale: u32) -> usize {
        let mut count = 0;
        macro_rules! get_at {
            ($x:expr, $y:expr) => {
                &self.tiles[($y + $x * self.height) as usize];
            };
        }

        for x in 0..self.width {
            for y in 0..self.height {
                let start = get_at!(x, y);
                if start.borrow().parent.is_some() {
                    continue;
                }

                let shift = start.borrow().size;
                let mut sx = shift.0;
                let mut horiz_tiles = vec![];
                let mut sy = shift.1;
                let mut vert_tiles = vec![];

                // determine longest horizontal merge
                while x + sx < self.width {
                    let t = get_at!(x + sx, y);
                    let t_size = t.borrow().size.0;
                    if (sx + t_size) * tile_scale > 500 || !start.borrow().similar_line(&t.borrow())
                    {
                        break;
                    }
                    horiz_tiles.push(t);
                    sx += t_size;
                }

                // determine longest vertical merge
                while y + sy < self.height {
                    let t = get_at!(x, y + sy);
                    let t_size = t.borrow().size.1;
                    if (sy + t_size) * tile_scale > 500 || !start.borrow().similar_line(&t.borrow())
                    {
                        break;
                    }
                    vert_tiles.push(t);
                    sy += t_size;
                }

                count += max(horiz_tiles.len(), vert_tiles.len());

                // merge whichever is largest
                start
                    .borrow_mut()
                    .merge_line(if horiz_tiles.len() > vert_tiles.len() {
                        horiz_tiles
                    } else {
                        vert_tiles
                    });
            }
        }

        count
    }

    // convert quadtree state into bricks
    pub fn into_bricks(self, options: GenOptions) -> Vec<Brick> {
        self.tiles
            .into_iter()
            .map(|t| {
                let t = t.borrow();
                if t.parent.is_some() || options.cull && (t.height == 0 || t.color[3] == 0) {
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
                // add a brick with a max height of 250
                while desired_height > 0 {
                    // pick height for this brick

                    let height =
                        min(max(desired_height, if options.stud { 5 } else { 2 }), 250) as u32;
                    let height = height + height % (if options.stud { 5 } else { 2 });

                    bricks.push(brs::Brick {
                        asset_name_index: options.asset,
                        size: (
                            t.size.0 * options.size,
                            t.size.1 * options.size,
                            // if it's a microbrick image, just use the block size so it's cubes
                            if options.img && options.micro {
                                options.size
                            } else {
                                height
                            },
                        ),
                        position: (
                            ((t.center.0 * 2 + t.size.0) * options.size) as i32,
                            ((t.center.1 * 2 + t.size.1) * options.size) as i32,
                            z as i32 - height as i32 + 2,
                        ),
                        direction: Direction::ZPositive,
                        rotation: Rotation::Deg0,
                        collision: !options.nocollide,
                        visibility: true,
                        material_index: 0,
                        color: ColorMode::Custom(Color::from_rgba(
                            t.color[0], t.color[1], t.color[2], t.color[3],
                        )),
                        owner_index: Some(0),
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

// Generate a heightmap with brick conservation optimizations
pub fn gen_opt_heightmap(
    heightmap: &dyn Heightmap,
    colormap: &dyn Colormap,
    options: GenOptions,
) -> Vec<Brick> {
    println!("Building initial quadtree");
    let (width, height) = heightmap.size();
    let area = width * height;
    let mut quad = QuadTree::new(heightmap, colormap);

    println!("Optimizing quadtree");
    let mut scale = 0;
    // loop until the bricks would be too wide or we stop optimizing bricks
    while 2_i32.pow(scale + 1) * (options.size as i32) < 500 {
        let count = quad.quad_optimize_level(scale);
        if count == 0 {
            break;
        } else {
            println!("  Removed {:?} {}x bricks", count, 2_i32.pow(scale));
        }
        scale += 1;
    }

    println!("Optimizing linear");
    loop {
        let count = quad.line_optimize(options.size);
        if count == 0 {
            break;
        }
        println!("  Removed {} bricks", count);
    }

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
}
