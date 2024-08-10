use uuid::Uuid;

use crate::state::{ClipState, ConnectionPoolState};
use crate::{db, junk_drawer, queries, thumbnails};
use crate::{preprocessing, state::SearchState};
use imghdr;
use crate::interface::{FileMetadata, FileUuid, ImageSize, Thumbnail, ThumbnailUuid};

use rayon::prelude::*; // For par_iter


// TODO Note that this currently returns base64 png encodings of the images.
//      This is because the front-end is not allowed to access arbitrary files on the system
//      (which I think is dumb - see https://github.com/tauri-apps/tauri/issues/3591 contention).
//      But nevertheless, eventually we want to move to displaying *thumbnails* in the vast majority of cases,
//      which we'll be able to store in a whitelisted location for the frontend.
//      So bear in mind this command API will change to accomodate that (returning a struct
//      with data related to thumbnail paths etc most likely, with base64 as some fallback for edge cases
//      such as very wide images that we can't thumbnail effectively).
// TODO If the distance is large enough, we should not include it in the results.
//   i.e. we need to filter the search_results on the distance to pass some constant threshold (tweaked by us)

/// Search for image UUIDs which match a query string according to CLIP encodings.
/// Returns a list of UUIDs of images which match the query.
/// We return the UUIDs so that separate API calls can be made to fetch the metadata
/// and thumbnails; this allows us to display metadata and results more quickly
/// while the thumbnails are still loading/generating.
#[tauri::command]
pub async fn search_images<'a>(
        query_string: &str,
        search_state: tauri::State<'_, SearchState<'a>>,
        clip_state: tauri::State<'_, ClipState>,
    ) -> Result<Vec<FileUuid>, String>
{
    if query_string.is_empty() {
        return Ok(vec![]);
    }

    let mut hnsw_search = search_state.0.lock().unwrap();
    let hnsw = &mut hnsw_search.hnsw;

    let query = preprocessing::tokenize(query_string);

    let clip = &mut clip_state.0.lock().unwrap().clip;

    let query_vector = clip.encode_text(query).unwrap();

    let query_vector_slice = query_vector.as_slice().unwrap();

    let search_results = hnsw.search(query_vector_slice, 10, 400);

    let search_results_uuids: Vec<FileUuid> = search_results.iter().map(|x| FileUuid(x.0.to_string())).collect();

    Ok(search_results_uuids)
}

/// Fetches the thumbnail filenames for a list of file IDs.
/// Returns a list of (thumbnail UUID, thumbnail filename) for the files.
/// The intention is the frontend can use the filename in an Image tag.
/// The UUID is returns for React to map elements if necessary on a unique ID.
/// Note that the UUID is *not* the file ID, but the UUID of the thumbnail.
/// The filename is local to APPDATA.
#[tauri::command]
pub async fn fetch_thumbnails(
        file_ids: Vec<String>,
        app_handle: tauri::AppHandle,
        pool_state: tauri::State<'_, ConnectionPoolState>
    ) -> Result<Vec<Thumbnail>, String>
{
    let results = file_ids.par_iter().map(
        |file_id| {
            let (thumbnail_uuid, thumbnail_filepath) = thumbnails::ensure_thumbnail_exists(
                Uuid::parse_str(&file_id).unwrap(),
                &app_handle,
                &pool_state
            );
            Thumbnail
            {
                uuid: ThumbnailUuid(thumbnail_uuid.to_string()),
                file_uuid: FileUuid(file_id.to_string()),
                path: thumbnail_filepath,
            }
        }
    ).collect();

    Ok(results)
}


/// Fetches the metadata for the given file ID
/// 
/// Returns an error if the file is not found.
/// For most fields, if the metadata is not available or an error occurs while fetching it,
/// the field will be None, and errors will be silently ignored. For example, trying to determine
/// the image type on a file that is not an image will fail, so the field will be none - but we
/// will not return an error, since other metadata may still be available and useful.
#[tauri::command]
pub async fn fetch_metadata(
    file_id: String,
    pool_state: tauri::State<'_, ConnectionPoolState>
) -> Result<FileMetadata, String>
{
    let uuid = Uuid::parse_str(&file_id).unwrap();
    let mut connection = db::get_db_connection(&pool_state);

    let filepath = queries::get_filepath(uuid, &mut connection);

    if filepath.is_none() {
        // TODO Use thiserror to manage error types https://jonaskruckenberg.github.io/tauri-docs-wip/development/inter-process-communication.html#error-handling
        return Err("File not found".to_string());
    }
    let filepath = filepath.unwrap();
    
    let fs_metadata = std::fs::metadata(&filepath);
    let (date_created, date_modified, date_accessed) = match fs_metadata {
        Ok(metadata) => {
            let date_created = metadata.created().ok();
            let date_modified = metadata.modified().ok();
            let date_accessed = metadata.accessed().ok();
            
            // Convert each to a string using chrono
            let date_created = match date_created {
                Some(d) => Some(junk_drawer::system_time_to_string(d)),
                None => None
            };

            let date_modified = match date_modified {
                Some(d) => Some(junk_drawer::system_time_to_string(d)),
                None => None
            };

            let date_accessed = match date_accessed {
                Some(d) => Some(junk_drawer::system_time_to_string(d)),
                None => None
            };

            (date_created, date_modified, date_accessed)
        },
        Err(_) => (None, None, None)
    };

    // TODO This can also fail for e.g. bad permissions, also use thiserror for this.
    let image_type = imghdr::from_file(&filepath).expect("Error determining image type");
    
    let dimensions = imagesize::size(&filepath);
    let dimensions = match dimensions {
        Ok(dim) => Some(ImageSize { width: dim.width as u32, height: dim.height as u32 }),
        Err(_) => None
    };
    
    let filename = filepath.file_name().unwrap().to_str().unwrap().to_string();
    
    let metadata = FileMetadata
    {
        file_id,
        filename,
        image_type,
        size: dimensions,
        date_created,
        date_modified,
        date_accessed,
    };

    Ok(metadata)
}
