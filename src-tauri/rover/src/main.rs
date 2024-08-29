#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::process::exit;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::Mutex;

use app::ann;
use app::ann::HnswSearch;
use app::clip::Clip;
use app::db;
use app::notify_handlers;
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
const LOG_LEVEL: LevelFilter = LevelFilter::Debug;
#[cfg(not(debug_assertions))]
const LOG_LEVEL: LevelFilter = LevelFilter::Warn;

const FS_WATCHER_DEBOUNCER_DURATION: std::time::Duration = std::time::Duration::from_millis(500);

fn main() -> anyhow::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_log::Builder::default().targets([
                LogTarget::LogDir,
                LogTarget::Stdout,
                LogTarget::Webview,
            ])
            // Note that Trace level causes massive slowdowns
            .level(LOG_LEVEL)
            .build())
        .manage(
            ConnectionPoolState(
                    Mutex::new(InnerConnectionPoolState { pool: db::get_connection_pool()? })
                )
        )
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

            // TODO Remove this, just doing for now... Will need to replace with our watched directories thing.
            tauri::scope::FsScope::allow_directory(&app.fs_scope(), "D:\\refrover_photos", true)?;

            

            // We rebuild every time the app launches; it is fast enough, and it handles the fact that
            // we can't remove elements from the HNSW index.
            info!("Populating HNSW index...");
            let now = std::time::Instant::now();
            ann::populate_hnsw(app)?;
            let elapsed = now.elapsed();
            info!("HNSW rebuild took {:?}", elapsed);
            info!("HNSW EF_CONSTRUCTION: {:?}", ann::DEFAULT_EF_CONSTRUCTION);
            info!("HNSW_MAX_ELEMS: {:?}", ann::DEFAULT_MAX_ELEMS);

            // TODO We need a table that stores the watched directories.
            //      We need to populate the front-end with them,
            //      and create watchers for them.
            //      Right now it's hardcoded to D:\refrover_photos that gets added on click.

            let fs_event_handler = notify_handlers::FsEventHandler {
                app_handle: app.handle().clone(),
            };
            let watcher = notify_debouncer_full::new_debouncer(FS_WATCHER_DEBOUNCER_DURATION, None, fs_event_handler);
            match watcher {
                Ok(watcher) => {
                    let watcher_state = FsWatcherState(Mutex::new(FsInnerWatcherState { watcher }));
                    app.manage(watcher_state);
                },
                Err(e) => {
                    error!("Error initializing FS watcher: {:?}", e);
                    return Err(anyhow::Error::new(e).into());
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::commands::search_images,
            app::commands::fetch_thumbnails,
            app::commands::fetch_metadata,
            app::commands::add_watched_directory,
            ])
        .run(tauri::generate_context!())?;

    Ok(())
}
