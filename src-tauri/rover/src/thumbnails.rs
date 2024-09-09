use std::path::Path;

use image::{imageops, DynamicImage, GenericImageView, ImageBuffer, RgbaImage};

use log::warn;
use uuid::Uuid;

use crate::{db, models::NewThumbnail, queries, state::ConnectionPoolState};

const MAX_THUMBNAIL_DIMENSION: u32 = 600;

// TODO Consider storing thumbnails in a thumbnails/ dir within AppData.
//   Keeps things a bit more organized on disk.

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

/// Ensures a thumbnail exists for the given image.
/// If a thumbnail already exists, it is returned.
/// If an entry for the thumbnail exists in the database but not on disk, the old
/// database entry is deleted and a new thumbnail is generated.
/// If a thumbnail does not exist, it is generated and saved to disk and the database.
/// 
/// Returns the UUID of the thumbnail and the full path to the filename (typically in $APPDATA),
/// inclusing a file::// prefix for rendering in the browser.
pub fn ensure_thumbnail_exists(
    file_id: Uuid,
    app_handle: &tauri::AppHandle,
    pool_state: &tauri::State<'_, ConnectionPoolState>
) -> anyhow::Result<(Uuid, String)>
{
    let mut connection = db::get_db_connection(pool_state)?;

    let app_data_path = app_handle.path_resolver().app_data_dir().ok_or(anyhow::anyhow!("Error getting app data path"))?;

    let db_thumbnail = queries::get_thumbnail_by_file_id(file_id, &mut connection)?;

    match db_thumbnail
    {
        Some(thumbail) => {
            // Check if the thumbnail exists on disk.
            let full_path = app_data_path.join(&thumbail.path);

            if full_path.exists() {
                return Ok((
                    Uuid::parse_str(&thumbail.id)?,
                    full_path.to_str().ok_or(anyhow::anyhow!("Error converting path"))?.to_string()
                ));
            }

            // The thumbnail exists in the DB but not on disk.
            // Delete the DB entry.
            queries::delete_thumbnail_by_id(Uuid::parse_str(&thumbail.id)?, &mut connection)?;
        },
        None => {},
    }

    // The thumbnail was not present in the DB, or it was present but not on disk.
    // Generate the a new thumbnail for the file.

    let new_thumbnail_id = Uuid::new_v4();
    let new_thumbnail_filename = format!("{}.webp", new_thumbnail_id);
    let new_thumbnail_full_path = app_data_path.join(&new_thumbnail_filename);

    // Create the thumbnail + save it.
    // We do expect the file to exist by this point, so it's an error if it doesn't.
    let file_path = queries::get_filepath(file_id, &mut connection)?.ok_or(anyhow::anyhow!("File not found for UUID {:?}", file_id))?;

    // Load exif data to determine image rotation, if applicable
    let file = std::fs::File::open(&file_path)?;
    let mut bufreader = std::io::BufReader::new(file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader)?;

    // EXIF Rotation data is stored as a value 1-8, where:
    // 1 = 0 degrees: the correct orientation, no adjustment is required.
    // 2 = 0 degrees, mirrored: image has been flipped back-to-front.
    // 3 = 180 degrees: image is upside down.
    // 4 = 180 degrees, mirrored: image has been flipped back-to-front and is upside down.
    // 5 = 90 degrees: image has been flipped back-to-front and is on its side.
    // 6 = 90 degrees, mirrored: image is on its side.
    // 7 = 270 degrees: image has been flipped back-to-front and is on its far side.
    // 8 = 270 degrees, mirrored: image is on its far side.
    //
    // In our case, we assume that if there is no rotation data available or EXIF loading fails,
    // that the image is in the correct orientation.
    //
    // Orientation is stored as a SHORT.  You could match `orientation.value`
    // against `Value::Short`, but the standard recommends that readers
    // should accept BYTE, SHORT, or LONG values for any unsigned integer
    // field.  `Value::get_uint` is provided for that purpose.
    let orientation = match exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY) {
        Some(orientation) => {
            let orientation = orientation.value.get_uint(0);
            match orientation {
                Some(v) => v,
                None => 1,
            }
        },
        None => 1,
    };

    // Load the image from the file 
    let orig_image = image::open(&file_path)?;
    let mut thumbnail = thumbnail(&orig_image);
    fix_orientation(&mut thumbnail, orientation);
    thumbnail.save_with_format(new_thumbnail_full_path.clone(), image::ImageFormat::WebP)?;

    // Add the thumbnail to the thumbnails table.
    let new_thumbnail_db = NewThumbnail {
        id: &new_thumbnail_id.to_string(),
        file_id: &file_id.to_string(),
        path: &new_thumbnail_filename,
    };

    queries::insert_thumbnail(&new_thumbnail_db, &mut connection)?;

    let file_prepend = Path::new("file://");
    let full_path = file_prepend
        .join(&new_thumbnail_full_path)
        .to_str().ok_or(anyhow::anyhow!("Unable to join thumbnail file path for {:?}. Is it valid UTF-8?", new_thumbnail_full_path))?
        .to_string();

    Ok((new_thumbnail_id, full_path))
}

fn fix_orientation(image: &mut RgbaImage, orientation: u32) {
    // TODO imageops doesn't support in-place rotation for every case -
    //      maybe go make a pull request to add that functionality?
    // TODO It would further be better to use matrix ops, as in:
    //      https://magnushoff.com/articles/jpeg-orientation/
    //      But that adds some complication interacting with the `image` crate.
    match orientation
    {
        1 => {},
        2 => imageops::flip_horizontal_in_place(image),
        3 => imageops::rotate180_in_place(image),
        4 => {
            imageops::flip_vertical_in_place(image);
        },
        5 => {
            *image = imageops::rotate90(image);
            imageops::flip_horizontal_in_place(image);
        },
        6 => *image = imageops::rotate90(image),
        7 => {
            *image = imageops::rotate270(image);
            imageops::flip_horizontal_in_place(image);
        },
        8 => *image = imageops::rotate270(image),
        _ => {
            warn!("Unsupported EXIF orientation: {}", orientation);
        }
    }
}