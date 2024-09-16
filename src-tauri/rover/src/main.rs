#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Mutex;

use app::ann;
use app::ann::HnswSearch;
use app::clip::Clip;
use app::db;
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
use log::error;
use log::info;
use log::LevelFilter;
use tauri::Manager;
use tauri_plugin_log::LogTarget;

#[cfg(debug_assertions)]
// Note that Trace level causes massive slowdowns due to I/O in HNSW.
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

            // TODO For files that were added while the program *wasn't* running, we need to
            // scan them and add them to the HNSW index (and any other relevant tables).
            // Thumbnails we can ignore for now, generating them on the fly is OK.
            // We'll probably share a lot of code with that in the FS watcher.
            // In fact, we might want to do it in an initial scan callback...? Maybe not.

            // TODO We need a table that stores the watched directories.
            //      We need to populate the front-end with them,
            //      and create watchers for them.
            //      Right now it's hardcoded to D:\refrover_photos that gets added on click.

            // TODO And probably initialize a default watche directory if it doesn't exist?
            let watcher_state = FsWatcherState(Mutex::new(FsInnerWatcherState { watchers: std::collections::HashMap::new() }));
            app.manage(watcher_state);

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
            //      in case we ever need to recover information.
            info!("Populating HNSW index...");
            let now = std::time::Instant::now();
            ann::populate_hnsw(app)?;
            let elapsed = now.elapsed();
            info!("HNSW rebuild took {:?}", elapsed);
            info!("HNSW EF_CONSTRUCTION: {:?}", ann::DEFAULT_EF_CONSTRUCTION);
            info!("HNSW_MAX_ELEMS: {:?}", ann::DEFAULT_MAX_ELEMS);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::search_images,
            app::commands::fetch_thumbnails,
            app::commands::fetch_metadata,
            app::commands::add_watched_directory,
            app::commands::delete_watched_directory,
            ])
        .run(tauri::generate_context!())?;

    Ok(())
}
