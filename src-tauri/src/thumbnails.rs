use std::path::Path;

use image::{DynamicImage, GenericImageView, ImageBuffer};

use uuid::Uuid;

use crate::{db, models::NewThumbnail, queries, state::ConnectionPoolState};

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

    // TODO This does not handle EXIF rotation. We have a ROVER issue open regarding this, image may
    // get a fix for it very soon as well.
    // Though rather than waiting, this might be sufficient: https://docs.rs/kamadak-exif/latest/exif/

    // Create the thumbnail + save it.
    // We do expect the file to exist by this point, so it's an error if it doesn't.
    let file_path = queries::get_filepath(file_id, &mut connection)?.ok_or(anyhow::anyhow!("File not found for UUID {:?}", file_id))?;
    let orig_image = image::open(file_path)?;
    let thumbnail = thumbnail(&orig_image);
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
