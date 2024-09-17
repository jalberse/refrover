use log::info;
use tauri::Manager;
use uuid::Uuid;

use crate::clip::Clip;
use crate::models::NewFile;
use crate::notify_handlers::{FsEventHandler, FS_WATCHER_DEBOUNCER_DURATION};
use crate::state::{ClipState, ClipTokenizerState, ConnectionPoolState, FsWatcherState, SearchState};
use crate::uuid::UUID;
use crate::{db, junk_drawer, queries, thumbnails};
use crate::preprocessing;
use imghdr;
use crate::interface::{FileMetadata, ImageSize, Thumbnail};
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
    ) -> TAResult<Vec<UUID>>
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
    info!("Found {:?} results", search_results.len());

    let search_results_uuids: Vec<UUID> = search_results.iter().map(|x| x.0).collect();

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
        file_ids: Vec<UUID>,
        app_handle: tauri::AppHandle,
        pool_state: tauri::State<'_, ConnectionPoolState>
    ) -> TAResult<Vec<Thumbnail>>
{
    // TODO Actually, Commands just need to be serializable. We could use UUID instead of the FileUuid wrapper, I think?
    let results: Vec<Thumbnail> = file_ids.par_iter().map(
            |file_id| {
                let (thumbnail_uuid, thumbnail_filepath) = thumbnails::ensure_thumbnail_exists(
                    *file_id,
                    &app_handle,
                    &pool_state
                )?;
                Ok(Thumbnail
                {
                    uuid: thumbnail_uuid,
                    file_uuid: *file_id,
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
    file_id: UUID,
    app_handle: tauri::AppHandle,
    pool_state: tauri::State<'_, ConnectionPoolState>
) -> TAResult<FileMetadata>
{
    let mut connection = db::get_db_connection(&pool_state)?;

    let filepath = queries::get_filepaths(&[file_id], &mut connection)?;
    if filepath.is_empty() {
        return Err(anyhow::anyhow!("File not found for file_id: {}", file_id).into());
    }
    let filepath = &filepath[0].clone().1;
    
    // Get the thumbnail filepath - the thumbnail should typically exist by this point, but we ensure it here.
    let (_, thumbnail_filepath) = thumbnails::ensure_thumbnail_exists(
        file_id,
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
    directory: String,
    watcher_state: tauri::State<'_, FsWatcherState>,
    clip_state: tauri::State<'_, ClipState>,
    pool_state: tauri::State<'_, ConnectionPoolState>,
    search_state: tauri::State<'_, SearchState<'_>>,
    app_handle: tauri::AppHandle,
) -> TAResult<()>
{
    // TODO Search doesn't seem to be invoked while this is running. Why? Shouldn't they launch as separate async tasks/threads?

    // TODO - consider - we don't want to allow a new watched dir that is a subdirectory of an existing watched dir, OR a parent of an existing watched dir.
    //      That (as well as checking if it already exists and perhaps exists on the filesystem) should be in some validate_new_watched_dir command or something,
    //      which would be non-async.

    // TODO Consider if we want to make watched directories recursive.
    //      I strongly suspect we do - we'd want to drop folders into a top level scheme and have access.
    // If we do so, then when the user wants to add a new watched dir to their list, we should stop
    //      them if it is already watched via an ancestor directory and disallow it.
    //      That makes the user story pretty simple I think? Just add all the top level dirs they want to watched.
    //      They can drag and drop files and folders into it.
    //      Just iterate over the existing watched dirs and check starts_with(), I think?
    // And we'd need to handle that recursive stuff in the remove_watched_directory command as well.

    // TODO If we do make them recursive, consider making watched directories and base directories separate concepts in the DB.
    //      A based directory may be watched recursively, but it's not the root that's passed to the watcher.

    // TODO Consider that base directories save ~zero space. Windows allows 260 bytes for file paths, e.g.
    //      Even with 10k images, that's just 2.6 MB. Removing redundant data (shared prefixes) is just *not* worth the hassle.
    //      Before, it was vaguely worth it because I conceptualized base directories as watched directories, partially,
    //      so saying "grab everything in this directory" or "delete all of these" made a bit more sense.
    //      But with recursively watched directories (which I think we want), then the "relative path" includes the path up to some prefix,
    //      including potentially a lot of intermediary folders.
    //      I think things just get conceptually simpler if files stored the absolute filepath.

    // TODO Okay, I'm cutting out the base_directories table. change this to add the watched directory.
    // TODO We'll make the switch keeping the watched dirs flat, and then move to make them recursive.
    // TODO And once we do that, we'll want the frontend to check that the directory isn't already watched by an ancestor directory.
    //      ... and we'll want to do that here, returning an error if so.
    //      I want the frontend to check so that we can check that before adding it to the displayed list of watched dirs...
    //      Maybe we need a separate, non-async command for that which just checks if a directory is already watched (contained in another watched dir).
    //      Non-async commands are fine as long as we're just quickly checking the DB or something.

    let directory_path = std::path::Path::new(&directory);
    
    if !directory_path.is_dir() {
        return Err(anyhow::anyhow!("Directory {:?} does not exist, is not a directory, or there are permissions/access errors.", directory_path).into());
    }

    let mut connection = pool_state.get_connection().into_ta_result()?;
    if queries::watched_dir_exists(
            directory_path.to_str().ok_or(anyhow::anyhow!("Unable to convert directory path to string"))?,
            &mut connection
        ).into_ta_result()? {
        return Err(anyhow::anyhow!("Attempted to add Directory {:?}, which is already in the database.", directory_path).into());
    }

    // I expect this directory should already be allowed via a call to open(): https://tauri.app/v1/api/js/dialog/#open
    // But we'll ensure it's allowed here, too.
    tauri::scope::FsScope::allow_directory(&app_handle.fs_scope(), directory_path, true).into_ta_result()?;

    // TODO Should we wrap this in a transaction so we can revert it if we e.g. fail to create a watcher?
    // https://docs.diesel.rs/2.0.x/diesel/connection/trait.Connection.html#method.transaction
    let watched_dir_uuid = queries::insert_watched_directory(
        &directory,
        &mut connection
    ).into_ta_result()?;

    let fs_event_handler = FsEventHandler {
        app_handle: app_handle.clone(),
        watch_directory_id: watched_dir_uuid,
        watch_directory_path: directory_path.to_path_buf(),
    };
    let watcher = notify_debouncer_full::new_debouncer(
        FS_WATCHER_DEBOUNCER_DURATION,
        None,
        fs_event_handler).into_ta_result()?;

    // Add to the map of watchers in the app state.
    // Note that an alternative architecture would be to have *one* watcher and just have it watch
    // multiple directories; however, we want to be able to track *which* watcher emitted an event,
    // which isn't possible by default. So we create a watcher for each directory, storing the watched
    // directory ID and path in the event handler.
    // This is a recommended pattern from `notify` https://docs.rs/notify/latest/notify/#with-different-configurations
    {
        let mut watcher_state = watcher_state.0.lock().unwrap();
        watcher_state.watchers.insert(directory.clone(), watcher);
    }

    // TODO We're switching to recursively watched directories, so we'll need to add sub-directory files, too.
    
    // Add its files in the directory to the DB.
    let files = std::fs::read_dir(directory_path).into_ta_result()?;
    // Filter to only files. We will ignore any path that is not a file (directories, symlinks, etc.)
    let files = files.filter_map(|x| x.ok()).filter(|x| x.file_type().into_ta_result().ok().map(|y| y.is_file()).unwrap_or(false));
    
    let mut file_ids: Vec<UUID> = Vec::with_capacity(files.size_hint().0);
    let new_file_rows: Vec<NewFile> = files.map(|x| -> anyhow::Result<NewFile> {
        let file_uuid = Uuid::new_v4().into();
        let binding = x.path().clone();
        let file_path_str = binding.to_str().ok_or(anyhow::anyhow!("Unable to convert path to string"))?;
        
        file_ids.push(file_uuid);
        
        Ok(NewFile
            {
                id: file_uuid.into(),
                filepath: file_path_str.to_string(),
                watched_directory_id: Some(watched_dir_uuid.into()),
            })
        }).collect::<Result<Vec<NewFile>, anyhow::Error>>()?;
        
    queries::insert_files_rows(&new_file_rows, &mut connection).into_ta_result()?;

    // Encode images and store results in the DB.
    // Note this is relatively long-running; this command is async, so it will not block the main thread.
    // But it's a good idea to keep this as the last step in the command so other tables are updated quickly.
    Clip::encode_files_and_add_to_search(&file_ids, &mut connection, clip_state, search_state)?;

    Ok(())
}

/// Deletes a watched directory and all its contents from the database.
#[tauri::command]
pub async fn delete_watched_directory(
    directory: String,
    watcher_state: tauri::State<'_, FsWatcherState>,
    pool_state: tauri::State<'_, ConnectionPoolState>,
    app_handle: tauri::AppHandle,
) -> TAResult<()>
{
    // I'm considering having some system where we track deleted dirs/files/encodings,
    // so if they get added back easily? But that adds complexity to the simple case of just
    // straightforwardly adding/deleting things. Just go simple for now.

    let directory_path = std::path::Path::new(&directory);

    {
        let watchers = &mut watcher_state.0.lock().unwrap();
        watchers.watchers.remove(&directory);
    }

    let mut connection = pool_state.get_connection().into_ta_result()?;
    let watched_dir_uuid = queries::get_watched_directory_from_path(&directory, &mut connection).into_ta_result()?;

    match watched_dir_uuid {
        Some(uuid) => {
            queries::delete_watched_directories_cascade(&[uuid], &mut connection, app_handle)?;
            Ok(())
        },
        None => {
            info!("Directory {:?} not found in the database. Nothing to do.", directory_path);
            Ok(()) // Nothing to do
        }
    }
}

// TODO We want non-blocking (non-async) commands that just check if a directory is watched already or not (and maybe check if it exists on DB or something?)
//      We'll call that when we want to add/remove dirs from our list.
//      Then we'll call the async commands to actually add/remove them, which would take longer.
//   Possibly we want to mark them as deleted? And then we have a command that says "okay take a dir marked for deletion and go do that".