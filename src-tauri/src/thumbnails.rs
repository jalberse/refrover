use image::{DynamicImage, GenericImageView, ImageBuffer};

use rayon::prelude::*;

const MAX_THUMBNAIL_DIMENSION: u32 = 600;

pub fn thumbnail(
    orig_image: &DynamicImage,
) -> ImageBuffer<image::Rgba<u8>, Vec<u8>>
{
    let (width, height) = orig_image.dimensions();
    let (new_width, new_height) = if width > height {
        (MAX_THUMBNAIL_DIMENSION, MAX_THUMBNAIL_DIMENSION * height / width)
    } else {
        (MAX_THUMBNAIL_DIMENSION * width / height, MAX_THUMBNAIL_DIMENSION)
    };
    image::imageops::thumbnail(orig_image, new_width, new_height)
}

pub fn thumbnail_parallel(
    orig_images: &Vec<DynamicImage>,
) -> Vec<ImageBuffer<image::Rgba<u8>, Vec<u8>>>
{
    orig_images
        .par_iter()
        .map(|img| thumbnail(img))
        .collect()
}

// Possibly some ensure_thumbnail that ensures a fileID has a thumbnail and generates it if not?
// Would I guess take a file's UUID, check if there's a thumbnail in some (new) thumbnails table,
//   generate if not and insert into the table, and return the path to the thumbnail?