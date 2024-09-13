use anyhow::Context;
use log::info;
use notify_debouncer_full::notify::{RecursiveMode, Watcher};
use tauri::Manager;
use uuid::Uuid;

use crate::models::NewFileOwned;
use crate::state::{ClipState, ClipTokenizerState, ConnectionPoolState, FsWatcherState};
use crate::{db, junk_drawer, queries, thumbnails};
use crate::{preprocessing, state::SearchState};
use imghdr;
use crate::interface::{FileMetadata, FileUuid, ImageSize, Payload, Thumbnail, ThumbnailUuid};
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

/// Note that this does not handle recursive watching of subdirectories.
/// If the user wants to watch subdirectories, the front-end should construct that set of
/// directories and invoke this command for each one.
#[tauri::command]
pub async fn add_watched_directory(
    directory_path: String,
    watcher_state: tauri::State<'_, FsWatcherState>,
    clip_state: tauri::State<'_, ClipState>,
    pool_state: tauri::State<'_, ConnectionPoolState>,
    app_handle: tauri::AppHandle,
) -> TAResult<()>
{
    let directory_path = std::path::Path::new(&directory_path);
    
    if !directory_path.is_dir() {
        return Err(anyhow::anyhow!("Directory {:?} does not exist, or is not a directory", directory_path).into());
    }

    let mut connection = pool_state.get_connection().into_ta_result()?;
    if queries::base_dir_exists(
            directory_path.to_str().ok_or(anyhow::anyhow!("Unable to convert directory path to string"))?,
            &mut connection
        ).into_ta_result()? {
        return Err(anyhow::anyhow!("Attempted to add Directory {:?}, which is already in the database.", directory_path).into());
    }

    tauri::scope::FsScope::allow_directory(&app_handle.fs_scope(), directory_path, true).into_ta_result()?;

    // Start watching the directory for new files
    let watcher_debouncer = &mut watcher_state.0.lock().unwrap().watcher;
    watcher_debouncer.watcher().watch(directory_path, RecursiveMode::NonRecursive).into_ta_result()?;


    let base_dir_uuid = queries::insert_base_directory(
        &directory_path.to_str().ok_or(anyhow::anyhow!("Unable to convert directory path to string")).into_ta_result()?,
        &mut connection
    ).into_ta_result()?;

    // Add its files in the directory to the DB.
    let files = std::fs::read_dir(directory_path).into_ta_result()?;
    // Filter to only files. We will ignore any path that is not a file (directories, symlinks, etc.)
    let files = files.filter_map(|x| x.ok()).filter(|x| x.file_type().into_ta_result().ok().map(|y| y.is_file()).unwrap_or(false));

    let mut to_encode = Vec::new();
    let new_file_rows: Vec<NewFileOwned> = files.map(|x| -> anyhow::Result<NewFileOwned> {
        let file_path = x.path();
        let file_path_str = file_path.to_str().ok_or(anyhow::anyhow!("Unable to convert file path to string")).into_ta_result()?;
        
        let file_uuid = Uuid::new_v4();

        to_encode.push((file_uuid, x.path().to_path_buf()));
        
        Ok(NewFileOwned
        {
            id: file_uuid.to_string(),
            base_directory_id: base_dir_uuid.to_string(),
            relative_path: file_path_str.to_string(),
        })
    }).collect::<Result<Vec<NewFileOwned>, anyhow::Error>>()?;

    queries::insert_files_rows(&new_file_rows, &mut connection).into_ta_result()?;

    // Encode images and store results in the DB.
    // Note this is relatively long-running; this command is async, so it will not block the main thread.
    // But it's a good idea to keep this as the last step in the command so other tables are updated quickly.
    {
        let clip_state = clip_state.0.lock().unwrap();
        let clip = &clip_state.clip;
        clip.encode_image_files(&to_encode, &mut connection)?;
    }

    Ok(())
}

// TODO delete_watched_directory
//      Remove it and contained files from the database, and stop watching it.
//      let watcher_debouncer = &mut watcher_state.0.lock().unwrap().watcher;
//      watcher_debouncer.watcher().unwatch(path).into_ta_result()?;
// TODO *add this to commands in main*.

#[tauri::command]
pub async fn delete_watched_directory(
    directory_path: String,
    watcher_state: tauri::State<'_, FsWatcherState>,
    clip_state: tauri::State<'_, ClipState>,
    pool_state: tauri::State<'_, ConnectionPoolState>,
    app_handle: tauri::AppHandle,
) -> TAResult<()>
{
    // I'm considering having some system where we track deleted dirs/files/encodings,
    // so if they get added back easily? But that adds complexity to the simple case of just
    // straightforwardly adding/deleting things. Just go simple for now.
    
    let directory_path = std::path::Path::new(&directory_path);

    let watcher_debouncer = &mut watcher_state.0.lock().unwrap().watcher;
    watcher_debouncer.watcher().unwatch(directory_path).into_ta_result()?;

    // TODO Get the contained files (from the DB)
    
    // TODO Look at what we can share with notify_handlers...
    //      After all we're just removing a set of files. Basically the same work.

    // TODO Delete the edges of contained files
    
    // TODO Delete failed encodings
    
    // TODO Delete the encodings of contained files

    // TODO Delete thumbnails of contained files
    //      In the DB and on the filesystem

    // TODO Delete the dir from the DB

    Ok(())
}