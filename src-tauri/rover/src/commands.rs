use anyhow::Context;
use log::info;
use notify_debouncer_full::notify::{RecursiveMode, Watcher};
use tauri::Manager;
use uuid::Uuid;

use crate::state::{ClipState, ClipTokenizerState, ConnectionPoolState, FsWatcherState};
use crate::{db, junk_drawer, queries, thumbnails};
use crate::{preprocessing, state::SearchState};
use imghdr;
use crate::interface::{FileMetadata, FileUuid, ImageSize, Thumbnail, ThumbnailUuid};
use anyhow_tauri::{IntoTAResult, TAResult};

use rayon::prelude::*; // For par_iter

/// Search for image UUIDs which match a query string according to CLIP encodings.
/// Returns a list of UUIDs of images which match the query.
/// We return the UUIDs so that separate API calls can be made to fetch the metadata
/// and thumbnails; this allows us to display metadata and results more quickly
/// while the thumbnails are still loading/generating.
#[tauri::command]
pub async fn search_images<'a>(
        query_string: &str,
        number_neighbors: usize,
        ef_arg: usize,
        distance_threshold: f32,
        search_state: tauri::State<'_, SearchState<'a>>,
        clip_state: tauri::State<'_, ClipState>,
        tokenizer_state: tauri::State<'_, ClipTokenizerState>,
    ) -> TAResult<Vec<FileUuid>>
{
    if query_string.is_empty() {
        return Ok(vec![]);
    }

    let mut hnsw_search = search_state.0.lock().unwrap();
    let hnsw = &mut hnsw_search.hnsw;

    let tokenizer = &tokenizer_state.0.lock().unwrap().tokenizer;
    let query = preprocessing::tokenize(query_string, tokenizer);

    let clip = &mut clip_state.0.lock().unwrap().clip;

    let query_vector = clip.encode_text(query).into_ta_result()?;

    let query_vector_slice = query_vector.as_slice()
        .ok_or(anyhow::anyhow!("Error converting query vector to slice for query {:?}", query_string))
        .into_ta_result()?;

    info!("Searching for {:?}", query_string);
    let now = std::time::Instant::now();
    // Ensure ef_arg >= num_neighbors.
    let ef_arg = ef_arg.max(number_neighbors);
    let search_results = hnsw.search(query_vector_slice, number_neighbors, ef_arg, distance_threshold);
    let elapsed = now.elapsed();
    info!("Search took {:?} for {:?} neighbors with ef_ arg {:?} and distance threshold {:?}", elapsed, number_neighbors, ef_arg, distance_threshold);

    let search_results_uuids: Vec<FileUuid> = search_results.iter().map(|x| FileUuid(x.0.to_string())).collect();

    Ok(search_results_uuids)
}

