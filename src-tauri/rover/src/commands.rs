use log::info;
use tauri::Manager;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::clip::Clip;
use crate::error::{Error, TaskError};
use crate::events::{Event, TaskEndPayload, TaskStatusPayload};
use crate::models::NewFile;
use crate::notify_handlers::{FsEventHandler, FS_WATCHER_DEBOUNCER_DURATION};
use crate::state::{ClipState, ClipTokenizerState, ConnectionPoolState, FsWatcherState, SearchState};
use crate::uuid::UUID;
use crate::{db, junk_drawer, queries, thumbnails};
use crate::preprocessing;
use imghdr;
use crate::interface::{FileMetadata, ImageSize, Thumbnail};
use anyhow_tauri::{IntoTAResult, TAResult};

use rayon::prelude::*;

// TODO Other commands should probably return a TaskError as well. WE can remove the TAResult dependency, I suppose.

/// Search for image UUIDs which match a query string according to CLIP encodings.
/// Returns a list of UUIDs of images which match the query.
/// We return the UUIDs so that separate API calls can be made to fetch the metadata
/// and thumbnails; this allows us to display metadata and results more quickly
/// while the thumbnails are still loading/generating.
#[tauri::command]
pub async fn search_images<'a>(
        path_prefixes: Vec<String>,
        query_string: &str,
        number_neighbors: usize,
        ef_arg: usize,
        distance_threshold: f32,
        search_state: tauri::State<'_, SearchState<'a>>,
        clip_state: tauri::State<'_, ClipState>,
        tokenizer_state: tauri::State<'_, ClipTokenizerState>,
        pool_state: tauri::State<'_, ConnectionPoolState>,
    ) -> TAResult<Vec<UUID>>
{
    // Ensure that each entry of path_prefixes has a trailing backslash,
    // since they should be directories.
    let path_prefixes: Vec<String> = path_prefixes.into_iter().map(|x| {
        if x.ends_with(std::path::MAIN_SEPARATOR) {
            x
        } else {
            x + std::path::MAIN_SEPARATOR.to_string().as_str()
        }
    }).collect();

    match (path_prefixes.is_empty(), query_string.is_empty()) {
        (true, true) => {
            // No search criteria provided; return an empty list.
            info!("No search criteria provided; returning an empty list.");
            Ok(Vec::new())
        },
        (false, false) => {
            info!("Searching for \"{:?}\" with path prefixes {:?}", query_string, path_prefixes);
            // We have both a natural language query and a filter for specific folders.
            // We want to do an HNSW search, and filter the resulting UUIDs to only those in the specified folders.
            let uuids = hnsw_search(query_string, number_neighbors, ef_arg, distance_threshold, search_state, clip_state, tokenizer_state)?;
            let mut connection = pool_state.get_connection().into_ta_result()?;
            let file_ids_matching_prefix = queries::get_files_with_prefix(&path_prefixes, &mut connection)?;
            let file_ids_matching_prefix: Vec<UUID> = file_ids_matching_prefix.into_iter().map(|x| x.id).collect();
            // Filter the UUIDs to only those in the specified folders.
            // First, construct a set from the file_ids_matching_prefix, for O(1) lookup.
            let file_ids_matching_prefix_set: std::collections::HashSet<UUID> = file_ids_matching_prefix.iter().cloned().collect();
            let out: Vec<UUID> = uuids.into_iter().filter(|x| file_ids_matching_prefix_set.contains(x)).collect();
            info!("Found {:?} results", out.len());
            Ok(out)
        },
        (true, false) => {
            info!("Searching for \"{:?}\" with no path prefix filter", query_string);
            // We have a natural language query but no filter for specific folders.
            // We want to do an HNSW search across all folders.
            let uuids = hnsw_search(query_string, number_neighbors, ef_arg, distance_threshold, search_state, clip_state, tokenizer_state)?;
            info!("Found {:?} results", uuids.len());
            Ok(uuids)
        },
        (false, true) => {
            info!("Searching for no query with path prefixes {:?}", path_prefixes);
            // We have a set of acceptable prefixes but no natural language query.
            // Simply return all UUIDs with any of the specified prefixes.
            let mut connection = pool_state.get_connection().into_ta_result()?;
            let file_ids_matching_prefix = queries::get_files_with_prefix(&path_prefixes, &mut connection)?;
            let file_ids_matching_prefix: Vec<UUID> = file_ids_matching_prefix.into_iter().map(|x| x.id).collect();
            info!("Found {:?} files matching prefix", file_ids_matching_prefix.len());
            Ok(file_ids_matching_prefix)
        }
    }
}

