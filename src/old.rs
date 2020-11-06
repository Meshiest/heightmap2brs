use crate::map::Colormap;
use crate::map::Heightmap;
use crate::util::ez_brick;
use crate::util::GenOptions;
use brs::Brick;
use core::cmp::max;
use core::cmp::min;

pub fn gen_heightmap(
    heightmap: &dyn Heightmap,
    colormap: &dyn Colormap,
    options: GenOptions,
) -> Vec<Brick> {
    if heightmap.size() != colormap.size() {
        panic!("Heightmap and colormap must have same dimensions");
    }

    let (img_width, img_height) = heightmap.size();

    // get the height of a pixel if it is in bounds
    let get_height = |x: i32, y: i32| {
        if x < 0 || x >= img_width as i32 || y < 0 || y >= img_height as i32 {
            0
        } else {
            heightmap.at(x as u32, y as u32)
        }
    };

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
    for y in 0..img_height as i32 {
        for x in 0..img_width as i32 {
            let raw_z = get_height(x, y);

            // cull bricks with 0 height
            if options.cull && raw_z == 0 {
                continue;
            }

            let raw_height = brick_height(x, y);
            let color = colormap.at(x as u32, y as u32);

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
                    [color[0], color[1], color[2]],
                    options.tile,
                ));

                // update Z and remaining height
                desired_height -= height as i32;
                z -= height as i32 * 2;
            }
        }
    }

    println!("Generated {} bricks", bricks.len());

    bricks
}