/// Fetches the thumbnail filenames for a list of file IDs.
/// Returns a list of (thumbnail UUID, thumbnail filename) for the files.
/// The intention is the frontend can use the filename in an Image tag.
/// The UUID is returns for React to map elements if necessary on a unique ID.
/// Note that the UUID is *not* the file ID, but the UUID of the thumbnail.
/// The filename is local to APPDATA.
/// 
/// If an error occurs fetching the thumbnail for a given UUID, that UUID will be omitted from the results.
/// For this reason, use the file UUIDs in the Thumbnail objects, not the original set, when handling the results.
#[tauri::command]
pub async fn fetch_thumbnails(
        file_ids: Vec<FileUuid>,
        app_handle: tauri::AppHandle,
        pool_state: tauri::State<'_, ConnectionPoolState>
    ) -> TAResult<Vec<Thumbnail>>
{
    let results: Vec<Thumbnail> = file_ids.par_iter().map(
            |file_id| {
                let (thumbnail_uuid, thumbnail_filepath) = thumbnails::ensure_thumbnail_exists(
                    Uuid::parse_str(&file_id.0).context("Unable to parse file ID")?,
                    &app_handle,
                    &pool_state
                )?;
                Ok(Thumbnail
                {
                    uuid: ThumbnailUuid(thumbnail_uuid.to_string()),
                    file_uuid: FileUuid(file_id.0.to_string()),
                    path: thumbnail_filepath,
                })
            }
        )
        .filter_map(|x: anyhow::Result<Thumbnail> | x.ok())
        .collect();
    
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
    app_handle: tauri::AppHandle,
    pool_state: tauri::State<'_, ConnectionPoolState>
) -> TAResult<FileMetadata>
{
    let uuid = Uuid::parse_str(&file_id).into_ta_result()?;
    let mut connection = db::get_db_connection(&pool_state)?;

    let filepath = queries::get_filepath(uuid, &mut connection)?
        .ok_or(anyhow::anyhow!("File not found for UUID {:?}", uuid)).into_ta_result()?;
    
    // Get the thumbnail filepath - the thumbnail should typically exist by this point, but we ensure it here.
    let (_, thumbnail_filepath) = thumbnails::ensure_thumbnail_exists(
        Uuid::parse_str(&file_id).into_ta_result()?,
        &app_handle,
        &pool_state
    )?;

    let fs_metadata = std::fs::metadata(&filepath);
    let (date_created, date_modified) = match fs_metadata {
        Ok(metadata) => {
            let date_created = metadata.created().ok();
            let date_modified = metadata.modified().ok();
            
            // Convert each to a string using chrono
            let date_created = match date_created {
                Some(d) => Some(junk_drawer::system_time_to_string(d)),
                None => None
            };

            let date_modified = match date_modified {
                Some(d) => Some(junk_drawer::system_time_to_string(d)),
                None => None
            };

            (date_created, date_modified)
        },
        Err(_) => (None, None)
    };

    let image_type = imghdr::from_file(&filepath).into_ta_result()?;
    
    let dimensions = imagesize::size(&filepath);
    let dimensions = match dimensions {
        Ok(dim) => Some(ImageSize { width: dim.width as u32, height: dim.height as u32 }),
        Err(_) => None
    };
    
    let filename = filepath.file_name()
        .ok_or(anyhow::anyhow!("Unable to get filename from {:?}. Does it end with ..?", filepath))?
        .to_str().ok_or(anyhow::anyhow!("Unable to convert from OsStr to str"))?.to_string();
    
    let metadata = FileMetadata
    {
        file_id,
        filename,
        thumbnail_filepath,
        image_type,
        size: dimensions,
        date_created,
        date_modified,
    };

    Ok(metadata)
}

#[tauri::command]
pub async fn add_watched_directory(
    directory_path: String,
    recursive: bool,
    watcher_state: tauri::State<'_, FsWatcherState>,
    clip_state: tauri::State<'_, ClipState>,
    app_handle: tauri::AppHandle,
) -> TAResult<()>
{
    // TODO Ensure that if the directory is already added, we don't add it again.
    //      Consider recursive as well?

    let directory_path = std::path::Path::new(&directory_path);

    if !directory_path.is_dir() {
        return Err(anyhow::anyhow!("Directory {:?} does not exist, or is not a directory", directory_path).into());
    }

    tauri::scope::FsScope::allow_directory(&app_handle.fs_scope(), "D:\\refrover_photos", true).into_ta_result()?;

    // TODO consider adding watcher last, at least after we've inserted the base dir.
    //   otherwise it might start trying to add files to a base dir that doesn't exist, or query for
    //   one that doesn't exist.
    // TODO also, we probably need an index on the base dir path? So we can search for it.
    //   OR we can somehow inform the FsEventHandler of some mapping of watched dirs to their IDs in some way
    let watcher_debouncer = &mut watcher_state.0.lock().unwrap().watcher;
    let recursive_mode: RecursiveMode = if recursive { RecursiveMode::Recursive } else { RecursiveMode::NonRecursive };
    watcher_debouncer.watcher().watch(directory_path, recursive_mode).into_ta_result()?;

    println!("Added directory {:?}", directory_path);

    // TODO We could test our FsEventHandler logic now. Probably do this first.
    //      Just spoof adding a new watched directory (hardcode) and check out if logs/prints are working like I would expect
    //      as we add/remove files/dirs from the watched directory.
    //      Once that's working we can pretty confidently move forward.

    // TODO Add the base directory to the DB.
    // TODO If recursive, add all subdirectories to the DB.
    // TODO And for each added directory... add its files in the directory to the DB.
    // TODO And kick off encoding for those files, maybe in a new background thread?
    //      clip_state is already in a Mutex, so we can lock it while we process.
    //      As long as we don't block the main thread, we should be fine.
    //      TODO - We might need it to be an Arc<Mutex> though, not just a Mutex.
    Ok(())
}