fn hnsw_search<'a>(
    query_string: &str,
    number_neighbors: usize,
    ef_arg: usize,
    distance_threshold: f32,
    search_state: tauri::State<'_, SearchState<'a>>,
    clip_state: tauri::State<'_, ClipState>,
    tokenizer_state: tauri::State<'_, ClipTokenizerState>,
) -> anyhow::Result<Vec<UUID>>
{
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
) -> Result<(), TaskError>
{
    let task_uuid = Uuid::new_v4().to_string();
    app_handle.emit_all(Event::TaskStatus.event_name(), TaskStatusPayload
    {
        task_uuid: task_uuid.clone(),
        status: format!("Adding watched directory: {}...", directory),
    }).map_err(|e| TaskError {
        task_uuid: task_uuid.clone(),
        error: Error::Tauri(e)
    })?;

    let directory_path = std::path::Path::new(&directory);
    
    if !directory_path.is_dir() {
        return Err(TaskError {
            task_uuid,
            error: Error::NotADirectory
        });
    }

    let mut connection = pool_state.get_connection().map_err(|e| TaskError {
        task_uuid: task_uuid.clone(),
        error: Error::Anyhow(e)
    })?;
    if queries::watched_dir_exists(
            directory_path.to_str().ok_or(anyhow::anyhow!("Unable to convert directory path to string")).map_err(|e| TaskError {
                task_uuid: task_uuid.clone(),
                error: Error::Anyhow(e)
            })?,
            &mut connection
        ).map_err(|e| TaskError {
            task_uuid: task_uuid.clone(),
            error: Error::Anyhow(e)
        })? {
        return Err(TaskError {
            task_uuid,
            error: Error::DirectoryAlreadyExistsInDb
        });
    }

    let recursive = true;
    tauri::scope::FsScope::allow_directory(&app_handle.fs_scope(), directory_path, recursive).map_err(|e| TaskError {
        task_uuid: task_uuid.clone(),
        error: Error::Tauri(e)
    })?;

    // TODO Should we wrap this in a transaction so we can revert it if we e.g. fail to create a watcher?
    // https://docs.diesel.rs/2.0.x/diesel/connection/trait.Connection.html#method.transaction
    let watched_dir_uuid = queries::insert_watched_directory(
        &directory,
        &mut connection
    ).map_err(|e| TaskError {
        task_uuid: task_uuid.clone(),
        error: Error::Anyhow(e)
    })?;

    let fs_event_handler = FsEventHandler {
        app_handle: app_handle.clone(),
        watch_directory_id: watched_dir_uuid,
        watch_directory_path: directory_path.to_path_buf(),
    };
    let watcher = notify_debouncer_full::new_debouncer(
        FS_WATCHER_DEBOUNCER_DURATION,
        None,
        fs_event_handler).map_err(|e| TaskError {
            task_uuid: task_uuid.clone(),
            error: Error::Notify(e)
        })?;

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

    // Recursively get all entries in the directory, skipping errors
    let mut file_ids: Vec<UUID> = Vec::new();
    let mut new_files: Vec<NewFile> = Vec::new();
    for entry in WalkDir::new(directory_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|x| x.ok())
    {
        let file_path = entry.path();
        if file_path.is_file() {
            let file_uuid = Uuid::new_v4().into();
            let file_path_str = file_path.to_str().ok_or(anyhow::anyhow!("Unable to convert path to string")).map_err(|e| TaskError {
                task_uuid: task_uuid.clone(),
                error: Error::Anyhow(e)
            })?;
            file_ids.push(file_uuid);
            let new_file = NewFile
            {
                id: file_uuid.into(),
                filepath: file_path_str.to_string(),
                watched_directory_id: Some(watched_dir_uuid.into()),
            };
            new_files.push(new_file);
        }
    }
    queries::insert_files_rows(&new_files, &mut connection).map_err(|e| TaskError {
        task_uuid: task_uuid.clone(),
        error: Error::Anyhow(e)
    })?;

    // Encode images and store results in the DB.
    // Note this is relatively long-running; this command is async, so it will not block the main thread.
    // But it's a good idea to keep this as the last step in the command so other tables are updated quickly.
    Clip::encode_files_and_add_to_search(&file_ids, &mut connection, clip_state, search_state).map_err(|e| TaskError {
        task_uuid: task_uuid.clone(),
        error: Error::Anyhow(e)
    })?;

    app_handle.emit_all(Event::TaskEnd.event_name(), TaskEndPayload
    {
        task_uuid: task_uuid.clone(),
    }).map_err(|e| TaskError {
        task_uuid,
        error: Error::Tauri(e)
    })?;

    Ok(())
}

#[tauri::command]
pub fn get_watched_directories(
    pool_state: tauri::State<'_, ConnectionPoolState>
) -> TAResult<Vec<String>>
{
    let mut connection = pool_state.get_connection().into_ta_result()?;
    let watched_dirs = queries::get_watched_directories(&mut connection).into_ta_result()?;
    let watched_dirs = watched_dirs.into_iter().map(|x| x.filepath).collect();
    Ok(watched_dirs)
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