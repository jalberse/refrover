#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;

use app::ann;
use app::ann::HnswSearch;
use app::clip::Clip;
use app::db;
use app::error::Error;
use app::error::TaskError;
use app::events::Event;
use app::events::TaskEndPayload;
use app::events::TaskStatusPayload;
use app::models::NewFile;
use app::notify_handlers::FsEventHandler;
use app::notify_handlers::FS_WATCHER_DEBOUNCER_DURATION;
use app::queries;
use app::state::ClipState;
use app::state::ClipTokenizerState;
use app::state::ConnectionPoolState;
use app::state::InnerClipState;
use app::state::InnerClipTokenizerState;
use app::state::InnerConnectionPoolState;
use app::state::InnerSearchState;
use app::state::FsInnerWatcherState;
use app::state::SearchState;
use app::state::FsWatcherState;
use app::uuid::UUID;
use log::error;
use log::info;
use log::LevelFilter;
use notify_debouncer_full::notify::RecursiveMode;
use notify_debouncer_full::notify::Watcher;
use tauri::Manager;
use tauri_plugin_log::LogTarget;
use app::models::WatchedDirectory;
use uuid::Uuid;
use walkdir::WalkDir;

#[cfg(debug_assertions)]
// Note that Trace level causes massive slowdowns due to I/O in HNSW
// (that crate could probably be excluded from logging, but it's not a big deal for now).
const LOG_LEVEL: LevelFilter = LevelFilter::Debug;
#[cfg(not(debug_assertions))]
const LOG_LEVEL: LevelFilter = LevelFilter::Warn;

