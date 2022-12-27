use std::path::Path;

use egui::ColorImage;
use heightmap::{
    map::{Colormap, ColormapPNG, Heightmap, HeightmapFlat, HeightmapPNG},
    util::{file_ext, GenOptions},
};
use image::{GenericImageView, ImageError};

type MapPair = (Box<dyn Heightmap>, Box<dyn Colormap>);

pub fn maps_from_files(
    options: &GenOptions,
    heightmap_files: Vec<String>,
    colormap_file: Option<String>,
) -> Result<MapPair, String> {
    let heightmap_files: Vec<String> = heightmap_files.into_iter().collect();
    let first_heightmap = heightmap_files
        .first()
        .map(|s| s.to_owned())
        .unwrap_or_else(|| "".to_string());
    let colormap_file = colormap_file.unwrap_or(first_heightmap);

    // colormap file parsing
    let colormap = match file_ext(&colormap_file.to_lowercase()) {
        Some("png") => ColormapPNG::new(&colormap_file, options.lrgb)
            .map_err(|e| format!("Error reading colormap: {:?}", e))?,
        Some(ext) => {
            return Err(format!("Unsupported colormap format '{}'", ext));
        }
        None => {
            return Err(format!("Missing colormap format for '{}'", colormap_file));
        }
    };

    // heightmap file parsing
    let heightmap: Box<dyn Heightmap> =
        if heightmap_files.iter().all(|f| file_ext(f) == Some("png")) {
            if options.img {
                Box::new(HeightmapFlat::new(colormap.size()).unwrap())
            } else {
                match HeightmapPNG::new(
                    heightmap_files.iter().map(|s| s.as_ref()).collect(),
                    options.hdmap,
                ) {
                    Ok(map) => Box::new(map),
                    Err(error) => {
                        return Err(format!("Error reading heightmap: {:?}", error));
                    }
                }
            }
        } else {
            return Err("Unsupported heightmap format".to_string());
        };

    Ok((heightmap, Box::new(colormap)))
}

pub fn load_image_from_path(path: &Path) -> Result<ColorImage, ImageError> {
    let image = image::io::Reader::open(path)?.decode()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
}
