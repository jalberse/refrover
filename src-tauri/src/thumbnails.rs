use diesel::SqliteConnection;
use image::{DynamicImage, GenericImageView, ImageBuffer};

use rayon::prelude::*;
use uuid::Uuid;

use crate::{models::NewThumbnail, queries};

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

/// Ensures a thumbnail exists for the given image.
/// If a thumbnail already exists, it is returned.
/// If an entry for the thumbnail exists in the database but not on disk, the old
/// database entry is deleted and a new thumbnail is generated.
/// If a thumbnail does not exist, it is generated and saved to disk and the database.
/// Returns the UUID of the thumbnail and the filename of the thumbnail.
pub fn ensure_thumbnail_exists(
    file_id: Uuid,
    app_handle: &tauri::AppHandle,
    connection: &mut SqliteConnection,
) -> (Uuid, String)
{
    let app_data_path = app_handle.path_resolver().app_data_dir().unwrap();

    let db_thumbnail = queries::get_thumbnail_by_file_id(file_id, connection);

    match db_thumbnail
    {
        Some(thumbail) => {
            // Check if the thumbnail exists on disk.
            let full_path = app_data_path.join(&thumbail.path);

            if full_path.exists() {
                return (Uuid::parse_str(&thumbail.id).unwrap(), thumbail.path);
            }

            // The thumbnail exists in the DB but not on disk.
            // Delete the DB entry.
            queries::delete_thumbnail_by_id(Uuid::parse_str(&thumbail.id).unwrap(), connection);
        },
        None => {},
    }

    // The thumbnail was not present in the DB, or it was present but not on disk.
    // Generate the a new thumbnail for the file.

    let new_thumbnail_id = Uuid::new_v4();
    let new_thumbnail_filename = format!("{}.webp", new_thumbnail_id);
    let new_thumbnail_full_path = app_data_path.join(&new_thumbnail_filename);

    // TODO Does this handle EXIF rotation? Conversion to base64 doesn't,
    // but this may be fine. We have an issue open regarding this, image may
    // get a fix for it very soon as well.

    // Create the thumbnail + save it
    let file_path = queries::get_filepath(file_id, connection).unwrap();
    let orig_image = image::open(file_path).unwrap();
    let thumbnail = thumbnail(&orig_image);
    thumbnail.save_with_format(new_thumbnail_full_path, image::ImageFormat::WebP).unwrap();

    // Add the thumbnail to the thumbnails table.
    let new_thumbnail_db = NewThumbnail {
        id: &new_thumbnail_id.to_string(),
        file_id: &file_id.to_string(),
        path: &new_thumbnail_filename,
    };

    queries::insert_thumbnail(&new_thumbnail_db, connection);

    (new_thumbnail_id, new_thumbnail_filename)
}