fn main() -> anyhow::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_log::Builder::default().targets([
                LogTarget::LogDir,
                LogTarget::Stdout,
                LogTarget::Webview,
            ])
            .level(LOG_LEVEL)
            .build())
        .manage(
            ClipState(
                    Mutex::new(InnerClipState { clip: Clip::new()? })
                )
            )
        .manage(
            ClipTokenizerState(
                    Mutex::new(InnerClipTokenizerState { tokenizer: instant_clip_tokenizer::Tokenizer::new() })
                )
        )
        .manage(
            SearchState(
                    Mutex::new(InnerSearchState { hnsw: HnswSearch::new() })
                )
            )
        .setup(|app| {

            app.manage(
                ConnectionPoolState(
                    Mutex::new(InnerConnectionPoolState { pool: db::get_connection_pool(&app.app_handle())? })
                )
            );

            // Use e.g. `pnpm tauri dev --release -- -- -p` to pass arguments.
            // Multiple `--` are needed to pass arguments to the binary.
            let populate_dummy_data = match app.get_cli_matches() {
                Ok(matches) => 
                {
                    matches.args["populate-dummy-data"].value.as_bool().ok_or(anyhow::anyhow!("Failed to get value for populate-dummy-data"))?
                },
                Err(_) => 
                {
                    false
                },
            };

            let pool_state = app.state::<ConnectionPoolState>();

            let init_db_result = db::init(&pool_state, populate_dummy_data);
            match init_db_result {
                Ok(_) => {},
                Err(e) => {
                    error!("Error initializing DB: {:?}", e);
                    return Err(e.into());
                }
            }

            let watcher_state = FsWatcherState(Mutex::new(FsInnerWatcherState { watchers: std::collections::HashMap::new() }));
            app.manage(watcher_state);

            let mut connection = pool_state.get_connection()?;
            let watched_directories = queries::get_watched_directories(&mut connection)?;

            // Start watching watched directoreis.
            for watched_directory_row in &watched_directories {
                // These should generally already by allowed via the persisted scope, but ensure it here.
                let watched_path = &watched_directory_row.filepath;
                let watched_uuid = watched_directory_row.id;
                let recursive = true;
                tauri::scope::FsScope::allow_directory(&app.fs_scope(), &watched_path, recursive)?;

                let fs_event_handler = FsEventHandler {
                    app_handle: app.app_handle().clone(),
                    watch_directory_id: watched_uuid.clone(),
                    watch_directory_path: PathBuf::from(watched_path.clone()),
                };
                let mut debouncer = notify_debouncer_full::new_debouncer(
                    FS_WATCHER_DEBOUNCER_DURATION,
                    None,
                    fs_event_handler)?;
                debouncer.watcher().watch(Path::new(watched_path), RecursiveMode::Recursive)?;
                
                let watcher_state = app.state::<FsWatcherState>();
                watcher_state.0.lock().unwrap().watchers.insert(watched_path.clone(), debouncer);
            }

            // TODO And probably initialize a default watched directory if it doesn't exist?

            // We rebuild every time the app launches;
            // it is fast enough, and it handles the fact that
            // we can't remove elements from the HNSW index.
            // TODO Consider a more sophisticated approach if this becomes a bottleneck?
            // We could be dumping the index and loading it, but then we don't notice deleted files.
            // We could periodically rebuild the index and overwrite the old index (keeping backups, likely).
            // That could be on some count of removed files which can trigger the rebuild.
            // We only need to rebuild due to the removal of files, since we can add elements whenever we want.
            // Say every (configurable) 100 files removed, we rebuild (check after batches of removed files, though).
            // TODO Speaking of, maybe we should have a table that stores removed files and their UUIDs,
            //      in case we ever need to recover information
            //      Or rather, a "deleted" flag in most tables, so we can mark it as deleted and recover if needed.
            //      Would need to modify queries to check for not-deleted, though.
            info!("Populating HNSW index...");
            let now = std::time::Instant::now();
            ann::populate_hnsw(app)?;
            let elapsed = now.elapsed();
            info!("HNSW rebuild took {:?}", elapsed);
            info!("HNSW EF_CONSTRUCTION: {:?}", ann::DEFAULT_EF_CONSTRUCTION);
            info!("HNSW_MAX_ELEMS: {:?}", ann::DEFAULT_MAX_ELEMS);

            // Handle potentially long-running work that we don't want to block the application opening.
            let app_handle = app.app_handle().clone();
            thread::spawn(move || -> anyhow::Result<()> {
                let result = initial_scan(&watched_directories, app_handle.clone());
                if let Err(e) = result {
                    error!("Error during initial scan: {:?}", e);
                    // Also emit a TaskEnd event to clear the task status
                    let emit_result = app_handle.emit_all(Event::TaskEnd.event_name(), TaskEndPayload {
                        task_uuid: e.task_uuid,
                    });
                    if emit_result.is_err()
                    {
                        error!("Failed to emit TaskEnd event: {:?}", emit_result.err());
                    }
                }
                Ok(())
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::search_images,
            app::commands::fetch_thumbnails,
            app::commands::fetch_metadata,
            app::commands::add_watched_directory,
            app::commands::delete_watched_directory,
            app::commands::get_watched_directories,
            ])
        .run(tauri::generate_context!())?;

    Ok(())
}

/// Perform the initial scan of the filesystem, updating the database and HNSW index as needed.
/// This is something we want to do on startup, but which takes too long to accomplish in setup()
/// on the main thread. So we launch this function in a separate thread, allowing the frontend
/// to launch and the user to interact with the app while the initial scan is performed.
fn initial_scan(
    watched_directories: &[WatchedDirectory],
    app_handle: tauri::AppHandle,
) -> Result<(), TaskError> {
    // TODO This TaskStatus may fire before the frontend is ready to receive it.
    //      I think if we sprinkle more in (which we should, to show actual progress)
    //      then the frontend will eventually receive one. I'll need to test and see how it is.
    //      An alternative approach might be to not do the initial_scan() in setup(), but have a command
    //      that we expect the frontend to call (and so we know the frontend is ready when it starts).
    //      But need to have that only run once. Maybe on mount of the application? Not sure the best
    //      way to do that React-side.

    // TODO Consider - if a file is in the database but NOT in the filesystem, should we remove it from the database?
    //      Maybe? Better if we have "mark as deleted" instead with a column in the files table, so it can be recovered (possibly by the user?).
    //      But as a baseline, we'd have like "fetch this file" "oh it doesn't exist" errors, and clog up results with non-existent files.

    let task_uuid: String = Uuid::new_v4().into();

    let emit_result = app_handle.emit_all(Event::TaskStatus.event_name(),
        TaskStatusPayload {
            task_uuid: task_uuid.clone(),
            status: "Initialization: Scanning watched directories...".to_owned(),
        }
    );
    if emit_result.is_err() {
        error!("Failed to emit TaskStatus event: {:?}", emit_result.err());
    }

    // Scan the files in the watched directories, and add any new files to the database.
    let mut connection = app_handle.state::<ConnectionPoolState>().get_connection()
        .map_err(|e| TaskError { task_uuid: task_uuid.clone(), error: Error::Anyhow(e) })?;
    let mut file_ids: Vec<UUID> = Vec::new();
    let mut new_files: Vec<NewFile> = Vec::new();
    for watched_directory in watched_directories
    {
        for entry in WalkDir::new(&watched_directory.filepath)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();
            if file_path.is_file()
            {
                if !queries::file_exists(
                    file_path.to_str().ok_or(TaskError { task_uuid: task_uuid.clone(), error: Error::PathBufToString })?,
                    &mut connection).map_err(|e| TaskError { task_uuid: task_uuid.clone(), error: Error::Anyhow(e) })?
                {
                    let new_file = NewFile {
                        id: uuid::Uuid::new_v4().into(),
                        filepath: file_path.to_string_lossy().to_string(),
                        watched_directory_id: Some(watched_directory.id),
                    };
                    file_ids.push(new_file.id.clone());
                    new_files.push(new_file);
                }
            }
        }
    }

    if !new_files.is_empty()
    {
        info!("Inserting {} new files into the database...", new_files.len());
        queries::insert_files_rows(&new_files, &mut connection).map_err(|e| TaskError { task_uuid: task_uuid.clone(), error: Error::Anyhow(e) })?;
        let clip_state = app_handle.state::<ClipState>();

        info!("Adding to HNSW index...");
        let search_state = app_handle.state::<SearchState>();
        Clip::encode_files_and_add_to_search(&file_ids, &mut connection, clip_state, search_state)
            .map_err(|e| TaskError { task_uuid: task_uuid.clone(), error: Error::Anyhow(e) })?;
        info!("Added to HNSW index.");
    }

    let emit_result = app_handle.emit_all(Event::TaskEnd.event_name(),
        TaskEndPayload {
            task_uuid: task_uuid.clone(),
        }
    );
    if emit_result.is_err() {
        error!("Failed to emit TaskEnd event: {:?}", emit_result.err());
    }

    Ok(())
}